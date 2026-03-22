//! Vector feature HTTP routes
//!
//! Exposes four endpoints:
//! - `GET /stats`                          → pipeline health / counts
//! - `GET /search`                         → semantic similarity search
//! - `GET /:entry_id/neighbors`            → KNN for a single entry
//! - `GET /tiles/:run_id/:z/:x/:y`        → pre-rendered graph tiles

use crate::api::response::{ApiResponse, ErrorResponse};
use crate::features::FeatureState;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::collections::HashMap;

use super::queries::{
    GetNeighborsError, GetNeighborsQuery, GetTileError, GetTileQuery, GetVectorStatsQuery,
    SemanticSearchError, SemanticSearchQuery,
};

pub fn vectors_routes() -> Router<FeatureState> {
    Router::new()
        .route("/stats", get(get_stats))
        .route("/search", get(semantic_search))
        .route("/:entry_id/neighbors", get(get_neighbors))
        .route("/tiles/:run_id/:z/:x/:y", get(get_tile))
}

#[tracing::instrument(skip(state))]
async fn get_stats(State(state): State<FeatureState>) -> Response {
    match state.dispatch(GetVectorStatsQuery).await {
        Ok(stats) => (StatusCode::OK, Json(ApiResponse::success(stats))).into_response(),
        Err(e) => {
            tracing::error!("get_stats error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to fetch stats")),
            )
                .into_response()
        },
    }
}

#[tracing::instrument(skip(state, query), fields(q = %query.q, k = %query.k))]
async fn semantic_search(
    State(state): State<FeatureState>,
    Query(query): Query<SemanticSearchQuery>,
) -> Response {
    match state.dispatch(query).await {
        Ok(resp) => (StatusCode::OK, Json(ApiResponse::success(resp.items))).into_response(),
        Err(SemanticSearchError::QueryEmpty) | Err(SemanticSearchError::InvalidK) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("VALIDATION_ERROR", "Invalid query parameters")),
        )
            .into_response(),
        Err(SemanticSearchError::EmbeddingUnavailable(ref msg)) => {
            tracing::warn!("Embedding service unavailable: {}", msg);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "SERVICE_UNAVAILABLE",
                    "Embedding service unavailable",
                )),
            )
                .into_response()
        },
        Err(e) => {
            tracing::error!("semantic_search error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Search failed")),
            )
                .into_response()
        },
    }
}

#[tracing::instrument(skip(state), fields(entry_id = %entry_id))]
async fn get_neighbors(
    State(state): State<FeatureState>,
    Path(entry_id): Path<uuid::Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let k = params.get("k").and_then(|v| v.parse().ok()).unwrap_or(10);
    let query = GetNeighborsQuery { entry_id, k };
    match state.dispatch(query).await {
        Ok(resp) => {
            (StatusCode::OK, Json(ApiResponse::success(resp.neighbors))).into_response()
        },
        Err(GetNeighborsError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Entry has no embedding")),
        )
            .into_response(),
        Err(GetNeighborsError::InvalidK) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("VALIDATION_ERROR", "k must be between 1 and 100")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("get_neighbors error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Neighbor lookup failed")),
            )
                .into_response()
        },
    }
}

#[tracing::instrument(skip(state), fields(run_id = %run_id, z = %z, x = %x, y = %y))]
async fn get_tile(
    State(state): State<FeatureState>,
    Path((run_id, z, x, y)): Path<(String, u32, u32, u32)>,
) -> Response {
    let query = GetTileQuery { run_id, z, x, y };
    match state.dispatch(query).await {
        Ok(tile) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
            ],
            Body::from(tile.body),
        )
            .into_response(),
        Err(GetTileError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_tile error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_structure() {
        let router = vectors_routes();
        assert!(format!("{:?}", router).contains("Router"));
    }
}
