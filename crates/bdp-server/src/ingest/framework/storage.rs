//! Generic storage adapter interface
//!
//! Provides trait for type-specific storage implementations.
//! Each data source type implements StorageAdapter to insert records
//! into their specific tables (proteins, genomes, compounds, etc.).

use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::types::{RecordStatus, StagedRecord};

/// Generic storage adapter for persisting records to final tables
#[async_trait]
pub trait StorageAdapter: Send + Sync {
    /// Get the record type this adapter handles
    fn record_type(&self) -> &str;

    /// Get supported output formats for this adapter
    ///
    /// Returns list of formats that can be generated (e.g., ["fasta", "json"])
    fn supported_formats(&self) -> Vec<String>;

    /// Store a batch of records to final tables
    ///
    /// This method should:
    /// 1. Insert records into type-specific tables (proteins, genomes, etc.)
    /// 2. Create data_sources entries
    /// 3. Link version_files
    /// 4. Return list of successfully stored record IDs
    async fn store_batch(&self, records: Vec<StagedRecord>) -> Result<Vec<Uuid>>;

    /// Upload generated files to S3 (FASTA, JSON, etc.)
    ///
    /// This method should:
    /// 1. Generate file content in specified format
    /// 2. Upload to S3 at correct path: {org}/{accession}/{version}/{file}
    /// 3. Compute MD5 of uploaded file
    /// 4. Record in ingestion_file_uploads table
    /// 5. Update data_sources with MD5s
    async fn upload_files(&self, record_id: Uuid, formats: Vec<String>) -> Result<Vec<Uuid>>;

    /// Mark record as stored
    async fn mark_stored(&self, staged_record_id: Uuid) -> Result<()>;
}

/// Storage orchestrator that manages batch processing
pub struct StorageOrchestrator {
    pool: Arc<PgPool>,
    batch_size: usize,
}

impl StorageOrchestrator {
    pub fn new(pool: Arc<PgPool>, batch_size: usize) -> Self {
        Self { pool, batch_size }
    }

