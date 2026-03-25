// crates/bdp-ingest/src/pipelines/string_db/runner.rs

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use sqlx::PgPool;
use std::collections::HashMap;
use std::io::Read;
use tracing::{info, warn};

use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::string_db::{
    config::StringConfig,
    parser::{parse_alias_row, parse_links_row, should_keep},
    storage::StringStorage,
};

pub struct StringPipelineRunner {
    pub config: StringConfig,
    pub pool: PgPool,
}

impl StringPipelineRunner {
    pub fn new(config: StringConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for StringPipelineRunner {
    fn name(&self) -> &'static str {
        "string_db"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let client = Client::new();
        let storage = StringStorage::new(self.pool.clone());

        info!("downloading STRING aliases (~30MB)");
        let alias_content =
            download_gz(&client, &self.config.aliases_url, self.config.max_retries).await?;
        let ensp_to_uniprot: HashMap<String, String> = alias_content
            .lines()
            .filter_map(parse_alias_row)
            .collect();
        info!(entries = ensp_to_uniprot.len(), "alias map built");

        let protein_map = storage.build_protein_map(&ensp_to_uniprot).await?;
        info!(resolved = protein_map.len(), "ENSP→UUID resolved");

        info!("downloading STRING links (~130MB)");
        let links_content =
            download_gz(&client, &self.config.links_url, self.config.max_retries).await?;
        let min_score = self.config.min_combined_score;

        let mut insert_rows = Vec::new();
        for line in links_content.lines().skip(1) {
            let row = match parse_links_row(line) {
                Ok(r) => r,
                Err(_) => {
                    stats.records_failed += 1;
                    continue;
                }
            };
            if row.combined_score < 0 {
                warn!(score = row.combined_score, "STRING row has negative combined_score, skipping");
                continue;
            }
            if (row.combined_score as u16) < min_score {
                continue;
            }
            if !should_keep(&row.protein1, &row.protein2) {
                continue;
            }
            let Some(&a_id) = protein_map.get(&row.protein1) else {
                continue;
            };
            let Some(&b_id) = protein_map.get(&row.protein2) else {
                continue;
            };
            insert_rows.push((
                a_id,
                b_id,
                row.score_neighborhood,
                row.score_fusion,
                row.score_cooccurrence,
                row.score_coexpression,
                row.score_experimental,
                row.score_database,
                row.score_textmining,
                row.combined_score,
            ));
        }
        info!(rows = insert_rows.len(), "inserting STRING interactions");
        let inserted = storage.insert_interactions(&insert_rows).await?;
        stats.records_ingested = inserted as u64;
        Ok(stats)
    }
}

async fn download_gz(client: &Client, url: &str, max_retries: u32) -> Result<String> {
    let mut last_err = anyhow::anyhow!("no attempts");
    for attempt in 0..=max_retries {
        match client.get(url).send().await {
            Ok(resp) => {
                let resp = resp.error_for_status().context("STRING download error")?;
                let bytes = resp.bytes().await?;
                let mut gz = GzDecoder::new(bytes.as_ref());
                let mut s = String::new();
                gz.read_to_string(&mut s)?;
                return Ok(s);
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
