//! Shared utilities and types for feature modules
//!
//! This module contains reusable code to reduce duplication across feature implementations.
//!
//! # Contents
//!
//! - **pagination**: Common pagination types and helpers
//! - **validation**: Input validation utilities
//! - **error_helpers**: Database error handling utilities
//! - **test_helpers**: Test fixtures and utilities (test-only)

pub mod error_helpers;
pub mod pagination;
pub mod validation;

#[cfg(test)]
pub mod test_helpers;

// Re-export commonly used types
pub use pagination::{Paginated, PaginationMetadata, PaginationParams};
pub use validation::{validate_name, validate_slug, validate_url, SlugValidationError};
