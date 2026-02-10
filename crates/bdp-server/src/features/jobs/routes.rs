//! Job routes
//!
//! Public read-only routes for querying job status and sync progress.
//! These endpoints do NOT require authentication and do NOT allow triggering jobs.

use crate::api::response::{ApiResponse, ErrorResponse};
use crate::features::FeatureState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use super::queries::{
    GetJobError, GetJobQuery, GetSyncStatusQuery, ListJobsError, ListJobsQuery,
    ListSyncStatusQuery, SyncStatusError,
};

/// Create job routes
pub fn jobs_routes() -> Router<FeatureState> {
    Router::new()
        .route("/", get(list_jobs))
        .route("/:job_id", get(get_job))
}

/// Create sync status routes
pub fn sync_status_routes() -> Router<FeatureState> {
    Router::new()
        .route("/", get(list_sync_status))
        .route("/:organization_id", get(get_sync_status))
}

/// List all jobs
///
/// GET /?job_type=UniProtIngestJob&status=Running&limit=50&offset=0
#[tracing::instrument(skip(state))]
async fn list_jobs(
    State(state): State<FeatureState>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Response, JobsApiError> {
    let response = state.dispatch(query).await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

/// Get a specific job by ID
///
/// GET /:job_id
#[tracing::instrument(skip(state))]
async fn get_job(
    State(state): State<FeatureState>,
    Path(job_id): Path<String>,
) -> Result<Response, JobsApiError> {
    let query = GetJobQuery { job_id };
    let job = state.dispatch(query).await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(job))).into_response())
}

/// List all sync statuses
///
/// GET /sync-status?organization_id=<uuid>&status=running
#[tracing::instrument(skip(state))]
async fn list_sync_status(
    State(state): State<FeatureState>,
    Query(query): Query<ListSyncStatusQuery>,
) -> Result<Response, JobsApiError> {
    let response = state.dispatch(query).await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

/// Get sync status for a specific organization
///
/// GET /sync-status/:organization_id
#[tracing::instrument(skip(state))]
async fn get_sync_status(
    State(state): State<FeatureState>,
    Path(organization_id): Path<Uuid>,
) -> Result<Response, JobsApiError> {
    let query = GetSyncStatusQuery { organization_id };
    let status = state.dispatch(query).await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(status))).into_response())
}

// ============================================================================
// Error Handling
// ============================================================================

/// Unified error type for job/sync API endpoints
#[derive(Debug)]
enum JobsApiError {
    ListJobs(ListJobsError),
    GetJob(GetJobError),
    SyncStatus(SyncStatusError),
}

impl From<ListJobsError> for JobsApiError {
    fn from(err: ListJobsError) -> Self {
        Self::ListJobs(err)
    }
}

impl From<GetJobError> for JobsApiError {
    fn from(err: GetJobError) -> Self {
        Self::GetJob(err)
    }
}

impl From<SyncStatusError> for JobsApiError {
    fn from(err: SyncStatusError) -> Self {
        Self::SyncStatus(err)
    }
}

impl IntoResponse for JobsApiError {
    fn into_response(self) -> Response {
        match self {
            JobsApiError::ListJobs(ListJobsError::Database(ref e)) => {
                tracing::error!("Database error listing jobs: {e}");
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
            JobsApiError::GetJob(GetJobError::NotFound) => {
                let error = ErrorResponse::new("NOT_FOUND", "Job not found");
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            JobsApiError::GetJob(GetJobError::Database(ref e)) => {
                tracing::error!("Database error getting job: {e}");
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
            JobsApiError::SyncStatus(SyncStatusError::NotFound) => {
                let error = ErrorResponse::new("NOT_FOUND", "Sync status not found");
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            JobsApiError::SyncStatus(SyncStatusError::Database(ref e)) => {
                tracing::error!("Database error for sync status: {e}");
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for JobsApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListJobs(e) => write!(f, "{e}"),
            Self::GetJob(e) => write!(f, "{e}"),
            Self::SyncStatus(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jobs_routes_exist() {
        // Test that routes can be built
        let _router = jobs_routes();
    }
}
