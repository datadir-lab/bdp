//! NCBI Taxonomy ingestion pipeline
//!
//! Orchestrates the full ingestion process from FTP download to database storage.

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use super::config::NcbiTaxonomyFtpConfig;
use super::ftp::NcbiTaxonomyFtp;
use super::parser::TaxdumpParser;
use super::storage::{NcbiTaxonomyStorage, StorageStats};
use super::version_discovery::TaxonomyVersionDiscovery;
use crate::storage::Storage;

/// NCBI Taxonomy ingestion pipeline
pub struct NcbiTaxonomyPipeline {
    config: NcbiTaxonomyFtpConfig,
    db: PgPool,
    s3: Option<Storage>,
}

impl NcbiTaxonomyPipeline {
    /// Create a new pipeline
    pub fn new(config: NcbiTaxonomyFtpConfig, db: PgPool) -> Self {
        Self {
            config,
            db,
            s3: None,
        }
    }

    /// Create pipeline with S3 support
    pub fn with_s3(config: NcbiTaxonomyFtpConfig, db: PgPool, s3: Storage) -> Self {
        Self {
            config,
            db,
            s3: Some(s3),
        }
    }

    /// Run the full ingestion pipeline (current version)
    ///
    /// Steps:
    /// 1. Version discovery - check if new version available
    /// 2. Download taxdump from FTP
    /// 3. Parse taxdump files
    /// 4. Determine internal version
    /// 5. Store to database
    /// 6. Record version mapping
    ///
    /// Returns: Statistics about what was stored
    pub async fn run(&self, organization_id: Uuid) -> Result<PipelineResult> {
        self.run_version(organization_id, None).await
    }

    /// Run the full ingestion pipeline for a specific version
    ///
    /// # Arguments
    /// * `organization_id` - Organization to ingest under
    /// * `version` - Optional specific version to ingest
    ///   - `None` for current version (with version discovery)
    ///   - `Some("2024-01-01")` for historical version (skips version discovery)
    ///
    /// Returns: Statistics about what was stored
    pub async fn run_version(
        &self,
        organization_id: Uuid,
        version: Option<&str>,
    ) -> Result<PipelineResult> {
        if let Some(ver) = version {
            info!("Starting NCBI Taxonomy ingestion pipeline for version {}", ver);
        } else {
            info!("Starting NCBI Taxonomy ingestion pipeline (current version)");
        }

        let external_version = if let Some(ver) = version {
            // Historical version - skip version discovery, use provided version
            info!("Using historical version: {}", ver);
            ver.to_string()
        } else {
            // Current version - do version discovery
            info!("Phase 1: Version Discovery");
            let version_discovery =
                TaxonomyVersionDiscovery::new(self.config.clone(), self.db.clone());

            let discovered_version = match version_discovery.discover_current_version().await? {
                Some(v) => v,
                None => {
                    info!("No new version to ingest");
                    return Ok(PipelineResult {
                        external_version: None,
                        internal_version: None,
                        storage_stats: None,
                        skipped: true,
                    });
                },
            };

            info!(
                external_version = %discovered_version.external_version,
                "New version discovered"
            );

            discovered_version.external_version
        };

        // 2. Download taxdump
        info!("Phase 2: Downloading taxdump from FTP");
        let ftp = NcbiTaxonomyFtp::new(self.config.clone());
        let taxdump_files = ftp
            .download_taxdump_version(version)
            .await
            .context("Failed to download taxdump")?;

        info!(
            "Downloaded taxdump files ({} bytes rankedlineage, {} bytes merged, {} bytes delnodes)",
            taxdump_files.rankedlineage.len(),
            taxdump_files.merged.len(),
            taxdump_files.delnodes.len()
        );

        // 3. Parse taxdump
        info!("Phase 3: Parsing taxdump files");
        let parser = if let Some(limit) = self.config.parse_limit {
            warn!(limit = limit, "Parse limit is set, will only process {} entries", limit);
            TaxdumpParser::with_limit(limit)
        } else {
            TaxdumpParser::new()
        };

        let taxdump_data = parser
            .parse(
                &taxdump_files.rankedlineage,
                &taxdump_files.merged,
                &taxdump_files.delnodes,
                external_version.clone(),
            )
            .context("Failed to parse taxdump")?;

        info!(
            "Parsed {} taxonomy entries, {} merged, {} deleted",
            taxdump_data.entries.len(),
            taxdump_data.merged.len(),
            taxdump_data.deleted.len()
        );

        // 4. Determine internal version
        info!("Phase 4: Determining internal version");
        let version_discovery = TaxonomyVersionDiscovery::new(self.config.clone(), self.db.clone());
        let has_major_changes = !taxdump_data.merged.is_empty() || !taxdump_data.deleted.is_empty();
        let internal_version = version_discovery
            .determine_next_version(has_major_changes)
            .await
            .context("Failed to determine next version")?;

        info!(
            internal_version = %internal_version,
            has_major_changes = has_major_changes,
            "Determined internal version"
        );

        // 5. Store to database
        info!("Phase 5: Storing to database");
        let storage = if let Some(s3) = &self.s3 {
            NcbiTaxonomyStorage::with_s3(
                self.db.clone(),
                s3.clone(),
                organization_id,
                internal_version.clone(),
                external_version.clone(),
            )
        } else {
            NcbiTaxonomyStorage::new(
                self.db.clone(),
                organization_id,
                internal_version.clone(),
                external_version.clone(),
            )
        };

        // Set up citation policy (idempotent)
        storage
            .setup_citations()
            .await
            .context("Failed to setup citation policy")?;

        let storage_stats = storage
            .store(&taxdump_data)
            .await
            .context("Failed to store taxdump to database")?;

        info!(
            stored = storage_stats.stored,
            updated = storage_stats.updated,
            failed = storage_stats.failed,
            "Storage completed"
        );

        // 6. Record version mapping
        info!("Phase 6: Recording version mapping");
        version_discovery
            .record_version_mapping(&external_version, &internal_version)
            .await
            .context("Failed to record version mapping")?;

        info!(
            external_version = %external_version,
            internal_version = %internal_version,
            stored = storage_stats.stored,
            updated = storage_stats.updated,
            "NCBI Taxonomy ingestion completed successfully"
        );

        Ok(PipelineResult {
            external_version: Some(external_version),
            internal_version: Some(internal_version),
            storage_stats: Some(storage_stats),
            skipped: false,
        })
    }

