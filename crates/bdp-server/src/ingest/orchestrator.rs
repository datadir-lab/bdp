//! Robust ingestion orchestrator with retry support
//!
//! Discovers versions from UniProt, filters to missing versions, retries failed jobs,
//! and ingests them. Includes FTP timeout protection and comprehensive error handling.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::time::{sleep, timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    config::IngestConfig,
    framework::BatchConfig,
    uniprot::{DiscoveredVersion, UniProtFtpConfig, UniProtPipeline, VersionDiscovery},
};
use crate::storage::Storage;

/// Default timeout for FTP operations (5 minutes)
const FTP_OPERATION_TIMEOUT_SECS: u64 = 300;

/// Maximum retries for job-level failures
const DEFAULT_MAX_JOB_RETRIES: i32 = 3;

/// Delay between retry attempts (exponential backoff base)
const RETRY_DELAY_SECS: u64 = 30;

/// Simple ingestion orchestrator with retry support
pub struct IngestOrchestrator {
    config: IngestConfig,
    db: Arc<PgPool>,
    storage: Storage,
    org_id: Uuid,
}

/// Job status from the database
#[derive(Debug)]
struct JobStatus {
    id: Uuid,
    external_version: String,
    status: String,
    retry_count: i32,
    max_retries: i32,
}

impl IngestOrchestrator {
    /// Create new orchestrator
    pub fn new(config: IngestConfig, db: Arc<PgPool>, storage: Storage, org_id: Uuid) -> Self {
        Self {
            config,
            db,
            storage,
            org_id,
        }
    }

    /// Start the orchestrator in background
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Ingestion orchestrator started");

            // Initial delay to let server start
            sleep(Duration::from_secs(5)).await;

            // Run ingestion cycle with error recovery
            match self.run_ingestion_cycle().await {
                Ok(_) => info!("Ingestion cycle completed successfully"),
                Err(e) => error!("Ingestion cycle failed: {:#}", e),
            }

