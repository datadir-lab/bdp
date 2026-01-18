//! Ingestion worker for parallel batch processing
//!
//! Workers claim work units atomically, parse batches of records,
//! and maintain heartbeat for fault tolerance.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use uuid::Uuid;

use super::checksum::compute_md5;
use super::parser::DataSourceParser;
use super::types::{
    BatchConfig, ClaimedWorkUnit, GenericRecord, RecordStatus, WorkUnitStatus,
};

/// Worker for processing ingestion work units
pub struct IngestionWorker {
    worker_id: Uuid,
    hostname: String,
    pool: Arc<PgPool>,
    config: BatchConfig,
}

impl IngestionWorker {
    pub fn new(pool: Arc<PgPool>, config: BatchConfig) -> Self {
        Self {
            worker_id: Uuid::new_v4(),
            hostname: hostname::get()
                .unwrap_or_else(|_| "unknown".into())
                .to_string_lossy()
                .to_string(),
            pool,
            config,
        }
    }

    pub fn worker_id(&self) -> Uuid {
        self.worker_id
    }

    /// Claim a pending work unit atomically
    pub async fn claim_work_unit(&self, job_id: Uuid) -> Result<Option<ClaimedWorkUnit>> {
        let result: Option<(Option<Uuid>, Option<i32>, Option<i64>, Option<i64>, Option<i32>)> = sqlx::query_as(
            "SELECT * FROM claim_work_unit($1, $2, $3)"
        )
        .bind(job_id)
        .bind(self.worker_id)
        .bind(&self.hostname)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to claim work unit")?;

        Ok(result.map(|(id, batch_number, start_offset, end_offset, record_count)| ClaimedWorkUnit {
            id: id.unwrap(),
            batch_number: batch_number.unwrap(),
            start_offset: start_offset.unwrap(),
            end_offset: end_offset.unwrap(),
            record_count,
        }))
    }

    /// Update work unit heartbeat
    pub async fn heartbeat(&self, work_unit_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_work_units
            SET heartbeat_at = NOW()
            WHERE id = $1
            "#,
            work_unit_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update heartbeat")?;

        Ok(())
    }

    /// Start heartbeat task for a work unit
    pub fn start_heartbeat_task(
        &self,
        work_unit_id: Uuid,
    ) -> tokio::task::JoinHandle<Result<()>> {
        let pool = self.pool.clone();
        let interval_secs = self.config.heartbeat_interval_secs;

        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_secs(interval_secs));

