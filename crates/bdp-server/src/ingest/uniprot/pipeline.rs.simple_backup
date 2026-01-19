//! UniProt data ingestion pipeline
//!
//! Core pipeline for downloading, parsing, and storing UniProt data.

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::PgPool;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

use super::{DatParser, ReleaseInfo, UniProtFtp, UniProtFtpConfig, UniProtStorage, VersionDiscovery};
use crate::ingest::config::{HistoricalConfig, IngestionMode, LatestConfig, UniProtConfig};
use crate::ingest::jobs::IngestStats;
use crate::storage::Storage;

/// UniProt ingestion pipeline
///
/// Handles complete workflow:
/// 1. Download release notes to get actual version
/// 2. Check if version already exists in database
/// 3. Download DAT file if new version
/// 4. Parse and store proteins
/// 5. Create aggregate source
pub struct UniProtPipeline {
    db: PgPool,
    s3: Option<Storage>,
    organization_id: Uuid,
    config: UniProtFtpConfig,
}

impl UniProtPipeline {
    /// Create a new pipeline for an organization
    pub fn new(db: PgPool, organization_id: Uuid, config: UniProtFtpConfig) -> Self {
        Self {
            db,
            s3: None,
            organization_id,
            config,
        }
    }

    /// Create pipeline with S3 support
    pub fn with_s3(
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
        config: UniProtFtpConfig,
    ) -> Self {
        Self {
            db,
            s3: Some(s3),
            organization_id,
            config,
        }
    }

    /// Run the full ingestion pipeline
    ///
    /// Downloads release notes, extracts actual version, checks if already exists,
    /// and downloads/stores data only if it's a new version.
    pub async fn run(&self, version: Option<&str>) -> Result<IngestStats> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        info!("Starting UniProt ingestion pipeline");

        let ftp = UniProtFtp::new(self.config.clone());

        // 1. Download release notes to get actual version
        info!("Downloading release notes to extract version...");
        let notes = ftp
            .download_release_notes(version)
            .await
            .context("Failed to download release notes")?;

        let release_info = ftp
            .parse_release_notes(&notes)
            .context("Failed to parse release notes")?;

        let actual_version = &release_info.external_version;
        info!("Detected UniProt version: {}", actual_version);

        // 2. Check if this version already exists
        if self.version_exists(actual_version).await? {
            info!("Version {} already exists in database, skipping download", actual_version);
            let duration = start_time.elapsed();
            return Ok(IngestStats {
                total_entries: 0,
                entries_inserted: 0,
                entries_updated: 0,
                entries_skipped: 0,
                entries_failed: 0,
                bytes_processed: 0,
                duration_secs: duration.as_secs_f64(),
                version_synced: Some(actual_version.clone()),
                started_at: Some(started_at),
                completed_at: Some(Utc::now()),
            });
        }

        // 3. Download DAT file
        info!("Downloading DAT file for version {}...", actual_version);
        let dat_data = ftp
            .download_dat_file(version, None)
            .await
            .context("Failed to download DAT file")?;

        // 4. Parse entries
        info!("Parsing DAT file...");
        let parser = if let Some(limit) = self.config.parse_limit {
            DatParser::with_limit(limit)
        } else {
            DatParser::new()
        };
        let entries = parser
            .parse_bytes(&dat_data)
            .context("Failed to parse DAT file")?;
        info!("Parsed {} protein entries", entries.len());

        // 5. Store in database
        info!("Storing proteins in database...");
        let storage = if let Some(ref s3) = self.s3 {
            UniProtStorage::with_s3(
                self.db.clone(),
                s3.clone(),
                self.organization_id,
                "1.0".to_string(),
                actual_version.clone(),
            )
        } else {
            UniProtStorage::new(
                self.db.clone(),
                self.organization_id,
                "1.0".to_string(),
                actual_version.clone(),
            )
        };

        let stored_count = storage
            .store_entries(&entries)
            .await
            .context("Failed to store entries")?;
        info!("Stored {} proteins", stored_count);

        // 6. Create aggregate source
        info!("Creating aggregate source...");
        storage
            .create_aggregate_source(stored_count)
            .await
            .context("Failed to create aggregate source")?;

        let duration = start_time.elapsed();

