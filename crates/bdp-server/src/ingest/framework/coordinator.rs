//! Ingestion job coordinator
//!
//! Orchestrates the ETL pipeline:
//! 1. Download phase: Fetch raw files, verify MD5, upload to S3 ingest/
//! 2. Parse phase: Create work units for parallel processing
//! 3. Store phase: Monitor workers, handle completion

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::checksum::{compute_file_md5, verify_file_md5};
use super::metalink::MetalinkInfo;
use super::types::{
    BatchConfig, CreateJobParams, IngestionJob, JobStatus, RawFile, WorkUnitStatus,
};

/// Coordinates ingestion jobs and manages pipeline state
pub struct IngestionCoordinator {
    pool: Arc<PgPool>,
    config: BatchConfig,
}

impl IngestionCoordinator {
    pub fn new(pool: Arc<PgPool>, config: BatchConfig) -> Self {
        Self { pool, config }
    }

    /// Create a new ingestion job
    pub async fn create_job(&self, params: CreateJobParams) -> Result<Uuid> {
        let job_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO ingestion_jobs (
                id, organization_id, job_type, external_version, internal_version,
                source_url, source_metadata, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            job_id,
            params.organization_id,
            params.job_type,
            params.external_version,
            params.internal_version,
            params.source_url,
            params.source_metadata,
            JobStatus::Pending.as_str()
        )
        .execute(&*self.pool)
        .await
        .context("Failed to create ingestion job")?;

        Ok(job_id)
    }

    /// Start download phase
    pub async fn start_download(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = $1, started_at = NOW()
            WHERE id = $2
            "#,
            JobStatus::Downloading.as_str(),
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to downloading")?;

        Ok(())
    }

    /// Register a downloaded raw file
    pub async fn register_raw_file(
        &self,
        job_id: Uuid,
        file_type: &str,
        file_purpose: Option<&str>,
        s3_key: &str,
        expected_md5: Option<&str>,
        size_bytes: i64,
        compression: Option<&str>,
    ) -> Result<Uuid> {
        let file_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO ingestion_raw_files (
                id, job_id, file_type, file_purpose, s3_key,
                expected_md5, size_bytes, compression, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'downloaded')
            "#,
            file_id,
            job_id,
            file_type,
            file_purpose,
            s3_key,
            expected_md5,
            size_bytes,
            compression
        )
        .execute(&*self.pool)
        .await
        .context("Failed to register raw file")?;

