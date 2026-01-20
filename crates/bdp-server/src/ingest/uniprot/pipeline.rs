//! UniProt ingestion pipeline
//!
//! Idempotent pipeline that discovers versions, checks what's been ingested,
//! and processes only new versions. Handles the "current" → versioned migration gracefully.
//!
//! Uses the new schema with:
//! - registry_entries → data_sources → protein_metadata
//! - Deduplicated protein_sequences
//! - Organisms as data sources (organism_metadata)
//! - Semantic versioning (MAJOR.MINOR.PATCH)
//! - Bundle aggregates for complete releases

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::config::UniProtFtpConfig;
use super::ftp::UniProtFtp;
use super::models::UniProtEntry;
use super::parser::DatParser;
use super::storage::UniProtStorage;
use super::version_discovery::{DiscoveredVersion, VersionDiscovery};
use crate::audit::{create_audit_entry, AuditAction, CreateAuditEntry, ResourceType};
use crate::ingest::config::{HistoricalConfig, IngestionMode, LatestConfig, UniProtConfig};
use crate::ingest::framework::{
    BatchConfig, CreateJobParams, IngestionCoordinator, IngestionWorker,
};
use crate::ingest::jobs::IngestStats;
use crate::storage::Storage;

/// UniProt pipeline that handles version discovery and incremental ingestion
pub struct UniProtPipeline {
    pool: Arc<PgPool>,
    organization_id: Uuid,
    config: UniProtFtpConfig,
    batch_config: BatchConfig,
    storage: Storage,
    cache_dir: std::path::PathBuf,
}

