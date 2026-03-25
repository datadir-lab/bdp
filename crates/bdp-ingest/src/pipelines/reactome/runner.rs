// crates/bdp-ingest/src/pipelines/reactome/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::reactome::{
    parser, storage::ReactomeStorage, REACTOME_PATHWAYS_URL, REACTOME_UNIPROT_URL,
};

#[derive(Debug, Clone)]
pub struct ReactomeConfig {
    pub pathways_url: String,
    pub uniprot_url: String,
    pub max_retries: u32,
    pub release: String,
    pub org_id: Uuid,
    /// Filter to specific species (e.g. "Homo sapiens"). None = all species.
    pub species_filter: Option<String>,
}

impl ReactomeConfig {
    pub fn human_only(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            pathways_url: REACTOME_PATHWAYS_URL.to_string(),
            uniprot_url: REACTOME_UNIPROT_URL.to_string(),
            max_retries: 3,
            release: release.into(),
            org_id,
            species_filter: Some("Homo sapiens".to_string()),
        }
    }

    pub fn all_species(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            species_filter: None,
            ..Self::human_only(release, org_id)
        }
    }
}

pub struct ReactomePipelineRunner {
    config: ReactomeConfig,
    pool: PgPool,
}

impl ReactomePipelineRunner {
    pub fn new(config: ReactomeConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for ReactomePipelineRunner {
    fn name(&self) -> &'static str {
        "reactome"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        // 1. Pathways
        info!("downloading ReactomePathways.txt");
        let pathways_content =
            download_text(&self.config.pathways_url, self.config.max_retries).await?;
        let pathways = parser::parse_pathways(&pathways_content, &self.config.release)?;

        // 2. UniProt->Reactome mappings
        info!("downloading UniProt2Reactome.txt");
        let uniprot_content =
            download_text(&self.config.uniprot_url, self.config.max_retries).await?;
        let links = parser::parse_uniprot_reactome(
            &uniprot_content,
            &self.config.release,
            self.config.species_filter.as_deref(),
        )?;

        stats.records_ingested = (pathways.len() + links.len()) as u64;

        info!(pathways = pathways.len(), links = links.len(), "Reactome parsed");

        let storage = ReactomeStorage::new(self.pool);
        storage
            .ingest_release(self.config.org_id, &self.config.release, &pathways, &links)
            .await?;

        Ok(stats)
    }
}
