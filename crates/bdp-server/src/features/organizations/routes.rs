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
//!
//! # Examples
//!
//! ## Creating a Router
//!
//! ```rust,ignore
//! use axum::Router;
//! use bdp_server::features::organizations::routes::organizations_routes;
//!
//! let app = Router::new()
//!     .nest("/api/v1/organizations", organizations_routes())
//!     .with_state(pool);
//! ```

use crate::api::response::{ApiResponse, ErrorResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;

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

/// Creates the organizations router with all routes configured
///
/// # Examples
///
/// ```rust,ignore
/// use axum::Router;
/// use bdp_server::features::organizations::routes::organizations_routes;
///
/// let app = Router::new()
///     .nest("/api/v1/organizations", organizations_routes())
///     .with_state(pool);
/// ```
pub fn organizations_routes() -> Router<PgPool> {
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

/// Create a new organization
///
/// # Endpoint
///
/// `POST /api/v1/organizations`
///
/// # Request Body
///
/// ```json
/// {
///   "slug": "acme-corp",
///   "name": "ACME Corporation",
///   "website": "https://acme.com",
///   "description": "Leading provider of quality products",
///   "logo_url": null,
///   "is_system": false
/// }
/// ```
///
/// # Response
///
/// - `201 Created` - Organization created successfully
/// - `400 Bad Request` - Validation error
/// - `409 Conflict` - Organization with slug already exists
/// - `500 Internal Server Error` - Database error
#[tracing::instrument(
    skip(pool, command),
    fields(slug = %command.slug, name = %command.name)
)]
async fn create_organization(
    State(pool): State<PgPool>,
    Json(command): Json<CreateOrganizationCommand>,
) -> Result<Response, OrganizationApiError> {
    let response = super::commands::create::handle(pool, command).await?;

    tracing::info!(
        org_id = %response.id,
        org_slug = %response.slug,
        "Organization created via API"
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))).into_response())
}

/// Update an existing organization
///
/// # Endpoint
///
/// `PUT /api/v1/organizations/:slug`
///
/// # Request Body
///
/// ```json
/// {
///   "name": "ACME Corp",
///   "website": "https://acme.com",
///   "description": "Updated description"
/// }
/// ```
///
/// # Response
///
/// - `200 OK` - Organization updated successfully
/// - `400 Bad Request` - Validation error
/// - `404 Not Found` - Organization not found
/// - `500 Internal Server Error` - Database error
#[tracing::instrument(
    skip(pool, command),
    fields(slug = %slug)
)]
async fn update_organization(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Json(mut command): Json<UpdateOrganizationCommand>,
) -> Result<Response, OrganizationApiError> {
    // Set slug from path parameter
    command.slug = slug;

    let response = super::commands::update::handle(pool, command).await?;

    tracing::info!(
        org_id = %response.id,
        org_slug = %response.slug,
        "Organization updated via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

/// Delete an organization
///
/// # Endpoint
///
/// `DELETE /api/v1/organizations/:slug`
///
/// # Response
///
/// - `200 OK` - Organization deleted successfully
/// - `404 Not Found` - Organization not found
/// - `409 Conflict` - Cannot delete (has dependencies)
/// - `500 Internal Server Error` - Database error
#[tracing::instrument(
    skip(pool),
    fields(slug = %slug)
)]
async fn delete_organization(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Result<Response, OrganizationApiError> {
    let command = DeleteOrganizationCommand { slug };

    let response = super::commands::delete::handle(pool, command).await?;

    tracing::info!(
        org_slug = %response.slug,
        "Organization deleted via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

// ============================================================================
// Query Handlers (Read Operations)
// ============================================================================

/// Get a single organization by slug
///
/// # Endpoint
///
/// `GET /api/v1/organizations/:slug`
///
/// # Response
///
/// - `200 OK` - Organization found
/// - `404 Not Found` - Organization not found
/// - `500 Internal Server Error` - Database error
#[tracing::instrument(
    skip(pool),
    fields(slug = %slug)
)]
async fn get_organization(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Result<Response, OrganizationApiError> {
    let query = GetOrganizationQuery {
        slug: Some(slug),
        id: None,
    };

    let response = super::queries::get::handle(pool, query).await?;

    tracing::debug!(
        org_id = %response.id,
        org_slug = %response.slug,
        "Organization retrieved via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

/// List organizations with pagination and filters
///
/// # Endpoint
///
/// `GET /api/v1/organizations?page=1&per_page=20&is_system=true&name_contains=protein`
///
/// # Query Parameters
///
/// - `page` - Page number (default: 1)
/// - `per_page` - Items per page (default: 20, max: 100)
/// - `is_system` - Filter by system flag
/// - `name_contains` - Filter by name (case-insensitive partial match)
///
/// # Response
///
/// - `200 OK` - List of organizations with pagination metadata
/// - `400 Bad Request` - Invalid query parameters
/// - `500 Internal Server Error` - Database error
#[tracing::instrument(
    skip(pool, query),
    fields(
        page = ?query.pagination.page,
        per_page = ?query.pagination.per_page,
        is_system = ?query.is_system
    )
)]
async fn list_organizations(
    State(pool): State<PgPool>,
    Query(query): Query<ListOrganizationsQuery>,
) -> Result<Response, OrganizationApiError> {
    let response = super::queries::list::handle(pool, query).await?;

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
            // Create errors
            OrganizationApiError::CreateError(CreateOrganizationError::SlugRequired)
            | OrganizationApiError::CreateError(CreateOrganizationError::SlugLength)
            | OrganizationApiError::CreateError(CreateOrganizationError::SlugFormat)
            | OrganizationApiError::CreateError(CreateOrganizationError::NameRequired)
            | OrganizationApiError::CreateError(CreateOrganizationError::NameLength)
            | OrganizationApiError::CreateError(CreateOrganizationError::WebsiteInvalid(_))
            | OrganizationApiError::CreateError(CreateOrganizationError::LogoUrlInvalid(_)) => {
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

            // Update errors
            OrganizationApiError::UpdateError(UpdateOrganizationError::SlugRequired)
            | OrganizationApiError::UpdateError(UpdateOrganizationError::NoFieldsToUpdate)
            | OrganizationApiError::UpdateError(UpdateOrganizationError::NameLength)
            | OrganizationApiError::UpdateError(UpdateOrganizationError::NameEmpty)
            | OrganizationApiError::UpdateError(UpdateOrganizationError::WebsiteInvalid(_))
            | OrganizationApiError::UpdateError(UpdateOrganizationError::LogoUrlInvalid(_)) => {
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
            OrganizationApiError::GetError(super::queries::GetOrganizationError::SlugOrIdRequired) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            OrganizationApiError::GetError(super::queries::GetOrganizationError::NotFound { .. }) => {
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
        let err = OrganizationApiError::CreateError(CreateOrganizationError::SlugRequired);
        assert!(err.to_string().contains("Slug is required"));
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