impl UniProtPipeline {
    pub fn new(
        pool: Arc<PgPool>,
        organization_id: Uuid,
        config: UniProtFtpConfig,
        batch_config: BatchConfig,
        storage: Storage,
        cache_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            pool,
            organization_id,
            config,
            batch_config,
            storage,
            cache_dir,
        }
    }

    // ========================================================================
    // Cache Management Methods
    // ========================================================================

    /// Get the cache file path for a specific version
    fn get_cache_path(&self, version: &str) -> std::path::PathBuf {
        self.cache_dir.join("uniprot").join(format!("{}.dat", version))
    }

    /// Get the lock file path for a specific version
    fn get_lock_path(&self, version: &str) -> std::path::PathBuf {
        self.cache_dir.join("uniprot").join(format!("{}.dat.lock", version))
    }

    /// Check if cached DAT file exists for a version
    fn is_cached(&self, version: &str) -> bool {
        let cache_path = self.get_cache_path(version);
        cache_path.exists() && cache_path.is_file()
    }

    /// Write decompressed DAT data to cache atomically
    async fn write_to_cache(&self, version: &str, dat_data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let cache_path = self.get_cache_path(version);
        let lock_path = self.get_lock_path(version);

        // Create cache directory
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create cache directory")?;
        }

        // Create lock file to prevent race conditions
        let _lock = tokio::fs::File::create(&lock_path).await
            .context("Failed to create lock file")?;

        // Write to temporary file
        let temp_path = cache_path.with_extension("tmp");
        let mut file = tokio::fs::File::create(&temp_path).await
            .context("Failed to create temp cache file")?;

        file.write_all(dat_data).await
            .context("Failed to write to temp cache file")?;

        file.flush().await
            .context("Failed to flush temp cache file")?;

        // Atomic rename
        tokio::fs::rename(&temp_path, &cache_path).await
            .context("Failed to rename temp cache file to final cache file")?;

        // Remove lock file
        let _ = tokio::fs::remove_file(&lock_path).await;

        tracing::info!(
            version = version,
            cache_path = ?cache_path,
            size_bytes = dat_data.len(),
            "Successfully wrote decompressed DAT to cache"
        );

        Ok(())
    }

    /// Read decompressed DAT data from cache
    async fn read_from_cache(&self, version: &str) -> Result<Vec<u8>> {
        let cache_path = self.get_cache_path(version);

        let data = tokio::fs::read(&cache_path).await
            .with_context(|| format!("Failed to read cache file: {:?}", cache_path))?;

        tracing::info!(
            version = version,
            cache_path = ?cache_path,
            size_bytes = data.len(),
            "Successfully read decompressed DAT from cache (CACHE HIT)"
        );

        Ok(data)
    }

    /// Clean up cache files older than the specified number of days
    pub async fn cleanup_cache(&self, max_age_days: u64) -> Result<usize> {
        use std::time::SystemTime;

        let cache_dir = self.cache_dir.join("uniprot");
        if !cache_dir.exists() {
            return Ok(0);
        }

        let mut removed_count = 0;
        let max_age = std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);
        let now = SystemTime::now();

        let mut entries = tokio::fs::read_dir(&cache_dir).await
            .context("Failed to read cache directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Skip lock files and non-DAT files
            if !path.extension().map_or(false, |ext| ext == "dat") {
                continue;
            }

            // Check file age
            if let Ok(metadata) = tokio::fs::metadata(&path).await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            match tokio::fs::remove_file(&path).await {
                                Ok(_) => {
                                    tracing::info!(
                                        path = ?path,
                                        age_days = age.as_secs() / (24 * 60 * 60),
                                        "Removed old cache file"
                                    );
                                    removed_count += 1;
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        path = ?path,
                                        error = %e,
                                        "Failed to remove old cache file"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!(
            removed_count = removed_count,
            max_age_days = max_age_days,
            "Cache cleanup completed"
        );

        Ok(removed_count)
    }

    // ========================================================================
    // Mode-Based Execution Methods
    // ========================================================================

    /// Run ingestion based on configured mode (Latest or Historical)
    ///
    /// This is the main entry point for mode-based ingestion.
    /// Dispatches to run_latest_mode() or run_historical_mode() based on config.
    pub async fn run_with_mode(&self, config: &UniProtConfig) -> Result<IngestStats> {
        match &config.ingestion_mode {
            IngestionMode::Latest(latest_config) => {
                tracing::info!("Running in Latest mode");
                self.run_latest_mode(latest_config).await
            }
            IngestionMode::Historical(historical_config) => {
                tracing::info!("Running in Historical mode");
                self.run_historical_mode(historical_config).await
            }
        }
    }

    /// Run in Latest mode: ingest only the newest available version
    ///
    /// Steps:
    /// 1. Use VersionDiscovery.check_for_newer_version() to find newer version
    /// 2. Apply ignore_before filter if configured
    /// 3. If newer version available, ingest it with is_current=true
    /// 4. If up-to-date, return empty stats (no-op)
    pub async fn run_latest_mode(&self, config: &LatestConfig) -> Result<IngestStats> {
        tracing::info!(
            check_interval_secs = config.check_interval_secs,
            ignore_before = ?config.ignore_before,
            "Starting Latest mode ingestion"
        );

        let started_at = chrono::Utc::now();

        // 1. Check for newer version
        let discovery = VersionDiscovery::new(self.config.clone());
        let newer_version = discovery
            .check_for_newer_version(&self.pool, self.organization_id)
            .await
            .context("Failed to check for newer version")?;

        let Some(mut version) = newer_version else {
            tracing::info!("No newer version available - database is up-to-date");
            return Ok(IngestStats {
                total_entries: 0,
                entries_inserted: 0,
                entries_updated: 0,
                entries_skipped: 0,
                entries_failed: 0,
                bytes_processed: 0,
                duration_secs: 0.0,
                version_synced: None,
                started_at: Some(started_at),
                completed_at: Some(chrono::Utc::now()),
            });
        };

        // 2. Apply ignore_before filter if configured
        if let Some(ignore_before) = &config.ignore_before {
            if version.external_version < *ignore_before {
                tracing::info!(
                    version = %version.external_version,
                    ignore_before = %ignore_before,
                    "Skipping version (older than ignore_before threshold)"
                );
                return Ok(IngestStats {
                    total_entries: 0,
                    entries_inserted: 0,
                    entries_updated: 0,
                    entries_skipped: 0,
                    entries_failed: 0,
                    bytes_processed: 0,
                    duration_secs: 0.0,
                    version_synced: None,
                    started_at: Some(started_at),
                    completed_at: Some(chrono::Utc::now()),
                });
            }
        }

        // 3. Ingest the newer version (ensure is_current=true)
        version.is_current = true;

        tracing::info!(
            version = %version.external_version,
            release_date = %version.release_date,
            "Found newer version - starting ingestion"
        );

        let job_id = self.ingest_version(&version).await?;

        let completed_at = chrono::Utc::now();
        let duration_secs = (completed_at - started_at).num_milliseconds() as f64 / 1000.0;

        // Fetch final stats from database
        let stats = self.get_job_stats(job_id).await?;

        Ok(IngestStats {
            total_entries: stats.total_entries,
            entries_inserted: stats.entries_inserted,
            entries_updated: stats.entries_updated,
            entries_skipped: stats.entries_skipped,
            entries_failed: stats.entries_failed,
            bytes_processed: stats.bytes_processed,
            duration_secs,
            version_synced: Some(version.external_version),
            started_at: Some(started_at),
            completed_at: Some(completed_at),
        })
    }

    /// Run in Historical mode: backfill multiple versions within a range
    ///
    /// Steps:
    /// 1. Use VersionDiscovery.discover_all_versions()
    /// 2. Filter by start_version..end_version range
    /// 3. If skip_existing=true, check database for existing versions
    /// 4. Process versions in batches (sequential, batch_size from config)
    /// 5. Store is_current=false in source_metadata
    /// 6. Merge stats from all versions and return
    pub async fn run_historical_mode(&self, config: &HistoricalConfig) -> Result<IngestStats> {
        tracing::info!(
            start_version = %config.start_version,
            end_version = ?config.end_version,
            batch_size = config.batch_size,
            skip_existing = config.skip_existing,
            "Starting Historical mode ingestion"
        );

        let started_at = chrono::Utc::now();

        // 1. Discover all available versions
        let discovery = VersionDiscovery::new(self.config.clone());
        let all_versions = discovery
            .discover_all_versions()
            .await
            .context("Failed to discover versions")?;

        // 2. Filter by version range
        let mut filtered_versions: Vec<_> = all_versions
            .into_iter()
            .filter(|v| {
                // Start version filter
                if v.external_version < config.start_version {
                    return false;
                }

                // End version filter (if specified)
                if let Some(ref end) = config.end_version {
                    if v.external_version > *end {
                        return false;
                    }
                }

                true
            })
            .collect();

        tracing::info!(
            filtered_count = filtered_versions.len(),
            "Filtered versions within range"
        );

        // 3. Skip existing versions if configured
        if config.skip_existing {
            let ingested = self.get_ingested_versions().await?;
            let before_skip = filtered_versions.len();
            filtered_versions.retain(|v| !ingested.contains(&v.external_version));

            tracing::info!(
                skipped_count = before_skip - filtered_versions.len(),
                remaining_count = filtered_versions.len(),
                "Skipped already-ingested versions"
            );
        }

        if filtered_versions.is_empty() {
            tracing::info!("No versions to ingest");
            return Ok(IngestStats {
                total_entries: 0,
                entries_inserted: 0,
                entries_updated: 0,
                entries_skipped: 0,
                entries_failed: 0,
                bytes_processed: 0,
                duration_secs: 0.0,
                version_synced: None,
                started_at: Some(started_at),
                completed_at: Some(chrono::Utc::now()),
            });
        }

        // 4. Process versions in batches (sequential processing)
        let mut total_stats = IngestStats {
            total_entries: 0,
            entries_inserted: 0,
            entries_updated: 0,
            entries_skipped: 0,
            entries_failed: 0,
            bytes_processed: 0,
            duration_secs: 0.0,
            version_synced: None,
            started_at: Some(started_at),
            completed_at: None,
        };

        let mut versions_ingested = Vec::new();

        // Process in chunks of batch_size
        for chunk in filtered_versions.chunks(config.batch_size) {
            tracing::info!(
                chunk_size = chunk.len(),
                "Processing version batch"
            );

            for mut version in chunk.iter().cloned() {
                // 5. Ensure is_current=false for historical versions
                version.is_current = false;

                tracing::info!(
                    version = %version.external_version,
                    release_date = %version.release_date,
                    "Ingesting historical version"
                );

                match self.ingest_version(&version).await {
                    Ok(job_id) => {
                        // Merge stats
                        let stats = self.get_job_stats(job_id).await?;
                        total_stats.total_entries += stats.total_entries;
                        total_stats.entries_inserted += stats.entries_inserted;
                        total_stats.entries_updated += stats.entries_updated;
                        total_stats.entries_skipped += stats.entries_skipped;
                        total_stats.entries_failed += stats.entries_failed;
                        total_stats.bytes_processed += stats.bytes_processed;

                        versions_ingested.push(version.external_version.clone());

                        tracing::info!(
                            version = %version.external_version,
                            job_id = %job_id,
                            "Successfully ingested historical version"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            version = %version.external_version,
                            error = %e,
                            "Failed to ingest historical version"
                        );
                        total_stats.entries_failed += 1;
                    }
                }
            }
        }

        let completed_at = chrono::Utc::now();
        total_stats.duration_secs = (completed_at - started_at).num_milliseconds() as f64 / 1000.0;
        total_stats.completed_at = Some(completed_at);
        total_stats.version_synced = Some(format!(
            "Historical: {} versions ({})",
            versions_ingested.len(),
            versions_ingested.join(", ")
        ));

        tracing::info!(
            versions_count = versions_ingested.len(),
            total_entries = total_stats.total_entries,
            duration_secs = total_stats.duration_secs,
            "Historical mode ingestion completed"
        );

        Ok(total_stats)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get ingestion statistics from a completed job
    async fn get_job_stats(&self, job_id: Uuid) -> Result<IngestStats> {
        let job = sqlx::query!(
            r#"
            SELECT
                records_processed,
                records_stored,
                records_failed
            FROM ingestion_jobs
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_one(&*self.pool)
        .await
        .context("Failed to fetch job stats")?;

        Ok(IngestStats {
            total_entries: job.records_processed.unwrap_or(0),
            entries_inserted: job.records_stored.unwrap_or(0),
            entries_updated: 0, // UniProt uses UPSERT, so we don't track separately
            entries_skipped: 0,
            entries_failed: job.records_failed.unwrap_or(0),
            bytes_processed: 0, // Not tracked in ingestion_jobs currently
            duration_secs: 0.0, // Calculated by caller
            version_synced: None,
            started_at: None,
            completed_at: None,
        })
    }

    // ========================================================================
    // Idempotent Pipeline (Legacy Method)
    // ========================================================================

    /// Run the complete idempotent ingestion pipeline
    ///
    /// Steps:
    /// 1. Discover all available versions from FTP
    /// 2. Check which versions we've already ingested
    /// 3. Process new versions in chronological order (oldest first)
    /// 4. Handle "current" → versioned migration
    pub async fn run_idempotent(&self) -> Result<IdempotentStats> {
        tracing::info!("Starting idempotent UniProt ingestion pipeline");

        // 1. Discover all available versions
        let discovery = VersionDiscovery::new(self.config.clone());
        let discovered = discovery
            .discover_all_versions()
            .await
            .context("Failed to discover UniProt versions")?;

        tracing::info!(
            discovered_count = discovered.len(),
            "Discovered UniProt versions"
        );

        // 2. Get already ingested versions
        let ingested = self.get_ingested_versions().await?;

        tracing::info!(
            ingested_count = ingested.len(),
            "Found previously ingested versions"
        );

        // 3. Filter to new versions only
        let new_versions = discovery.filter_new_versions(discovered, ingested.clone());

        if new_versions.is_empty() {
            tracing::info!("No new versions to ingest");
            return Ok(IdempotentStats {
                discovered_count: 0,
                already_ingested_count: ingested.len(),
                newly_ingested_count: 0,
                failed_count: 0,
            });
        }

        tracing::info!(
            new_versions_count = new_versions.len(),
            "Processing new versions"
        );

        // 4. Process each new version
        let mut stats = IdempotentStats {
            discovered_count: new_versions.len(),
            already_ingested_count: ingested.len(),
            newly_ingested_count: 0,
            failed_count: 0,
        };

        for version in new_versions {
            match self.ingest_version(&version).await {
                Ok(_) => {
                    tracing::info!(
                        version = %version.external_version,
                        "Successfully ingested version"
                    );
                    stats.newly_ingested_count += 1;
                }
                Err(e) => {
                    tracing::error!(
                        version = %version.external_version,
                        error = %e,
                        "Failed to ingest version"
                    );
                    stats.failed_count += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Get list of external versions we've already ingested
    async fn get_ingested_versions(&self) -> Result<Vec<String>> {
        let versions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT external_version
            FROM ingestion_jobs
            WHERE organization_id = $1
              AND job_type LIKE 'uniprot_%'
              AND status = 'completed'
            ORDER BY external_version
            "#,
        )
        .bind(self.organization_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch ingested versions")?;

        Ok(versions)
    }

    /// Ingest a specific version using the generic ETL framework
    pub async fn ingest_version(&self, version: &DiscoveredVersion) -> Result<Uuid> {
        tracing::info!(
            version = %version.external_version,
            is_current = version.is_current,
            "Starting ingestion for version"
        );

        // Create coordinator
        let coordinator = IngestionCoordinator::new(self.pool.clone(), self.batch_config.clone());

        // Create ingestion job
        let job_id = coordinator
            .create_job(CreateJobParams {
                organization_id: self.organization_id,
                job_type: "uniprot_swissprot".to_string(),
                external_version: version.external_version.clone(),
                internal_version: "1.0".to_string(), // Our internal versioning
                source_url: Some(format!(
                    "ftp://ftp.uniprot.org/pub/databases/uniprot/{}",
                    version.ftp_path
                )),
                source_metadata: Some(serde_json::json!({
                    "is_current": version.is_current,
                    "release_date": version.release_date.to_string(),
                    "ftp_path": version.ftp_path,
                })),
                total_records: None,
            })
            .await
            .context("Failed to create ingestion job")?;

        tracing::info!(job_id = %job_id, "Created ingestion job");

        // Audit: Job started
        if let Err(e) = create_audit_entry(
            &self.pool,
            CreateAuditEntry::builder()
                .action(AuditAction::Ingest)
                .resource_type(ResourceType::IngestionJob)
                .resource_id(Some(job_id))
                .user_id(None) // System-initiated
                .metadata(serde_json::json!({
                    "status": "started",
                    "version": version.external_version,
                    "organization_id": self.organization_id,
                    "is_current": version.is_current,
                }))
                .build(),
        )
        .await
        {
            tracing::warn!(error = %e, "Failed to create audit log for job start");
        }

        // Execute the full pipeline
        match self.execute_pipeline(&coordinator, job_id, &version).await {
            Ok(_) => {
                tracing::info!(job_id = %job_id, "Pipeline completed successfully");

                // Audit: Job completed
                if let Err(e) = create_audit_entry(
                    &self.pool,
                    CreateAuditEntry::builder()
                        .action(AuditAction::Ingest)
                        .resource_type(ResourceType::IngestionJob)
                        .resource_id(Some(job_id))
                        .user_id(None)
                        .metadata(serde_json::json!({
                            "status": "completed",
                            "version": version.external_version,
                            "organization_id": self.organization_id,
                        }))
                        .build(),
                )
                .await
                {
                    tracing::warn!(error = %e, "Failed to create audit log for job completion");
                }

                Ok(job_id)
            }
            Err(e) => {
                tracing::error!(job_id = %job_id, error = %e, "Pipeline failed");

                // Audit: Job failed
                if let Err(audit_err) = create_audit_entry(
                    &self.pool,
                    CreateAuditEntry::builder()
                        .action(AuditAction::Ingest)
                        .resource_type(ResourceType::IngestionJob)
                        .resource_id(Some(job_id))
                        .user_id(None)
                        .metadata(serde_json::json!({
                            "status": "failed",
                            "version": version.external_version,
                            "organization_id": self.organization_id,
                            "error": e.to_string(),
                        }))
                        .build(),
                )
                .await
                {
                    tracing::warn!(error = %audit_err, "Failed to create audit log for job failure");
                }

                // Mark job as failed
                coordinator
                    .fail_job(job_id, &e.to_string())
                    .await
                    .context("Failed to mark job as failed")?;
                Err(e)
            }
        }
    }

    /// Execute the full ingestion pipeline for a version
    async fn execute_pipeline(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        version: &DiscoveredVersion,
    ) -> Result<()> {
        tracing::info!(job_id = %job_id, version = %version.external_version, "Starting pipeline execution");

        // Phase 1: Download from FTP and upload to S3
        let (s3_key, dat_data) = self.download_phase(coordinator, job_id, version).await?;

        // Phase 2: Count entries and create work units for parallel processing
        let total_records = self.parse_phase(coordinator, job_id, &s3_key, &dat_data).await?;

        // Phase 3: Process work units in parallel (spawn multiple workers, streaming parse+store)
        self.storage_phase(coordinator, job_id, &s3_key, total_records, dat_data, version).await?;

        // Phase 4: Create bundles after all proteins stored
        self.bundle_phase(coordinator, job_id, version).await?;

        // Complete the job
        coordinator.complete_job(job_id).await?;

        tracing::info!(job_id = %job_id, "Pipeline execution completed");
        Ok(())
    }

    /// Phase 1: Download files from FTP and upload to S3
    async fn download_phase(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        version: &DiscoveredVersion,
    ) -> Result<(String, Vec<u8>)> {
        tracing::info!(job_id = %job_id, "Starting download phase");
        coordinator.start_download(job_id).await?;

        // Run cache cleanup (delete files older than 7 days)
        if let Err(e) = self.cleanup_cache(7).await {
            tracing::warn!(error = %e, "Cache cleanup failed, continuing");
        }

        // Check if we have a cached decompressed DAT file
        let dat_data = if self.is_cached(&version.external_version) {
            // CACHE HIT - Read from disk cache
            tracing::info!(
                job_id = %job_id,
                version = %version.external_version,
                "Cache hit - reading decompressed DAT from cache"
            );
            self.read_from_cache(&version.external_version).await?
        } else {
            // CACHE MISS - Download from FTP and decompress
            tracing::info!(
                job_id = %job_id,
                version = %version.external_version,
                "Cache miss - downloading from FTP"
            );

            let ftp = UniProtFtp::new(self.config.clone());
            let compressed_data = ftp
                .download_dat_file(
                    if version.is_current {
                        None
                    } else {
                        Some(&version.external_version)
                    },
                    Some("sprot"), // Swiss-Prot filename is "sprot" not "swissprot"
                )
                .await
                .context("Failed to download DAT file from FTP")?;

            tracing::info!(
                job_id = %job_id,
                compressed_size = compressed_data.len(),
                "Downloaded compressed DAT file from FTP"
            );

            // Decompress the data
            let parser = DatParser::new();
            let decompressed_data = parser.extract_dat_data(&compressed_data)
                .context("Failed to decompress DAT file")?;

            tracing::info!(
                job_id = %job_id,
                decompressed_size = decompressed_data.len(),
                "Decompressed DAT file"
            );

            // Write to cache for future use
            if let Err(e) = self.write_to_cache(&version.external_version, &decompressed_data).await {
                tracing::warn!(
                    job_id = %job_id,
                    error = %e,
                    "Failed to write to cache, continuing"
                );
            }

            decompressed_data
        };

        tracing::info!(
            job_id = %job_id,
            size_bytes = dat_data.len(),
            "DAT data ready (decompressed)"
        );

        // Note: We no longer upload the compressed file to S3 since we're caching the decompressed version
        // S3 upload is skipped to avoid storing duplicate data
        let s3_key = format!(
            "ingest/uniprot/{}/{}_swissprot.dat.gz",
            job_id, version.external_version
        );

        let (s3_uploaded, file_size, checksum): (bool, i64, Option<String>) = (false, dat_data.len() as i64, None);

        // Register the raw file in database (only if S3 upload succeeded)
        if s3_uploaded {
            coordinator
                .register_raw_file(
                    job_id,
                    "dat",
                    Some("swissprot_proteins"),
                    &s3_key,
                    None, // No expected MD5 yet
                    file_size,
                    Some("gzip"),
                )
                .await
                .context("Failed to register raw file")?;
        }

        coordinator.complete_download(job_id).await?;
        tracing::info!(job_id = %job_id, s3_key = %s3_key, "Download phase completed");

        Ok((s3_key, dat_data))
    }

    /// Phase 2: Count records and create work units for parallel processing
    async fn parse_phase(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        s3_key: &str,
        dat_data: &[u8],
    ) -> Result<usize> {
        tracing::info!(
            job_id = %job_id,
            s3_key = %s3_key,
            size_bytes = dat_data.len(),
            "Starting parse phase - counting records"
        );

        // Count total records efficiently (just count "//" markers)
        // Use count_entries_predecompressed since dat_data is already decompressed from cache
        let parser = DatParser::new();
        let total_records = parser.count_entries_predecompressed(dat_data)?;

        tracing::info!(
            job_id = %job_id,
            total_records = total_records,
            "Counted protein entries"
        );

        // Create work units for parallel processing
        let num_work_units = coordinator
            .create_work_units(job_id, "parse_store", total_records)
            .await
            .context("Failed to create work units")?;

        tracing::info!(
            job_id = %job_id,
            work_units = num_work_units,
            batch_size = self.batch_config.parse_batch_size,
            "Created work units for parallel processing"
        );

        Ok(total_records)
    }

    /// Phase 3: Parallel batch processing with multiple workers
    ///
    /// Spawns worker tasks that compete for work units using SKIP LOCKED.
    /// Each worker parses its range on-demand and stores proteins (streaming).
    async fn storage_phase(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        s3_key: &str,
        total_records: usize,
        dat_data: Vec<u8>,
        version: &DiscoveredVersion,
    ) -> Result<()> {
        tracing::info!(
            job_id = %job_id,
            total_records = total_records,
            "Starting storage phase with parallel workers"
        );

        // Update status to storing
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = 'storing'
            WHERE id = $1
            "#,
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to storing")?;

        // Share raw bytes across workers (they parse their range on-demand)
        let dat_data = Arc::new(dat_data);

        tracing::info!(
            job_id = %job_id,
            size_bytes = dat_data.len(),
            "Prepared data for streaming parallel parse+store"
        );

        // Determine number of parallel workers (max 16 for improved throughput)
        let num_workers = std::cmp::min(16, total_records / self.batch_config.parse_batch_size + 1);

        tracing::info!(
            job_id = %job_id,
            num_workers = num_workers,
            "Spawning parallel workers"
        );

        // Spawn worker tasks
        let mut handles = vec![];
        for worker_num in 0..num_workers {
            let pool = self.pool.clone();
            let batch_config = self.batch_config.clone();
            let dat_data_clone = dat_data.clone();
            let org_id = self.organization_id;
            let storage = self.storage.clone();
            let external_version = version.external_version.clone();

            let handle = tokio::spawn(async move {
                Self::worker_task(
                    worker_num,
                    job_id,
                    pool,
                    batch_config,
                    dat_data_clone,
                    org_id,
                    storage,
                    external_version,
                )
                .await
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        let mut total_processed = 0;
        let mut total_failed = 0;

        for (idx, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok((processed, failed))) => {
                    total_processed += processed;
                    total_failed += failed;
                    tracing::info!(
                        job_id = %job_id,
                        worker = idx,
                        processed = processed,
                        failed = failed,
                        "Worker completed"
                    );
                }
                Ok(Err(e)) => {
                    tracing::error!(job_id = %job_id, worker = idx, error = %e, "Worker failed");
                    total_failed += 1;
                }
                Err(e) => {
                    tracing::error!(job_id = %job_id, worker = idx, error = %e, "Worker panicked");
                    total_failed += 1;
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            total_processed = total_processed,
            total_failed = total_failed,
            "All workers completed"
        );

        // Update final counts
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET records_processed = $1,
                records_stored = $2,
                records_failed = $3
            WHERE id = $4
            "#,
            total_processed as i64,
            total_processed as i64,
            total_failed as i64,
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job record counts")?;

        tracing::info!(job_id = %job_id, "Storage phase completed");
        Ok(())
    }

    /// Phase 4: Create bundles from stored proteins
    ///
    /// Queries database for all proteins in this version and creates:
    /// - Organism-specific bundles (one per unique organism)
    /// - Swissprot bundle (all reviewed proteins)
    async fn bundle_phase(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        version: &DiscoveredVersion,
    ) -> Result<()> {
        tracing::info!(
            job_id = %job_id,
            version = %version.external_version,
            "Starting bundle creation phase"
        );

        // Query all protein metadata for this version to get organism groupings
        let proteins = sqlx::query!(
            r#"
            SELECT
                pm.accession,
                pm.protein_name,
                om.taxonomy_id,
                om.scientific_name as organism_name
            FROM protein_metadata pm
            JOIN taxonomy_metadata om ON om.data_source_id = pm.taxonomy_id
            WHERE pm.uniprot_version = $1
            "#,
            version.external_version
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch protein metadata for bundle creation")?;

        tracing::info!(
            job_id = %job_id,
            protein_count = proteins.len(),
            "Fetched protein metadata for bundle creation"
        );

        // Convert to UniProtEntry-like structure (minimal fields needed for bundling)
        let entries: Vec<UniProtEntry> = proteins
            .into_iter()
            .map(|p| UniProtEntry {
                accession: p.accession,
                entry_name: String::new(), // Not needed for bundling
                protein_name: p.protein_name.unwrap_or_default(),
                gene_name: None,
                organism_name: p.organism_name,
                taxonomy_id: p.taxonomy_id,
                taxonomy_lineage: Vec::new(), // Not needed for bundling
                sequence: String::new(), // Not needed for bundling
                sequence_length: 0,
                mass_da: 0,
                release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                alternative_names: Vec::new(),
                ec_numbers: Vec::new(),
                features: Vec::new(),
                cross_references: Vec::new(),
                comments: Vec::new(),
                protein_existence: None,
                keywords: Vec::new(),
                organelle: None,
                organism_hosts: Vec::new(),
            })
            .collect();

        // Create storage handler for bundle creation
        let storage = UniProtStorage::new(
            (*self.pool).clone(),
            self.organization_id,
            "1.0".to_string(), // Internal version
            version.external_version.clone(),
        );

        // Create all bundles (organism + swissprot)
        storage
            .create_bundles(&entries)
            .await
            .context("Failed to create bundles")?;

        tracing::info!(job_id = %job_id, "Bundle creation phase completed");
        Ok(())
    }

    /// Worker task that claims and processes work units
    ///
    /// Uses SKIP LOCKED to atomically claim work units without blocking.
    /// Processes batches idempotently - can be run by multiple workers in parallel.
    /// Each worker parses its range on-demand (streaming parse+store).
    async fn worker_task(
        worker_num: usize,
        job_id: Uuid,
        pool: Arc<PgPool>,
        batch_config: BatchConfig,
        dat_data: Arc<Vec<u8>>,
        org_id: Uuid,
        storage: Storage,
        external_version: String,
    ) -> Result<(usize, usize)> {
        let worker = IngestionWorker::new(pool.clone(), batch_config.clone());
        let worker_id = worker.worker_id();

        tracing::info!(
            job_id = %job_id,
            worker_num = worker_num,
            worker_id = %worker_id,
            "Worker started"
        );

        let mut total_processed = 0;
        let mut total_failed = 0;

        // Create parser for this worker
        let parser = DatParser::new();

        // Loop until no more work units available
        loop {
            // Claim a work unit atomically (SKIP LOCKED)
            let work_unit = match worker.claim_work_unit(job_id).await? {
                Some(unit) => unit,
                None => {
                    tracing::info!(
                        job_id = %job_id,
                        worker_id = %worker_id,
                        "No more work units available"
                    );
                    break;
                }
            };

            tracing::info!(
                job_id = %job_id,
                worker_id = %worker_id,
                work_unit_id = %work_unit.id,
                batch_number = work_unit.batch_number,
                start_offset = work_unit.start_offset,
                end_offset = work_unit.end_offset,
                "Claimed work unit"
            );

            // Start heartbeat task
            let heartbeat_handle = worker.start_heartbeat_task(work_unit.id);

            // Process the batch (parse + store streaming)
            let result = Self::process_work_unit(
                &worker,
                &work_unit,
                &dat_data,
                &parser,
                &pool,
                org_id,
                &storage,
                &external_version,
            )
            .await;

            // Cancel heartbeat
            heartbeat_handle.abort();

            match result {
                Ok(count) => {
                    total_processed += count;
                    tracing::info!(
                        job_id = %job_id,
                        worker_id = %worker_id,
                        work_unit_id = %work_unit.id,
                        records_processed = count,
                        "Work unit completed successfully"
                    );
                }
                Err(e) => {
                    total_failed += 1;
                    tracing::error!(
                        job_id = %job_id,
                        worker_id = %worker_id,
                        work_unit_id = %work_unit.id,
                        error = %e,
                        "Work unit failed"
                    );

                    // Mark work unit as failed
                    worker.fail_work_unit(work_unit.id, &e.to_string()).await?;
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            worker_id = %worker_id,
            total_processed = total_processed,
            total_failed = total_failed,
            "Worker finished"
        );

        Ok((total_processed, total_failed))
    }

    /// Process a single work unit - parse range on-demand and insert proteins (streaming)
    async fn process_work_unit(
        worker: &IngestionWorker,
        work_unit: &super::super::framework::types::ClaimedWorkUnit,
        dat_data: &[u8],
        parser: &DatParser,
        pool: &PgPool,
        org_id: Uuid,
        storage_backend: &Storage,
        external_version: &str,
    ) -> Result<usize> {
        let start = work_unit.start_offset as usize;
        let end = work_unit.end_offset as usize;

        tracing::debug!(
            work_unit_id = %work_unit.id,
            start_offset = start,
            end_offset = end,
            "Parsing range on-demand"
        );

        // Parse only this worker's range (streaming parse)
        // Use parse_range_predecompressed since dat_data is already decompressed from cache
        let entries = parser.parse_range_predecompressed(dat_data, start, end)?;

        if entries.is_empty() {
            tracing::warn!(
                work_unit_id = %work_unit.id,
                start = start,
                end = end,
                "No entries parsed in range"
            );
            worker.complete_work_unit(work_unit.id).await?;
            return Ok(0);
        }

        tracing::debug!(
            work_unit_id = %work_unit.id,
            parsed_count = entries.len(),
            "Parsed entries, starting storage"
        );

        // Calculate internal version from external version
        // For now, use "1.0" as default - TODO: implement proper version mapping
        let internal_version = "1.0".to_string();

        // Create storage handler with S3
        let storage = UniProtStorage::with_s3(
            pool.clone(),
            storage_backend.clone(),
            org_id,
            internal_version,
            external_version.to_string(),
        );

        // Store entries using new schema (registry_entries → data_sources → protein_metadata)
        let inserted = storage.store_entries(&entries).await?;

        // Mark work unit as completed
        worker.complete_work_unit(work_unit.id).await?;

        tracing::info!(
            work_unit_id = %work_unit.id,
            inserted = inserted,
            parsed_count = entries.len(),
            "Batch parsed and inserted successfully (streaming)"
        );

        Ok(inserted)
    }

    /// Check if a specific version has been ingested
    pub async fn is_version_ingested(&self, external_version: &str) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM ingestion_jobs
            WHERE organization_id = $1
              AND external_version = $2
              AND status = 'completed'
            "#,
        )
        .bind(self.organization_id)
        .bind(external_version)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to check if version is ingested")?;

        Ok(count > 0)
    }
}

/// Statistics from an idempotent pipeline run
#[derive(Debug, Clone)]
pub struct IdempotentStats {
    pub discovered_count: usize,
    pub already_ingested_count: usize,
    pub newly_ingested_count: usize,
    pub failed_count: usize,
}

impl IdempotentStats {
    pub fn total_versions(&self) -> usize {
        self.discovered_count + self.already_ingested_count
    }

    pub fn success_rate(&self) -> f64 {
        if self.discovered_count == 0 {
            return 100.0;
        }
        (self.newly_ingested_count as f64 / self.discovered_count as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotent_stats() {
        let stats = IdempotentStats {
            discovered_count: 3,
            already_ingested_count: 2,
            newly_ingested_count: 3,
            failed_count: 0,
        };

        assert_eq!(stats.total_versions(), 5);
        assert_eq!(stats.success_rate(), 100.0);
    }

    #[test]
    fn test_idempotent_stats_partial_failure() {
        let stats = IdempotentStats {
            discovered_count: 3,
            already_ingested_count: 2,
            newly_ingested_count: 2,
            failed_count: 1,
        };

        assert_eq!(stats.total_versions(), 5);
        assert!((stats.success_rate() - 66.67).abs() < 0.1);
    }
}
