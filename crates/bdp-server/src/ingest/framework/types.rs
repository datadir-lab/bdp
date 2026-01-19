//! Core types for the generic ingestion framework

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generic record parsed from any data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericRecord {
    /// Type of record: 'protein', 'genome', 'compound', etc.
    pub record_type: String,
    /// Primary identifier (lowercase): 'p01234', 'gcf_000001405', 'cid_2244', etc.
    pub record_identifier: String,
    /// Human-readable name (optional, lowercase)
    pub record_name: Option<String>,
    /// Flexible JSONB data structure
    pub record_data: serde_json::Value,
    /// MD5 of entire record_data for deduplication
    pub content_md5: Option<String>,
    /// MD5 of primary content field (sequence, SMILES, etc.)
    pub sequence_md5: Option<String>,
    /// Source file this came from
    pub source_file: Option<String>,
    /// Offset in source file
    pub source_offset: Option<i64>,
}

/// Ingestion job status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Downloading,
    DownloadVerified,
    Parsing,
    Storing,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Downloading => "downloading",
            JobStatus::DownloadVerified => "download_verified",
            JobStatus::Parsing => "parsing",
            JobStatus::Storing => "storing",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
}

impl From<String> for JobStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => JobStatus::Pending,
            "downloading" => JobStatus::Downloading,
            "download_verified" => JobStatus::DownloadVerified,
            "parsing" => JobStatus::Parsing,
            "storing" => JobStatus::Storing,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            _ => JobStatus::Pending,
        }
    }
}

/// Ingestion job (maps to ingestion_jobs table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionJob {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub job_type: String,
    pub external_version: String,
    pub internal_version: String,
    pub source_url: Option<String>,
    pub source_metadata: Option<serde_json::Value>,
    pub status: JobStatus,
    pub total_records: Option<i64>,
    pub records_processed: i64,
    pub records_stored: i64,
    pub records_failed: i64,
    pub records_skipped: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Work unit status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkUnitStatus {
    Pending,
    Claimed,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl WorkUnitStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WorkUnitStatus::Pending => "pending",
            WorkUnitStatus::Claimed => "claimed",
            WorkUnitStatus::Processing => "processing",
            WorkUnitStatus::Completed => "completed",
            WorkUnitStatus::Failed => "failed",
            WorkUnitStatus::Cancelled => "cancelled",
        }
    }
}

/// Ingestion work unit (maps to ingestion_work_units table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionWorkUnit {
    pub id: Uuid,
    pub job_id: Uuid,
    pub unit_type: String,
    pub batch_number: i32,
    pub start_offset: i64,
    pub end_offset: i64,
    pub record_count: Option<i32>,
    pub worker_id: Option<Uuid>,
    pub worker_hostname: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub status: WorkUnitStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
    pub started_processing_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub processing_duration_ms: Option<i64>,
}

/// Staged record status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Staged,
    UploadingFiles,
    FilesUploaded,
    StoringDb,
    Stored,
    Failed,
}

impl RecordStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RecordStatus::Staged => "staged",
            RecordStatus::UploadingFiles => "uploading_files",
            RecordStatus::FilesUploaded => "files_uploaded",
            RecordStatus::StoringDb => "storing_db",
            RecordStatus::Stored => "stored",
            RecordStatus::Failed => "failed",
        }
    }
}

/// Staged record (maps to ingestion_staged_records table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedRecord {
    pub id: Uuid,
    pub job_id: Uuid,
    pub work_unit_id: Option<Uuid>,
    pub record_type: String,
    pub record_identifier: String,
    pub record_name: Option<String>,
    pub record_data: serde_json::Value,
    pub content_md5: Option<String>,
    pub sequence_md5: Option<String>,
    pub source_file: Option<String>,
    pub source_offset: Option<i64>,
    pub parsed_at: DateTime<Utc>,
    pub status: RecordStatus,
    pub stored_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// File upload tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    pub id: Uuid,
    pub job_id: Uuid,
    pub staged_record_id: Option<Uuid>,
    pub format: String,
    pub s3_key: String,
    pub size_bytes: i64,
    pub md5_checksum: String,
    pub content_type: Option<String>,
    pub status: String,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Raw file download tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFile {
    pub id: Uuid,
    pub job_id: Uuid,
    pub file_type: String,
    pub file_purpose: Option<String>,
    pub s3_key: String,
    pub expected_md5: Option<String>,
    pub computed_md5: Option<String>,
    pub verified_md5: bool,
    pub size_bytes: Option<i64>,
    pub compression: Option<String>,
    pub content_type: Option<String>,
    pub status: String,
    pub downloaded_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Configuration for creating a new ingestion job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobParams {
    pub organization_id: Uuid,
    pub job_type: String,
    pub external_version: String,
    pub internal_version: String,
    pub source_url: Option<String>,
    pub source_metadata: Option<serde_json::Value>,
    pub total_records: Option<i64>,
}

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub parse_batch_size: usize,
    pub store_batch_size: usize,
    pub max_retries: i32,
    pub heartbeat_interval_secs: u64,
    pub worker_timeout_secs: i64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            parse_batch_size: 1000,
            store_batch_size: 100,
            max_retries: 3,
            heartbeat_interval_secs: 30,
            worker_timeout_secs: 120,
        }
    }
}

/// Result of claiming a work unit
#[derive(Debug, Clone)]
pub struct ClaimedWorkUnit {
    pub id: Uuid,
    pub batch_number: i32,
    pub start_offset: i64,
    pub end_offset: i64,
    pub record_count: Option<i32>,
}
