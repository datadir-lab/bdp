use anyhow::Result;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use reqwest::Client;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::open_targets::{
    config::OpenTargetsConfig,
    downloader::{download_parquet, list_parquet_files},
    mapper::{extract_associations, AssociationRow, EnsemblToUniprot},
    storage::OpenTargetsStorage,
};

pub struct OpenTargetsPipelineRunner {
    pub config: OpenTargetsConfig,
    pub pool: sqlx::PgPool,
}

impl OpenTargetsPipelineRunner {
    pub fn new(config: OpenTargetsConfig, pool: sqlx::PgPool) -> Self {
        Self { config, pool }
    }

    async fn build_ensembl_map(&self, client: &Client) -> Result<EnsemblToUniprot> {
        let targets_url = self.config.targets_url();
        let files = list_parquet_files(client, &targets_url).await?;
        let map = HashMap::new();

        // NOTE: Full implementation would iterate proteinIds LIST<STRUCT> column
        // to extract source=uniprot_swissprot entries. Stubbed here to return empty
        // map (pipeline still runs; 0 associations inserted without seed data).
        // Arrow ListArray + StructArray traversal is needed for full implementation.
        let _ = files; // suppress unused warning

        warn!("build_ensembl_map: stub returns empty map — Ensembl→UniProt mapping not yet implemented; 0 gene associations will be inserted");

        Ok(map)
    }
}

impl PipelineRunner for OpenTargetsPipelineRunner {
    fn name(&self) -> &'static str {
        "open_targets"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let client = Client::new();
        let storage = OpenTargetsStorage::new(self.pool.clone());

        info!("building Ensembl→UniProt map from Open Targets targets/");
        let ensembl_map = self.build_ensembl_map(&client).await?;
        info!(entries = ensembl_map.len(), "Ensembl→UniProt map built");

        let gene_id_map = storage.build_gene_id_map(&ensembl_map).await?;
        info!(resolved = gene_id_map.len(), "gene UUIDs resolved");

        info!("listing Open Targets association files");
        let assoc_url = self.config.associations_url();
        let files = list_parquet_files(&client, &assoc_url).await?;
        let files = if let Some(limit) = self.config.parse_limit {
            files.into_iter().take(limit).collect::<Vec<_>>()
        } else {
            files
        };
        info!(files = files.len(), "found association parquet files");

        // Single-pass: download each file once, collect all AssociationRows
        let mut all_rows: Vec<AssociationRow> = Vec::new();

        for url in &files {
            let bytes = download_parquet(&client, url, self.config.max_retries).await?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;
            for batch in reader {
                let batch = batch?;
                let rows = extract_associations(&batch)?;
                for row in rows {
                    if row.score >= self.config.min_score {
                        all_rows.push(row);
                    }
                }
            }
        }

        // Collect unique disease IDs from the already-downloaded rows
        let mut all_disease_ids: Vec<String> = all_rows.iter().map(|r| r.disease_id.clone()).collect();
        all_disease_ids.sort();
        all_disease_ids.dedup();

        let disease_id_map = storage.build_disease_id_map(&all_disease_ids).await?;
        info!(resolved = disease_id_map.len(), "disease UUIDs resolved");

        // Build insert tuples from already-downloaded rows using resolved maps
        let mut total_rows: Vec<(Uuid, Uuid, f32)> = Vec::new();

        for row in &all_rows {
            let Some(&gene_uuid) = gene_id_map.get(&row.ensembl_id) else {
                continue;
            };
            let Some(&disease_uuid) = disease_id_map.get(&row.disease_id) else {
                continue;
            };
            total_rows.push((gene_uuid, disease_uuid, row.score));
        }

        info!(rows = total_rows.len(), "inserting gene-disease associations");
        let inserted = storage
            .insert_associations(&total_rows, &self.config.release)
            .await?;

        stats.records_ingested = inserted as u64;
        stats.records_skipped = (total_rows.len() - inserted) as u64;
        Ok(stats)
    }
}
