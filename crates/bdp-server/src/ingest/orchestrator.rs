//! Simple ingestion orchestrator
//!
//! Discovers versions from UniProt, filters to missing versions, and ingests them.
//! Bypasses apalis job queue for simplicity - just runs directly in background task.

use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    config::IngestConfig,
    framework::BatchConfig,
    uniprot::{DiscoveredVersion, UniProtFtpConfig, UniProtPipeline, VersionDiscovery},
};
use crate::storage::Storage;

/// Simple ingestion orchestrator
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

            // Run once
            if let Err(e) = self.run_ingestion_cycle().await {
                error!("Ingestion cycle failed: {}", e);
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

        // Discover available versions from UniProt FTP
        let ftp_config = UniProtFtpConfig::default();
        let discovery = VersionDiscovery::new(ftp_config.clone());

        info!("Discovering available versions from UniProt FTP...");
        let all_versions = discovery.discover_previous_versions_only().await?;
        info!("Found {} historical versions", all_versions.len());

        // Filter to versions >= start_version
        let versions_to_check: Vec<_> = all_versions
            .into_iter()
            .filter(|v| v.external_version >= *start_version)
            .collect();

        info!("Versions >= {}: {}", start_version, versions_to_check.len());

        // Check which versions are already ingested
        let missing_versions = self.filter_missing_versions(versions_to_check).await?;

        if missing_versions.is_empty() {
            info!("No missing versions to ingest!");
            return Ok(());
        }

        info!("Missing versions to ingest: {}", missing_versions.len());
        for v in &missing_versions {
            info!("  - {} ({})", v.external_version, v.release_date);
        }

        // Ingest each missing version
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

        for version in missing_versions {
            info!("Ingesting version: {}", version.external_version);

            match pipeline.ingest_version(&version).await {
                Ok(job_id) => {
                    info!(
                        "✓ Version {} ingested successfully (job: {})",
                        version.external_version, job_id
                    );
                    succeeded += 1;
                },
                Err(e) => {
                    error!("✗ Version {} failed: {}", version.external_version, e);
                    failed += 1;
                },
            }
        }

        info!("Ingestion cycle completed: {} succeeded, {} failed", succeeded, failed);

        Ok(())
    }

    /// Filter out versions that are already in the database
    async fn filter_missing_versions(
        &self,
        versions: Vec<DiscoveredVersion>,
    ) -> Result<Vec<DiscoveredVersion>> {
        let mut missing = Vec::new();

        for version in versions {
            // Check if this version exists in ingestion_jobs
            let exists = sqlx::query!(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM ingestion_jobs
                    WHERE organization_id = $1
                      AND job_type = 'uniprot_swissprot'
                      AND external_version = $2
                      AND status = 'completed'
                ) as "exists!"
                "#,
                self.org_id,
                version.external_version
            )
            .fetch_one(&*self.db)
            .await?
            .exists;

            if !exists {
                missing.push(version);
            }
        }

        Ok(missing)
    }
}
