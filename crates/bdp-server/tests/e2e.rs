//! End-to-end tests for BDP ingestion pipeline
//!
//! These tests spin up real Docker containers (PostgreSQL, MinIO, bdp-server)
//! and test the complete data ingestion flow.
//!
//! # Running Tests
//!
//! ```bash
//! # CI mode (fast, uses committed sample data)
//! just e2e-ci
//!
//! # Real mode (uses downloaded UniProt data)
//! just e2e-real
//!
//! # Debug mode (full logging)
//! just e2e-debug
//!
//! # Run specific test
//! cargo test --test e2e test_ingestion_happy_path_ci -- --nocapture
//! ```
//!
//! # Environment Variables
//!
//! - `BDP_E2E_MODE`: "ci" (default) or "real"
//! - `RUST_LOG`: Log level (e.g., "debug", "info")
//!
//! # Test Data
//!
//! - **CI Mode**: Uses `tests/fixtures/uniprot_ci_sample.dat` (3 proteins, ~3KB)
//! - **Real Mode**: Uses downloaded data in `tests/fixtures/real/` (cached, gitignored)
//!
//! # Prerequisites
//!
//! - Docker daemon running
//! - testcontainers library (automatically manages containers)
//! - Real data downloaded (for real mode): `just e2e-download-data`

#![allow(dead_code)]

mod e2e {
    pub mod assertions;
    pub mod fixtures;
    pub mod harness;
    mod ingestion_tests;
    pub mod observability;

    // Re-export main types for convenience
    pub use assertions::E2EAssertions;
    pub use fixtures::{TestDataManager, TestDataMode};
    pub use harness::E2EEnvironment;
    pub use observability::E2EObservability;
}

use e2e::*;

/// Initialize tracing for tests
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,bdp_server=debug,sqlx=warn")),
        )
        .with_test_writer()
        .try_init();
}

#[ctor::ctor]
fn init() {
    init_tracing();
}
