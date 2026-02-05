//! Ingestion orchestrator - runs all pipelines in parallel
//!
//! Launches UniProt, NCBI Taxonomy, GenBank, Gene Ontology, and InterPro
//! pipelines concurrently on server startup.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    config::IngestConfig,
    framework::BatchConfig,
    genbank::{GenbankFtpConfig, GenbankOrchestrator},
    gene_ontology::{GoHttpConfig, GoPipeline},
    interpro::{config::InterProConfig, pipeline::InterProPipeline},
    ncbi_taxonomy::{NcbiTaxonomyFtpConfig, NcbiTaxonomyOrchestrator},
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

    /// Start the orchestrator in background - launches all pipelines in parallel
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Ingestion orchestrator started - all pipelines");

            // Initial delay to let server start
            sleep(Duration::from_secs(5)).await;

            let mut set = tokio::task::JoinSet::new();

            // 1. UniProt
            {
                let config = self.config.clone();
                let db = self.db.clone();
                let storage = self.storage.clone();
                let org_id = self.org_id;
                set.spawn(async move { Self::run_uniprot(config, db, storage, org_id).await });
            }

            // 2. NCBI Taxonomy
            if self.config.ncbi_taxonomy_enabled {
                let db = self.db.clone();
                let storage = self.storage.clone();
                let org_id = self.org_id;
                let ncbi_start_date = self.config.ncbi_start_date.clone();
                set.spawn(async move {
                    Self::run_ncbi_taxonomy(db, storage, org_id, ncbi_start_date).await
                });
            } else {
                info!("NCBI Taxonomy pipeline disabled (INGEST_NCBI_ENABLED=false)");
            }

            // 3. GenBank
            if self.config.genbank_enabled {
                let db = self.db.clone();
                let storage = self.storage.clone();
                let org_id = self.org_id;
                set.spawn(async move { Self::run_genbank(db, storage, org_id).await });
            } else {
                info!("GenBank pipeline disabled (INGEST_GENBANK_ENABLED=false)");
            }

            // 4. Gene Ontology
            if self.config.gene_ontology_enabled {
                let db = self.db.clone();
                let storage = self.storage.clone();
                let org_id = self.org_id;
                let go_start_date = self.config.go_start_date.clone();
                set.spawn(async move {
                    Self::run_gene_ontology(db, storage, org_id, go_start_date).await
                });
            } else {
                info!("Gene Ontology pipeline disabled (INGEST_GO_ENABLED=false)");
            }

            // 5. InterPro
            if self.config.interpro_enabled {
                let db = self.db.clone();
                let cache_dir = self.config.uniprot.cache_dir.clone();
                let interpro_start_version = self.config.interpro_start_version.clone();
                set.spawn(async move {
                    Self::run_interpro(db, cache_dir, interpro_start_version).await
                });
            } else {
                info!("InterPro pipeline disabled (INGEST_INTERPRO_ENABLED=false)");
            }

            // Wait for all pipelines, log results
            while let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(name)) => info!("Pipeline '{}' completed successfully", name),
                    Ok(Err(e)) => error!("Pipeline failed: {:#}", e),
                    Err(e) => error!("Pipeline task panicked: {}", e),
                }
            }

            info!("All ingestion pipelines completed");
        })
    }

    // ========================================================================
    // Individual pipeline runners (static to avoid lifetime issues with JoinSet)
    // ========================================================================

    /// Run UniProt ingestion pipeline
    async fn run_uniprot(
        config: IngestConfig,
        db: Arc<PgPool>,
        storage: Storage,
        org_id: Uuid,
    ) -> Result<&'static str> {
        info!("Starting UniProt pipeline");

        let start_version = &config.uniprot.start_from_version;
        if start_version.is_empty() {
            warn!("No start version configured (INGEST_START_FROM_VERSION), skipping UniProt");
            return Ok("uniprot (skipped)");
        }

        info!("UniProt start version: {}", start_version);

        // Discover available versions
        let all_versions = {
            let ftp_config = UniProtFtpConfig::default();
            let discovery = VersionDiscovery::new(ftp_config);

            info!("Discovering available versions from UniProt FTP...");

            let discover_future = discovery.discover_previous_versions_only();
            let timeout_duration = Duration::from_secs(FTP_OPERATION_TIMEOUT_SECS);

            match timeout(timeout_duration, discover_future).await {
                Ok(Ok(versions)) => {
                    info!("Found {} historical versions", versions.len());
                    versions
                },
                Ok(Err(e)) => {
                    return Err(e).context("Failed to discover versions from UniProt FTP");
                },
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "FTP discovery timed out after {} seconds",
                        FTP_OPERATION_TIMEOUT_SECS
                    ));
                },
            }
        };

        // Filter to versions >= start_version
        let versions_to_check: Vec<_> = all_versions
            .into_iter()
            .filter(|v| v.external_version >= *start_version)
            .collect();

        info!("UniProt versions >= {}: {}", start_version, versions_to_check.len());

        // Find versions that need processing
        let versions_to_ingest =
            Self::filter_uniprot_versions(&db, org_id, versions_to_check).await?;

        if versions_to_ingest.is_empty() {
            info!("UniProt: no versions to ingest - all up to date!");
            return Ok("uniprot");
        }

        info!("UniProt versions to ingest: {}", versions_to_ingest.len());
        for v in &versions_to_ingest {
            info!("  - {} ({})", v.external_version, v.release_date);
        }

        // Process each version
        let ftp_config = UniProtFtpConfig::default();
        let batch_config = BatchConfig::default();
        let cache_dir = config.uniprot.cache_dir.clone();

        let pipeline =
            UniProtPipeline::new(db, org_id, ftp_config, batch_config, storage, cache_dir);

        let mut succeeded = 0;
        let mut failed = 0;

        for (i, version) in versions_to_ingest.iter().enumerate() {
            info!(
                "[{}/{}] UniProt: ingesting version {}",
                i + 1,
                versions_to_ingest.len(),
                version.external_version
            );

            let ingest_timeout = Duration::from_secs(config.job_timeout_secs);
            let ingest_future = pipeline.ingest_version(version);

            match timeout(ingest_timeout, ingest_future).await {
                Ok(Ok(job_id)) => {
                    info!(
                        "UniProt version {} ingested successfully (job: {})",
                        version.external_version, job_id
                    );
                    succeeded += 1;
                },
                Ok(Err(e)) => {
                    error!("UniProt version {} failed: {:#}", version.external_version, e);
                    failed += 1;

                    if i < versions_to_ingest.len() - 1 {
                        info!("Waiting {}s before next version...", RETRY_DELAY_SECS);
                        sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
                    }
                },
                Err(_) => {
                    error!(
                        "UniProt version {} timed out after {} seconds",
                        version.external_version, config.job_timeout_secs
                    );
                    failed += 1;
                },
            }
        }

        info!("UniProt pipeline completed: {} succeeded, {} failed", succeeded, failed);

        Ok("uniprot")
    }

    /// Run NCBI Taxonomy ingestion pipeline
    async fn run_ncbi_taxonomy(
        db: Arc<PgPool>,
        storage: Storage,
        org_id: Uuid,
        ncbi_start_date: String,
    ) -> Result<&'static str> {
        info!("Starting NCBI Taxonomy pipeline");

        let config = NcbiTaxonomyFtpConfig::default();
        let orchestrator = NcbiTaxonomyOrchestrator::with_s3(config, (*db).clone(), storage);

        let start_date = if ncbi_start_date.is_empty() {
            None
        } else {
            info!(start_date = %ncbi_start_date, "NCBI Taxonomy: using configured start date");
            Some(ncbi_start_date)
        };

        let results = orchestrator
            .catchup_and_current(org_id, start_date.as_deref())
            .await?;

        info!("NCBI Taxonomy pipeline completed: {} versions processed", results.len());

        Ok("ncbi_taxonomy")
    }

    /// Run GenBank ingestion pipeline
    async fn run_genbank(db: Arc<PgPool>, storage: Storage, org_id: Uuid) -> Result<&'static str> {
        info!("Starting GenBank pipeline");

        let concurrency: usize = std::env::var("INGEST_GENBANK_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let batch_size: usize = std::env::var("INGEST_GENBANK_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500);

        let config = GenbankFtpConfig::default()
            .with_concurrency(concurrency)
            .with_batch_size(batch_size);
        info!(concurrency, batch_size, "GenBank config loaded from env");
        let orchestrator = GenbankOrchestrator::new(config, (*db).clone(), storage);

        let result = orchestrator.run_release(org_id).await?;

        info!("GenBank pipeline completed: {:?}", result);

        Ok("genbank")
    }

    /// Run Gene Ontology ingestion pipeline
    async fn run_gene_ontology(
        db: Arc<PgPool>,
        storage: Storage,
        org_id: Uuid,
        go_start_date: String,
    ) -> Result<&'static str> {
        info!("Starting Gene Ontology pipeline");

        if go_start_date.is_empty() {
            // Default: just ingest latest (current) version
            info!("Gene Ontology: ingesting latest version");
            let config = GoHttpConfig::default();
            let pipeline = GoPipeline::new(config, (*db).clone(), storage, org_id);
            let stats = pipeline.run_full("1.0").await?;
            info!("Gene Ontology pipeline completed: {:?}", stats);
        } else {
            // Use the start date as a specific version to ingest from
            info!(
                start_date = %go_start_date,
                "Gene Ontology: ingesting specific version"
            );
            let config = GoHttpConfig {
                go_release_version: go_start_date,
                ..GoHttpConfig::default()
            };
            let pipeline = GoPipeline::new(config, (*db).clone(), storage, org_id);
            let stats = pipeline.run_full("1.0").await?;
            info!("Gene Ontology pipeline completed: {:?}", stats);
        }

        Ok("gene_ontology")
    }

    /// Run InterPro ingestion pipeline
    async fn run_interpro(
        db: Arc<PgPool>,
        cache_dir: std::path::PathBuf,
        interpro_start_version: String,
    ) -> Result<&'static str> {
        info!("Starting InterPro pipeline");

        let config = InterProConfig::default();
        let pipeline = InterProPipeline::new((*db).clone(), config, cache_dir);

        if interpro_start_version.is_empty() {
            // Default: just ingest latest version
            match pipeline.ingest_latest().await? {
                Some((version, stats)) => {
                    info!("InterPro pipeline completed: version {} ({:?})", version, stats);
                },
                None => {
                    info!("InterPro pipeline: no new versions to ingest");
                },
            }
        } else {
            info!(
                start_version = %interpro_start_version,
                "InterPro: ingesting all versions from start version"
            );
            let results = pipeline
                .ingest_from_version(&interpro_start_version, true)
                .await?;
            info!("InterPro pipeline completed: {} versions ingested", results.len());
        }

        Ok("interpro")
    }

    // ========================================================================
    // UniProt helpers
    // ========================================================================

    /// Filter UniProt versions to find those that need processing
    async fn filter_uniprot_versions(
        db: &PgPool,
        org_id: Uuid,
        versions: Vec<DiscoveredVersion>,
    ) -> Result<Vec<DiscoveredVersion>> {
        let mut to_ingest = Vec::new();

        for version in versions {
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
            .bind(org_id)
            .bind(&version.external_version)
            .fetch_optional(db)
            .await
            .context("Failed to check job status")?;

            let should_ingest = match job_status {
                None => {
                    info!("Version {} has no job record - will ingest", version.external_version);
                    true
                },
                Some(status) => match status.as_str() {
                    "completed" => false,
                    "pending" | "downloading" | "download_verified" | "parsing" | "storing" => {
                        info!(
                            "Version {} is currently {} - skipping",
                            version.external_version, status
                        );
                        false
                    },
                    "failed" => {
                        warn!(
                            "Version {} failed - requires manual reset to retry",
                            version.external_version
                        );
                        false
                    },
                    "cancelled" => {
                        info!("Version {} was cancelled - will retry", version.external_version);
                        true
                    },
                    _ => {
                        warn!(
                            "Version {} has unknown status '{}' - will retry",
                            version.external_version, status
                        );
                        true
                    },
                },
            };

            if should_ingest {
                to_ingest.push(version);
            }
        }

        Ok(to_ingest)
    }
}
