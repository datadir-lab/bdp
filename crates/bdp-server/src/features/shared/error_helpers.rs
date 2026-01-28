//! Database error handling utilities
//!
//! Provides helpers for handling common database errors like unique constraint
//! violations and foreign key violations.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::features::shared::error_helpers::{handle_unique_violation, handle_foreign_key_violation};
//!
//! sqlx::query!(...)
//!     .execute(&pool)
//!     .await
//!     .map_err(|e| handle_unique_violation(e, "slug", &command.slug))?;
//! ```

use sqlx::Error as SqlxError;

/// Result of checking for a database constraint violation
#[derive(Debug)]
pub enum ConstraintViolation {
    /// A unique constraint was violated
    UniqueViolation,
    /// A foreign key constraint was violated
    ForeignKeyViolation,
    /// No constraint violation - some other error occurred
    Other(SqlxError),
}

/// Check the type of database constraint violation
///
/// Useful when you need to handle multiple constraint types differently.
pub fn check_constraint_violation(error: SqlxError) -> ConstraintViolation {
    if let SqlxError::Database(ref db_err) = error {
        if db_err.is_unique_violation() {
            return ConstraintViolation::UniqueViolation;
        }
        if db_err.is_foreign_key_violation() {
            return ConstraintViolation::ForeignKeyViolation;
        }
    }
    ConstraintViolation::Other(error)
}

/// Check if the error is a unique constraint violation
pub fn is_unique_violation(error: &SqlxError) -> bool {
    if let SqlxError::Database(db_err) = error {
        return db_err.is_unique_violation();
    }
    false
}

/// Check if the error is a foreign key violation
pub fn is_foreign_key_violation(error: &SqlxError) -> bool {
    if let SqlxError::Database(db_err) = error {
        return db_err.is_foreign_key_violation();
    }
    false
}

/// Handle unique constraint violation with a custom error mapper
///
/// If the error is a unique constraint violation, calls the mapper function.
/// Otherwise, returns the original error wrapped in the default wrapper.
///
/// # Type Parameters
/// * `E` - The error type to return
///
/// # Arguments
/// * `error` - The sqlx error to check
/// * `unique_error` - Error to return on unique violation
/// * `default_wrapper` - Function to wrap non-unique errors
///
/// # Returns
/// The appropriate error type based on the error kind
pub fn map_unique_violation<E, F>(error: SqlxError, unique_error: E, default_wrapper: F) -> E
where
    F: FnOnce(SqlxError) -> E,
{
    if is_unique_violation(&error) {
        unique_error
    } else {
        default_wrapper(error)
    }
}

/// Handle foreign key constraint violation with a custom error mapper
///
/// If the error is a foreign key violation, calls the mapper function.
/// Otherwise, returns the original error wrapped in the default wrapper.
///
/// # Type Parameters
/// * `E` - The error type to return
///
/// # Arguments
/// * `error` - The sqlx error to check
/// * `fk_error` - Error to return on foreign key violation
/// * `default_wrapper` - Function to wrap non-FK errors
///
/// # Returns
/// The appropriate error type based on the error kind
pub fn map_foreign_key_violation<E, F>(error: SqlxError, fk_error: E, default_wrapper: F) -> E
where
    F: FnOnce(SqlxError) -> E,
{
    if is_foreign_key_violation(&error) {
        fk_error
    } else {
        default_wrapper(error)
    }
}

/// Handle database errors with custom mapping for both unique and FK violations
///
/// # Type Parameters
/// * `E` - The error type to return
///
/// # Arguments
/// * `error` - The sqlx error to check
/// * `unique_error` - Error to return on unique violation
/// * `fk_error` - Error to return on foreign key violation
/// * `default_wrapper` - Function to wrap other errors
///
/// # Returns
/// The appropriate error type based on the error kind
pub fn map_constraint_violation<E, F>(
    error: SqlxError,
    unique_error: E,
    fk_error: E,
    default_wrapper: F,
) -> E
where
    F: FnOnce(SqlxError) -> E,
{
    match check_constraint_violation(error) {
        ConstraintViolation::UniqueViolation => unique_error,
        ConstraintViolation::ForeignKeyViolation => fk_error,
        ConstraintViolation::Other(e) => default_wrapper(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests are limited since we can't easily create SqlxError instances
    // with specific constraint violation types in unit tests. Integration tests
    // with a real database would be needed for full coverage.

    #[derive(Debug, PartialEq)]
    enum TestError {
        Duplicate(String),
        HasDependencies(String),
        Database(String),
    }

    #[test]
    fn test_constraint_violation_enum() {
        // Test that the enum variants exist and can be matched
        let unique = ConstraintViolation::UniqueViolation;
        let fk = ConstraintViolation::ForeignKeyViolation;

        assert!(matches!(unique, ConstraintViolation::UniqueViolation));
        assert!(matches!(fk, ConstraintViolation::ForeignKeyViolation));
    }
}
