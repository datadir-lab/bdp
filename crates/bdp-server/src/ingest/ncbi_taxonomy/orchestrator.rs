//! NCBI Taxonomy ingestion orchestrator
//!
//! Coordinates historical catchup and sequential version ingestion.

use anyhow::Result;
use futures::stream::{self, StreamExt};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use super::config::NcbiTaxonomyFtpConfig;
use super::ftp::NcbiTaxonomyFtp;
use super::pipeline::{NcbiTaxonomyPipeline, PipelineResult};
use crate::storage::Storage;

/// Orchestrator for NCBI Taxonomy ingestion
///
/// Handles:
/// - Historical catchup (ingest from oldest to newest)
/// - Version range ingestion
/// - Filtering and skipping already-ingested versions
pub struct NcbiTaxonomyOrchestrator {
    config: NcbiTaxonomyFtpConfig,
    db: PgPool,
    s3: Option<Storage>,
}

impl NcbiTaxonomyOrchestrator {
    /// Minimum database connections required per concurrent pipeline
    /// Each pipeline uses approximately 5 connections for:
    /// - Transaction management (1)
    /// - Batch operations (2-3)
    /// - Version queries (1)
    const CONNECTIONS_PER_PIPELINE: u32 = 5;

    /// Create a new orchestrator
    pub fn new(config: NcbiTaxonomyFtpConfig, db: PgPool) -> Self {
        Self {
            config,
            db,
            s3: None,
        }
    }

    /// Create orchestrator with S3 support
    pub fn with_s3(config: NcbiTaxonomyFtpConfig, db: PgPool, s3: Storage) -> Self {
        Self {
            config,
            db,
            s3: Some(s3),
        }
    }

    /// Validate that database pool has sufficient connections for parallel processing
    ///
    /// # Arguments
    /// * `concurrency` - Number of concurrent pipelines
    ///
    /// # Returns
    /// Warning message if pool size may be insufficient, None if adequate
    fn validate_pool_size(&self, concurrency: usize) -> Option<String> {
        let pool_size = self.db.options().get_max_connections();
        let required = (concurrency as u32) * Self::CONNECTIONS_PER_PIPELINE;

        if pool_size < required {
            Some(format!(
                "Database pool size ({}) may be insufficient for concurrency level ({}). \
                 Recommended minimum: {} connections ({} pipelines × {} connections/pipeline). \
                 Consider increasing pool size or reducing concurrency to avoid connection exhaustion.",
                pool_size, concurrency, required, concurrency, Self::CONNECTIONS_PER_PIPELINE
            ))
        } else {
            None
        }
    }

    /// Catchup from a specific start date to current
    ///
    /// Lists all available archive versions, filters to those >= start_date,
    /// and ingests them sequentially from oldest to newest.
    ///
    /// # Arguments
    /// * `organization_id` - Organization to ingest under
    /// * `start_date` - Start date in format "YYYY-MM-DD" (e.g., "2024-01-01")
    ///   - `None` to ingest all available historical versions
    ///   - `Some("2024-01-01")` to start from specific date
    ///
    /// # Returns
    /// Vector of results for each ingested version
    pub async fn catchup_from_date(
        &self,
        organization_id: Uuid,
        start_date: Option<&str>,
    ) -> Result<Vec<PipelineResult>> {
        info!("Starting NCBI Taxonomy historical catchup");
        if let Some(date) = start_date {
            info!("Catchup start date: {}", date);
        } else {
            info!("Catchup from beginning (all available versions)");
        }

        // 1. List all available archive versions
        info!("Discovering available archive versions...");
        let ftp = NcbiTaxonomyFtp::new(self.config.clone());
        let mut all_versions = ftp.list_available_versions().await?;

        // 2. Filter to start_date if provided
        if let Some(date) = start_date {
            all_versions.retain(|v| v.as_str() >= date);
            info!(
                "Filtered to {} versions >= {}",
                all_versions.len(),
                date
            );
        }

        if all_versions.is_empty() {
            warn!("No versions found to ingest");
            return Ok(vec![]);
        }

        // all_versions is guaranteed non-empty due to the check above
        let first_version = all_versions.first().map(|s| s.as_str()).unwrap_or("unknown");
        let last_version = all_versions.last().map(|s| s.as_str()).unwrap_or("unknown");
        info!(
            "Found {} versions to ingest: {} to {}",
            all_versions.len(),
            first_version,
            last_version
        );

        // 3. Ingest each version sequentially (oldest to newest)
        let mut results = Vec::new();
        for (index, version) in all_versions.iter().enumerate() {
            info!(
                "Processing version {} / {}: {}",
                index + 1,
                all_versions.len(),
                version
            );

            let pipeline = if let Some(s3) = &self.s3 {
                NcbiTaxonomyPipeline::with_s3(self.config.clone(), self.db.clone(), s3.clone())
            } else {
                NcbiTaxonomyPipeline::new(self.config.clone(), self.db.clone())
            };

            match pipeline.run_version(organization_id, Some(version)).await {
                Ok(result) => {
                    if result.is_success() {
                        info!(
                            version = %version,
                            stored = result.storage_stats.as_ref().map(|s| s.stored).unwrap_or(0),
                            updated = result.storage_stats.as_ref().map(|s| s.updated).unwrap_or(0),
                            "✓ Version ingested successfully"
                        );
                    } else if result.skipped {
                        info!(
                            version = %version,
                            "✓ Version already ingested (skipped)"
                        );
                    }
                    results.push(result);
                }
                Err(e) => {
                    warn!(
                        version = %version,
                        error = %e,
                        "✗ Failed to ingest version (continuing with next version)"
                    );
                    // Continue with next version even if one fails
                }
            }
        }

        info!(
            "Historical catchup completed: {} versions processed, {} successful",
            all_versions.len(),
            results.len()
        );

        Ok(results)
    }

