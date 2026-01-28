//! Feature modules implementing the BDP API
//!
//! This module contains all feature slices following the CQRS (Command Query Responsibility
//! Segregation) pattern. Each feature is organized as a vertical slice with its own
//! commands, queries, and routes.
//!
//! # Features
//!
//! - **data_sources**: CRUD operations for data sources (proteins, genomes, etc.)
//! - **files**: File upload and download operations via S3-compatible storage
//! - **jobs**: Ingestion job management and status tracking
//! - **organisms**: Organism/taxonomy management
//! - **organizations**: Organization management (publishers like UniProt, NCBI)
//! - **protein_metadata**: Protein-specific metadata operations
//! - **resolve**: Manifest resolution for CLI dependency resolution
//! - **search**: Full-text search and autocomplete suggestions
//! - **version_files**: Version-specific file management
//!
//! # Architecture
//!
//! Each feature module follows the structure:
//! - `commands/` - Write operations (create, update, delete)
//! - `queries/` - Read operations (get, list, search)
//! - `routes.rs` - HTTP route definitions
//! - `types.rs` - Shared types (if needed)
//!
//! Commands and queries implement the mediator pattern using the `mediator` crate,
//! enabling clean separation of concerns and easy testing.

pub mod data_sources;
pub mod files;
pub mod jobs;
pub mod organisms;
pub mod organizations;
pub mod protein_metadata;
pub mod resolve;
pub mod search;
pub mod shared;
pub mod version_files;

use axum::Router;
use crate::storage::Storage;

/// Shared state for all feature routes
///
/// Contains the database connection pool and storage backend that are
/// passed to route handlers.
#[derive(Clone)]
pub struct FeatureState {
    /// PostgreSQL connection pool for database operations
    pub db: sqlx::PgPool,
    /// S3-compatible storage backend for file operations
    pub storage: Storage,
}

/// Creates the main API router with all feature routes mounted
///
/// Each feature is mounted under its own path prefix:
/// - `/organizations` - Organization management
/// - `/data-sources` - Data source operations
/// - `/search` - Search and suggestions
/// - `/resolve` - CLI manifest resolution
/// - `/jobs` - Ingestion job management
/// - `/sync-status` - Organization sync status
/// - `/files` - File upload/download
///
/// # Arguments
///
/// * `state` - Shared state containing database pool and storage backend
///
/// # Returns
///
/// An Axum router with all feature routes configured
pub fn router(state: FeatureState) -> Router<()> {
    Router::new()
        .nest("/organizations", organizations::organizations_routes().with_state(state.db.clone()))
        .nest("/data-sources", data_sources::data_sources_routes().with_state(state.db.clone()))
        .nest("/search", search::search_routes().with_state(state.db.clone()))
        .nest("/resolve", resolve::resolve_routes().with_state(state.db.clone()))
        .nest("/jobs", jobs::jobs_routes().with_state(state.db.clone()))
        .nest("/sync-status", jobs::sync_status_routes().with_state(state.db.clone()))
        .nest("/files", files::files_routes().with_state(state.storage.clone()))
}
