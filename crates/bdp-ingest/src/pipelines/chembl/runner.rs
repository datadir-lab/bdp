use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;

use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::chembl::{
    config::ChemblConfig,
    extractor::{extract_activities, parse_uniprot_mapping},
    mapper::{build_compound_map, build_target_map},
    storage::ChemblStorage,
};

pub struct ChemblPipelineRunner {
    pub config: ChemblConfig,
    pub pool: PgPool,
}

impl ChemblPipelineRunner {
    pub fn new(config: ChemblConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for ChemblPipelineRunner {
    fn name(&self) -> &'static str {
        "chembl"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        let chembl_to_uniprot: HashMap<String, String> =
            if let Some(ref path) = self.config.uniprot_mapping_path {
                let content = tokio::fs::read_to_string(path).await?;
                parse_uniprot_mapping(&content)
            } else {
                HashMap::new()
            };

        // IMPORTANT: rusqlite is synchronous — MUST use spawn_blocking
        let sqlite_path = self.config.sqlite_path.clone();
        let activities = tokio::task::spawn_blocking(move || -> Result<_> {
            let conn = rusqlite::Connection::open(&sqlite_path)?;
            extract_activities(&conn, None)
        })
        .await??;
        info!(count = activities.len(), "extracted ChEMBL activities");

        let inchikeys: Vec<String> = activities.iter().map(|a| a.inchikey.clone()).collect();
        let compound_map = build_compound_map(&self.pool, &inchikeys).await?;
        let target_map = build_target_map(&self.pool, &chembl_to_uniprot).await?;

        let insert_rows: Vec<_> = activities
            .iter()
            .filter_map(|a| {
                let compound_id = *compound_map.get(&a.inchikey)?;
                let target_id = *target_map.get(&a.target_chembl_id)?;
                Some((
                    compound_id,
                    target_id,
                    a.activity_type.clone(),
                    a.activity_value.map(|v| v as f32),
                    self.config.source_version.clone(),
                ))
            })
            .collect();

        let storage = ChemblStorage::new(self.pool.clone());
        let inserted = storage.insert_activities(&insert_rows).await?;
        stats.records_ingested = inserted as u64;
        stats.records_skipped = (insert_rows.len() - inserted) as u64;
        Ok(stats)
    }
}