        Ok(IngestStats {
            total_entries: entries.len() as i64,
            entries_inserted: stored_count as i64,
            entries_updated: 0,
            entries_skipped: 0,
            entries_failed: (entries.len() - stored_count) as i64,
            bytes_processed: dat_data.len() as i64,
            duration_secs: duration.as_secs_f64(),
            version_synced: Some(actual_version.clone()),
            started_at: Some(started_at),
            completed_at: Some(Utc::now()),
        })
    }

    /// Check if a version already exists in the database
    ///
    /// Queries the versions table to see if we have any version with this external_version
    async fn version_exists(&self, external_version: &str) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM versions WHERE external_version = $1"
        )
        .bind(external_version)
        .fetch_one(&self.db)
        .await
        .context("Failed to check if version exists")?;

        Ok(count > 0)
    }

    /// Get release information without downloading full dataset
    ///
    /// Useful for checking what version is available
    pub async fn get_release_info(&self, version: Option<&str>) -> Result<ReleaseInfo> {
        let ftp = UniProtFtp::new(self.config.clone());
        let notes = ftp.download_release_notes(version).await?;
        ftp.parse_release_notes(&notes)
    }

    // ========================================================================
    // Mode-Based Execution Methods
    // ========================================================================

    /// Run ingestion based on configured mode
    pub async fn run_with_mode(&self, config: &UniProtConfig) -> Result<IngestStats> {
        match &config.ingestion_mode {
            IngestionMode::Latest(cfg) => self.run_latest_mode(cfg).await,
            IngestionMode::Historical(cfg) => self.run_historical_mode(cfg).await,
        }
    }

    /// Run in latest mode - only ingest newest available version
    async fn run_latest_mode(&self, config: &LatestConfig) -> Result<IngestStats> {
        info!("Running UniProt ingestion in LATEST mode");

        let discovery = VersionDiscovery::new(self.config.clone());

        // Check for newer version
        let newer_version = discovery
            .check_for_newer_version(&self.db, self.organization_id)
            .await?;

        match newer_version {
            Some(version) => {
                // Apply ignore_before filter
                if let Some(ignore_before) = &config.ignore_before {
                    if version.external_version.as_str() < ignore_before.as_str() {
                        info!(
                            "Version {} is before ignore_before cutoff ({}), skipping",
                            version.external_version, ignore_before
                        );
                        return Ok(IngestStats::empty());
                    }
                }

                info!("Newer version available: {}", version.external_version);

                // Run ingestion for current release (version=None means current)
                if version.is_current {
                    self.run(None).await
                } else {
                    self.run(Some(&version.external_version)).await
                }
            }
            None => {
                info!("Already up-to-date, no ingestion needed");
                Ok(IngestStats::empty())
            }
        }
    }

    /// Run in historical mode - backfill multiple versions
    async fn run_historical_mode(&self, config: &HistoricalConfig) -> Result<IngestStats> {
        info!("Running UniProt ingestion in HISTORICAL mode");
        info!(
            "Version range: {} to {}",
            config.start_version,
            config.end_version.as_deref().unwrap_or("latest")
        );

        let discovery = VersionDiscovery::new(self.config.clone());

        // Discover all available versions
        let mut available_versions = discovery.discover_all_versions().await?;

        // Filter by range
        available_versions.retain(|v| {
            // Filter by start_version
            if v.external_version < config.start_version {
                return false;
            }

            // Filter by end_version if specified
            if let Some(ref end_version) = config.end_version {
                if v.external_version > *end_version {
                    return false;
                }
            }

            true
        });

        info!("Found {} versions in specified range", available_versions.len());

        // Filter out existing versions if skip_existing=true
        if config.skip_existing {
            let mut to_ingest = Vec::new();
            for version in available_versions {
                // Check if version exists in database
                let exists = discovery
                    .version_exists_in_db(&self.db, &version.external_version)
                    .await?;

                if !exists {
                    to_ingest.push(version);
                } else {
                    // Check if it was ingested as current but is now in historical
                    let was_current = discovery
                        .was_ingested_as_current(&self.db, &version.external_version)
                        .await?;

                    if was_current && !version.is_current {
                        // Same data, just moved location - skip
                        info!(
                            "Version {} was ingested as current and is now historical (same data), skipping",
                            version.external_version
                        );
                    } else {
                        info!("Version {} already exists, skipping", version.external_version);
                    }
                }
            }
            available_versions = to_ingest;
        }

        if available_versions.is_empty() {
            info!("No new versions to ingest");
            return Ok(IngestStats::empty());
        }

        info!("Will ingest {} versions", available_versions.len());

        // Process in batches
        let mut total_stats = IngestStats::empty();
        for (idx, version) in available_versions.iter().enumerate() {
            info!(
                "Processing version {}/{}: {}",
                idx + 1,
                available_versions.len(),
                version.external_version
            );

            let version_str = if version.is_current {
                None
            } else {
                Some(version.external_version.as_str())
            };

            match self.run(version_str).await {
                Ok(stats) => {
                    total_stats = total_stats.merge(stats);
                    info!("Successfully ingested version {}", version.external_version);
                }
                Err(e) => {
                    warn!(
                        "Failed to ingest version {}: {}",
                        version.external_version, e
                    );
                    // Continue with next version instead of failing entire job
                }
            }

            // Respect batch size (simple rate limiting)
            if (idx + 1) % config.batch_size == 0 && idx + 1 < available_versions.len() {
                info!("Completed batch of {}, pausing briefly", config.batch_size);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }

        Ok(total_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::uniprot::ReleaseType;

    #[test]
    fn test_uniprot_pipeline_new() {
        use sqlx::postgres::PgPoolOptions;

        // Note: This test just verifies struct creation, not full execution
        let config = UniProtFtpConfig::default()
            .with_release_type(ReleaseType::Current)
            .with_parse_limit(10);

        // Would need actual DB connection for full test
        // let pipeline = UniProtPipeline::new(db_pool, org_id, config);
    }
}
