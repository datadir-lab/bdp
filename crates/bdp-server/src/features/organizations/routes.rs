//! Organization API routes
//!
//! This module wires the CQRS commands and queries to Axum HTTP handlers,
//! providing a RESTful API for organization management.
//!
//! # Route Structure
//!
//! - `POST /api/v1/organizations` - Create a new organization
//! - `GET /api/v1/organizations` - List organizations with pagination and filters
//! - `GET /api/v1/organizations/:slug` - Get a single organization by slug
//! - `PUT /api/v1/organizations/:slug` - Update an organization
//! - `DELETE /api/v1/organizations/:slug` - Delete an organization

use crate::api::response::{ApiResponse, ErrorResponse};
use crate::features::FeatureState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::json;

use super::{
    commands::{
        CreateOrganizationCommand, CreateOrganizationError, DeleteOrganizationCommand,
        DeleteOrganizationError, UpdateOrganizationCommand, UpdateOrganizationError,
    },
    queries::{GetOrganizationQuery, ListOrganizationsQuery},
};

// ============================================================================
// Router Configuration
// ============================================================================

pub fn organizations_routes() -> Router<FeatureState> {
    Router::new()
        .route("/", post(create_organization))
        .route("/", get(list_organizations))
        .route("/:slug", get(get_organization))
        .route("/:slug", put(update_organization))
        .route("/:slug", delete(delete_organization))
}

// ============================================================================
// Command Handlers (Write Operations)
// ============================================================================

