use crate::api::response::{ApiResponse, ErrorResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use sqlx::PgPool;

use super::queries::{ExecuteQueryError, ExecuteQueryRequest};

pub fn query_routes() -> Router<PgPool> {
    Router::new().route("/", post(execute_query_handler))
}

#[tracing::instrument(
    skip(pool, request),
    fields(
        sql_preview = %request.sql.chars().take(100).collect::<String>()
    )
)]
async fn execute_query_handler(
    State(pool): State<PgPool>,
    Json(request): Json<ExecuteQueryRequest>,
) -> Result<Response, QueryApiError> {
    let response = super::queries::execute_query::handle(pool, request).await?;

    tracing::info!(
        columns = response.columns.len(),
        rows = response.rows.len(),
        "Query executed successfully"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[derive(Debug)]
enum QueryApiError {
    ExecuteError(ExecuteQueryError),
}

impl From<ExecuteQueryError> for QueryApiError {
    fn from(err: ExecuteQueryError) -> Self {
        Self::ExecuteError(err)
    }
}

impl IntoResponse for QueryApiError {
    fn into_response(self) -> Response {
        match self {
            QueryApiError::ExecuteError(ExecuteQueryError::InvalidSql(msg))
            | QueryApiError::ExecuteError(ExecuteQueryError::Forbidden(msg)) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", msg);
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            QueryApiError::ExecuteError(ExecuteQueryError::Timeout) => {
                let error =
                    ErrorResponse::new("TIMEOUT_ERROR", "Query execution timeout (30 seconds)");
                (StatusCode::REQUEST_TIMEOUT, Json(error)).into_response()
            },
            QueryApiError::ExecuteError(ExecuteQueryError::Database(_)) => {
                tracing::error!("Database error during query execution: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for QueryApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecuteError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = QueryApiError::ExecuteError(ExecuteQueryError::Timeout);
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_routes_structure() {
        let router = query_routes();
        assert!(format!("{:?}", router).contains("Router"));
    }
}
