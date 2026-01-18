//! Database operations for unified search across data sources and tools.
//!
//! This module provides full-text search capabilities using PostgreSQL's
//! tsvector and tsquery functionality, with support for filtering by type,
//! organism, and format.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::db::{search, create_pool, DbConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DbConfig::from_env()?;
//!     let pool = create_pool(&config).await?;
//!
//!     // Search for "insulin" in data sources
//!     let results = search::unified_search(
//!         &pool,
//!         "insulin",
//!         &search::SearchFilters {
//!             entry_type: Some(vec!["data_source".to_string()]),
//!             organism: Some("human".to_string()),
//!             format: Some("fasta".to_string()),
//!         },
//!         search::Pagination::default(),
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

use bdp_common::types::Pagination;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{DbError, DbResult};

// ============================================================================
// Types
// ============================================================================

/// Search filters for unified search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Filter by entry type (data_source, tool)
    pub entry_type: Option<Vec<String>>,

    /// Filter by organism (common name or scientific name)
    pub organism: Option<String>,

    /// Filter by file format (fasta, xml, json, etc.)
    pub format: Option<String>,
}

/// Unified search result that can represent either a data source or tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Entry ID
    pub id: Uuid,

    /// Organization slug
    pub organization_slug: String,

    /// Entry slug
    pub slug: String,

    /// Entry name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Entry type (data_source or tool)
    pub entry_type: String,

    /// Source type (for data sources: protein, genome, etc.)
    pub source_type: Option<String>,

    /// Tool type (for tools: alignment, assembly, etc.)
    pub tool_type: Option<String>,

    /// Organism info (for data sources)
    pub organism: Option<OrganismInfo>,

    /// Latest version
    pub latest_version: Option<String>,

    /// External version
    pub external_version: Option<String>,

    /// Available formats
    pub available_formats: Vec<String>,

    /// Total downloads
    pub total_downloads: i64,

    /// External ID (for data sources)
    pub external_id: Option<String>,
}

/// Organism information for search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismInfo {
    /// Scientific name
    pub scientific_name: String,

    /// Common name
    pub common_name: Option<String>,

    /// NCBI Taxonomy ID
    pub ncbi_taxonomy_id: Option<i32>,
}

// ============================================================================
// Search Operations
// ============================================================================

/// Performs unified search across data sources and tools.
///
/// This function uses PostgreSQL's full-text search capabilities to find
/// registry entries matching the query string. It supports filtering by
/// entry type, organism, and file format.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Search query string
/// * `filters` - Optional filters for entry type, organism, and format
/// * `pagination` - Pagination parameters
///
/// # Errors
///
/// Returns `DbError::Sqlx` for database errors.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::{search, create_pool, DbConfig};
/// use bdp_common::types::Pagination;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = DbConfig::from_env()?;
///     let pool = create_pool(&config).await?;
///
///     // Search for "insulin" with filters
///     let filters = search::SearchFilters {
///         entry_type: Some(vec!["data_source".to_string()]),
///         organism: Some("human".to_string()),
///         format: Some("fasta".to_string()),
///     };
///
///     let results = search::unified_search(
///         &pool,
///         "insulin",
///         &filters,
///         Pagination::default(),
///     ).await?;
///
///     for result in results {
///         println!("Found: {} ({})", result.name, result.slug);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn unified_search(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
    pagination: Pagination,
) -> DbResult<Vec<SearchResult>> {
    // Build the base query with full-text search
    let mut sql = String::from(
        r#"
        SELECT
            re.id,
            o.slug as organization_slug,
            re.slug,
            re.name,
            re.description,
            re.entry_type,
            ds.source_type,
            t.tool_type,
            org.scientific_name,
            org.common_name,
            org.ncbi_taxonomy_id,
            ds.external_id,
            (
                SELECT v.version
                FROM versions v
                WHERE v.entry_id = re.id
                ORDER BY v.published_at DESC
                LIMIT 1
            ) as latest_version,
            (
                SELECT v.external_version
                FROM versions v
                WHERE v.entry_id = re.id
                ORDER BY v.published_at DESC
                LIMIT 1
            ) as external_version,
            COALESCE(
                (
                    SELECT ARRAY_AGG(DISTINCT vf.format)
                    FROM versions v
                    JOIN version_files vf ON vf.version_id = v.id
                    WHERE v.entry_id = re.id
                ),
                ARRAY[]::VARCHAR[]
            ) as available_formats,
            COALESCE(
                (
                    SELECT SUM(v.download_count)
                    FROM versions v
                    WHERE v.entry_id = re.id
                ),
                0
            ) as total_downloads
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        LEFT JOIN tools t ON t.id = re.id
        LEFT JOIN organisms org ON org.id = ds.organism_id
        WHERE to_tsvector('english', re.name || ' ' || COALESCE(re.description, ''))
            @@ plainto_tsquery('english', $1)
        "#,
    );

    let mut param_index = 2; // $1 is the query
    let mut conditions = Vec::new();

    // Add entry type filter
    if let Some(ref types) = filters.entry_type {
        if !types.is_empty() {
            conditions.push(format!("re.entry_type = ANY(${})", param_index));
            param_index += 1;
        }
    }

    // Add organism filter
    if filters.organism.is_some() {
        conditions.push(format!(
            "(org.scientific_name ILIKE ${} OR org.common_name ILIKE ${})",
            param_index,
            param_index + 1
        ));
        param_index += 2;
    }

    // Add format filter
    if filters.format.is_some() {
        conditions.push(format!(
            r#"EXISTS (
                SELECT 1
                FROM versions v
                JOIN version_files vf ON vf.version_id = v.id
                WHERE v.entry_id = re.id AND vf.format = ${}
            )"#,
            param_index
        ));
        param_index += 1;
    }

    // Append additional conditions
    if !conditions.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(&conditions.join(" AND "));
    }

    // Add ordering and pagination
    sql.push_str(&format!(
        r#"
        ORDER BY
            ts_rank(
                to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')),
                plainto_tsquery('english', $1)
            ) DESC,
            total_downloads DESC,
            re.created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        param_index,
        param_index + 1
    ));

    // Build the query dynamically
    let mut query_builder = sqlx::query_as::<_, SearchResultRow>(&sql);

    // Bind parameters
    query_builder = query_builder.bind(query);

    if let Some(ref types) = filters.entry_type {
        if !types.is_empty() {
            query_builder = query_builder.bind(types);
        }
    }

    if let Some(ref organism) = filters.organism {
        let pattern = format!("%{}%", organism);
        query_builder = query_builder.bind(&pattern);
        query_builder = query_builder.bind(&pattern);
    }

    if let Some(ref format) = filters.format {
        query_builder = query_builder.bind(format);
    }

    query_builder = query_builder.bind(pagination.limit);
    query_builder = query_builder.bind(pagination.offset);

    // Execute query
    let rows = query_builder.fetch_all(pool).await?;

    // Convert rows to SearchResult
    let results = rows
        .into_iter()
        .map(|row| SearchResult {
            id: row.id,
            organization_slug: row.organization_slug,
            slug: row.slug,
            name: row.name,
            description: row.description,
            entry_type: row.entry_type,
            source_type: row.source_type,
            tool_type: row.tool_type,
            organism: if row.scientific_name.is_some() {
                Some(OrganismInfo {
                    scientific_name: row.scientific_name.unwrap(),
                    common_name: row.common_name,
                    ncbi_taxonomy_id: row.ncbi_taxonomy_id,
                })
            } else {
                None
            },
            latest_version: row.latest_version,
            external_version: row.external_version,
            available_formats: row.available_formats,
            total_downloads: row.total_downloads,
            external_id: row.external_id,
        })
        .collect();

    Ok(results)
}

