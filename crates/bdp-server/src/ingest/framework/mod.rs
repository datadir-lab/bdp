//! Generic ETL ingestion framework
//!
//! Provides a distributed, parallel, idempotent ETL pipeline for any data source type.
//! All state tracked in PostgreSQL for resilience and observability.

pub mod types;
pub mod parser;
pub mod coordinator;
pub mod worker;
pub mod storage;
pub mod checksum;
pub mod metalink;

// Re-export commonly used types
pub use types::{
    GenericRecord, IngestionJob, IngestionWorkUnit, StagedRecord, FileUpload,
    JobStatus, WorkUnitStatus, RecordStatus, BatchConfig, CreateJobParams,
};
pub use parser::{DataSourceParser, RecordFormatter};
pub use coordinator::{IngestionCoordinator, JobProgress};
pub use worker::IngestionWorker;
pub use storage::{StorageAdapter, StorageOrchestrator};
pub use checksum::{compute_md5, verify_file_md5};
pub use metalink::MetalinkInfo;
