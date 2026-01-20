// GenBank/RefSeq ingestion orchestrator
//
// Coordinates parallel processing of multiple divisions for maximum throughput.
// Uses tokio streams with buffer_unordered for concurrent division processing.
//
// Performance: With concurrency=4, processes 4 divisions simultaneously
// Expected speedup: 3-4x compared to sequential processing

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::config::GenbankFtpConfig;
use super::ftp::GenbankFtp;
use super::models::{Division, OrchestratorResult, PipelineResult};
use super::pipeline::GenbankPipeline;
use crate::storage::Storage;

pub struct GenbankOrchestrator {
    config: GenbankFtpConfig,
    db: PgPool,
    s3: Storage,
}

impl GenbankOrchestrator {
    /// Create a new orchestrator
    pub fn new(config: GenbankFtpConfig, db: PgPool, s3: Storage) -> Self {
        Self { config, db, s3 }
    }

    /// Run ingestion for entire GenBank release (all divisions in parallel)
    pub async fn run_release(&self, organization_id: Uuid) -> Result<OrchestratorResult> {
        let start_time = Instant::now();

        info!("Starting GenBank orchestrator for full release");

        // Step 1: Get current release number
        let ftp = GenbankFtp::new(self.config.clone());
        let release = ftp
            .get_current_release()
            .await
            .context("Failed to get current release")?;

        info!("Processing GenBank release: {}", release);

        // Step 2: Get divisions to process
        let divisions = if self.config.parse_limit.is_some() {
            // For testing, use only phage division
            vec![GenbankFtpConfig::get_test_division()]
        } else {
            // For production, use all primary divisions
            GenbankFtpConfig::get_primary_divisions()
        };

        info!(
            "Processing {} divisions with concurrency={}",
            divisions.len(),
            self.config.concurrency
        );

        // Step 3: Process divisions in parallel
        let results = self
            .run_divisions_parallel(organization_id, &divisions, &release)
            .await?;

        // Step 4: Aggregate results
        let total_records: usize = results.iter().map(|r| r.records_processed).sum();
        let total_sequences: usize = results.iter().map(|r| r.sequences_inserted).sum();
        let total_mappings: usize = results.iter().map(|r| r.mappings_created).sum();
        let total_bytes: u64 = results.iter().map(|r| r.bytes_uploaded).sum();

        let duration = start_time.elapsed();

        info!(
            "Orchestrator complete: {} divisions, {} records, {} sequences, {} mappings, {} bytes in {:.2}s",
            results.len(),
            total_records,
            total_sequences,
            total_mappings,
            total_bytes,
            duration.as_secs_f64()
        );

        Ok(OrchestratorResult {
            release,
            divisions_processed: results.len(),
            total_records,
            total_sequences,
            total_mappings,
            total_bytes,
            duration_seconds: duration.as_secs_f64(),
            division_results: results,
        })
    }

    /// Run ingestion for specific divisions
    pub async fn run_divisions(
        &self,
        organization_id: Uuid,
        divisions: &[Division],
        release: Option<String>,
    ) -> Result<OrchestratorResult> {
        let start_time = Instant::now();

        // Get release number
        let release = if let Some(r) = release {
            r
        } else {
            let ftp = GenbankFtp::new(self.config.clone());
            ftp.get_current_release().await?
        };

        info!(
            "Processing {} divisions for release {}",
            divisions.len(),
            release
        );

        // Process divisions in parallel
        let results = self
            .run_divisions_parallel(organization_id, divisions, &release)
            .await?;

        // Aggregate results
        let total_records: usize = results.iter().map(|r| r.records_processed).sum();
        let total_sequences: usize = results.iter().map(|r| r.sequences_inserted).sum();
        let total_mappings: usize = results.iter().map(|r| r.mappings_created).sum();
        let total_bytes: u64 = results.iter().map(|r| r.bytes_uploaded).sum();

        let duration = start_time.elapsed();

        Ok(OrchestratorResult {
            release,
            divisions_processed: results.len(),
            total_records,
            total_sequences,
            total_mappings,
            total_bytes,
            duration_seconds: duration.as_secs_f64(),
            division_results: results,
        })
    }

    /// Process divisions in parallel using buffer_unordered
    async fn run_divisions_parallel(
        &self,
        organization_id: Uuid,
        divisions: &[Division],
        release: &str,
    ) -> Result<Vec<PipelineResult>> {
        let concurrency = self.config.concurrency;

        info!(
            "Processing {} divisions in parallel (concurrency={})",
            divisions.len(),
            concurrency
        );

        // Create pipeline for each division
        let results: Vec<Option<PipelineResult>> = stream::iter(divisions.iter().enumerate())
            .map(|(index, division): (usize, &Division)| {
                let pipeline = GenbankPipeline::new(
                    self.config.clone(),
                    self.db.clone(),
                    self.s3.clone(),
                );
                let division = division.clone();
                let release = release.to_string();
                let org_id = organization_id;

                async move {
                    let div_name = division.as_str();

                    info!(
                        "Starting division {} ({} / {})",
                        div_name,
                        index + 1,
                        divisions.len()
                    );

                    match pipeline.run_division(org_id, division.clone(), &release).await {
                        Ok(result) => {
                            info!(
                                "Completed division {} ({} / {}): {} records in {:.2}s",
                                result.division,
                                index + 1,
                                divisions.len(),
                                result.records_processed,
                                result.duration_seconds
                            );
                            Some(result)
                        }
                        Err(e) => {
                            error!(
                                "Failed division {} ({} / {}): {}",
                                div_name,
                                index + 1,
                                divisions.len(),
                                e
                            );
                            None
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        // Filter out failures
        let successful: Vec<PipelineResult> = results.into_iter().flatten().collect();

        info!(
            "Parallel processing complete: {} / {} divisions successful",
            successful.len(),
            divisions.len()
        );

        if successful.is_empty() {
            return Err(anyhow::anyhow!("All divisions failed"));
        }

        Ok(successful)
    }

    /// Run ingestion for a single division (convenience method)
    pub async fn run_single_division(
        &self,
        organization_id: Uuid,
        division: Division,
    ) -> Result<PipelineResult> {
        info!("Running single division: {}", division.as_str());

        // Get current release
        let ftp = GenbankFtp::new(self.config.clone());
        let release = ftp.get_current_release().await?;

        // Run pipeline
        let pipeline = GenbankPipeline::new(self.config.clone(), self.db.clone(), self.s3.clone());
        pipeline.run_division(organization_id, division, &release).await
    }

    /// Run ingestion for test division (phage - smallest)
    pub async fn run_test(&self, organization_id: Uuid) -> Result<PipelineResult> {
        info!("Running test ingestion (phage division)");

        let test_division = GenbankFtpConfig::get_test_division();
        self.run_single_division(organization_id, test_division).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_division() {
        let div = GenbankFtpConfig::get_test_division();
        assert_eq!(div, Division::Phage);
    }

    #[test]
    fn test_primary_divisions() {
        let divs = GenbankFtpConfig::get_primary_divisions();
        assert!(divs.len() > 0);
        assert!(divs.contains(&Division::Viral));
        assert!(divs.contains(&Division::Bacterial));
    }
}
