//! API response types
//!
//! Standard response structures for the BDP API following the design specification.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Standard success response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a new success response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            meta: None,
        }
    }

    /// Create a success response with metadata
    pub fn success_with_meta(data: T, meta: serde_json::Value) -> Self {
        Self {
            success: true,
            data,
            meta: Some(meta),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Standard error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// Create an error response with details
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            },
        }
    }
}

/// Pagination metadata for list responses
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub pages: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_next: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_prev: Option<bool>,
}

impl PaginationMeta {
    /// Create pagination metadata from page parameters and total count
    pub fn new(page: i64, per_page: i64, total: i64) -> Self {
        let pages = (total as f64 / per_page as f64).ceil() as i64;
        Self {
            page,
            per_page,
            total,
            pages,
            has_next: Some(page < pages),
            has_prev: Some(page > 1),
        }
    }

    /// Create pagination metadata without navigation flags
    pub fn simple(page: i64, per_page: i64, total: i64) -> Self {
        let pages = (total as f64 / per_page as f64).ceil() as i64;
        Self {
            page,
            per_page,
            total,
            pages,
            has_next: None,
            has_prev: None,
        }
    }
}

/// Application error type that can be converted to HTTP responses
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    ValidationError(String),
    InternalError(String),
    Database(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            AppError::ValidationError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", msg)
            },
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            },
            AppError::Database(err) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "A database error occurred".to_string(),
                )
            },
        };

        let error_response = ErrorResponse::new(code, message);
        (status, Json(error_response)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<crate::db::DbError> for AppError {
    fn from(err: crate::db::DbError) -> Self {
        match err {
            crate::db::DbError::NotFound(msg) => AppError::NotFound(msg),
            crate::db::DbError::Duplicate(msg) => AppError::Conflict(msg),
            crate::db::DbError::Config(msg) => AppError::BadRequest(msg),
            crate::db::DbError::Sqlx(err) => AppError::Database(err),
        }
    }
}

/// Alias for Result with AppError
pub type ApiResult<T> = Result<T, AppError>;
