//! Database operations for data sources.
//!
//! This module provides comprehensive CRUD operations for data sources,
//! which represent biological data like proteins, genomes, and annotations.
//! Data sources inherit from registry_entries and extend them with
//! source-specific metadata.
//!
//! # Key Operations
//!
//! - `create_data_source()` - Create data source with registry entry in transaction
//! - `get_data_source()` - Get by org and name with all metadata
//! - `list_data_sources()` - Paginated list with filters
//! - `search_data_sources()` - Full-text search
//! - `update_data_source()` - Update with metadata
//! - `delete_data_source()` - Cascade deletion
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::db::{data_sources, create_pool, DbConfig};
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DbConfig::from_env()?;
//!     let pool = create_pool(&config).await?;
//!
//!     let org_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
//!
//!     // Create data source
//!     let ds = data_sources::create_data_source(
//!         &pool,
//!         org_id,
//!         "P01308",
//!         "Insulin",
//!         Some("Human insulin protein"),
//!         "protein",
//!         Some("P01308"),
//!         None,
//!         None,
//!     ).await?;
//!
//!     // Get data source
//!     let ds = data_sources::get_data_source(&pool, "uniprot", "P01308").await?;
//!
//!     Ok(())
//! }
//! ```

use chrono::Utc;
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{DbError, DbResult};
use bdp_common::types::Pagination;

// ============================================================================
// Types
// ============================================================================

/// Represents a data source with all related metadata.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DataSource {
    // Registry entry fields
    pub id: Uuid,
    pub organization_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub entry_type: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,

    // Data source fields
    pub source_type: String,
    pub external_id: Option<String>,
    pub organism_id: Option<Uuid>,
    pub additional_metadata: Option<JsonValue>,
}

/// Represents a data source with organization information.
#[derive(Debug, Clone)]
pub struct DataSourceWithOrg {
    pub data_source: DataSource,
    pub organization_slug: String,
    pub organization_name: String,
}

/// Filter options for listing data sources.
#[derive(Debug, Clone, Default)]
pub struct DataSourceFilter {
    pub source_type: Option<String>,
    pub organism_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
}

// ============================================================================
// Query Operations
// ============================================================================

/// Retrieves a data source by organization slug and data source slug.
///
/// This function performs a JOIN to get both registry entry and data source
/// information in a single query.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `org_slug` - Organization slug (e.g., "uniprot")
/// * `source_slug` - Data source slug (e.g., "P01308")
///
/// # Errors
///
/// Returns `DbError::NotFound` if the data source doesn't exist.
///
/// # Examples
///
/// ```rust,ignore
/// let ds = data_sources::get_data_source(&pool, "uniprot", "P01308").await?;
/// println!("Found: {} ({})", ds.name, ds.source_type);
/// ```
pub async fn get_data_source(
    pool: &PgPool,
    org_slug: &str,
    source_slug: &str,
) -> DbResult<DataSource> {
    let ds = sqlx::query_as!(
        DataSource,
        r#"
        SELECT
            re.id,
            re.organization_id,
            re.slug,
            re.name,
            re.description,
            re.entry_type,
            re.created_at,
            re.updated_at,
            ds.source_type,
            ds.external_id,
            ds.organism_id,
            ds.additional_metadata
        FROM registry_entries re
        JOIN data_sources ds ON ds.id = re.id
        JOIN organizations o ON o.id = re.organization_id
        WHERE o.slug = $1 AND re.slug = $2 AND re.entry_type = 'data_source'
        "#,
        org_slug,
        source_slug
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        DbError::NotFound(format!("Data source '{}:{}' not found", org_slug, source_slug))
    })?;

    Ok(ds)
}

