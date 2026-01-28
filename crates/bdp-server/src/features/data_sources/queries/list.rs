//! List data sources query
//!
//! Retrieves a paginated list of data sources with optional filtering
//! by organization, source type, and organism.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::features::shared::pagination::{PaginationMetadata, PaginationParams};

/// Query to list data sources with pagination and filtering
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::data_sources::queries::ListDataSourcesQuery;
/// use bdp_server::features::shared::pagination::PaginationParams;
///
/// // List all protein data sources
/// let query = ListDataSourcesQuery {
///     pagination: PaginationParams::new(Some(1), Some(20)),
///     organization_id: None,
///     source_type: Some("protein".to_string()),
///     organism_id: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDataSourcesQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism_id: Option<Uuid>,
}

/// A single data source item in the list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceListItem {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_slug: String,
    pub slug: String,
    pub name: String,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism_scientific_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub total_downloads: i64,
    pub created_at: DateTime<Utc>,
}

/// Response containing paginated list of data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDataSourcesResponse {
    /// The data sources on this page
    pub items: Vec<DataSourceListItem>,
    /// Pagination metadata
    pub pagination: PaginationMetadata,
}


/// Errors that can occur when listing data sources
#[derive(Debug, thiserror::Error)]
pub enum ListDataSourcesError {
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

impl Request<Result<ListDataSourcesResponse, ListDataSourcesError>> for ListDataSourcesQuery {}

impl crate::cqrs::middleware::Query for ListDataSourcesQuery {}

impl ListDataSourcesQuery {
    /// Validates the query parameters using shared validation
    pub fn validate(&self) -> Result<(), ListDataSourcesError> {
        self.pagination.validate().map_err(|msg| match msg {
            "Page must be greater than 0" => ListDataSourcesError::InvalidPage,
            _ => ListDataSourcesError::InvalidPerPage,
        })
    }
}

/// Handles the list data sources query
///
/// Returns a paginated list of data sources with optional filters.
/// Results include latest version and total download counts.
/// Ordered by creation date descending (newest first).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Query parameters including pagination and filters
///
/// # Returns
///
/// Returns a paginated list of data sources on success.
///
/// # Errors
///
/// - `InvalidPage` - Page is less than 1
/// - `InvalidPerPage` - Per page is less than 1 or greater than 100
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: ListDataSourcesQuery,
) -> Result<ListDataSourcesResponse, ListDataSourcesError> {
    query.validate()?;

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let offset = query.pagination.offset();

    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM registry_entries re
        JOIN data_sources ds ON re.id = ds.id
        LEFT JOIN protein_metadata pm ON ds.id = pm.data_source_id
        LEFT JOIN taxonomy_metadata om_direct ON ds.id = om_direct.data_source_id AND ds.source_type = 'organism'
        WHERE ($1::UUID IS NULL OR re.organization_id = $1)
          AND ($2::TEXT IS NULL OR ds.source_type = $2)
          AND ($3::UUID IS NULL OR pm.taxonomy_id = $3 OR (ds.source_type = 'organism' AND ds.id = $3))
        "#,
        query.organization_id,
        query.source_type.as_deref(),
        query.organism_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let records = sqlx::query_as!(
        DataSourceRecord,
        r#"
        SELECT
            re.id,
            re.organization_id,
            o.slug as organization_slug,
            re.slug,
            re.name,
            ds.source_type,
            ds.external_id,
            COALESCE(om_ref.scientific_name, om_direct.scientific_name) as organism_scientific_name,
            (
                SELECT v.version
                FROM versions v
                WHERE v.entry_id = re.id
                ORDER BY v.published_at DESC
                LIMIT 1
            ) as latest_version,
            COALESCE(
                (
                    SELECT SUM(v.download_count)::bigint
                    FROM versions v
                    WHERE v.entry_id = re.id
                ),
                0
            ) as "total_downloads!",
            re.created_at as "created_at!",
            re.updated_at as "updated_at!"
        FROM registry_entries re
        JOIN data_sources ds ON re.id = ds.id
        JOIN organizations o ON re.organization_id = o.id
        LEFT JOIN protein_metadata pm ON ds.id = pm.data_source_id
        LEFT JOIN taxonomy_metadata om_ref ON pm.taxonomy_id = om_ref.data_source_id
        LEFT JOIN taxonomy_metadata om_direct ON ds.id = om_direct.data_source_id AND ds.source_type = 'organism'
        WHERE ($1::UUID IS NULL OR re.organization_id = $1)
          AND ($2::TEXT IS NULL OR ds.source_type = $2)
          AND ($3::UUID IS NULL OR pm.taxonomy_id = $3 OR (ds.source_type = 'organism' AND ds.id = $3))
        ORDER BY re.created_at DESC
        LIMIT $4
        OFFSET $5
        "#,
        query.organization_id,
        query.source_type.as_deref(),
        query.organism_id,
        per_page,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let items = records
        .into_iter()
        .map(|r| DataSourceListItem {
            id: r.id,
            organization_id: r.organization_id,
            organization_slug: r.organization_slug,
            slug: r.slug,
            name: r.name,
            source_type: r.source_type,
            external_id: r.external_id,
            organism_scientific_name: r.organism_scientific_name,
            latest_version: r.latest_version,
            total_downloads: r.total_downloads,
            created_at: r.created_at,
        })
        .collect();

    Ok(ListDataSourcesResponse {
        items,
        pagination: PaginationMetadata::new(page, per_page, total),
    })
}

#[derive(Debug)]
#[allow(dead_code)]
struct DataSourceRecord {
    id: Uuid,
    organization_id: Uuid,
    organization_slug: String,
    slug: String,
    name: String,
    source_type: String,
    external_id: Option<String>,
    organism_scientific_name: Option<String>,
    latest_version: Option<String>,
    total_downloads: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = ListDataSourcesQuery {
            pagination: PaginationParams::new(Some(1), Some(20)),
            organization_id: None,
            source_type: None,
            organism_id: None,
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_invalid_page() {
        let query = ListDataSourcesQuery {
            pagination: PaginationParams::new(Some(0), Some(20)),
            organization_id: None,
            source_type: None,
            organism_id: None,
        };
        assert!(matches!(
            query.validate(),
            Err(ListDataSourcesError::InvalidPage)
        ));
    }

    #[test]
    fn test_validation_invalid_per_page() {
        let query = ListDataSourcesQuery {
            pagination: PaginationParams::new(Some(1), Some(101)),
            organization_id: None,
            source_type: None,
            organism_id: None,
        };
        assert!(matches!(
            query.validate(),
            Err(ListDataSourcesError::InvalidPerPage)
        ));
    }

    #[sqlx::test]
    async fn test_handle_lists_data_sources(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let query = ListDataSourcesQuery {
            pagination: PaginationParams::new(Some(1), Some(10)),
            organization_id: None,
            source_type: None,
            organism_id: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.pagination.total, 1);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_filters_by_source_type(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry1_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "protein-1",
            "Protein 1"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry1_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let entry2_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "genome-1",
            "Genome 1"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry2_id,
            "genome"
        )
        .execute(&pool)
        .await?;

        let query = ListDataSourcesQuery {
            pagination: PaginationParams::new(Some(1), Some(10)),
            organization_id: None,
            source_type: Some("protein".to_string()),
            organism_id: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].source_type, "protein");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_pagination(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        for i in 1..=25 {
            let entry_id = sqlx::query_scalar!(
                r#"
                INSERT INTO registry_entries (organization_id, slug, name, entry_type)
                VALUES ($1, $2, $3, 'data_source')
                RETURNING id
                "#,
                org_id,
                format!("protein-{}", i),
                format!("Protein {}", i)
            )
            .fetch_one(&pool)
            .await?;

            sqlx::query!(
                "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
                entry_id,
                "protein"
            )
            .execute(&pool)
            .await?;
        }

        let query = ListDataSourcesQuery {
            pagination: PaginationParams::new(Some(2), Some(10)),
            organization_id: None,
            source_type: None,
            organism_id: None,
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