        Ok(file_id)
    }

    /// Verify raw file MD5 checksum
    pub async fn verify_raw_file(
        &self,
        file_id: Uuid,
        computed_md5: &str,
        verified: bool,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_raw_files
            SET computed_md5 = $1, verified_md5 = $2, verified_at = NOW()
            WHERE id = $3
            "#,
            computed_md5,
            verified,
            file_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update raw file verification")?;

        Ok(())
    }

    /// Mark download phase as complete and verified
    pub async fn complete_download(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = $1
            WHERE id = $2
            "#,
            JobStatus::DownloadVerified.as_str(),
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to download_verified")?;

        Ok(())
    }

    /// Create work units for parallel parsing
    ///
    /// Splits total_records into batches of parse_batch_size
    pub async fn create_work_units(
        &self,
        job_id: Uuid,
        unit_type: &str,
        total_records: usize,
    ) -> Result<i32> {
        let batch_size = self.config.parse_batch_size;
        let num_batches = (total_records + batch_size - 1) / batch_size;

        for i in 0..num_batches {
            let start_offset = i * batch_size;
            let end_offset = ((i + 1) * batch_size).min(total_records) - 1;

            sqlx::query!(
                r#"
                INSERT INTO ingestion_work_units (
                    id, job_id, unit_type, batch_number,
                    start_offset, end_offset, status, max_retries
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                Uuid::new_v4(),
                job_id,
                unit_type,
                i as i32,
                start_offset as i64,
                end_offset as i64,
                WorkUnitStatus::Pending.as_str(),
                self.config.max_retries
            )
            .execute(&*self.pool)
            .await
            .context("Failed to create work unit")?;
        }

        // Update job with total records
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = $1, total_records = $2
            WHERE id = $3
            "#,
            JobStatus::Parsing.as_str(),
            total_records as i64,
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to parsing")?;

        Ok(num_batches as i32)
    }

    /// Reclaim stale work units from dead workers
    pub async fn reclaim_stale_work_units(&self, worker_timeout_secs: i64) -> Result<i32> {
        let result: (Option<i32>,) = sqlx::query_as(
            "SELECT reclaim_stale_work_units($1)"
        )
        .bind(worker_timeout_secs as i32)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to reclaim stale work units")?;

        Ok(result.0.unwrap_or(0))
    }

    /// Get job progress statistics
    pub async fn get_job_progress(&self, job_id: Uuid) -> Result<JobProgress> {
        let job = sqlx::query!(
            r#"
            SELECT status, total_records, records_processed, records_stored,
                   records_failed, records_skipped
            FROM ingestion_jobs
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_one(&*self.pool)
        .await
        .context("Failed to fetch job progress")?;

        let work_units = sqlx::query!(
            r#"
            SELECT status, COUNT(*) as count
            FROM ingestion_work_units
            WHERE job_id = $1
            GROUP BY status
            "#,
            job_id
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch work unit stats")?;

        let mut pending = 0i64;
        let mut claimed = 0i64;
        let mut processing = 0i64;
        let mut completed = 0i64;
        let mut failed = 0i64;

        for row in work_units {
            let count = row.count.unwrap_or(0);
            match row.status.as_str() {
                "pending" => pending = count,
                "claimed" => claimed = count,
                "processing" => processing = count,
                "completed" => completed = count,
                "failed" => failed = count,
                _ => {}
            }
        }

        Ok(JobProgress {
            status: job.status.into(),
            total_records: job.total_records.unwrap_or(0),
            records_processed: job.records_processed.unwrap_or(0),
            records_stored: job.records_stored.unwrap_or(0),
            records_failed: job.records_failed.unwrap_or(0),
            records_skipped: job.records_skipped.unwrap_or(0),
            work_units_pending: pending,
            work_units_claimed: claimed,
            work_units_processing: processing,
            work_units_completed: completed,
            work_units_failed: failed,
        })
    }

    /// Check if all work units are complete
    pub async fn check_parsing_complete(&self, job_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as incomplete
            FROM ingestion_work_units
            WHERE job_id = $1 AND status NOT IN ('completed', 'cancelled')
            "#,
            job_id
        )
        .fetch_one(&*self.pool)
        .await
        .context("Failed to check parsing completion")?;

        Ok(result.incomplete.unwrap_or(0) == 0)
    }

    /// Transition job to storing phase
    pub async fn start_storing(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = $1
            WHERE id = $2
            "#,
            JobStatus::Storing.as_str(),
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to storing")?;

        Ok(())
    }

    /// Mark job as completed
    pub async fn complete_job(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = $1, completed_at = NOW()
            WHERE id = $2
            "#,
            JobStatus::Completed.as_str(),
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to completed")?;

        Ok(())
    }

    /// Mark job as failed
    pub async fn fail_job(&self, job_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET status = $1, completed_at = NOW(),
                metadata = jsonb_set(COALESCE(metadata, '{}'), '{error}', to_jsonb($2::text))
            WHERE id = $3
            "#,
            JobStatus::Failed.as_str(),
            error_message,
            job_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job status to failed")?;

        Ok(())
    }
}

/// Job progress snapshot
#[derive(Debug, Clone)]
pub struct JobProgress {
    pub status: JobStatus,
    pub total_records: i64,
    pub records_processed: i64,
    pub records_stored: i64,
    pub records_failed: i64,
    pub records_skipped: i64,
    pub work_units_pending: i64,
    pub work_units_claimed: i64,
    pub work_units_processing: i64,
    pub work_units_completed: i64,
    pub work_units_failed: i64,
}

impl JobProgress {
    /// Calculate completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        (self.records_stored as f64 / self.total_records as f64) * 100.0
    }

    /// Calculate work unit completion percentage
    pub fn work_unit_percentage(&self) -> f64 {
        let total = self.work_units_pending
            + self.work_units_claimed
            + self.work_units_processing
            + self.work_units_completed
            + self.work_units_failed;

        if total == 0 {
            return 0.0;
        }
        (self.work_units_completed as f64 / total as f64) * 100.0
    }
}
