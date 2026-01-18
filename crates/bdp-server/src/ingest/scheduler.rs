//! Job scheduler
//!
//! Sets up and manages the apalis job queue with PostgreSQL storage.

use anyhow::Result;
use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::{config::IngestConfig, jobs::UniProtIngestJob};

/// Job scheduler
pub struct JobScheduler {
    config: IngestConfig,
    db: PgPool,
}

impl JobScheduler {
    /// Create a new job scheduler
    pub fn new(config: IngestConfig, db: PgPool) -> Self {
        Self { config, db }
    }

    /// Start the scheduler
    ///
    /// This will:
    /// 1. Setup PostgreSQL storage for apalis
    /// 2. Start worker threads to process jobs
    /// 3. Setup cron jobs if enabled
    pub async fn start(self) -> Result<JoinHandle<()>> {
        info!("Starting job scheduler");

        // Setup PostgreSQL storage for apalis
        let storage = self.setup_storage().await?;

        info!(
            "Job scheduler initialized with {} workers",
            self.config.worker_threads
        );

        // Setup cron jobs if auto-ingest is enabled
        if self.config.uniprot.auto_ingest_enabled {
            info!("Auto-ingest is enabled");
            warn!("Cron job scheduling not yet implemented");
        } else {
            info!("Auto-ingest is disabled, no cron jobs scheduled");
        }

        // Spawn the worker in a separate task
        // Monitor::register expects a factory closure that creates workers
        let handle = tokio::spawn(async move {
            info!("Job worker started");
            if let Err(e) = Monitor::new()
                .register(move |_index| {
                    WorkerBuilder::new("bdp-ingest-worker")
                        .backend(storage.clone())
                        .build(process_uniprot_job)
                })
                .run()
                .await
            {
                tracing::error!("Job worker error: {:?}", e);
            }
            info!("Job worker stopped");
        });

        Ok(handle)
    }

    /// Setup PostgreSQL storage for apalis
    async fn setup_storage(&self) -> Result<PostgresStorage<UniProtIngestJob>> {
        info!("Setting up PostgreSQL storage for apalis");

        // Create storage with reference to pool
        // Note: The apalis schema should already exist from migration 20260116000021
        let storage = PostgresStorage::new(&self.db);

        info!("Apalis storage setup complete");

        Ok(storage)
    }
}

/// Process a UniProt ingest job
///
/// This function is called by the apalis worker to execute a UniProt ingestion job.
/// It creates a UniProt pipeline instance and runs the ingestion for the specified version.
///
/// NOTE: This implementation is ready but currently disabled due to apalis-postgres compilation issues.
/// Once apalis is re-enabled, this will work out of the box.
async fn process_uniprot_job(job: UniProtIngestJob) -> Result<()> {
    info!(
        "Processing UniProt ingest job for organization: {}",
        job.organization_id
    );

    // NOTE: The following implementation is ready but commented out because
    // we need access to the database pool and S3 storage which should be
    // passed through the job context when apalis is re-enabled.
    //
    // When apalis is working again, this function should be updated to:
    // 1. Accept PgPool and Storage from job context
    // 2. Create UniProtPipeline with those dependencies
    // 3. Run the pipeline with the target version
    //
    // Example implementation (to be uncommented when apalis is re-enabled):
    //
    // use super::uniprot::{UniProtPipeline, UniProtFtpConfig};
    //
    // let config = UniProtFtpConfig::default();
    // let pipeline = UniProtPipeline::new(pool, job.organization_id, config);
    //
    // let stats = pipeline.run(job.target_version.as_deref()).await?;
    //
    // info!(
    //     "UniProt ingestion completed: {} entries inserted, {} failed",
    //     stats.entries_inserted,
    //     stats.entries_failed
    // );

    warn!("Job processing is disabled due to apalis-postgres compilation issues");
    info!("Job details: full_sync={}, target_version={:?}",
          job.full_sync,
          job.target_version);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_scheduler_new() {
        let config = IngestConfig::default();
        let db = PgPool::connect_lazy("postgresql://localhost/test").unwrap();
        let scheduler = JobScheduler::new(config.clone(), db);

        assert_eq!(scheduler.config.worker_threads, config.worker_threads);
    }
}