/// Counts total search results for pagination.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Search query string
/// * `filters` - Optional filters for entry type, organism, and format
///
/// # Errors
///
/// Returns `DbError::Sqlx` for database errors.
pub async fn count_search_results(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> DbResult<i64> {
    let mut sql = String::from(
        r#"
        SELECT COUNT(DISTINCT re.id) as "count!"
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        LEFT JOIN tools t ON t.id = re.id
        LEFT JOIN organisms org ON org.id = ds.organism_id
        WHERE to_tsvector('english', re.name || ' ' || COALESCE(re.description, ''))
            @@ plainto_tsquery('english', $1)
        "#,
    );

    let mut param_index = 2;
    let mut conditions = Vec::new();

    if let Some(ref types) = filters.entry_type {
        if !types.is_empty() {
            conditions.push(format!("re.entry_type = ANY(${})", param_index));
            param_index += 1;
        }
    }

    if filters.organism.is_some() {
        conditions.push(format!(
            "(org.scientific_name ILIKE ${} OR org.common_name ILIKE ${})",
            param_index,
            param_index + 1
        ));
        param_index += 2;
    }

    if filters.format.is_some() {
        conditions.push(format!(
            r#"EXISTS (
                SELECT 1
                FROM versions v
                JOIN version_files vf ON vf.version_id = v.id
                WHERE v.entry_id = re.id AND vf.format = ${}
            )"#,
            param_index
        ));
    }

    if !conditions.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(&conditions.join(" AND "));
    }

    let mut query_builder = sqlx::query_scalar::<_, i64>(&sql);

    query_builder = query_builder.bind(query);

    if let Some(ref types) = filters.entry_type {
        if !types.is_empty() {
            query_builder = query_builder.bind(types);
        }
    }

    if let Some(ref organism) = filters.organism {
        let pattern = format!("%{}%", organism);
        query_builder = query_builder.bind(&pattern);
        query_builder = query_builder.bind(&pattern);
    }

    if let Some(ref format) = filters.format {
        query_builder = query_builder.bind(format);
    }

    let count = query_builder.fetch_one(pool).await?;

    Ok(count)
}

// ============================================================================
// Internal Types
// ============================================================================

/// Internal row type for search results.
#[derive(Debug, sqlx::FromRow)]
struct SearchResultRow {
    id: Uuid,
    organization_slug: String,
    slug: String,
    name: String,
    description: Option<String>,
    entry_type: String,
    source_type: Option<String>,
    tool_type: Option<String>,
    scientific_name: Option<String>,
    common_name: Option<String>,
    ncbi_taxonomy_id: Option<i32>,
    external_id: Option<String>,
    latest_version: Option<String>,
    external_version: Option<String>,
    available_formats: Vec<String>,
    total_downloads: i64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.entry_type.is_none());
        assert!(filters.organism.is_none());
        assert!(filters.format.is_none());
    }

    #[test]
    fn test_search_filters_with_values() {
        let filters = SearchFilters {
            entry_type: Some(vec!["data_source".to_string()]),
            organism: Some("human".to_string()),
            format: Some("fasta".to_string()),
        };

        assert_eq!(filters.entry_type.unwrap().len(), 1);
        assert_eq!(filters.organism.unwrap(), "human");
        assert_eq!(filters.format.unwrap(), "fasta");
    }

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_unified_search() {
        // This test would require a test database setup
        // Implementation left for integration tests
    }

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_count_search_results() {
        // This test would require a test database setup
        // Implementation left for integration tests
    }
}