    /// Catchup and then keep up-to-date with current version
    ///
    /// 1. Catchup from start_date to latest historical archive
    /// 2. Ingest current version if not already ingested
    ///
    /// # Arguments
    /// * `organization_id` - Organization to ingest under
    /// * `start_date` - Start date for historical catchup (e.g., "2024-01-01")
    ///
    /// # Returns
    /// Vector of results (historical + current)
    pub async fn catchup_and_current(
        &self,
        organization_id: Uuid,
        start_date: Option<&str>,
    ) -> Result<Vec<PipelineResult>> {
        // 1. Catchup historical versions
        let mut results = self.catchup_from_date(organization_id, start_date).await?;

        // 2. Ingest current version
        info!("Checking for current version...");
        let pipeline = if let Some(s3) = &self.s3 {
            NcbiTaxonomyPipeline::with_s3(self.config.clone(), self.db.clone(), s3.clone())
        } else {
            NcbiTaxonomyPipeline::new(self.config.clone(), self.db.clone())
        };

        match pipeline.run(organization_id).await {
            Ok(result) => {
                if result.is_success() {
                    info!("✓ Current version ingested successfully");
                } else if result.skipped {
                    info!("✓ Current version already ingested (skipped)");
                }
                results.push(result);
            }
            Err(e) => {
                warn!(error = %e, "✗ Failed to ingest current version");
            }
        }

        Ok(results)
    }

