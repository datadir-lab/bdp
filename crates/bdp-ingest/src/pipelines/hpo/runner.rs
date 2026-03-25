// crates/bdp-ingest/src/pipelines/hpo/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::hpo::{parser, storage::HpoStorage, HPO_HPOA_URL, HPO_OBO_URL};

#[derive(Debug, Clone)]
pub struct HpoConfig {
    pub obo_url: String,
    pub hpoa_url: String,
    pub release: String,
    pub max_retries: u32,
    pub parse_limit: Option<usize>,
    pub org_id: Uuid,
}

impl HpoConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            obo_url: HPO_OBO_URL.to_string(),
            hpoa_url: HPO_HPOA_URL.to_string(),
            release: release.into(),
            max_retries: 3,
            parse_limit: None,
            org_id,
        }
    }
}

pub struct HpoPipelineRunner {
    config: HpoConfig,
    pool: PgPool,
}

impl HpoPipelineRunner {
    pub fn new(config: HpoConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for HpoPipelineRunner {
    fn name(&self) -> &'static str {
        "hpo"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        // 1. Download + parse OBO terms
        info!("downloading HPO OBO (~7MB)");
        let obo_content = download_text(&self.config.obo_url, self.config.max_retries).await?;

        info!(bytes = obo_content.len(), "parsing HPO OBO");
        let parsed = parser::HpoParser::parse_obo(
            &obo_content,
            &self.config.release,
            self.config.parse_limit,
        )?;

        info!(terms = parsed.terms.len(), rels = parsed.relationships.len(), "HPO OBO parsed");

        let storage = HpoStorage::new(self.pool.clone(), self.config.org_id);
        storage
            .store_ontology(&parsed.terms, &parsed.relationships, &self.config.release, "1.0")
            .await?;

        stats.records_ingested = parsed.terms.len() as u64;

        // 2. Download + parse HPOA annotations
        info!("downloading HPO annotations (~8MB)");
        let hpoa_content = download_text(&self.config.hpoa_url, self.config.max_retries).await?;

        info!(bytes = hpoa_content.len(), "parsing HPOA TSV");
        let annotations = parser::HpoParser::parse_hpoa(
            &hpoa_content,
            &self.config.release,
            self.config.parse_limit,
        )?;

        info!(annotations = annotations.len(), "HPOA parsed");

        let stored = storage
            .store_annotations(&annotations, &self.config.release)
            .await?;
        stats.records_ingested += stored as u64;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = HpoConfig::new("2026-03-01", org_id);
        assert_eq!(cfg.obo_url, HPO_OBO_URL);
        assert_eq!(cfg.hpoa_url, HPO_HPOA_URL);
        assert_eq!(cfg.max_retries, 3);
        assert!(cfg.parse_limit.is_none());
    }
}
