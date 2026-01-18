//! Server-specific error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Result type alias for server operations
pub type ServerResult<T> = std::result::Result<T, ServerError>;

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("BDP error: {0}")]
    Bdp(#[from] bdp_common::BdpError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "A database error occurred".to_string())
            },
            AppError::NotFound(ref message) => (StatusCode::NOT_FOUND, message.clone()),
            AppError::Validation(ref message) => (StatusCode::BAD_REQUEST, message.clone()),
            AppError::Internal(ref message) => {
                tracing::error!("Internal error: {}", message);
                (StatusCode::INTERNAL_SERVER_ERROR, message.clone())
            },
            AppError::Config(ref message) => {
                tracing::error!("Configuration error: {}", message);
                (StatusCode::INTERNAL_SERVER_ERROR, "Server configuration error".to_string())
            },
            AppError::Io(ref e) => {
                tracing::error!("IO error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "An IO error occurred".to_string())
            },
            AppError::Bdp(ref e) => {
                tracing::error!("BDP error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            },
            AppError::Unauthorized(ref message) => (StatusCode::UNAUTHORIZED, message.clone()),
            AppError::BadRequest(ref message) => (StatusCode::BAD_REQUEST, message.clone()),
        };

        let body = Json(json!({
            "error": {
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
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("BDP error: {0}")]
    Bdp(#[from] bdp_common::BdpError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl From<AppError> for ServerError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Database(e) => ServerError::Database(e),
            AppError::NotFound(msg) => ServerError::NotFound(msg),
            AppError::Validation(msg) => ServerError::Internal(msg),
            AppError::Internal(msg) => ServerError::Internal(msg),
            AppError::Config(msg) => ServerError::Config(msg),
            AppError::Io(e) => ServerError::Io(e),
            AppError::Bdp(e) => ServerError::Bdp(e),
            AppError::Unauthorized(msg) => ServerError::Internal(msg),
            AppError::BadRequest(msg) => ServerError::Internal(msg),
        }
    }
}
