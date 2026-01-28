//! BDP Server Library
#![recursion_limit = "256"]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::useless_format)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::new_without_default)]
#![allow(clippy::impl_trait_in_params)]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::map_clone)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::get_first)]
#![allow(clippy::host_endian_bytes)]
#![allow(clippy::io_other_error)]
#![allow(clippy::type_complexity)]
//!
//! HTTP server for managing biological datasets.
//!
//! # Overview
//!
//! The BDP server provides a REST API for managing biological datasets:
//!
//! - **API Endpoints**: RESTful API for dataset management
//! - **Database Management**: PostgreSQL integration with SQLx
//! - **Storage Backend**: Configurable storage backends (S3-compatible)
//! - **Configuration**: Environment-based configuration management
//! - **Middleware**: CORS, request logging, and rate limiting
//!
//! # Architecture
//!
//! The server follows a **CQRS (Command Query Responsibility Segregation)** architecture:
//!
//! ## CQRS Pattern
//!
//! - **Commands** (Write Operations): Create, Update, Delete operations that modify state
//!   - All commands are audited in the `audit_logs` table
//!   - Executed via HTTP POST, PUT, PATCH, DELETE methods
//!   - Examples: Create organization, update source, delete version
//!
//! - **Queries** (Read Operations): Retrieve operations that read state
//!   - Not audited to reduce noise and improve performance
//!   - Executed via HTTP GET method
//!   - Examples: List organizations, get source details
//!
//! ## Audit Logging
//!
//! The audit system tracks all commands with:
//! - User ID (when authenticated)
//! - Action performed (create, update, delete)
//! - Entity type and ID
//! - Changes made (JSON diff)
//! - Client IP and user agent
//! - Timestamp
//!
//! Query the audit trail via `/api/v1/audit` endpoint.
//!
//! ## Framework Stack
//!
//! - **Axum**: Modern, ergonomic web framework
//! - **SQLx**: Type-safe SQL queries with compile-time verification
//! - **Tower**: Middleware and service abstractions
//!
//! # Example
//!
//! ```no_run
//! use bdp_server::{api, config::Config, storage};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load()?;
//!     let storage = storage::init(&config).await?;
//!     api::serve(config, storage).await?;
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod audit;
pub mod config;
pub mod cqrs;
pub mod db;
pub mod error;
pub mod features;
pub mod ingest;
pub mod middleware;
pub mod models;
pub mod storage;

// Re-export commonly used types
pub use error::{AppError, ServerError, ServerResult};