    /// Check if a new version is available without running the full pipeline
    pub async fn check_new_version(&self) -> Result<bool> {
        let version_discovery = TaxonomyVersionDiscovery::new(self.config.clone(), self.db.clone());
        let discovered = version_discovery.discover_current_version().await?;
        Ok(discovered.is_some())
    }
}

/// Result of running the pipeline
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// External version that was ingested (None if skipped)
    pub external_version: Option<String>,
    /// Internal version that was assigned (None if skipped)
    pub internal_version: Option<String>,
    /// Storage statistics (None if skipped)
    pub storage_stats: Option<StorageStats>,
    /// Whether ingestion was skipped (version already exists)
    pub skipped: bool,
}

impl PipelineResult {
    /// Check if the pipeline completed successfully with new data
    pub fn is_success(&self) -> bool {
        !self.skipped && self.storage_stats.is_some()
    }

    /// Get a summary message
    pub fn summary(&self) -> String {
        if self.skipped {
            "Ingestion skipped - no new version available".to_string()
        } else if let (Some(ext), Some(int), Some(stats)) =
            (&self.external_version, &self.internal_version, &self.storage_stats)
        {
            format!(
                "Successfully ingested {} → {} ({} stored, {} updated, {} failed)",
                ext, int, stats.stored, stats.updated, stats.failed
            )
        } else {
            "Pipeline completed with unknown status".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_result_summary() {
        // Skipped result
        let result = PipelineResult {
            external_version: None,
            internal_version: None,
            storage_stats: None,
            skipped: true,
        };
        assert_eq!(result.summary(), "Ingestion skipped - no new version available");
        assert!(!result.is_success());

        // Successful result
        let result = PipelineResult {
            external_version: Some("2026-01-15".to_string()),
            internal_version: Some("1.0".to_string()),
            storage_stats: Some(StorageStats {
                total: 100,
                stored: 95,
                updated: 5,
                failed: 0,
            }),
            skipped: false,
        };
        assert!(result
            .summary()
            .contains("Successfully ingested 2026-01-15 → 1.0"));
        assert!(result.is_success());
    }
}
