//! Data ingestion infrastructure
//!
//! This module provides the job queue infrastructure for Phase 2 data ingestion.
//!
//! # Architecture
//!
//! - **config**: Configuration for ingestion jobs (INGEST_* environment variables)
//! - **jobs**: Job definitions for apalis queue (UniProtIngestJob, IngestStats)
//! - **models**: Database models for sync status tracking
//! - **scheduler**: Apalis scheduler setup and worker management
//! - **version_mapping**: Version mapping utilities (Agent 3)
//! - **uniprot**: UniProt-specific ingestion logic (Agent 4)
//!
//! # Public API
//!
//! The public API endpoints are provided through the `features::jobs` module:
//! - `GET /api/v1/jobs` - List all jobs
//! - `GET /api/v1/jobs/:id` - Get job details
//! - `GET /api/v1/sync-status` - List sync statuses
//! - `GET /api/v1/sync-status/:org_id` - Get sync status for organization
//!
//! These endpoints are read-only and require NO authentication.

pub mod config;
pub mod jobs;
pub mod models;
pub mod ncbi_taxonomy;
pub mod scheduler;
pub mod orchestrator;
pub mod uniprot;
pub mod version_mapping;
pub mod framework;

pub use config::{IngestConfig, UniProtConfig};
pub use jobs::{IngestStats, UniProtIngestJob};
pub use models::{OrganizationSyncStatus, SyncStatus};
pub use ncbi_taxonomy::NcbiTaxonomyPipeline;
pub use scheduler::JobScheduler;
pub use orchestrator::IngestOrchestrator;
pub use uniprot::UniProtPipeline;
pub use version_mapping::VersionMapper;