#[tracing::instrument(
    skip(state, command),
    fields(slug = %command.slug, name = %command.name)
)]
async fn create_organization(
    State(state): State<FeatureState>,
    Json(command): Json<CreateOrganizationCommand>,
) -> Result<Response, OrganizationApiError> {
    let response = state.dispatch(command).await?;

    tracing::info!(
        org_id = %response.id,
        org_slug = %response.slug,
        "Organization created via API"
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(
    skip(state, command),
    fields(slug = %slug)
)]
async fn update_organization(
    State(state): State<FeatureState>,
    Path(slug): Path<String>,
    Json(mut command): Json<UpdateOrganizationCommand>,
) -> Result<Response, OrganizationApiError> {
    // Set slug from path parameter
    command.slug = slug;

    let response = state.dispatch(command).await?;

    tracing::info!(
        org_id = %response.id,
        org_slug = %response.slug,
        "Organization updated via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(
    skip(state),
    fields(slug = %slug)
)]
async fn delete_organization(
    State(state): State<FeatureState>,
    Path(slug): Path<String>,
) -> Result<Response, OrganizationApiError> {
    let command = DeleteOrganizationCommand { slug };

    let response = state.dispatch(command).await?;

    tracing::info!(
        org_slug = %response.slug,
        "Organization deleted via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

// ============================================================================
// Query Handlers (Read Operations)
// ============================================================================

#[tracing::instrument(
    skip(state),
    fields(slug = %slug)
)]
async fn get_organization(
    State(state): State<FeatureState>,
    Path(slug): Path<String>,
) -> Result<Response, OrganizationApiError> {
    let query = GetOrganizationQuery {
        slug: Some(slug),
        id: None,
    };

    let response = state.dispatch(query).await?;

    tracing::debug!(
        org_id = %response.id,
        org_slug = %response.slug,
        "Organization retrieved via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(
    skip(state, query),
    fields(
        page = ?query.pagination.page,
        per_page = ?query.pagination.per_page,
        is_system = ?query.is_system
    )
)]
async fn list_organizations(
    State(state): State<FeatureState>,
    Query(query): Query<ListOrganizationsQuery>,
) -> Result<Response, OrganizationApiError> {
    let response = state.dispatch(query).await?;

    tracing::debug!(
        count = response.items.len(),
        total = response.pagination.total,
        "Organizations listed via API"
    );

    let meta = json!({
        "pagination": response.pagination
    });

    Ok(
        (StatusCode::OK, Json(ApiResponse::success_with_meta(response.items, meta)))
            .into_response(),
    )
}

// ============================================================================
// Error Handling
// ============================================================================

/// Unified error type for organization API endpoints
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum OrganizationApiError {
    CreateError(CreateOrganizationError),
    UpdateError(UpdateOrganizationError),
    DeleteError(DeleteOrganizationError),
    GetError(super::queries::GetOrganizationError),
    ListError(super::queries::ListOrganizationsError),
}

impl From<CreateOrganizationError> for OrganizationApiError {
    fn from(err: CreateOrganizationError) -> Self {
        Self::CreateError(err)
    }
}

impl From<UpdateOrganizationError> for OrganizationApiError {
    fn from(err: UpdateOrganizationError) -> Self {
        Self::UpdateError(err)
    }
}

impl From<DeleteOrganizationError> for OrganizationApiError {
    fn from(err: DeleteOrganizationError) -> Self {
        Self::DeleteError(err)
    }
}

impl From<super::queries::GetOrganizationError> for OrganizationApiError {
    fn from(err: super::queries::GetOrganizationError) -> Self {
        Self::GetError(err)
    }
}

impl From<super::queries::ListOrganizationsError> for OrganizationApiError {
    fn from(err: super::queries::ListOrganizationsError) -> Self {
        Self::ListError(err)
    }
}

impl IntoResponse for OrganizationApiError {
    fn into_response(self) -> Response {
        match self {
            // Create errors - validation errors are now wrapped
            OrganizationApiError::CreateError(CreateOrganizationError::SlugValidation(_))
            | OrganizationApiError::CreateError(CreateOrganizationError::NameValidation(_))
            | OrganizationApiError::CreateError(CreateOrganizationError::UrlValidation(_)) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            OrganizationApiError::CreateError(CreateOrganizationError::DuplicateSlug(slug)) => {
                let error = ErrorResponse::new(
                    "CONFLICT",
                    format!("Organization with slug '{}' already exists", slug),
                );
                (StatusCode::CONFLICT, Json(error)).into_response()
            },
            OrganizationApiError::CreateError(CreateOrganizationError::Database(_)) => {
                tracing::error!("Database error during organization creation: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            // Update errors - validation errors are now wrapped
            OrganizationApiError::UpdateError(UpdateOrganizationError::SlugRequired)
            | OrganizationApiError::UpdateError(UpdateOrganizationError::NoFieldsToUpdate)
            | OrganizationApiError::UpdateError(UpdateOrganizationError::NameValidation(_))
            | OrganizationApiError::UpdateError(UpdateOrganizationError::UrlValidation(_)) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            OrganizationApiError::UpdateError(UpdateOrganizationError::NotFound(slug)) => {
                let error = ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Organization with slug '{}' not found", slug),
                );
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            OrganizationApiError::UpdateError(UpdateOrganizationError::Database(_)) => {
                tracing::error!("Database error during organization update: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            // Delete errors
            OrganizationApiError::DeleteError(DeleteOrganizationError::SlugRequired) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            OrganizationApiError::DeleteError(DeleteOrganizationError::NotFound(slug)) => {
                let error = ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Organization with slug '{}' not found", slug),
                );
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            OrganizationApiError::DeleteError(DeleteOrganizationError::HasDependencies(slug)) => {
                let error = ErrorResponse::new(
                    "CONFLICT",
                    format!(
                        "Cannot delete organization '{}': it has associated registry entries",
                        slug
                    ),
                );
                (StatusCode::CONFLICT, Json(error)).into_response()
            },
            OrganizationApiError::DeleteError(DeleteOrganizationError::Database(_)) => {
                tracing::error!("Database error during organization deletion: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            // Get errors
            OrganizationApiError::GetError(
                super::queries::GetOrganizationError::SlugOrIdRequired,
            ) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            OrganizationApiError::GetError(super::queries::GetOrganizationError::NotFound {
                ..
            }) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            OrganizationApiError::GetError(super::queries::GetOrganizationError::Database(_)) => {
                tracing::error!("Database error during organization retrieval: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            // List errors
            OrganizationApiError::ListError(
                super::queries::ListOrganizationsError::InvalidPage,
            )
            | OrganizationApiError::ListError(
                super::queries::ListOrganizationsError::InvalidPerPage,
            ) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            OrganizationApiError::ListError(super::queries::ListOrganizationsError::Database(
                _,
            )) => {
                tracing::error!("Database error during organizations listing: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for OrganizationApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateError(e) => write!(f, "{}", e),
            Self::UpdateError(e) => write!(f, "{}", e),
            Self::DeleteError(e) => write!(f, "{}", e),
            Self::GetError(e) => write!(f, "{}", e),
            Self::ListError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        use crate::features::shared::validation::SlugValidationError;
        let err = OrganizationApiError::CreateError(CreateOrganizationError::SlugValidation(
            SlugValidationError::Required,
        ));
        assert!(err.to_string().contains("Slug"));
    }

    #[test]
    fn test_routes_structure() {
        // Verify that the router can be constructed
        let router = organizations_routes();
        // This is a basic smoke test - more comprehensive testing would require
        // integration tests with a real database
        assert!(format!("{:?}", router).contains("Router"));
    }
}
