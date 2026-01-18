//! Job routes
//!
//! Public read-only routes for querying job status and sync progress.
//! These endpoints do NOT require authentication and do NOT allow triggering jobs.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::queries::{
    get_job::handle as handle_get_job,
    get_sync_status::{handle_get as handle_get_sync_status, handle_list as handle_list_sync_status},
    list_jobs::handle as handle_list_jobs,
    GetJobQuery, GetSyncStatusQuery, ListJobsQuery, ListSyncStatusQuery,
};

/// Create job routes
pub fn jobs_routes() -> Router<PgPool> {
    Router::new()
        .route("/jobs", get(list_jobs))
        .route("/jobs/:job_id", get(get_job))
        .route("/sync-status", get(list_sync_status))
        .route("/sync-status/:organization_id", get(get_sync_status))
}

/// List all jobs
///
/// GET /jobs?job_type=UniProtIngestJob&status=Running&limit=50&offset=0
async fn list_jobs(
    State(db): State<PgPool>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Response, StatusCode> {
    match handle_list_jobs(db, query).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response))).into_response()),
        Err(e) => {
            tracing::error!("Failed to list jobs: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific job by ID
///
/// GET /jobs/:job_id
async fn get_job(
    State(db): State<PgPool>,
    Path(job_id): Path<String>,
) -> Result<Response, StatusCode> {
    let query = GetJobQuery { job_id };

    match handle_get_job(db, query).await {
        Ok(job) => Ok((StatusCode::OK, Json(json!(job))).into_response()),
        Err(e) => {
            tracing::debug!("Job not found or error: {:?}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// List all sync statuses
///
/// GET /sync-status?organization_id=<uuid>&status=running
async fn list_sync_status(
    State(db): State<PgPool>,
    Query(query): Query<ListSyncStatusQuery>,
) -> Result<Response, StatusCode> {
    match handle_list_sync_status(db, query).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response))).into_response()),
        Err(e) => {
            tracing::error!("Failed to list sync statuses: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get sync status for a specific organization
///
/// GET /sync-status/:organization_id
async fn get_sync_status(
    State(db): State<PgPool>,
    Path(organization_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let query = GetSyncStatusQuery { organization_id };

    match handle_get_sync_status(db, query).await {
        Ok(status) => Ok((StatusCode::OK, Json(json!(status))).into_response()),
        Err(e) => {
            tracing::debug!("Sync status not found or error: {:?}", e);
            Err(StatusCode::NOT_FOUND)
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