/// Retrieves a data source by its UUID.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the data source
///
/// # Errors
///
/// Returns `DbError::NotFound` if the data source doesn't exist.
pub async fn get_data_source_by_id(pool: &PgPool, id: Uuid) -> DbResult<DataSource> {
    let ds = sqlx::query_as!(
        DataSource,
        r#"
        SELECT
            re.id,
            re.organization_id,
            re.slug,
            re.name,
            re.description,
            re.entry_type,
            re.created_at,
            re.updated_at,
            ds.source_type,
            ds.external_id,
            ds.organism_id,
            ds.additional_metadata
        FROM registry_entries re
        JOIN data_sources ds ON ds.id = re.id
        WHERE re.id = $1 AND re.entry_type = 'data_source'
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Data source with id '{}' not found", id)))?;

    Ok(ds)
}

/// Lists data sources with optional filtering and pagination.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `filter` - Optional filter criteria (type, organism, organization)
/// * `pagination` - Pagination parameters
///
/// # Examples
///
/// ```rust,ignore
/// // List all protein data sources
/// let filter = DataSourceFilter {
///     source_type: Some("protein".to_string()),
///     ..Default::default()
/// };
/// let proteins = data_sources::list_data_sources(&pool, Some(filter), Pagination::default()).await?;
/// ```
pub async fn list_data_sources(
    pool: &PgPool,
    filter: Option<DataSourceFilter>,
    pagination: Pagination,
) -> DbResult<Vec<DataSource>> {
    let filter = filter.unwrap_or_default();

    // Build dynamic query based on filters
    let mut query_str = String::from(
        r#"
        SELECT
            re.id,
            re.organization_id,
            re.slug,
            re.name,
            re.description,
            re.entry_type,
            re.created_at,
            re.updated_at,
            ds.source_type,
            ds.external_id,
            ds.organism_id,
            ds.additional_metadata
        FROM registry_entries re
        JOIN data_sources ds ON ds.id = re.id
        WHERE re.entry_type = 'data_source'
        "#,
    );

    let mut conditions = Vec::new();
    let mut bind_idx = 1;

    if filter.source_type.is_some() {
        conditions.push(format!("ds.source_type = ${}", bind_idx));
        bind_idx += 1;
    }
    if filter.organism_id.is_some() {
        conditions.push(format!("ds.organism_id = ${}", bind_idx));
        bind_idx += 1;
    }
    if filter.organization_id.is_some() {
        conditions.push(format!("re.organization_id = ${}", bind_idx));
        bind_idx += 1;
    }

    if !conditions.is_empty() {
        query_str.push_str(" AND ");
        query_str.push_str(&conditions.join(" AND "));
    }

    query_str.push_str(&format!(
        " ORDER BY re.created_at DESC LIMIT ${} OFFSET ${}",
        bind_idx,
        bind_idx + 1
    ));

    let mut query = sqlx::query_as::<Postgres, DataSource>(&query_str);

    if let Some(ref source_type) = filter.source_type {
        query = query.bind(source_type);
    }
    if let Some(organism_id) = filter.organism_id {
        query = query.bind(organism_id);
    }
    if let Some(organization_id) = filter.organization_id {
        query = query.bind(organization_id);
    }

    query = query.bind(pagination.limit).bind(pagination.offset);

    let sources = query.fetch_all(pool).await?;

    Ok(sources)
}

/// Searches data sources using full-text search.
///
/// This function uses PostgreSQL's full-text search capabilities to search
/// across name and description fields.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `search_term` - Search query
/// * `pagination` - Pagination parameters
///
/// # Examples
///
/// ```rust,ignore
/// let results = data_sources::search_data_sources(&pool, "insulin", Pagination::default()).await?;
/// ```
pub async fn search_data_sources(
    pool: &PgPool,
    search_term: &str,
    pagination: Pagination,
) -> DbResult<Vec<DataSource>> {
    let sources = sqlx::query_as!(
        DataSource,
        r#"
        SELECT
            re.id,
            re.organization_id,
            re.slug,
            re.name,
            re.description,
            re.entry_type,
            re.created_at,
            re.updated_at,
            ds.source_type,
            ds.external_id,
            ds.organism_id,
            ds.additional_metadata
        FROM registry_entries re
        JOIN data_sources ds ON ds.id = re.id
        WHERE
            re.entry_type = 'data_source' AND
            to_tsvector('english', re.name || ' ' || COALESCE(re.description, ''))
            @@ plainto_tsquery('english', $1)
        ORDER BY
            ts_rank(
                to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')),
                plainto_tsquery('english', $1)
            ) DESC
        LIMIT $2 OFFSET $3
        "#,
        search_term,
        pagination.limit,
        pagination.offset
    )
    .fetch_all(pool)
    .await?;

    Ok(sources)
}

/// Counts total data sources matching the filter.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `filter` - Optional filter criteria
///
/// # Examples
///
/// ```rust,ignore
/// let total = data_sources::count_data_sources(&pool, None).await?;
/// let pages = (total as f64 / 50.0).ceil() as i64;
/// ```
pub async fn count_data_sources(pool: &PgPool, filter: Option<DataSourceFilter>) -> DbResult<i64> {
    let filter = filter.unwrap_or_default();

    let mut query_str = String::from(
        r#"
        SELECT COUNT(*) as count
        FROM registry_entries re
        JOIN data_sources ds ON ds.id = re.id
        WHERE re.entry_type = 'data_source'
        "#,
    );

    let mut conditions = Vec::new();
    let mut bind_idx = 1;

    if filter.source_type.is_some() {
        conditions.push(format!("ds.source_type = ${}", bind_idx));
        bind_idx += 1;
    }
    if filter.organism_id.is_some() {
        conditions.push(format!("ds.organism_id = ${}", bind_idx));
        bind_idx += 1;
    }
    if filter.organization_id.is_some() {
        conditions.push(format!("re.organization_id = ${}", bind_idx));
    }

    if !conditions.is_empty() {
        query_str.push_str(" AND ");
        query_str.push_str(&conditions.join(" AND "));
    }

    let mut query = sqlx::query_scalar::<Postgres, i64>(&query_str);

    if let Some(ref source_type) = filter.source_type {
        query = query.bind(source_type);
    }
    if let Some(organism_id) = filter.organism_id {
        query = query.bind(organism_id);
    }
    if let Some(organization_id) = filter.organization_id {
        query = query.bind(organization_id);
    }

    let count = query.fetch_one(pool).await?;

    Ok(count)
}

// ============================================================================
// Mutation Operations
// ============================================================================

/// Creates a new data source along with its registry entry in a transaction.
///
/// This function creates both the registry entry and data source records
/// atomically. The registry entry is created first, then the data source
/// record references it.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `organization_id` - UUID of the organization
/// * `slug` - URL-safe slug (must be unique globally)
/// * `name` - Display name
/// * `description` - Optional description
/// * `source_type` - Type of source ('protein', 'genome', 'annotation', 'structure', 'other')
/// * `external_id` - Optional external ID (e.g., UniProt accession)
/// * `organism_id` - Optional organism reference
/// * `additional_metadata` - Optional JSONB metadata
///
/// # Errors
///
/// Returns `DbError::Duplicate` if a data source with the same slug exists.
///
/// # Examples
///
/// ```rust,ignore
/// let ds = data_sources::create_data_source(
///     &pool,
///     org_id,
///     "P01308",
///     "Insulin",
///     Some("Human insulin protein"),
///     "protein",
///     Some("P01308"),
///     None,
///     None,
/// ).await?;
/// ```
pub async fn create_data_source(
    pool: &PgPool,
    organization_id: Uuid,
    slug: &str,
    name: &str,
    description: Option<&str>,
    source_type: &str,
    external_id: Option<&str>,
    organism_id: Option<Uuid>,
    additional_metadata: Option<JsonValue>,
) -> DbResult<DataSource> {
    // Validate source_type
    if !["protein", "genome", "annotation", "structure", "other"].contains(&source_type) {
        return Err(DbError::Config(format!(
            "Invalid source_type: {}. Must be one of: protein, genome, annotation, structure, other",
            source_type
        )));
    }

    let mut tx = pool.begin().await?;

    // Create registry entry first
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO registry_entries (id, organization_id, slug, name, description, entry_type, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'data_source', $6, $7)
        "#,
        id,
        organization_id,
        slug,
        name,
        description,
        now,
        now
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return DbError::Duplicate(format!("Data source with slug '{}' already exists", slug));
            }
        }
        DbError::from(e)
    })?;

    // Create data source record
    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type, external_id, organism_id, additional_metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        id,
        source_type,
        external_id,
        organism_id,
        additional_metadata
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(
        data_source_id = %id,
        slug = %slug,
        source_type = %source_type,
        "Created data source"
    );

    // Fetch and return the created data source
    get_data_source_by_id(pool, id).await
}

