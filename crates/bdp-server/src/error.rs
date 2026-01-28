//! Server-specific error types
//!
//! This module provides structured error types for the BDP server with
//! user-friendly messages for API responses and detailed internal logging.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Result type alias for server operations
pub type ServerResult<T> = std::result::Result<T, ServerError>;

/// General-purpose error type for ingest modules and internal operations
/// Re-export ServerError as Error for compatibility
pub type Error = ServerError;

/// Application error types for API responses
///
/// These errors are designed to provide helpful information to API clients
/// while keeping sensitive implementation details internal.
#[derive(Error, Debug)]
pub enum AppError {
    /// Database operation failed
    #[error("Database operation failed: {0}")]
    Database(#[from] sqlx::Error),

    /// Requested resource does not exist
    #[error("{0}")]
    NotFound(String),

    /// Request data failed validation
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Unexpected server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Server configuration is invalid
    #[error("Server configuration error: {0}")]
    Config(String),

    /// File system operation failed
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// BDP-specific error
    #[error("{0}")]
    Bdp(#[from] bdp_common::BdpError),

    /// Authentication required or failed
    #[error("Authentication required: {0}")]
    Unauthorized(String),

    /// Invalid request format or parameters
    #[error("Invalid request: {0}")]
    BadRequest(String),
}

impl AppError {
    /// Create a not found error with context
    pub fn not_found(resource_type: &str, identifier: &str) -> Self {
        Self::NotFound(format!(
            "{} '{}' not found. Verify the identifier and try again.",
            resource_type, identifier
        ))
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Create a bad request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, error_message) = match self {
            AppError::Database(ref e) => {
                tracing::error!(error = ?e, "Database error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
            AppError::NotFound(ref message) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", message.clone())
            }
            AppError::Validation(ref message) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message.clone())
            }
            AppError::Internal(ref message) => {
                tracing::error!(error = %message, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An unexpected error occurred. Please try again later.".to_string(),
                )
            }
            AppError::Config(ref message) => {
                tracing::error!(error = %message, "Configuration error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONFIG_ERROR",
                    "Server configuration error. Please contact support.".to_string(),
                )
            }
            AppError::Io(ref e) => {
                tracing::error!(error = ?e, "IO error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "IO_ERROR",
                    "A file operation failed. Please try again later.".to_string(),
                )
            }
            AppError::Bdp(ref e) => {
                tracing::error!(error = ?e, "BDP error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, "BDP_ERROR", e.to_string())
            }
            AppError::Unauthorized(ref message) => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message.clone())
            }
            AppError::BadRequest(ref message) => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST", message.clone())
            }
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": error_message,
                "status": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}

/// Legacy server error type (for backwards compatibility)
#[derive(Error, Debug)]
pub enum ServerError {
    /// Database operation failed
    #[error("Database operation failed: {0}")]
    Database(#[from] sqlx::Error),

    /// File system operation failed
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// BDP-specific error
    #[error("{0}")]
    Bdp(#[from] bdp_common::BdpError),

    /// Server configuration is invalid
    #[error("Configuration error: {0}")]
    Config(String),

    /// Requested resource does not exist
    #[error("{0}")]
    NotFound(String),

    /// Unexpected server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Other error (for generic use cases)
    #[error("{0}")]
    Other(String),
}

impl From<AppError> for ServerError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Database(e) => ServerError::Database(e),
            AppError::NotFound(msg) => ServerError::NotFound(msg),
            AppError::Validation(msg) => ServerError::Internal(format!("Validation failed: {}", msg)),
            AppError::Internal(msg) => ServerError::Internal(msg),
            AppError::Config(msg) => ServerError::Config(msg),
            AppError::Io(e) => ServerError::Io(e),
            AppError::Bdp(e) => ServerError::Bdp(e),
            AppError::Unauthorized(msg) => ServerError::Internal(format!("Unauthorized: {}", msg)),
            AppError::BadRequest(msg) => ServerError::Internal(format!("Bad request: {}", msg)),
        }
    }
}
