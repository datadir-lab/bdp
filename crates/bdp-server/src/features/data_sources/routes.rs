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
        CreateDataSourceCommand, CreateDataSourceError, DeleteDataSourceCommand,
        DeleteDataSourceError, PublishVersionCommand, PublishVersionError, UpdateDataSourceCommand,
        UpdateDataSourceError,
    },
    queries::{GetDataSourceQuery, GetVersionQuery, ListDataSourcesQuery, ListDependenciesQuery},
};

pub fn data_sources_routes() -> Router<PgPool> {
    Router::new()
        .route("/", post(create_data_source))
        .route("/", get(list_data_sources))
        .route("/source-types", get(get_source_types))
        .route("/:org/:slug", get(get_data_source))
        .route("/:org/:slug", put(update_data_source))
        .route("/:org/:slug", delete(delete_data_source))
        .route("/:org/:slug/versions", post(publish_version))
        .route("/:org/:slug/:version", get(get_version))
        .route(
            "/:org/:slug/:version/protein-metadata",
            get(super::queries::get_protein_metadata::get_protein_metadata),
        )
        .route("/:org/:slug/:version/dependencies", get(list_dependencies))
}

#[tracing::instrument(skip(pool, command), fields(slug = %command.slug, name = %command.name))]
async fn create_data_source(
    State(pool): State<PgPool>,
    Json(command): Json<CreateDataSourceCommand>,
) -> Result<Response, DataSourceApiError> {
    let response = super::commands::create::handle(pool, command).await?;

    tracing::info!(
        data_source_id = %response.id,
        data_source_slug = %response.slug,
        "Data source created via API"
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(pool, command), fields(id = %id))]
async fn update_data_source(
    State(pool): State<PgPool>,
    Path((_org, id)): Path<(String, uuid::Uuid)>,
    Json(mut command): Json<UpdateDataSourceCommand>,
) -> Result<Response, DataSourceApiError> {
    command.id = id;

    let response = super::commands::update::handle(pool, command).await?;

    tracing::info!(
        data_source_id = %response.id,
        data_source_slug = %response.slug,
        "Data source updated via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(pool), fields(id = %id))]
async fn delete_data_source(
    State(pool): State<PgPool>,
    Path((_org, id)): Path<(String, uuid::Uuid)>,
) -> Result<Response, DataSourceApiError> {
    let command = DeleteDataSourceCommand { id };

    let response = super::commands::delete::handle(pool, command).await?;

    tracing::info!(
        data_source_id = %response.id,
        "Data source deleted via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(pool, command), fields(data_source_id = %command.data_source_id, version = %command.version))]
async fn publish_version(
    State(pool): State<PgPool>,
    Path((_org, _slug)): Path<(String, String)>,
    Json(command): Json<PublishVersionCommand>,
) -> Result<Response, DataSourceApiError> {
    let response = super::commands::publish::handle(pool, command).await?;

    tracing::info!(
        version_id = %response.id,
        version = %response.version,
        "Version published via API"
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(pool), fields(org = %org, slug = %slug))]
async fn get_data_source(
    State(pool): State<PgPool>,
    Path((org, slug)): Path<(String, String)>,
) -> Result<Response, DataSourceApiError> {
    let query = GetDataSourceQuery {
        organization_slug: org,
        slug,
    };

    let response = super::queries::get::handle(pool, query).await?;

    tracing::debug!(
        data_source_id = %response.id,
        data_source_slug = %response.slug,
        "Data source retrieved via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(pool))]
async fn get_source_types(State(pool): State<PgPool>) -> Result<Response, DataSourceApiError> {
    let source_types = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT source_type
        FROM data_sources
        ORDER BY source_type
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        DataSourceApiError::ListError(super::queries::ListDataSourcesError::Database(e))
    })?;

    tracing::debug!(count = source_types.len(), "Source types retrieved via API");

    Ok((StatusCode::OK, Json(ApiResponse::success(source_types))).into_response())
}

#[tracing::instrument(skip(pool, query), fields(page = ?query.pagination.page, per_page = ?query.pagination.per_page))]
async fn list_data_sources(
    State(pool): State<PgPool>,
    Query(query): Query<ListDataSourcesQuery>,
) -> Result<Response, DataSourceApiError> {
    let response = super::queries::list::handle(pool, query).await?;

    tracing::debug!(
        count = response.items.len(),
        total = response.pagination.total,
        "Data sources listed via API"
    );

    let meta = json!({
        "pagination": response.pagination
    });

    Ok(
        (StatusCode::OK, Json(ApiResponse::success_with_meta(response.items, meta)))
            .into_response(),
    )
}

#[tracing::instrument(skip(pool), fields(org = %org, slug = %slug, version = %version))]
async fn get_version(
    State(pool): State<PgPool>,
    Path((org, slug, version)): Path<(String, String, String)>,
) -> Result<Response, DataSourceApiError> {
    let query = GetVersionQuery {
        organization_slug: org,
        data_source_slug: slug,
        version,
    };

    let response = super::queries::get_version::handle(pool, query).await?;

    tracing::debug!(
        version_id = %response.id,
        version = %response.version,
        "Version retrieved via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(pool, query), fields(org = %org, slug = %slug, version = %version))]
async fn list_dependencies(
    State(pool): State<PgPool>,
    Path((org, slug, version)): Path<(String, String, String)>,
    Query(mut query): Query<ListDependenciesQuery>,
) -> Result<Response, DataSourceApiError> {
    query.organization_slug = org;
    query.data_source_slug = slug;
    query.version = version;

    let response = super::queries::list_dependencies::handle(pool, query).await?;

    tracing::debug!(dependency_count = response.dependency_count, "Dependencies listed via API");

    let meta = json!({
        "pagination": response.pagination
    });

    Ok((StatusCode::OK, Json(ApiResponse::success_with_meta(response, meta))).into_response())
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum DataSourceApiError {
    CreateError(CreateDataSourceError),
    UpdateError(UpdateDataSourceError),
    DeleteError(DeleteDataSourceError),
    PublishError(PublishVersionError),
    GetError(super::queries::GetDataSourceError),
    ListError(super::queries::ListDataSourcesError),
    GetVersionError(super::queries::GetVersionError),
    ListDependenciesError(super::queries::ListDependenciesError),
}

impl From<CreateDataSourceError> for DataSourceApiError {
    fn from(err: CreateDataSourceError) -> Self {
        Self::CreateError(err)
    }
}

impl From<UpdateDataSourceError> for DataSourceApiError {
    fn from(err: UpdateDataSourceError) -> Self {
        Self::UpdateError(err)
    }
}

impl From<DeleteDataSourceError> for DataSourceApiError {
    fn from(err: DeleteDataSourceError) -> Self {
        Self::DeleteError(err)
    }
}

impl From<PublishVersionError> for DataSourceApiError {
    fn from(err: PublishVersionError) -> Self {
        Self::PublishError(err)
    }
}

impl From<super::queries::GetDataSourceError> for DataSourceApiError {
    fn from(err: super::queries::GetDataSourceError) -> Self {
        Self::GetError(err)
    }
}

impl From<super::queries::ListDataSourcesError> for DataSourceApiError {
    fn from(err: super::queries::ListDataSourcesError) -> Self {
        Self::ListError(err)
    }
}

impl From<super::queries::GetVersionError> for DataSourceApiError {
    fn from(err: super::queries::GetVersionError) -> Self {
        Self::GetVersionError(err)
    }
}

impl From<super::queries::ListDependenciesError> for DataSourceApiError {
    fn from(err: super::queries::ListDependenciesError) -> Self {
        Self::ListDependenciesError(err)
    }
}

impl IntoResponse for DataSourceApiError {
    fn into_response(self) -> Response {
        match self {
            DataSourceApiError::CreateError(CreateDataSourceError::SlugValidation(_))
            | DataSourceApiError::CreateError(CreateDataSourceError::NameValidation(_))
            | DataSourceApiError::CreateError(CreateDataSourceError::SourceTypeValidation(_)) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::CreateError(CreateDataSourceError::OrganizationNotFound(_))
            | DataSourceApiError::CreateError(CreateDataSourceError::OrganismNotFound(_)) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::CreateError(CreateDataSourceError::DuplicateSlug(_)) => {
                let error = ErrorResponse::new("CONFLICT", self.to_string());
                (StatusCode::CONFLICT, Json(error)).into_response()
            },
            DataSourceApiError::CreateError(CreateDataSourceError::Database(_)) => {
                tracing::error!("Database error during data source creation: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::UpdateError(UpdateDataSourceError::NoFieldsToUpdate)
            | DataSourceApiError::UpdateError(UpdateDataSourceError::NameLength)
            | DataSourceApiError::UpdateError(UpdateDataSourceError::NameEmpty) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::UpdateError(UpdateDataSourceError::NotFound(_))
            | DataSourceApiError::UpdateError(UpdateDataSourceError::OrganismNotFound(_)) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::UpdateError(UpdateDataSourceError::Database(_)) => {
                tracing::error!("Database error during data source update: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::DeleteError(DeleteDataSourceError::NotFound(_)) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::DeleteError(DeleteDataSourceError::HasVersions(_)) => {
                let error = ErrorResponse::new("CONFLICT", self.to_string());
                (StatusCode::CONFLICT, Json(error)).into_response()
            },
            DataSourceApiError::DeleteError(DeleteDataSourceError::Database(_)) => {
                tracing::error!("Database error during data source deletion: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::PublishError(PublishVersionError::VersionRequired)
            | DataSourceApiError::PublishError(PublishVersionError::VersionLength)
            | DataSourceApiError::PublishError(PublishVersionError::InvalidSize) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::PublishError(PublishVersionError::DataSourceNotFound(_)) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::PublishError(PublishVersionError::DuplicateVersion(_, _)) => {
                let error = ErrorResponse::new("CONFLICT", self.to_string());
                (StatusCode::CONFLICT, Json(error)).into_response()
            },
            DataSourceApiError::PublishError(PublishVersionError::Database(_)) => {
                tracing::error!("Database error during version publish: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::GetError(
                super::queries::GetDataSourceError::OrganizationSlugRequired,
            )
            | DataSourceApiError::GetError(super::queries::GetDataSourceError::SlugRequired) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::GetError(super::queries::GetDataSourceError::NotFound(_, _)) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::GetError(super::queries::GetDataSourceError::Database(_)) => {
                tracing::error!("Database error during data source retrieval: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::ListError(super::queries::ListDataSourcesError::InvalidPage)
            | DataSourceApiError::ListError(super::queries::ListDataSourcesError::InvalidPerPage) =>
            {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::ListError(super::queries::ListDataSourcesError::Database(_)) => {
                tracing::error!("Database error during data sources listing: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::GetVersionError(
                super::queries::GetVersionError::OrganizationSlugRequired,
            )
            | DataSourceApiError::GetVersionError(
                super::queries::GetVersionError::DataSourceSlugRequired,
            )
            | DataSourceApiError::GetVersionError(
                super::queries::GetVersionError::VersionRequired,
            ) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::GetVersionError(super::queries::GetVersionError::NotFound(
                _,
                _,
                _,
            )) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::GetVersionError(super::queries::GetVersionError::Database(_)) => {
                tracing::error!("Database error during version retrieval: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::OrganizationSlugRequired,
            )
            | DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::DataSourceSlugRequired,
            )
            | DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::VersionRequired,
            )
            | DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::InvalidPage,
            )
            | DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::InvalidPerPage,
            ) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::NotFound(_, _, _),
            ) => {
                let error = ErrorResponse::new("NOT_FOUND", self.to_string());
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            DataSourceApiError::ListDependenciesError(
                super::queries::ListDependenciesError::Database(_),
            ) => {
                tracing::error!("Database error during dependencies listing: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for DataSourceApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateError(e) => write!(f, "{}", e),
            Self::UpdateError(e) => write!(f, "{}", e),
            Self::DeleteError(e) => write!(f, "{}", e),
            Self::PublishError(e) => write!(f, "{}", e),
            Self::GetError(e) => write!(f, "{}", e),
            Self::ListError(e) => write!(f, "{}", e),
            Self::GetVersionError(e) => write!(f, "{}", e),
            Self::ListDependenciesError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        use crate::features::shared::validation::SlugValidationError;
        let err = DataSourceApiError::CreateError(CreateDataSourceError::SlugValidation(
            SlugValidationError::Required,
        ));
        assert!(err.to_string().contains("Slug"));
    }

    #[test]
    fn test_routes_structure() {
        let router = data_sources_routes();
        assert!(format!("{:?}", router).contains("Router"));
    }
}
