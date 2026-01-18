//! End-to-end testing infrastructure for BDP ingestion pipeline
//!
//! This module provides comprehensive E2E testing capabilities including:
//! - Docker container orchestration (PostgreSQL, MinIO, bdp-server)
//! - Test data management (CI fixtures, downloaded real data)
//! - Helper functions for common operations
//! - Observability and debugging support
//!
//! # Modes
//!
//! **CI Mode**: Uses small committed sample data, fast execution
//! **Dev Mode**: Downloads real UniProt data once, caches for reuse
//!
//! # Usage
//!
//! ```no_run
//! use bdp_server::e2e::E2EEnvironment;
//!
//! #[tokio::test]
//! async fn test_ingestion() {
//!     let env = E2EEnvironment::new().await;
//!     // ... test logic
//!     env.cleanup().await;
//! }
//! ```

pub mod harness;
pub mod fixtures;
pub mod assertions;
pub mod observability;

// Re-export main types
pub use harness::E2EEnvironment;
pub use fixtures::{TestDataManager, TestDataMode};
pub use assertions::E2EAssertions;
pub use observability::E2EObservability;
