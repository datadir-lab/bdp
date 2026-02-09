use crate::api::response::{ApiResponse, ErrorResponse};
use crate::features::FeatureState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

use super::commands::RecordDownloadCommand;
use super::queries::{ResolveManifestError, ResolveManifestQuery};

pub fn resolve_routes() -> Router<FeatureState> {
    Router::new()
        .route("/", post(resolve_manifest))
        .route("/downloads", post(record_download))
}

#[tracing::instrument(skip(state, query), fields(sources = query.sources.len(), tools = query.tools.len()))]
async fn resolve_manifest(
    State(state): State<FeatureState>,
    Json(query): Json<ResolveManifestQuery>,
) -> Result<Response, ResolveApiError> {
    let response = super::queries::resolve_manifest::handle(state.db, state.storage, query).await?;

    tracing::info!(
        sources_count = response.sources.len(),
        tools_count = response.tools.len(),
        "Successfully resolved manifest"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(state, command), fields(org = %command.org, name = %command.name, version = %command.version))]
async fn record_download(
    State(state): State<FeatureState>,
    Json(command): Json<RecordDownloadCommand>,
) -> Result<Response, RecordDownloadApiError> {
    let response = super::commands::record_download::handle(state.db, command).await?;

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[derive(Debug)]
enum RecordDownloadApiError {
    RecordError(super::commands::RecordDownloadError),
}

impl From<super::commands::RecordDownloadError> for RecordDownloadApiError {
    fn from(err: super::commands::RecordDownloadError) -> Self {
        Self::RecordError(err)
    }
}

impl IntoResponse for RecordDownloadApiError {
    fn into_response(self) -> Response {
        match self {
            RecordDownloadApiError::RecordError(
                super::commands::RecordDownloadError::NotFound(msg),
            ) => {
                let error = ErrorResponse::new("NOT_FOUND", msg);
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            RecordDownloadApiError::RecordError(
                super::commands::RecordDownloadError::Database(_),
            ) => {
                tracing::error!("Database error recording download: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for RecordDownloadApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordError(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Debug)]
enum ResolveApiError {
    ResolveError(ResolveManifestError),
}

impl From<ResolveManifestError> for ResolveApiError {
    fn from(err: ResolveManifestError) -> Self {
        Self::ResolveError(err)
    }
}

impl IntoResponse for ResolveApiError {
    fn into_response(self) -> Response {
        match self {
            ResolveApiError::ResolveError(ResolveManifestError::InvalidSourceSpec(msg))
            | ResolveApiError::ResolveError(ResolveManifestError::InvalidToolSpec(msg)) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", msg);
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            ResolveApiError::ResolveError(ResolveManifestError::SourceNotFound(msg))
            | ResolveApiError::ResolveError(ResolveManifestError::ToolNotFound(msg))
            | ResolveApiError::ResolveError(ResolveManifestError::VersionNotFound(msg))
            | ResolveApiError::ResolveError(ResolveManifestError::FormatNotAvailable(msg)) => {
                let error = ErrorResponse::new("NOT_FOUND", msg);
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            ResolveApiError::ResolveError(ResolveManifestError::DependencyConflict(msg))
            | ResolveApiError::ResolveError(ResolveManifestError::CircularDependency(msg)) => {
                let error = ErrorResponse::new("CONFLICT", msg);
                (StatusCode::CONFLICT, Json(error)).into_response()
            },
            ResolveApiError::ResolveError(ResolveManifestError::Database(_)) => {
                tracing::error!("Database error during manifest resolution: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for ResolveApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResolveError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_structure() {
        let router = resolve_routes();
        assert!(format!("{:?}", router).contains("Router"));
    }

    #[test]
    fn test_error_display() {
        let err = ResolveApiError::ResolveError(ResolveManifestError::InvalidSourceSpec(
            "test error".to_string(),
        ));
        assert!(err.to_string().contains("test error"));
    }
}