            info!("Ingestion orchestrator stopped");
        })
    }

    /// Run one ingestion cycle with retry support
    async fn run_ingestion_cycle(&self) -> Result<()> {
        info!("Starting ingestion cycle");

        // Get start version from config
        let start_version = &self.config.uniprot.start_from_version;

        if start_version.is_empty() {
            warn!("No start version configured (INGEST_START_FROM_VERSION), skipping ingestion");
            return Ok(());
        }

        info!("Start version: {}", start_version);

        // Step 1: Reset and retry any failed jobs that haven't exceeded max retries
        let retried = self.retry_failed_jobs().await?;
        if retried > 0 {
            info!("Reset {} failed jobs for retry", retried);
        }

        // Step 2: Discover available versions from UniProt FTP (with timeout)
        let all_versions = self.discover_versions_with_timeout().await?;

        // Filter to versions >= start_version
        let versions_to_check: Vec<_> = all_versions
            .into_iter()
            .filter(|v| v.external_version >= *start_version)
            .collect();

        info!("Versions >= {}: {}", start_version, versions_to_check.len());

        // Step 3: Find versions that need processing (not completed, not currently running)
        let versions_to_ingest = self.filter_versions_to_ingest(versions_to_check).await?;

        if versions_to_ingest.is_empty() {
            info!("No versions to ingest - all up to date!");
            return Ok(());
        }

        info!("Versions to ingest: {}", versions_to_ingest.len());
        for v in &versions_to_ingest {
            info!("  - {} ({})", v.external_version, v.release_date);
        }

        // Step 4: Process each version
        self.process_versions(versions_to_ingest).await
    }

    /// Discover versions with timeout protection
    async fn discover_versions_with_timeout(&self) -> Result<Vec<DiscoveredVersion>> {
        let ftp_config = UniProtFtpConfig::default();
        let discovery = VersionDiscovery::new(ftp_config);

        info!("Discovering available versions from UniProt FTP...");

        let discover_future = discovery.discover_previous_versions_only();
        let timeout_duration = Duration::from_secs(FTP_OPERATION_TIMEOUT_SECS);

        match timeout(timeout_duration, discover_future).await {
            Ok(Ok(versions)) => {
                info!("Found {} historical versions", versions.len());
                Ok(versions)
            }
            Ok(Err(e)) => {
                error!("FTP discovery failed: {:#}", e);
                Err(e).context("Failed to discover versions from UniProt FTP")
            }
            Err(_) => {
                error!(
                    "FTP discovery timed out after {} seconds",
                    FTP_OPERATION_TIMEOUT_SECS
                );
                Err(anyhow::anyhow!(
                    "FTP discovery timed out after {} seconds",
                    FTP_OPERATION_TIMEOUT_SECS
                ))
            }
        }
    }

    /// Retry failed jobs that haven't exceeded max retries
    async fn retry_failed_jobs(&self) -> Result<i32> {
        // Find failed jobs that can be retried
        let retryable_jobs = sqlx::query_as!(
            JobStatus,
            r#"
            SELECT
                id,
                external_version,
                status,
                COALESCE(retry_count, 0) as "retry_count!",
                COALESCE(max_retries, $3) as "max_retries!"
            FROM ingestion_jobs
            WHERE organization_id = $1
              AND job_type = 'uniprot_swissprot'
              AND status = 'failed'
              AND COALESCE(retry_count, 0) < COALESCE(max_retries, $3)
            ORDER BY created_at ASC
            LIMIT $2
            "#,
            self.org_id,
            10i64, // Limit to 10 retries per cycle
            DEFAULT_MAX_JOB_RETRIES
        )
        .fetch_all(&*self.db)
        .await
        .context("Failed to query retryable jobs")?;

        if retryable_jobs.is_empty() {
            return Ok(0);
        }

        info!(
            "Found {} failed jobs eligible for retry",
            retryable_jobs.len()
        );

        let mut reset_count = 0;
        for job in retryable_jobs {
            info!(
                "Resetting job {} (version: {}, attempt {}/{})",
                job.id,
                job.external_version,
                job.retry_count + 1,
                job.max_retries
            );

            // Reset job to pending state and increment retry count
            let result = sqlx::query!(
                r#"
                UPDATE ingestion_jobs
                SET status = 'pending',
                    retry_count = COALESCE(retry_count, 0) + 1,
                    started_at = NULL,
                    completed_at = NULL,
                    records_processed = 0,
                    records_stored = 0,
                    records_failed = 0,
                    updated_at = NOW()
                WHERE id = $1
                "#,
                job.id
            )
            .execute(&*self.db)
            .await;

            match result {
                Ok(_) => reset_count += 1,
                Err(e) => warn!("Failed to reset job {}: {}", job.id, e),
            }
        }

        Ok(reset_count)
    }

    /// Filter versions to find those that need processing
    async fn filter_versions_to_ingest(
        &self,
        versions: Vec<DiscoveredVersion>,
    ) -> Result<Vec<DiscoveredVersion>> {
        let mut to_ingest = Vec::new();

        for version in versions {
            // Check job status for this version
            let job_status = sqlx::query!(
                r#"
                SELECT
                    status,
                    COALESCE(retry_count, 0) as "retry_count!",
                    COALESCE(max_retries, $3) as "max_retries!"
                FROM ingestion_jobs
                WHERE organization_id = $1
                  AND job_type = 'uniprot_swissprot'
                  AND external_version = $2
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                self.org_id,
                version.external_version,
                DEFAULT_MAX_JOB_RETRIES
            )
            .fetch_optional(&*self.db)
            .await
            .context("Failed to check job status")?;

            let should_ingest = match job_status {
                // No job exists - needs ingestion
                None => {
                    info!(
                        "Version {} has no job record - will ingest",
                        version.external_version
                    );
                    true
                }
                Some(job) => {
                    match job.status.as_str() {
                        // Completed - skip
                        "completed" => false,

                        // Currently running - skip
                        "pending" | "downloading" | "download_verified" | "parsing" | "storing" => {
                            info!(
                                "Version {} is currently {} - skipping",
                                version.external_version, job.status
                            );
                            false
                        }

                        // Failed but not exceeding retries - already reset by retry_failed_jobs
                        "failed" if job.retry_count < job.max_retries => {
                            info!(
                                "Version {} failed but will be retried (attempt {}/{})",
                                version.external_version,
                                job.retry_count + 1,
                                job.max_retries
                            );
                            false // Already reset, will be picked up
                        }

                        // Failed and exceeded retries - skip with warning
                        "failed" => {
                            warn!(
                                "Version {} has permanently failed after {} attempts",
                                version.external_version, job.retry_count
                            );
                            false
                        }

                        // Cancelled - can retry
                        "cancelled" => {
                            info!(
                                "Version {} was cancelled - will retry",
                                version.external_version
                            );
                            true
                        }

                        // Unknown status - treat as needing ingestion
                        _ => {
                            warn!(
                                "Version {} has unknown status '{}' - will retry",
                                version.external_version, job.status
                            );
                            true
                        }
                    }
                }
            };

            if should_ingest {
                to_ingest.push(version);
            }
        }

        Ok(to_ingest)
    }

    /// Process versions for ingestion
    async fn process_versions(&self, versions: Vec<DiscoveredVersion>) -> Result<()> {
        let ftp_config = UniProtFtpConfig::default();
        let batch_config = BatchConfig::default();
        let cache_dir = self.config.uniprot.cache_dir.clone();

        let pipeline = UniProtPipeline::new(
            self.db.clone(),
            self.org_id,
            ftp_config,
            batch_config,
            self.storage.clone(),
            cache_dir,
        );

        let mut succeeded = 0;
        let mut failed = 0;

        for (i, version) in versions.iter().enumerate() {
            info!(
                "[{}/{}] Ingesting version: {}",
                i + 1,
                versions.len(),
                version.external_version
            );

            // Add timeout for entire ingestion operation
            let ingest_timeout = Duration::from_secs(self.config.job_timeout_secs);
            let ingest_future = pipeline.ingest_version(version);

            match timeout(ingest_timeout, ingest_future).await {
                Ok(Ok(job_id)) => {
                    info!(
                        "✓ Version {} ingested successfully (job: {})",
                        version.external_version, job_id
                    );
                    succeeded += 1;
                }
                Ok(Err(e)) => {
                    error!("✗ Version {} failed: {:#}", version.external_version, e);
                    failed += 1;

                    // Add delay before next attempt (exponential backoff would be better)
                    if i < versions.len() - 1 {
                        let delay = StdDuration::from_secs(RETRY_DELAY_SECS);
                        info!("Waiting {:?} before next version...", delay);
                        sleep(Duration::from_std(delay).unwrap_or(Duration::from_secs(30))).await;
                    }
                }
                Err(_) => {
                    error!(
                        "✗ Version {} timed out after {} seconds",
                        version.external_version, self.config.job_timeout_secs
                    );
                    failed += 1;

                    // Mark job as failed due to timeout
                    if let Err(e) = self.mark_job_timed_out(&version.external_version).await {
                        warn!("Failed to mark job as timed out: {}", e);
                    }
                }
            }
        }

        info!(
            "Ingestion cycle completed: {} succeeded, {} failed",
            succeeded, failed
        );

        Ok(())
    }

    /// Mark a job as timed out
    async fn mark_job_timed_out(&self, external_version: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = 'failed',
                last_error = 'Job timed out',
                error_details = $3,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE organization_id = $1
              AND job_type = 'uniprot_swissprot'
              AND external_version = $2
              AND status NOT IN ('completed', 'failed')
            "#,
            self.org_id,
            external_version,
            serde_json::json!({
                "error_type": "timeout",
                "timeout_secs": self.config.job_timeout_secs,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
        )
        .execute(&*self.db)
        .await
        .context("Failed to mark job as timed out")?;

        Ok(())
    }
}