            loop {
                interval_timer.tick().await;

                sqlx::query!(
                    r#"
                    UPDATE ingestion_work_units
                    SET heartbeat_at = NOW()
                    WHERE id = $1
                    "#,
                    work_unit_id
                )
                .execute(&*pool)
                .await?;
            }
        })
    }

    /// Mark work unit as processing
    pub async fn start_processing(&self, work_unit_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_work_units
            SET status = $1, started_processing_at = NOW()
            WHERE id = $2
            "#,
            WorkUnitStatus::Processing.as_str(),
            work_unit_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to mark work unit as processing")?;

        Ok(())
    }

    /// Process a work unit: parse records and stage them
    pub async fn process_work_unit<P: DataSourceParser>(
        &self,
        job_id: Uuid,
        work_unit: &ClaimedWorkUnit,
        parser: &P,
        raw_data: &[u8],
    ) -> Result<Vec<Uuid>> {
        // Start heartbeat task
        let heartbeat_handle = self.start_heartbeat_task(work_unit.id);

        // Mark as processing
        self.start_processing(work_unit.id).await?;

        // Parse the batch
        let records = parser
            .parse_range(
                raw_data,
                work_unit.start_offset as usize,
                work_unit.end_offset as usize,
            )
            .await
            .context("Failed to parse records")?;

        // Stage records in batches
        let mut staged_ids = Vec::new();
        for chunk in records.chunks(self.config.store_batch_size) {
            let batch_ids = self.stage_records(job_id, work_unit.id, chunk).await?;
            staged_ids.extend(batch_ids);
        }

        // Update work unit progress
        self.update_work_unit_progress(
            work_unit.id,
            records.len() as i32,
            records.len() as i32,
            0,
        )
        .await?;

        // Mark work unit as completed
        self.complete_work_unit(work_unit.id).await?;

        // Stop heartbeat
        heartbeat_handle.abort();

        Ok(staged_ids)
    }

    /// Stage records in database
    async fn stage_records(
        &self,
        job_id: Uuid,
        work_unit_id: Uuid,
        records: &[GenericRecord],
    ) -> Result<Vec<Uuid>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        let mut staged_ids = Vec::new();

        for record in records {
            let record_id = Uuid::new_v4();
            let content_md5 = record.content_md5.clone().unwrap_or_else(|| {
                compute_md5(
                    serde_json::to_string(&record.record_data)
                        .unwrap()
                        .as_bytes(),
                )
            });

            sqlx::query!(
                r#"
                INSERT INTO ingestion_staged_records (
                    id, job_id, work_unit_id, record_type, record_identifier,
                    record_name, record_data, content_md5, sequence_md5,
                    source_file, source_offset, status
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#,
                record_id,
                job_id,
                Some(work_unit_id),
                record.record_type,
                record.record_identifier.to_lowercase(),
                record.record_name.as_ref().map(|n| n.to_lowercase()),
                record.record_data,
                content_md5,
                record.sequence_md5,
                record.source_file,
                record.source_offset,
                RecordStatus::Staged.as_str()
            )
            .execute(&mut *tx)
            .await
            .context("Failed to insert staged record")?;

            staged_ids.push(record_id);
        }

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(staged_ids)
    }

    /// Update work unit progress counters
    async fn update_work_unit_progress(
        &self,
        work_unit_id: Uuid,
        record_count: i32,
        records_processed: i32,
        records_failed: i32,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_work_units
            SET record_count = $1
            WHERE id = $2
            "#,
            record_count,
            work_unit_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update work unit progress")?;

        // Update job counters
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET records_processed = records_processed + $1,
                records_failed = records_failed + $2
            WHERE id = (SELECT job_id FROM ingestion_work_units WHERE id = $3)
            "#,
            records_processed as i64,
            records_failed as i64,
            work_unit_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update job progress")?;

        Ok(())
    }

    /// Mark work unit as completed
    pub async fn complete_work_unit(&self, work_unit_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_work_units
            SET status = $1,
                completed_at = NOW(),
                processing_duration_ms = EXTRACT(EPOCH FROM (NOW() - started_processing_at)) * 1000
            WHERE id = $2
            "#,
            WorkUnitStatus::Completed.as_str(),
            work_unit_id
        )
        .execute(&*self.pool)
        .await
        .context("Failed to mark work unit as completed")?;

        Ok(())
    }

    /// Mark work unit as failed
    pub async fn fail_work_unit(&self, work_unit_id: Uuid, error_message: &str) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE ingestion_work_units
            SET status = CASE
                    WHEN retry_count + 1 >= max_retries THEN 'failed'::text
                    ELSE 'pending'::text
                END,
                retry_count = retry_count + 1,
                last_error = $1,
                worker_id = NULL,
                worker_hostname = NULL,
                claimed_at = NULL,
                heartbeat_at = NULL
            WHERE id = $2
            RETURNING retry_count, max_retries
            "#,
            error_message,
            work_unit_id
        )
        .fetch_one(&*self.pool)
        .await
        .context("Failed to mark work unit as failed")?;

        if result.retry_count >= result.max_retries {
            tracing::error!(
                work_unit_id = %work_unit_id,
                error = %error_message,
                "Work unit failed after max retries"
            );
        } else {
            tracing::warn!(
                work_unit_id = %work_unit_id,
                retry_count = result.retry_count,
                max_retries = result.max_retries,
                error = %error_message,
                "Work unit failed, will retry"
            );
        }

        Ok(())
    }

    /// Run worker loop for a job
    pub async fn run<P: DataSourceParser>(
        &self,
        job_id: Uuid,
        parser: &P,
        raw_data: &[u8],
    ) -> Result<()> {
        loop {
            // Try to claim a work unit
            let work_unit = match self.claim_work_unit(job_id).await? {
                Some(wu) => wu,
                None => {
                    tracing::info!(job_id = %job_id, "No more work units available");
                    break;
                }
            };

            tracing::info!(
                work_unit_id = %work_unit.id,
                batch_number = work_unit.batch_number,
                start_offset = work_unit.start_offset,
                end_offset = work_unit.end_offset,
                "Processing work unit"
            );

            // Process the work unit
            match self.process_work_unit(job_id, &work_unit, parser, raw_data).await {
                Ok(staged_ids) => {
                    tracing::info!(
                        work_unit_id = %work_unit.id,
                        records_staged = staged_ids.len(),
                        "Work unit completed successfully"
                    );
                }
                Err(e) => {
                    self.fail_work_unit(work_unit.id, &e.to_string()).await?;
                }
            }
        }

        Ok(())
    }
}
