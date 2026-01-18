use crate::api::response::{ApiResponse, ErrorResponse};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;

use super::queries::{
    SearchSuggestionsError, SearchSuggestionsQuery, UnifiedSearchError, UnifiedSearchQuery,
};

pub fn search_routes() -> Router<PgPool> {
    Router::new()
        .route("/", get(unified_search))
        .route("/suggestions", get(get_suggestions))
}

#[tracing::instrument(
    skip(pool, query),
    fields(
        q = %query.query,
        type_filter = ?query.type_filter,
        page = ?query.page,
        per_page = ?query.per_page
    )
)]
async fn unified_search(
    State(pool): State<PgPool>,
    Query(query): Query<UnifiedSearchQuery>,
) -> Result<Response, SearchApiError> {
    let response = super::queries::unified_search::handle(pool, query).await?;

    tracing::debug!(
        count = response.items.len(),
        total = response.pagination.total,
        "Search completed"
    );

    let meta = json!({
        "pagination": response.pagination
    });

    Ok(
        (StatusCode::OK, Json(ApiResponse::success_with_meta(response.items, meta)))
            .into_response(),
    )
}

#[tracing::instrument(
    skip(pool, query),
    fields(
        q = %query.q,
        limit = ?query.limit,
        type_filter = ?query.type_filter
    )
)]
async fn get_suggestions(
    State(pool): State<PgPool>,
    Query(query): Query<SearchSuggestionsQuery>,
) -> Result<Response, SearchApiError> {
    let response = super::queries::suggestions::handle(pool, query).await?;

    tracing::debug!(
        count = response.suggestions.len(),
        "Suggestions completed"
    );

    Ok(
        (StatusCode::OK, Json(ApiResponse::success(response.suggestions)))
            .into_response(),
    )
}

#[derive(Debug)]
enum SearchApiError {
    SearchError(UnifiedSearchError),
    SuggestionsError(SearchSuggestionsError),
}

impl From<UnifiedSearchError> for SearchApiError {
    fn from(err: UnifiedSearchError) -> Self {
        Self::SearchError(err)
    }
}

impl From<SearchSuggestionsError> for SearchApiError {
    fn from(err: SearchSuggestionsError) -> Self {
        Self::SuggestionsError(err)
    }
}

impl IntoResponse for SearchApiError {
    fn into_response(self) -> Response {
        match self {
            SearchApiError::SearchError(UnifiedSearchError::QueryRequired)
            | SearchApiError::SearchError(UnifiedSearchError::InvalidPerPage)
            | SearchApiError::SearchError(UnifiedSearchError::InvalidPage)
            | SearchApiError::SearchError(UnifiedSearchError::InvalidTypeFilter(_))
            | SearchApiError::SuggestionsError(SearchSuggestionsError::QueryTooShort)
            | SearchApiError::SuggestionsError(SearchSuggestionsError::InvalidLimit)
            | SearchApiError::SuggestionsError(SearchSuggestionsError::InvalidTypeFilter(_)) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            }
            SearchApiError::SearchError(UnifiedSearchError::Database(_))
            | SearchApiError::SuggestionsError(SearchSuggestionsError::Database(_)) => {
                tracing::error!("Database error during search: {}", self);
                let error = ErrorResponse::new("INTERNAL_ERROR", "A database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            }
        }
    }
}

impl std::fmt::Display for SearchApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SearchError(e) => write!(f, "{}", e),
            Self::SuggestionsError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SearchApiError::SearchError(UnifiedSearchError::QueryRequired);
        assert!(err.to_string().contains("Query is required"));
    }

    #[test]
    fn test_routes_structure() {
        let router = search_routes();
        assert!(format!("{:?}", router).contains("Router"));
    }
}