    /// Fetch staged records ready for storage
    pub async fn fetch_staged_records(
        &self,
        job_id: Uuid,
        limit: i64,
    ) -> Result<Vec<StagedRecord>> {
        let records = sqlx::query_as!(
            StagedRecordRow,
            r#"
            SELECT
                id, job_id, work_unit_id, record_type, record_identifier,
                record_name, record_data, content_md5, sequence_md5,
                source_file, source_offset, parsed_at, status,
                stored_at, error_message
            FROM ingestion_staged_records
            WHERE job_id = $1 AND status = 'parsed'
            ORDER BY parsed_at
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            "#,
            job_id,
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(records.into_iter().map(|r| r.into()).collect())
    }

    /// Process staged records with adapter
    pub async fn process_batch<A: StorageAdapter>(
        &self,
        job_id: Uuid,
        adapter: &A,
    ) -> Result<usize> {
        // Fetch batch
        let staged = self
            .fetch_staged_records(job_id, self.batch_size as i64)
            .await?;

        if staged.is_empty() {
            return Ok(0);
        }

        let count = staged.len();

        // Mark all as uploading files in parallel
        let status_futures: Vec<_> = staged
            .iter()
            .map(|record| self.mark_status(record.id, RecordStatus::UploadingFiles))
            .collect();
        try_join_all(status_futures).await?;

        // Upload files for each record in parallel
        // Note: We use a semaphore-like approach by processing in chunks to avoid overwhelming S3
        let formats = adapter.supported_formats();
        let upload_results: Vec<(Uuid, Result<Vec<Uuid>>)> = {
            let upload_futures: Vec<_> = staged
                .iter()
                .map(|record| {
                    let record_id = record.id;
                    let formats = formats.clone();
                    async move { (record_id, adapter.upload_files(record_id, formats).await) }
                })
                .collect();
            futures::future::join_all(upload_futures).await
        };

        // Process upload results: mark successful uploads and failed ones
        let mut successful_ids = Vec::new();
        for (record_id, result) in upload_results {
            match result {
                Ok(_) => {
                    self.mark_status(record_id, RecordStatus::FilesUploaded)
                        .await?;
                    successful_ids.push(record_id);
                },
                Err(e) => {
                    self.mark_failed(record_id, &e.to_string()).await?;
                },
            }
        }

        // Filter staged records to only those that were successfully uploaded
        let uploaded: Vec<_> = staged
            .into_iter()
            .filter(|record| successful_ids.contains(&record.id))
            .collect();

        if uploaded.is_empty() {
            return Ok(0);
        }

        // Mark as storing in DB in parallel
        let status_futures: Vec<_> = uploaded
            .iter()
            .map(|record| self.mark_status(record.id, RecordStatus::StoringDb))
            .collect();
        try_join_all(status_futures).await?;

        // Clone uploaded for error handling
        let uploaded_ids: Vec<Uuid> = uploaded.iter().map(|r| r.id).collect();

        // Store batch to final tables
        match adapter.store_batch(uploaded).await {
            Ok(stored_ids) => {
                // Mark all as stored in parallel
                let status_futures: Vec<_> = stored_ids
                    .iter()
                    .map(|id| self.mark_status(*id, RecordStatus::Stored))
                    .collect();
                try_join_all(status_futures).await?;

                // Update job counters
                self.update_job_stored_count(job_id, count as i64).await?;

                Ok(count)
            },
            Err(e) => {
                // Mark all as failed in parallel
                let error_msg = e.to_string();
                let fail_futures: Vec<_> = uploaded_ids
                    .iter()
                    .map(|id| self.mark_failed(*id, &error_msg))
                    .collect();
                let _ = futures::future::join_all(fail_futures).await;
                Err(e)
            },
        }
    }

    /// Mark record with new status
    async fn mark_status(&self, record_id: Uuid, status: RecordStatus) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_staged_records
            SET status = $1
            WHERE id = $2
            "#,
            status.as_str(),
            record_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Mark record as failed
    async fn mark_failed(&self, record_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_staged_records
            SET status = 'failed', error_message = $1
            WHERE id = $2
            "#,
            error_message,
            record_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Update job stored record count
    async fn update_job_stored_count(&self, job_id: Uuid, count: i64) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE ingestion_jobs
            SET records_stored = records_stored + $1
            WHERE id = $2
            "#,
            count,
            job_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Run storage loop until all records processed
    pub async fn run<A: StorageAdapter>(&self, job_id: Uuid, adapter: &A) -> Result<usize> {
        let mut total_stored = 0;

        loop {
            let stored = self.process_batch(job_id, adapter).await?;

            if stored == 0 {
                break;
            }

            total_stored += stored;
            tracing::info!(job_id = %job_id, batch_stored = stored, total_stored, "Batch stored");
        }

        Ok(total_stored)
    }
}

// Helper struct for sqlx query_as
#[derive(Debug)]
struct StagedRecordRow {
    id: Uuid,
    job_id: Uuid,
    work_unit_id: Option<Uuid>,
    record_type: String,
    record_identifier: String,
    record_name: Option<String>,
    record_data: serde_json::Value,
    content_md5: Option<String>,
    sequence_md5: Option<String>,
    source_file: Option<String>,
    source_offset: Option<i64>,
    parsed_at: Option<chrono::DateTime<chrono::Utc>>,
    status: String,
    stored_at: Option<chrono::DateTime<chrono::Utc>>,
    error_message: Option<String>,
}

impl From<StagedRecordRow> for StagedRecord {
    fn from(row: StagedRecordRow) -> Self {
        Self {
            id: row.id,
            job_id: row.job_id,
            work_unit_id: row.work_unit_id,
            record_type: row.record_type,
            record_identifier: row.record_identifier,
            record_name: row.record_name,
            record_data: row.record_data,
            content_md5: row.content_md5,
            sequence_md5: row.sequence_md5,
            source_file: row.source_file,
            source_offset: row.source_offset,
            parsed_at: row.parsed_at.unwrap_or_else(|| chrono::Utc::now()),
            status: match row.status.as_str() {
                "staged" => RecordStatus::Staged,
                "uploading_files" => RecordStatus::UploadingFiles,
                "files_uploaded" => RecordStatus::FilesUploaded,
                "storing_db" => RecordStatus::StoringDb,
                "stored" => RecordStatus::Stored,
                "failed" => RecordStatus::Failed,
                _ => RecordStatus::Staged,
            },
            stored_at: row.stored_at,
            error_message: row.error_message,
        }
    }
}