/// Updates an existing data source.
///
/// This function allows updating both registry entry fields (name, description)
/// and data source-specific fields (external_id, organism_id, metadata).
/// Only non-None optional parameters are updated.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the data source
/// * `name` - Optional new name
/// * `description` - Optional new description (use Some(None) to clear)
/// * `external_id` - Optional new external ID
/// * `organism_id` - Optional new organism ID
/// * `additional_metadata` - Optional new metadata
///
/// # Examples
///
/// ```rust,ignore
/// // Update name only
/// data_sources::update_data_source(
///     &pool,
///     ds_id,
///     Some("New Name"),
///     None,
///     None,
///     None,
///     None,
/// ).await?;
/// ```
pub async fn update_data_source(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<Option<&str>>,
    external_id: Option<Option<&str>>,
    organism_id: Option<Option<Uuid>>,
    additional_metadata: Option<Option<JsonValue>>,
) -> DbResult<DataSource> {
    let mut tx = pool.begin().await?;

    // Update registry entry if needed
    if name.is_some() || description.is_some() {
        let current = get_data_source_by_id(pool, id).await?;
        let updated_name = name.unwrap_or(&current.name);
        let updated_desc = match description {
            Some(Some(d)) => Some(d),
            Some(None) => None,
            None => current.description.as_deref(),
        };
        let now = Utc::now();

        sqlx::query!(
            r#"
            UPDATE registry_entries
            SET name = $2, description = $3, updated_at = $4
            WHERE id = $1
            "#,
            id,
            updated_name,
            updated_desc,
            now
        )
        .execute(&mut *tx)
        .await?;
    }

    // Update data source fields if needed
    if external_id.is_some() || organism_id.is_some() || additional_metadata.is_some() {
        let current = get_data_source_by_id(pool, id).await?;
        let updated_external_id = match external_id {
            Some(Some(e)) => Some(e),
            Some(None) => None,
            None => current.external_id.as_deref(),
        };
        let updated_organism_id = match organism_id {
            Some(Some(o)) => Some(o),
            Some(None) => None,
            None => current.organism_id,
        };
        let updated_metadata = match additional_metadata {
            Some(Some(m)) => Some(m),
            Some(None) => None,
            None => current.additional_metadata,
        };

        sqlx::query!(
            r#"
            UPDATE data_sources
            SET external_id = $2, organism_id = $3, additional_metadata = $4
            WHERE id = $1
            "#,
            id,
            updated_external_id,
            updated_organism_id,
            updated_metadata
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!(data_source_id = %id, "Updated data source");

    // Fetch and return updated data source
    get_data_source_by_id(pool, id).await
}

/// Deletes a data source by its UUID.
///
/// This function deletes the registry entry, which cascades to delete the
/// data source record, all versions, version files, and dependencies.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the data source
///
/// # Errors
///
/// Returns `DbError::NotFound` if the data source doesn't exist.
///
/// # Examples
///
/// ```rust,ignore
/// data_sources::delete_data_source(&pool, ds_id).await?;
/// ```
pub async fn delete_data_source(pool: &PgPool, id: Uuid) -> DbResult<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM registry_entries
        WHERE id = $1 AND entry_type = 'data_source'
        "#,
        id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Data source with id '{}' not found", id)));
    }

    tracing::info!(data_source_id = %id, "Deleted data source");

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create test organization
    async fn create_test_org(pool: &PgPool) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            id,
            format!("test-org-{}", id),
            "Test Organization",
            now,
            now
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    /// Helper to cleanup test organization
    async fn cleanup_test_org(pool: &PgPool, id: Uuid) {
        sqlx::query!("DELETE FROM organizations WHERE id = $1", id)
            .execute(pool)
            .await
            .unwrap();
    }

    #[sqlx::test]
    async fn test_create_and_get_data_source(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        let ds = create_data_source(
            &pool,
            org_id,
            "test-protein-1",
            "Test Protein",
            Some("A test protein"),
            "protein",
            Some("TST001"),
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(ds.slug, "test-protein-1");
        assert_eq!(ds.name, "Test Protein");
        assert_eq!(ds.source_type, "protein");
        assert_eq!(ds.external_id.as_deref(), Some("TST001"));

        let fetched = get_data_source_by_id(&pool, ds.id).await.unwrap();
        assert_eq!(fetched.id, ds.id);

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_create_duplicate_slug(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        create_data_source(
            &pool,
            org_id,
            "duplicate-slug",
            "First",
            None,
            "protein",
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let result = create_data_source(
            &pool,
            org_id,
            "duplicate-slug",
            "Second",
            None,
            "protein",
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(result, Err(DbError::Duplicate(_))));

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_list_data_sources_with_filter(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        // Create protein sources
        for i in 0..3 {
            create_data_source(
                &pool,
                org_id,
                &format!("protein-{}", i),
                &format!("Protein {}", i),
                None,
                "protein",
                None,
                None,
                None,
            )
            .await
            .unwrap();
        }

        // Create genome source
        create_data_source(&pool, org_id, "genome-1", "Genome 1", None, "genome", None, None, None)
            .await
            .unwrap();

        // Filter by protein type
        let filter = DataSourceFilter {
            source_type: Some("protein".to_string()),
            ..Default::default()
        };
        let proteins = list_data_sources(&pool, Some(filter), Pagination::default())
            .await
            .unwrap();

        assert_eq!(proteins.len(), 3);
        assert!(proteins.iter().all(|ds| ds.source_type == "protein"));

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_search_data_sources(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        create_data_source(
            &pool,
            org_id,
            "insulin-protein",
            "Insulin Protein",
            Some("Human insulin hormone"),
            "protein",
            None,
            None,
            None,
        )
        .await
        .unwrap();

        create_data_source(
            &pool,
            org_id,
            "other-protein",
            "Other Protein",
            Some("Not related to insulin"),
            "protein",
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let results = search_data_sources(&pool, "insulin", Pagination::default())
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slug, "insulin-protein");

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_update_data_source(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        let ds = create_data_source(
            &pool,
            org_id,
            "update-test",
            "Original Name",
            None,
            "protein",
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let updated = update_data_source(
            &pool,
            ds.id,
            Some("Updated Name"),
            Some(Some("New description")),
            Some(Some("EXT123")),
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description.as_deref(), Some("New description"));
        assert_eq!(updated.external_id.as_deref(), Some("EXT123"));

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_delete_data_source(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        let ds = create_data_source(
            &pool,
            org_id,
            "delete-test",
            "Delete Me",
            None,
            "protein",
            None,
            None,
            None,
        )
        .await
        .unwrap();

        delete_data_source(&pool, ds.id).await.unwrap();

        let result = get_data_source_by_id(&pool, ds.id).await;
        assert!(matches!(result, Err(DbError::NotFound(_))));

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_count_data_sources(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        for i in 0..5 {
            create_data_source(
                &pool,
                org_id,
                &format!("count-test-{}", i),
                &format!("Count Test {}", i),
                None,
                "protein",
                None,
                None,
                None,
            )
            .await
            .unwrap();
        }

        let total = count_data_sources(&pool, None).await.unwrap();
        assert!(total >= 5);

        let filter = DataSourceFilter {
            organization_id: Some(org_id),
            ..Default::default()
        };
        let org_count = count_data_sources(&pool, Some(filter)).await.unwrap();
        assert_eq!(org_count, 5);

        cleanup_test_org(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_invalid_source_type(pool: PgPool) {
        let org_id = create_test_org(&pool).await;

        let result = create_data_source(
            &pool,
            org_id,
            "invalid-type",
            "Invalid",
            None,
            "invalid_type",
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(result, Err(DbError::Config(_))));

        cleanup_test_org(&pool, org_id).await;
    }
}