    /// Catchup from a specific start date using parallel processing
    ///
    /// Processes multiple versions concurrently for faster catchup.
    /// Maintains version order in results but processes them in parallel.
    ///
    /// # Arguments
    /// * `organization_id` - Organization to ingest under
    /// * `start_date` - Start date for catchup (e.g., "2024-01-01")
    /// * `concurrency` - Number of versions to process concurrently (default: 2-4 recommended)
    ///
    /// # Returns
    /// Vector of results for each ingested version (in chronological order)
    ///
    /// # Performance
    /// With concurrency=4 and batch operations:
    /// - 86 versions × 5-10 min each = ~430-860 minutes sequentially
    /// - With 4x parallelism = ~110-215 minutes (1.8-3.6 hours)
    pub async fn catchup_from_date_parallel(
        &self,
        organization_id: Uuid,
        start_date: Option<&str>,
        concurrency: usize,
    ) -> Result<Vec<PipelineResult>> {
        let concurrency = if concurrency == 0 { 2 } else { concurrency };

        info!("Starting NCBI Taxonomy parallel catchup (concurrency: {})", concurrency);

        // Validate pool size for parallel processing
        if let Some(warning) = self.validate_pool_size(concurrency) {
            warn!("{}", warning);
        }
        if let Some(date) = start_date {
            info!("Catchup start date: {}", date);
        } else {
            info!("Catchup from beginning (all available versions)");
        }

        // 1. List all available archive versions
        info!("Discovering available archive versions...");
        let ftp = NcbiTaxonomyFtp::new(self.config.clone());
        let mut all_versions = ftp.list_available_versions().await?;

        // 2. Filter to start_date if provided
        if let Some(date) = start_date {
            all_versions.retain(|v| v.as_str() >= date);
            info!("Filtered to {} versions >= {}", all_versions.len(), date);
        }

        if all_versions.is_empty() {
            warn!("No versions found to ingest");
            return Ok(vec![]);
        }

        // all_versions is guaranteed non-empty due to the check above
        let first_version = all_versions.first().map(|s| s.as_str()).unwrap_or("unknown");
        let last_version = all_versions.last().map(|s| s.as_str()).unwrap_or("unknown");
        info!(
            "Found {} versions to ingest: {} to {}",
            all_versions.len(),
            first_version,
            last_version
        );

        // 3. Process versions in parallel with controlled concurrency
        info!("Processing {} versions with concurrency {}", all_versions.len(), concurrency);

        let results: Vec<Option<PipelineResult>> = stream::iter(all_versions.iter().enumerate())
            .map(|(index, version): (usize, &String)| {
                let organization_id = organization_id;
                let version = version.clone();
                let config = self.config.clone();
                let db = self.db.clone();
                let s3 = self.s3.clone();
                let total = all_versions.len();

                async move {
                    info!(
                        "Processing version {} / {}: {}",
                        index + 1,
                        total,
                        version
                    );

                    let pipeline = if let Some(s3) = s3 {
                        NcbiTaxonomyPipeline::with_s3(config, db, s3)
                    } else {
                        NcbiTaxonomyPipeline::new(config, db)
                    };

                    match pipeline.run_version(organization_id, Some(&version)).await {
                        Ok(result) => {
                            if result.is_success() {
                                info!(
                                    version = %version,
                                    stored = result.storage_stats.as_ref().map(|s| s.stored).unwrap_or(0),
                                    updated = result.storage_stats.as_ref().map(|s| s.updated).unwrap_or(0),
                                    "✓ Version ingested successfully"
                                );
                            } else if result.skipped {
                                info!(
                                    version = %version,
                                    "✓ Version already ingested (skipped)"
                                );
                            }
                            Some(result)
                        }
                        Err(e) => {
                            warn!(
                                version = %version,
                                error = %e,
                                "✗ Failed to ingest version"
                            );
                            None
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        let results: Vec<PipelineResult> = results.into_iter().flatten().collect();

        info!(
            "Parallel catchup completed: {} versions processed, {} successful",
            all_versions.len(),
            results.len()
        );

        Ok(results)
    }

    /// Catchup and keep current using parallel processing
    ///
    /// 1. Parallel catchup from start_date to latest historical archive
    /// 2. Ingest current version if not already ingested
    ///
    /// # Arguments
    /// * `organization_id` - Organization to ingest under
    /// * `start_date` - Start date for historical catchup
    /// * `concurrency` - Number of versions to process concurrently
    pub async fn catchup_and_current_parallel(
        &self,
        organization_id: Uuid,
        start_date: Option<&str>,
        concurrency: usize,
    ) -> Result<Vec<PipelineResult>> {
        // Validate pool size for parallel processing
        if let Some(warning) = self.validate_pool_size(concurrency) {
            warn!("{}", warning);
        }

        // 1. Parallel catchup historical versions
        let mut results = self.catchup_from_date_parallel(organization_id, start_date, concurrency).await?;

        // 2. Ingest current version
        info!("Checking for current version...");
        let pipeline = if let Some(s3) = &self.s3 {
            NcbiTaxonomyPipeline::with_s3(self.config.clone(), self.db.clone(), s3.clone())
        } else {
            NcbiTaxonomyPipeline::new(self.config.clone(), self.db.clone())
        };

        match pipeline.run(organization_id).await {
            Ok(result) => {
                if result.is_success() {
                    info!("✓ Current version ingested successfully");
                } else if result.skipped {
                    info!("✓ Current version already ingested (skipped)");
                }
                results.push(result);
            }
            Err(e) => {
                warn!(error = %e, "✗ Failed to ingest current version");
            }
        }

        Ok(results)
    }

    /// List available versions for ingestion
    ///
    /// Returns sorted list of available archive dates
    pub async fn list_available_versions(&self) -> Result<Vec<String>> {
        let ftp = NcbiTaxonomyFtp::new(self.config.clone());
        ftp.list_available_versions().await
    }

    /// Get summary of ingestion results
    pub fn summarize_results(results: &[PipelineResult]) -> String {
        let total = results.len();
        let successful = results.iter().filter(|r| r.is_success()).count();
        let skipped = results.iter().filter(|r| r.skipped).count();
        let failed = total - successful - skipped;

        let total_stored: usize = results
            .iter()
            .filter_map(|r| r.storage_stats.as_ref())
            .map(|s| s.stored)
            .sum();

        let total_updated: usize = results
            .iter()
            .filter_map(|r| r.storage_stats.as_ref())
            .map(|s| s.updated)
            .sum();

        format!(
            "Ingestion Summary:\n\
             - Total versions processed: {}\n\
             - Successful: {}\n\
             - Skipped (already ingested): {}\n\
             - Failed: {}\n\
             - Total taxa stored: {}\n\
             - Total taxa updated: {}",
            total, successful, skipped, failed, total_stored, total_updated
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_results_empty() {
        let results = vec![];
        let summary = NcbiTaxonomyOrchestrator::summarize_results(&results);
        assert!(summary.contains("Total versions processed: 0"));
    }

    #[test]
    fn test_summarize_results_with_data() {
        let results = vec![
            PipelineResult {
                external_version: Some("2024-01-01".to_string()),
                internal_version: Some("1.0".to_string()),
                storage_stats: Some(super::super::storage::StorageStats {
                    total: 100,
                    stored: 100,
                    updated: 0,
                    failed: 0,
                }),
                skipped: false,
            },
            PipelineResult {
                external_version: None,
                internal_version: None,
                storage_stats: None,
                skipped: true,
            },
        ];

        let summary = NcbiTaxonomyOrchestrator::summarize_results(&results);
        assert!(summary.contains("Total versions processed: 2"));
        assert!(summary.contains("Successful: 1"));
        assert!(summary.contains("Skipped (already ingested): 1"));
        assert!(summary.contains("Total taxa stored: 100"));
    }
}
