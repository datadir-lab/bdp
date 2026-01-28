//! BDP Common Library
#![deny(clippy::unwrap_used, clippy::expect_used)]
//!
//! Shared types, utilities, and error handling for the BDP project.
//!
//! # Overview
//!
//! This crate provides common functionality used across all BDP workspace members:
//!
//! - **Error Handling**: Custom error types and result types
//! - **Checksums**: File integrity verification utilities
//! - **Types**: Shared domain types and data structures
//!
//! # Example
//!
//! ```no_run
//! use bdp_common::{Result, BdpError};
//! use bdp_common::checksum::Checksum;
//!
//! fn process_file(path: &str) -> Result<()> {
//!     let checksum = Checksum::from_file(path)?;
//!     println!("File checksum: {}", checksum);
//!     Ok(())
//! }
//! ```

pub mod checksum;
pub mod error;
pub mod logging;
pub mod types;

// Re-export commonly used types
pub use error::{BdpError, Result};
