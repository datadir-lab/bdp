//! List organizations query
//!
//! Retrieves a paginated list of organizations with optional filtering.
//! Supports filtering by system organization status and name search.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::features::shared::pagination::{PaginationMetadata, PaginationParams};

/// Query to list organizations with pagination and filtering
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::organizations::queries::ListOrganizationsQuery;
///
/// // List all organizations (first page)
/// let query = ListOrganizationsQuery {
///     pagination: PaginationParams::new(Some(1), Some(20)),
///     is_system: None,
///     name_contains: None,
/// };
///
/// // Search for organizations containing "uni"
/// let query = ListOrganizationsQuery {
///     pagination: PaginationParams::new(Some(1), Some(10)),
///     is_system: Some(true),
///     name_contains: Some("uni".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrganizationsQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_system: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

/// A single organization item in the list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationListItem {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

/// Response containing paginated list of organizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrganizationsResponse {
    /// The organizations on this page
    pub items: Vec<OrganizationListItem>,
    /// Pagination metadata
    pub pagination: PaginationMetadata,
}

/// Errors that can occur when listing organizations
#[derive(Debug, thiserror::Error)]
pub enum ListOrganizationsError {
    /// Page number must be at least 1
    #[error("Page must be greater than 0")]
    InvalidPage,
    /// Per page must be between 1 and 100
    #[error("Per page must be between 1 and 100")]
    InvalidPerPage,
    /// A database error occurred
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<ListOrganizationsResponse, ListOrganizationsError>> for ListOrganizationsQuery {}

impl crate::cqrs::middleware::Query for ListOrganizationsQuery {}

impl ListOrganizationsQuery {
    /// Validates the query parameters
    ///
    /// # Errors
    ///
    /// - `InvalidPage` - Page is less than 1
    /// - `InvalidPerPage` - Per page is less than 1 or greater than 100
    pub fn validate(&self) -> Result<(), ListOrganizationsError> {
        self.pagination.validate().map_err(|msg| match msg {
            "Page must be greater than 0" => ListOrganizationsError::InvalidPage,
            _ => ListOrganizationsError::InvalidPerPage,
        })
    }
}

/// Handles the list organizations query
///
/// Returns a paginated list of organizations, optionally filtered by
/// system status and name search. Results are ordered by creation date
/// descending (newest first).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Query parameters including pagination and filters
///
/// # Returns
///
/// Returns a paginated list of organizations on success.
///
/// # Errors
///
/// - `InvalidPage` - Page is less than 1
/// - `InvalidPerPage` - Per page is less than 1 or greater than 100
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: ListOrganizationsQuery,
) -> Result<ListOrganizationsResponse, ListOrganizationsError> {
    query.validate()?;

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let offset = query.pagination.offset();

    let total = if query.is_system.is_some() || query.name_contains.is_some() {
        let name_pattern = query
            .name_contains
            .as_ref()
            .map(|s| format!("%{}%", s.to_lowercase()));

        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM organizations
            WHERE ($1::BOOLEAN IS NULL OR is_system = $1)
              AND ($2::TEXT IS NULL OR LOWER(name) LIKE $2)
            "#,
            query.is_system,
            name_pattern.as_deref()
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(0)
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM organizations
            "#
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(0)
    };

    let name_pattern = query
        .name_contains
        .as_ref()
        .map(|s| format!("%{}%", s.to_lowercase()));

    let records = sqlx::query_as!(
        OrganizationRecord,
        r#"
        SELECT id, slug, name, website, description, logo_url,
               is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
        FROM organizations
        WHERE ($1::BOOLEAN IS NULL OR is_system = $1)
          AND ($2::TEXT IS NULL OR LOWER(name) LIKE $2)
        ORDER BY created_at DESC
        LIMIT $3
        OFFSET $4
        "#,
        query.is_system,
        name_pattern.as_deref(),
        per_page,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let items = records
        .into_iter()
        .map(|r| OrganizationListItem {
            id: r.id,
            slug: r.slug,
            name: r.name,
            website: r.website,
            description: r.description,
            logo_url: r.logo_url,
            is_system: r.is_system,
            created_at: r.created_at,
        })
        .collect();

    Ok(ListOrganizationsResponse {
        items,
        pagination: PaginationMetadata::new(page, per_page, total),
    })
}

#[derive(Debug)]
#[allow(dead_code)]
struct OrganizationRecord {
    id: Uuid,
    slug: String,
    name: String,
    website: Option<String>,
    description: Option<String>,
    logo_url: Option<String>,
    is_system: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(1), Some(20)),
            is_system: None,
            name_contains: None,
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_invalid_page() {
        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(0), Some(20)),
            is_system: None,
            name_contains: None,
        };
        assert!(matches!(query.validate(), Err(ListOrganizationsError::InvalidPage)));
    }

    #[test]
    fn test_validation_invalid_per_page() {
        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(1), Some(101)),
            is_system: None,
            name_contains: None,
        };
        assert!(matches!(query.validate(), Err(ListOrganizationsError::InvalidPerPage)));
    }

    #[sqlx::test]
    async fn test_handle_lists_organizations(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org-1', 'Test Org 1', false),
                   ('test-org-2', 'Test Org 2', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(1), Some(10)),
            is_system: None,
            name_contains: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.pagination.total, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_filters_by_is_system(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org-1', 'Test Org 1', false),
                   ('test-org-2', 'Test Org 2', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(1), Some(10)),
            is_system: Some(true),
            name_contains: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].slug, "test-org-2");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_filters_by_name(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('uniprot', 'UniProt', true),
                   ('ncbi', 'NCBI', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(1), Some(10)),
            is_system: None,
            name_contains: Some("uni".to_string()),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].slug, "uniprot");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_pagination(pool: PgPool) -> sqlx::Result<()> {
        for i in 1..=25 {
            sqlx::query!(
                r#"
                INSERT INTO organizations (slug, name, is_system)
                VALUES ($1, $2, false)
                "#,
                format!("org-{}", i),
                format!("Org {}", i)
            )
            .execute(&pool)
            .await?;
        }

        let query = ListOrganizationsQuery {
            pagination: PaginationParams::new(Some(2), Some(10)),
            is_system: None,
            name_contains: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 10);
        assert_eq!(response.pagination.page, 2);
        assert_eq!(response.pagination.total, 25);
        assert_eq!(response.pagination.pages, 3);
        assert!(response.pagination.has_prev);
        assert!(response.pagination.has_next);
        Ok(())
    }
}
