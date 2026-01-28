//! Generic ETL ingestion framework
//!
//! Provides a distributed, parallel, idempotent ETL pipeline for any data source type.
//! All state tracked in PostgreSQL for resilience and observability.

pub mod checksum;
pub mod coordinator;
pub mod metalink;
pub mod parser;
pub mod storage;
pub mod types;
pub mod worker;

// Re-export commonly used types
pub use checksum::{compute_md5, verify_file_md5};
pub use coordinator::{IngestionCoordinator, JobProgress};
pub use metalink::MetalinkInfo;
pub use parser::{DataSourceParser, RecordFormatter};
pub use storage::{StorageAdapter, StorageOrchestrator};
pub use types::{
    BatchConfig, CreateJobParams, FileUpload, GenericRecord, IngestionJob, IngestionWorkUnit,
    JobStatus, RecordStatus, StagedRecord, WorkUnitStatus,
};
pub use worker::IngestionWorker;
