// crates/bdp-ingest/src/pipelines/pubmed/runner.rs

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use std::io::Read;
use tracing::{info, warn};

use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::pubmed::{
    config::PubmedConfig,
    manifest::get_pending_files,
    parser::parse_pubmed_xml,
    storage::PubmedStorage,
};
use sqlx::PgPool;

pub struct PubmedPipelineRunner {
    pub config: PubmedConfig,
    pub pool: PgPool,
}

impl PubmedPipelineRunner {
    pub fn new(config: PubmedConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for PubmedPipelineRunner {
    fn name(&self) -> &'static str {
        "pubmed"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let storage = PubmedStorage::new(self.pool.clone(), self.config.org_id);
        let client = Client::new();

        // Get pending files
        let pending = get_pending_files(&self.pool).await?;
        info!(files = pending.len(), "PubMed files pending ingestion");

        let limit = self.config.parse_limit.unwrap_or(usize::MAX);
        for (file_id, url) in pending.into_iter().take(limit) {
            info!(%url, "processing PubMed file");
            match process_file(&client, &url, &storage, self.config.max_retries).await {
                Ok(count) => {
                    stats.records_ingested += count as u64;
                    if let Err(e) = storage.mark_file_done(file_id).await {
                        warn!("failed to mark file {} done: {}", file_id, e);
                    }
                }
                Err(e) => {
                    warn!(%url, "PubMed file failed: {}", e);
                    stats.records_failed += 1;
                    if let Err(e2) = storage.mark_file_error(file_id, &e.to_string()).await {
                        warn!("failed to mark file {} error: {}", file_id, e2);
                    }
                }
            }
        }

        Ok(stats)
    }
}

async fn process_file(
    client: &Client,
    url: &str,
    storage: &PubmedStorage,
    max_retries: u32,
) -> Result<usize> {
    let bytes = download_bytes(client, url, max_retries).await?;
    let mut gz = GzDecoder::new(bytes.as_ref());
    let mut xml_content = Vec::new();
    gz.read_to_end(&mut xml_content)
        .context("decompressing PubMed gzip")?;
    let articles = parse_pubmed_xml(&xml_content)?;
    let inserted = storage.insert_publications_batch(&articles).await?;
    Ok(inserted)
}

async fn download_bytes(client: &Client, url: &str, max_retries: u32) -> Result<bytes::Bytes> {
    let mut last_err = anyhow::anyhow!("no attempts made");
    for attempt in 0..=max_retries {
        match client.get(url).send().await {
            Ok(resp) => {
                let resp = resp
                    .error_for_status()
                    .context("PubMed download HTTP error")?;
                return Ok(resp.bytes().await.context("reading PubMed response bytes")?);
            }
            Err(e) => {
                last_err = e.into();
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
        }
    }
    Err(last_err)
}
