// crates/bdp-ingest/src/pipelines/mondo/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::mondo::{parser, storage::MondoStorage, MONDO_OBO_URL};

#[derive(Debug, Clone)]
pub struct MondoConfig {
    pub obo_url: String,
    pub release: String,
    pub max_retries: u32,
    pub parse_limit: Option<usize>,
    pub org_id: Uuid,
}

impl MondoConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            obo_url: MONDO_OBO_URL.to_string(),
            release: release.into(),
            max_retries: 3,
            parse_limit: None,
            org_id,
        }
    }
}

pub struct MondoPipelineRunner {
    config: MondoConfig,
    pool: PgPool,
}

impl MondoPipelineRunner {
    pub fn new(config: MondoConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for MondoPipelineRunner {
    fn name(&self) -> &'static str {
        "mondo"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        info!("downloading MONDO OBO (~50MB)");
        let content = download_text(&self.config.obo_url, self.config.max_retries).await?;

        info!(bytes = content.len(), "parsing MONDO OBO");
        let parsed = parser::parse_obo(&content, &self.config.release, self.config.parse_limit)
            .map_err(|e| anyhow::anyhow!("MONDO parse error: {}", e))?;

        stats.records_ingested = parsed.term_count() as u64;
        stats.records_skipped = parsed.obsolete_count() as u64;

        info!(
            terms = parsed.term_count(),
            rels = parsed.relationship_count(),
            obsolete = parsed.obsolete_count(),
            "MONDO OBO parsed"
        );

        let storage = MondoStorage::new(self.pool, self.config.org_id);
        storage.store_release(&self.config.release, &parsed).await?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = MondoConfig::new("2026-03-01", org_id);
        assert_eq!(cfg.obo_url, MONDO_OBO_URL);
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.release, "2026-03-01");
        assert!(cfg.parse_limit.is_none());
    }
}
