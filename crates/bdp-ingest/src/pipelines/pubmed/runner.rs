// crates/bdp-ingest/src/pipelines/pubmed/runner.rs
// Implemented in Task 10 — full storage + entity_linker + runner

use crate::pipelines::pubmed::config::PubmedConfig;
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
