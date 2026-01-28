//! UniProt storage adapter for generic ETL framework
//!
//! Implements StorageAdapter trait to persist proteins to final tables.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::parser_adapter::UniProtFormatter;
use crate::ingest::framework::{compute_md5, RecordFormatter, StagedRecord, StorageAdapter};

/// UniProt storage adapter
pub struct UniProtStorageAdapter {
    pool: Arc<PgPool>,
    organization_id: Uuid,
    s3_client: Arc<aws_sdk_s3::Client>,
    s3_bucket: String,
    formatter: UniProtFormatter,
}

impl UniProtStorageAdapter {
    pub fn new(
        pool: Arc<PgPool>,
        organization_id: Uuid,
        s3_client: Arc<aws_sdk_s3::Client>,
        s3_bucket: String,
    ) -> Self {
        Self {
            pool,
            organization_id,
            s3_client,
            s3_bucket,
            formatter: UniProtFormatter,
        }
    }
}

#[async_trait]
impl StorageAdapter for UniProtStorageAdapter {
    fn record_type(&self) -> &str {
        "protein"
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["fasta".to_string(), "json".to_string()]
    }

    async fn store_batch(&self, records: Vec<StagedRecord>) -> Result<Vec<Uuid>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        let mut stored_ids = Vec::new();

        for record in records {
            // Extract protein-specific fields
            let accession = record
                .record_data
                .get("accession")
                .and_then(|v| v.as_str())
                .context("Missing accession")?
                .to_lowercase();

            let entry_name = record
                .record_data
                .get("entry_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase());

            let _organism = record.record_data.get("organism").and_then(|v| v.as_str());

            let _taxonomy_id = record
                .record_data
                .get("taxonomy_id")
                .and_then(|v| v.as_i64())
                .map(|i| i as i32);

            let _sequence = record.record_data.get("sequence").and_then(|v| v.as_str());

            let _sequence_length = record
                .record_data
                .get("sequence_length")
                .and_then(|v| v.as_i64())
                .map(|i| i as i32);

            // Check if registry_entry exists
            let registry_entry_id: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM registry_entries WHERE slug = $1 AND organization_id = $2",
            )
            .bind(&accession)
            .bind(self.organization_id)
            .fetch_optional(&mut *tx)
            .await?;

            let _data_source_id = if let Some(id) = registry_entry_id {
                id
            } else {
                // Create registry_entry first
                let entry_id = Uuid::new_v4();
                sqlx::query(
                    r#"
                    INSERT INTO registry_entries (
                        id, organization_id, slug, name, entry_type
                    )
                    VALUES ($1, $2, $3, $4, 'data_source')
                    "#,
                )
                .bind(entry_id)
                .bind(self.organization_id)
                .bind(&accession)
                .bind(entry_name.as_deref().unwrap_or(&accession))
                .execute(&mut *tx)
                .await
                .context("Failed to create registry_entry")?;

                // Create data_source
                sqlx::query(
                    r#"
                    INSERT INTO data_sources (
                        id, source_type, external_id
                    )
                    VALUES ($1, 'protein', $2)
                    "#,
                )
                .bind(entry_id)
                .bind(&accession)
                .execute(&mut *tx)
                .await
                .context("Failed to create data_source")?;

                entry_id
            };

            // For now, we just ensure the registry entry exists
            // The actual protein-specific table will be created in a future migration
            // The full data is already in record.record_data JSONB

            stored_ids.push(record.id);
        }

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(stored_ids)
    }

    async fn upload_files(
        &self,
        staged_record_id: Uuid,
        formats: Vec<String>,
    ) -> Result<Vec<Uuid>> {
        // Fetch the staged record (dynamic query)
        let record: (String, serde_json::Value, Uuid) = sqlx::query_as(
            "SELECT record_identifier, record_data, job_id FROM ingestion_staged_records WHERE id = $1"
        )
        .bind(staged_record_id)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to fetch staged record")?;

        let (record_identifier, record_data, job_id) = record;

        // Get internal version from job (dynamic query)
        let internal_version: String =
            sqlx::query_scalar("SELECT internal_version FROM ingestion_jobs WHERE id = $1")
                .bind(job_id)
                .fetch_one(&*self.pool)
                .await
                .context("Failed to fetch job")?;

        // Convert to GenericRecord
        let generic_record = crate::ingest::framework::GenericRecord {
            record_type: "protein".to_string(),
            record_identifier: record_identifier.clone(),
            record_name: None,
            record_data,
            content_md5: None,
            sequence_md5: None,
            source_file: None,
            source_offset: None,
        };

        let mut upload_ids = Vec::new();

        for format in formats {
            // Generate file content
            let (content, content_type) = self
                .formatter
                .format_record(&generic_record, &format)
                .await
                .context("Failed to format record")?;

            // Compute MD5
            let md5 = compute_md5(&content);

            // Get organization slug from database
            let org_slug: String =
                sqlx::query_scalar("SELECT slug FROM organizations WHERE id = $1")
                    .bind(self.organization_id)
                    .fetch_one(&*self.pool)
                    .await
                    .context("Failed to fetch organization slug")?;

            // S3 key: {org}/{accession}/{version}/{accession}.{format}
            let s3_key = format!(
                "{}/{}/{}/{}.{}",
                org_slug, record_identifier, internal_version, record_identifier, format
            );

            // Upload to S3
            self.s3_client
                .put_object()
                .bucket(&self.s3_bucket)
                .key(&s3_key)
                .body(content.clone().into())
                .content_type(&content_type)
                .send()
                .await
                .context("Failed to upload file to S3")?;

            // Record in ingestion_file_uploads (dynamic query)
            let upload_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO ingestion_file_uploads (
                    id, job_id, staged_record_id, format, s3_key,
                    size_bytes, md5_checksum, content_type, status, uploaded_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'uploaded', NOW())
                "#,
            )
            .bind(upload_id)
            .bind(job_id)
            .bind(staged_record_id)
            .bind(&format)
            .bind(&s3_key)
            .bind(content.len() as i64)
            .bind(&md5)
            .bind(&content_type)
            .execute(&*self.pool)
            .await
            .context("Failed to record file upload")?;

            upload_ids.push(upload_id);

            tracing::info!(
                record_id = %staged_record_id,
                format = %format,
                s3_key = %s3_key,
                size = content.len(),
                md5 = %md5,
                "File uploaded successfully"
            );
        }

        Ok(upload_ids)
    }

    async fn mark_stored(&self, staged_record_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE ingestion_staged_records SET status = 'stored', stored_at = NOW() WHERE id = $1"
        )
        .bind(staged_record_id)
        .execute(&*self.pool)
        .await
        .context("Failed to mark record as stored")?;

        Ok(())
    }
}
