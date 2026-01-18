//! Idempotent UniProt ingestion pipeline
//!
//! Discovers versions, checks what's been ingested, and processes only new versions.
//! Handles the "current" → versioned migration gracefully.

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::config::UniProtFtpConfig;
use super::ftp::UniProtFtp;
use super::parser::DatParser;
use super::version_discovery::{DiscoveredVersion, VersionDiscovery};
use crate::audit::{create_audit_entry, AuditAction, CreateAuditEntry, ResourceType};
use crate::ingest::config::{HistoricalConfig, IngestionMode, LatestConfig, UniProtConfig};
use crate::ingest::framework::{
    BatchConfig, CreateJobParams, IngestionCoordinator, IngestionWorker,
};
use crate::ingest::jobs::IngestStats;
use crate::storage::Storage;

/// Idempotent UniProt pipeline that handles version discovery and incremental ingestion
pub struct IdempotentUniProtPipeline {
    pool: Arc<PgPool>,
    organization_id: Uuid,
    config: UniProtFtpConfig,
    batch_config: BatchConfig,
    storage: Storage,
}

impl IdempotentUniProtPipeline {
    pub fn new(
        pool: Arc<PgPool>,
        organization_id: Uuid,
        config: UniProtFtpConfig,
        batch_config: BatchConfig,
        storage: Storage,
    ) -> Self {
        Self {
            pool,
            organization_id,
            config,
            batch_config,
            storage,
        }
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
    async fn ingest_version(&self, version: &DiscoveredVersion) -> Result<Uuid> {
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
        let s3_key = self.download_phase(coordinator, job_id, version).await?;

        // Phase 2: Parse file and create work units for parallel processing
        let total_records = self.parse_phase(coordinator, job_id, &s3_key).await?;

        // Phase 3: Process work units in parallel (spawn multiple workers)
        self.storage_phase(coordinator, job_id, &s3_key, total_records).await?;

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
    ) -> Result<String> {
        tracing::info!(job_id = %job_id, "Starting download phase");
        coordinator.start_download(job_id).await?;

        // Download DAT file from UniProt FTP
        let ftp = UniProtFtp::new(self.config.clone());
        let dat_data = ftp
            .download_dat_file(
                if version.is_current {
                    None
                } else {
                    Some(&version.external_version)
                },
                Some("swissprot"),
            )
            .await
            .context("Failed to download DAT file from FTP")?;

        tracing::info!(
            job_id = %job_id,
            size_bytes = dat_data.len(),
            "Downloaded DAT file from FTP"
        );

        // Upload to S3 ingest bucket
        let s3_key = format!(
            "ingest/uniprot/{}/{}_swissprot.dat.gz",
            job_id, version.external_version
        );

        let upload_result = self
            .storage
            .upload(&s3_key, dat_data, Some("application/x-gzip".to_string()))
            .await
            .context("Failed to upload DAT file to S3")?;

        tracing::info!(
            job_id = %job_id,
            s3_key = %s3_key,
            checksum = %upload_result.checksum,
            "Uploaded DAT file to S3"
        );

        // Register the raw file in database
        coordinator
            .register_raw_file(
                job_id,
                "dat",
                Some("swissprot_proteins"),
                &s3_key,
                None, // No expected MD5 yet
                upload_result.size,
                Some("gzip"),
            )
            .await
            .context("Failed to register raw file")?;

        coordinator.complete_download(job_id).await?;
        tracing::info!(job_id = %job_id, s3_key = %s3_key, "Download phase completed");

        Ok(s3_key)
    }

    /// Phase 2: Count records and create work units for parallel processing
    async fn parse_phase(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        s3_key: &str,
    ) -> Result<usize> {
        tracing::info!(job_id = %job_id, s3_key = %s3_key, "Starting parse phase - counting records");

        // Download file from S3
        let dat_data = self
            .storage
            .download(s3_key)
            .await
            .context("Failed to download DAT file from S3")?;

        tracing::info!(
            job_id = %job_id,
            size_bytes = dat_data.len(),
            "Downloaded DAT file from S3"
        );

        // Count total records without full parsing (faster)
        let parser = DatParser::new();
        let entries = parser.parse_bytes(&dat_data)?;
        let total_records = entries.len();

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
    /// Each worker downloads from S3, parses its batch, and stores proteins.
    async fn storage_phase(
        &self,
        coordinator: &IngestionCoordinator,
        job_id: Uuid,
        s3_key: &str,
        total_records: usize,
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

        // Download file from S3 once (shared by all workers)
        let dat_data = Arc::new(
            self.storage
                .download(s3_key)
                .await
                .context("Failed to download DAT file from S3")?,
        );

        // Parse once (shared by all workers)
        let parser = DatParser::new();
        let all_entries = Arc::new(parser.parse_bytes(&dat_data)?);

        tracing::info!(
            job_id = %job_id,
            entry_count = all_entries.len(),
            "Parsed all protein entries into memory"
        );

        // Determine number of parallel workers (max 4 for now)
        let num_workers = std::cmp::min(4, total_records / self.batch_config.parse_batch_size + 1);

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
            let all_entries_clone = all_entries.clone();
            let org_id = self.organization_id;

            let handle = tokio::spawn(async move {
                Self::worker_task(
                    worker_num,
                    job_id,
                    pool,
                    batch_config,
                    all_entries_clone,
                    org_id,
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

    /// Worker task that claims and processes work units
    ///
    /// Uses SKIP LOCKED to atomically claim work units without blocking.
    /// Processes batches idempotently - can be run by multiple workers in parallel.
    async fn worker_task(
        worker_num: usize,
        job_id: Uuid,
        pool: Arc<PgPool>,
        batch_config: BatchConfig,
        all_entries: Arc<Vec<super::models::UniProtEntry>>,
        org_id: Uuid,
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

            // Process the batch
            let result = Self::process_work_unit(
                &worker,
                &work_unit,
                &all_entries,
                &pool,
                org_id,
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

    /// Process a single work unit - parse batch and insert proteins
    async fn process_work_unit(
        worker: &IngestionWorker,
        work_unit: &super::super::framework::types::ClaimedWorkUnit,
        all_entries: &[super::models::UniProtEntry],
        pool: &PgPool,
        org_id: Uuid,
    ) -> Result<usize> {
        let start = work_unit.start_offset as usize;
        let end = (work_unit.end_offset as usize + 1).min(all_entries.len());

        if start >= end || start >= all_entries.len() {
            tracing::warn!(
                work_unit_id = %work_unit.id,
                start = start,
                end = end,
                total = all_entries.len(),
                "Invalid work unit range"
            );
            worker.complete_work_unit(work_unit.id).await?;
            return Ok(0);
        }

        let batch = &all_entries[start..end];

        tracing::debug!(
            work_unit_id = %work_unit.id,
            batch_size = batch.len(),
            "Processing batch"
        );

        // Insert proteins in transaction
        let mut tx = pool.begin().await?;
        let mut inserted = 0;

        for entry in batch {
            // Insert protein into database
            let result = sqlx::query!(
                r#"
                INSERT INTO proteins (
                    id, accession, name, organism, organism_scientific,
                    taxonomy_id, sequence, sequence_length, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
                ON CONFLICT (accession) DO UPDATE SET
                    name = EXCLUDED.name,
                    organism = EXCLUDED.organism,
                    organism_scientific = EXCLUDED.organism_scientific,
                    taxonomy_id = EXCLUDED.taxonomy_id,
                    sequence = EXCLUDED.sequence,
                    sequence_length = EXCLUDED.sequence_length,
                    updated_at = NOW()
                "#,
                Uuid::new_v4(),
                &entry.accession,
                &entry.protein_name,
                &entry.organism_name,
                &entry.organism_name,
                entry.taxonomy_id,
                &entry.sequence,
                entry.sequence_length,
            )
            .execute(&mut *tx)
            .await;

            match result {
                Ok(_) => inserted += 1,
                Err(e) => {
                    tracing::warn!(
                        work_unit_id = %work_unit.id,
                        accession = %entry.accession,
                        error = %e,
                        "Failed to insert protein (continuing)"
                    );
                }
            }
        }

        tx.commit().await?;

        // Mark work unit as completed
        worker.complete_work_unit(work_unit.id).await?;

        tracing::info!(
            work_unit_id = %work_unit.id,
            inserted = inserted,
            batch_size = batch.len(),
            "Batch inserted successfully"
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
