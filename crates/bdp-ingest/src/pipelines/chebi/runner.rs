// crates/bdp-ingest/src/pipelines/chebi/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::chebi::{parser, storage::ChebiStorage, CHEBI_OBO_URL};

#[derive(Debug, Clone)]
pub struct ChebiConfig {
    pub obo_url: String,
    pub max_retries: u32,
    pub release: String,
    pub org_id: Uuid,
    pub parse_limit: Option<usize>,
}

impl ChebiConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            obo_url: CHEBI_OBO_URL.to_string(),
            max_retries: 3,
            release: release.into(),
            org_id,
            parse_limit: None,
        }
    }
}

pub struct ChebiPipelineRunner {
    config: ChebiConfig,
    pool: PgPool,
}

impl ChebiPipelineRunner {
    pub fn new(config: ChebiConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for ChebiPipelineRunner {
    fn name(&self) -> &'static str {
        "chebi"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        info!("downloading ChEBI OBO (~280MB)");
        let content = download_text(&self.config.obo_url, self.config.max_retries).await?;

        info!(bytes = content.len(), "parsing ChEBI OBO");
        let parsed =
            parser::parse_obo(&content, &self.config.release, self.config.parse_limit)?;

        stats.records_ingested = parsed.terms.len() as u64;
        stats.records_skipped = parsed
            .terms
            .iter()
            .filter(|t| t.is_obsolete)
            .count() as u64;

        info!(
            terms = parsed.terms.len(),
            rels = parsed.relationships.len(),
            "ChEBI parsed"
        );

        let storage = ChebiStorage::new(self.pool);
        storage
            .ingest_release(self.config.org_id, &self.config.release, &parsed)
            .await?;

        Ok(stats)
    }
}
