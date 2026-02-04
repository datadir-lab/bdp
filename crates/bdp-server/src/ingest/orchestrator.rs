//! Ingestion orchestrator with FTP timeout protection
//!
//! Discovers versions from UniProt, filters to missing versions, and ingests them.
//! Includes FTP timeout protection and comprehensive error handling.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
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

/// Delay between version ingestions
const RETRY_DELAY_SECS: u64 = 30;

/// Ingestion orchestrator
pub struct IngestOrchestrator {
    config: IngestConfig,
    db: Arc<PgPool>,
    storage: Storage,
    org_id: Uuid,
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

    /// Run one ingestion cycle
    async fn run_ingestion_cycle(&self) -> Result<()> {
        info!("Starting ingestion cycle");

        // Get start version from config
        let start_version = &self.config.uniprot.start_from_version;

        if start_version.is_empty() {
            warn!("No start version configured (INGEST_START_FROM_VERSION), skipping ingestion");
            return Ok(());
        }

        info!("Start version: {}", start_version);

        // Step 1: Discover available versions from UniProt FTP (with timeout)
        let all_versions = self.discover_versions_with_timeout().await?;

        // Filter to versions >= start_version
        let versions_to_check: Vec<_> = all_versions
            .into_iter()
            .filter(|v| v.external_version >= *start_version)
            .collect();

        info!("Versions >= {}: {}", start_version, versions_to_check.len());

        // Step 2: Find versions that need processing (not completed, not currently running)
        let versions_to_ingest = self.filter_versions_to_ingest(versions_to_check).await?;

        if versions_to_ingest.is_empty() {
            info!("No versions to ingest - all up to date!");
            return Ok(());
        }

        info!("Versions to ingest: {}", versions_to_ingest.len());
        for v in &versions_to_ingest {
            info!("  - {} ({})", v.external_version, v.release_date);
        }

        // Step 3: Process each version
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

    /// Filter versions to find those that need processing
    async fn filter_versions_to_ingest(
        &self,
        versions: Vec<DiscoveredVersion>,
    ) -> Result<Vec<DiscoveredVersion>> {
        let mut to_ingest = Vec::new();

        for version in versions {
            // Check job status for this version using raw query (no compile-time checking)
            let job_status: Option<String> = sqlx::query_scalar(
                r#"
                SELECT status
                FROM ingestion_jobs
                WHERE organization_id = $1
                  AND job_type = 'uniprot_swissprot'
                  AND external_version = $2
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(self.org_id)
            .bind(&version.external_version)
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
                Some(status) => {
                    match status.as_str() {
                        // Completed - skip
                        "completed" => false,

                        // Currently running - skip
                        "pending" | "downloading" | "download_verified" | "parsing" | "storing" => {
                            info!(
                                "Version {} is currently {} - skipping",
                                version.external_version, status
                            );
                            false
                        }

                        // Failed - skip (manual intervention needed)
                        "failed" => {
                            warn!(
                                "Version {} failed - requires manual reset to retry",
                                version.external_version
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
                                version.external_version, status
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
                        "Version {} ingested successfully (job: {})",
                        version.external_version, job_id
                    );
                    succeeded += 1;
                }
                Ok(Err(e)) => {
                    error!("Version {} failed: {:#}", version.external_version, e);
                    failed += 1;

                    // Add delay before next version
                    if i < versions.len() - 1 {
                        info!("Waiting {}s before next version...", RETRY_DELAY_SECS);
                        sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
                    }
                }
                Err(_) => {
                    error!(
                        "Version {} timed out after {} seconds",
                        version.external_version, self.config.job_timeout_secs
                    );
                    failed += 1;
                }
            }
        }

        info!(
            "Ingestion cycle completed: {} succeeded, {} failed",
            succeeded, failed
        );

        Ok(())
    }
}
