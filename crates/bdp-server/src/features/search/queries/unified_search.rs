//! Unified search query
//!
//! Full-text search across organizations, data sources, and tools.
//! Uses PostgreSQL's full-text search with materialized views for performance.

use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::features::shared::pagination::{PaginationMetadata, PaginationParams};
use crate::features::shared::validation::VALID_SOURCE_TYPES;

/// Query for unified full-text search
///
/// Searches across organizations, data sources, and tools with optional
/// filtering by type, source type, organism, and format.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::search::queries::UnifiedSearchQuery;
///
/// // Search for insulin-related entries
/// let query = UnifiedSearchQuery {
///     query: "insulin".to_string(),
///     type_filter: Some(vec!["data_source".to_string()]),
///     source_type_filter: Some(vec!["protein".to_string()]),
///     organism: Some("human".to_string()),
///     format: None,
///     pagination: PaginationParams::new(Some(1), Some(20)),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchQuery {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Response containing search results with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResponse {
    /// Matching items ranked by relevance
    pub items: Vec<SearchResultItem>,
    /// Pagination metadata
    pub pagination: PaginationMetadata,
}

/// A single search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Unique identifier
    pub id: Uuid,
    /// Organization slug (for building URLs)
    pub organization_slug: String,
    /// Entry slug
    pub slug: String,
    /// Display name
    pub name: String,
    /// Entry description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Type: "data_source", "tool", or "organization"
    pub entry_type: String,
    /// Source type (for data sources): protein, genome, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// Tool type (for tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    /// Organism information (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism: Option<OrganismInfo>,
    /// Latest internal version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    /// External version (e.g., UniProt release)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    /// Available file formats
    pub available_formats: Vec<String>,
    /// Total download count across all versions
    pub total_downloads: i64,
    /// External identifier (e.g., UniProt accession)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Search relevance rank (higher is more relevant)
    pub rank: f32,
}

/// Organism information in search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismInfo {
    /// Scientific name (e.g., "Homo sapiens")
    pub scientific_name: String,
    /// Common name (e.g., "Human")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
    /// NCBI Taxonomy ID (e.g., 9606)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_taxonomy_id: Option<i32>,
}

/// Errors that can occur during unified search
#[derive(Debug, thiserror::Error)]
pub enum UnifiedSearchError {
    /// Search query was empty
    #[error("Query is required and cannot be empty")]
    QueryRequired,
    /// Per page was outside valid range (1-100)
    #[error("Per page must be between 1 and 100")]
    InvalidPerPage,
    /// Page number was less than 1
    #[error("Page must be greater than 0")]
    InvalidPage,
    /// Type filter contained an invalid value
    #[error("Invalid type filter: {0}. Must be 'data_source', 'tool', or 'organization'")]
    InvalidTypeFilter(String),
    /// Source type filter contained an invalid value
    #[error("Invalid source type filter: {0}. Must be one of: protein, genome, organism, taxonomy, bundle, transcript, annotation, structure, pathway, other")]
    InvalidSourceTypeFilter(String),
    /// A database error occurred
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<UnifiedSearchResponse, UnifiedSearchError>> for UnifiedSearchQuery {}

impl crate::cqrs::middleware::Query for UnifiedSearchQuery {}

impl UnifiedSearchQuery {
    /// Validates the search query parameters
    ///
    /// # Errors
    ///
    /// - `QueryRequired` - Search query is empty
    /// - `InvalidPage` - Page is less than 1
    /// - `InvalidPerPage` - Per page is less than 1 or greater than 100
    /// - `InvalidTypeFilter` - Type filter contains an invalid value
    /// - `InvalidSourceTypeFilter` - Source type filter contains an invalid value
    pub fn validate(&self) -> Result<(), UnifiedSearchError> {
        if self.query.trim().is_empty() {
            return Err(UnifiedSearchError::QueryRequired);
        }

        // Use shared pagination validation
        self.pagination.validate().map_err(|msg| match msg {
            "Page must be greater than 0" => UnifiedSearchError::InvalidPage,
            _ => UnifiedSearchError::InvalidPerPage,
        })?;

        if let Some(ref types) = self.type_filter {
            for t in types {
                if t != "data_source" && t != "tool" && t != "organization" {
                    return Err(UnifiedSearchError::InvalidTypeFilter(t.clone()));
                }
            }
        }

        // Use shared VALID_SOURCE_TYPES constant
        if let Some(ref source_types) = self.source_type_filter {
            for st in source_types {
                if !VALID_SOURCE_TYPES.contains(&st.as_str()) {
                    return Err(UnifiedSearchError::InvalidSourceTypeFilter(st.clone()));
                }
            }
        }

        Ok(())
    }

    fn page(&self) -> i64 {
        self.pagination.page()
    }

    fn per_page(&self) -> i64 {
        self.pagination.per_page()
    }

    fn offset(&self) -> i64 {
        self.pagination.offset()
    }
}

/// Handles the unified search query
///
/// Performs full-text search across organizations, data sources, and tools.
/// Uses PostgreSQL's `plainto_tsquery` for search and a materialized view
/// for efficient querying of registry entries.
///
/// Results are ranked by relevance using `ts_rank` and sorted by:
/// 1. Search relevance (highest first)
/// 2. Total downloads (highest first)
/// 3. Creation date (newest first)
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Search query with filters and pagination
///
/// # Returns
///
/// Returns paginated search results ranked by relevance.
///
/// # Errors
///
/// - Validation errors if query parameters are invalid
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: UnifiedSearchQuery,
) -> Result<UnifiedSearchResponse, UnifiedSearchError> {
    query.validate()?;

    let page = query.page();
    let per_page = query.per_page();
    let offset = query.offset();

    let type_filter = query.type_filter.as_ref();
    let has_org_filter =
        type_filter.is_some_and(|types| types.contains(&"organization".to_string()));
    let has_entry_filter = type_filter.is_none_or(|types| {
        types.contains(&"data_source".to_string()) || types.contains(&"tool".to_string())
    });

    // When searching only one type, we can use proper LIMIT/OFFSET in the query
    // When searching both types, we need to combine results and paginate in memory
    let searching_both_types = has_org_filter && has_entry_filter;

    // Run searches concurrently when querying multiple types
    // This provides significant performance improvement over sequential execution
    let (org_results, entry_results) = if has_org_filter && has_entry_filter {
        // Both searches can run in parallel
        let (org_res, entry_res) = tokio::try_join!(
            search_organizations(&pool, &query, searching_both_types),
            search_registry_entries(&pool, &query, searching_both_types)
        )?;
        (org_res, entry_res)
    } else if has_org_filter {
        let org_res = search_organizations(&pool, &query, searching_both_types).await?;
        (org_res, vec![])
    } else if has_entry_filter {
        let entry_res = search_registry_entries(&pool, &query, searching_both_types).await?;
        (vec![], entry_res)
    } else {
        (vec![], vec![])
    };

    let mut all_results = org_results;
    all_results.extend(entry_results);

    // Sort by rank - organizations and entries are ranked separately, so we need to merge
    all_results.sort_by(|a, b| {
        b.rank
            .partial_cmp(&a.rank)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total_downloads.cmp(&a.total_downloads))
            .then_with(|| std::cmp::Ordering::Equal)
    });

    let total = count_search_results(&pool, &query).await?;

    // If searching both types, we need to paginate in memory after sorting
    // Otherwise, pagination was already done in the query
    let items: Vec<SearchResultItem> = if searching_both_types {
        all_results
            .into_iter()
            .skip(offset as usize)
            .take(per_page as usize)
            .collect()
    } else {
        all_results
    };

    Ok(UnifiedSearchResponse {
        items,
        pagination: PaginationMetadata::new(page, per_page, total),
    })
}

async fn search_organizations(
    pool: &PgPool,
    query: &UnifiedSearchQuery,
    fetch_all_for_merge: bool,
) -> Result<Vec<SearchResultItem>, sqlx::Error> {
    // When searching both types, fetch all results for merging and sorting
    // When searching only organizations, use proper LIMIT/OFFSET
    let (limit, offset) = if fetch_all_for_merge {
        (query.per_page() + query.offset(), 0)
    } else {
        (query.per_page(), query.offset())
    };

    let records: Vec<OrganizationSearchRow> = sqlx::query_as!(
        OrganizationSearchRow,
        r#"
        SELECT
            id,
            slug,
            name,
            description,
            ts_rank(
                to_tsvector('english', name || ' ' || COALESCE(description, '')),
                plainto_tsquery('english', $1)
            ) as "rank!"
        FROM organizations
        WHERE to_tsvector('english', name || ' ' || COALESCE(description, ''))
            @@ plainto_tsquery('english', $1)
        ORDER BY 5 DESC, created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        query.query,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| SearchResultItem {
            id: r.id,
            organization_slug: r.slug.clone(),
            slug: r.slug,
            name: r.name,
            description: r.description,
            entry_type: "organization".to_string(),
            source_type: None,
            tool_type: None,
            organism: None,
            latest_version: None,
            external_version: None,
            available_formats: vec![],
            total_downloads: 0,
            external_id: None,
            rank: r.rank,
        })
        .collect())
}

async fn search_registry_entries(
    pool: &PgPool,
    query: &UnifiedSearchQuery,
    fetch_all_for_merge: bool,
) -> Result<Vec<SearchResultItem>, sqlx::Error> {
    let entry_types = query.type_filter.as_ref().and_then(|types| {
        let filtered: Vec<String> = types
            .iter()
            .filter(|t| *t == "data_source" || *t == "tool")
            .cloned()
            .collect();
        if filtered.is_empty() {
            None
        } else {
            Some(filtered)
        }
    });

    let organism_pattern = query.organism.as_ref().map(|o| format!("%{}%", o));

    // When searching both types, fetch all results for merging and sorting
    // When searching only registry entries, use proper LIMIT/OFFSET
    let (limit, offset) = if fetch_all_for_merge {
        (query.per_page() + query.offset(), 0)
    } else {
        (query.per_page(), query.offset())
    };

    // Query the materialized view instead of doing complex joins
    // This eliminates N+1 queries and uses pre-computed aggregations
    let records: Vec<RegistryEntrySearchRow> = sqlx::query_as!(
        RegistryEntrySearchRow,
        r#"
        SELECT
            mv.id as "id!",
            mv.organization_slug as "organization_slug!",
            mv.slug as "slug!",
            mv.name as "name!",
            mv.description as "description?",
            mv.entry_type as "entry_type!",
            mv.source_type as "source_type?",
            mv.tool_type as "tool_type?",
            mv.scientific_name as "scientific_name?",
            mv.common_name as "common_name?",
            mv.ncbi_taxonomy_id as "ncbi_taxonomy_id?",
            mv.external_id as "external_id?",
            mv.latest_version as "latest_version?",
            mv.external_version as "external_version?",
            mv.available_formats as "available_formats!",
            mv.total_downloads as "total_downloads!",
            -- Use pre-computed search_vector with ts_rank for better performance
            ts_rank(mv.search_vector, plainto_tsquery('english', $1)) as "rank!"
        FROM search_registry_entries_mv mv
        WHERE mv.search_vector @@ plainto_tsquery('english', $1)
          AND ($2::VARCHAR[] IS NULL OR mv.entry_type = ANY($2))
          AND ($3::TEXT IS NULL OR LOWER(mv.scientific_name) LIKE LOWER($3) OR LOWER(mv.common_name) LIKE LOWER($3))
          AND ($4::TEXT IS NULL OR $4 = ANY(mv.available_formats))
          AND ($6::VARCHAR[] IS NULL OR mv.source_type = ANY($6))
        ORDER BY 17 DESC, 16 DESC, mv.created_at DESC
        LIMIT $5 OFFSET $7
        "#,
        query.query,
        entry_types.as_deref(),
        organism_pattern.as_deref(),
        query.format,
        limit,
        query.source_type_filter.as_deref(),
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| SearchResultItem {
            id: r.id,
            organization_slug: r.organization_slug,
            slug: r.slug,
            name: r.name,
            description: r.description,
            entry_type: r.entry_type,
            source_type: r.source_type,
            tool_type: r.tool_type,
            organism: r.scientific_name.map(|name| OrganismInfo {
                scientific_name: name,
                common_name: r.common_name,
                ncbi_taxonomy_id: r.ncbi_taxonomy_id,
            }),
            latest_version: r.latest_version,
            external_version: r.external_version,
            available_formats: r.available_formats,
            total_downloads: r.total_downloads,
            external_id: r.external_id,
            rank: r.rank,
        })
        .collect())
}

async fn count_search_results(
    pool: &PgPool,
    query: &UnifiedSearchQuery,
) -> Result<i64, sqlx::Error> {
    let type_filter = query.type_filter.as_ref();
    let has_org_filter =
        type_filter.is_some_and(|types| types.contains(&"organization".to_string()));
    let has_entry_filter = type_filter.is_none_or(|types| {
        types.contains(&"data_source".to_string()) || types.contains(&"tool".to_string())
    });

    let mut total = 0i64;

    if has_org_filter {
        let org_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM organizations
            WHERE to_tsvector('english', name || ' ' || COALESCE(description, ''))
                @@ plainto_tsquery('english', $1)
            "#,
            query.query
        )
        .fetch_one(pool)
        .await?;
        total += org_count;
    }

    if has_entry_filter {
        let entry_types = query.type_filter.as_ref().and_then(|types| {
            let filtered: Vec<String> = types
                .iter()
                .filter(|t| *t == "data_source" || *t == "tool")
                .cloned()
                .collect();
            if filtered.is_empty() {
                None
            } else {
                Some(filtered)
            }
        });

        let organism_pattern = query.organism.as_ref().map(|o| format!("%{}%", o));

        // Use materialized view for counting - much faster than joining base tables
        let entry_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM search_registry_entries_mv mv
            WHERE mv.search_vector @@ plainto_tsquery('english', $1)
              AND ($2::VARCHAR[] IS NULL OR mv.entry_type = ANY($2))
              AND ($3::TEXT IS NULL OR LOWER(mv.scientific_name) LIKE LOWER($3) OR LOWER(mv.common_name) LIKE LOWER($3))
              AND ($4::TEXT IS NULL OR $4 = ANY(mv.available_formats))
              AND ($5::VARCHAR[] IS NULL OR mv.source_type = ANY($5))
            "#,
            query.query,
            entry_types.as_deref(),
            organism_pattern.as_deref(),
            query.format,
            query.source_type_filter.as_deref()
        )
        .fetch_one(pool)
        .await?;
        total += entry_count;
    }

    Ok(total)
}

#[derive(Debug)]
struct OrganizationSearchRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    rank: f32,
}

#[derive(Debug)]
struct RegistryEntrySearchRow {
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
    rank: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = UnifiedSearchQuery {
            query: "insulin".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(1), Some(20)),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_query() {
        let query = UnifiedSearchQuery {
            query: "".to_string(),
            type_filter: None,
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::default(),
        };
        assert!(matches!(query.validate(), Err(UnifiedSearchError::QueryRequired)));
    }

    #[test]
    fn test_validation_invalid_per_page() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: None,
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(1), Some(101)),
        };
        assert!(matches!(query.validate(), Err(UnifiedSearchError::InvalidPerPage)));
    }

    #[test]
    fn test_validation_invalid_page() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: None,
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(0), Some(20)),
        };
        assert!(matches!(query.validate(), Err(UnifiedSearchError::InvalidPage)));
    }

    #[test]
    fn test_validation_invalid_type_filter() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["invalid".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::default(),
        };
        assert!(matches!(query.validate(), Err(UnifiedSearchError::InvalidTypeFilter(_))));
    }

    #[test]
    fn test_validation_invalid_source_type_filter() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["invalid_type".to_string()]),
            organism: None,
            format: None,
            pagination: PaginationParams::default(),
        };
        assert!(matches!(query.validate(), Err(UnifiedSearchError::InvalidSourceTypeFilter(_))));
    }

    #[test]
    fn test_validation_valid_source_type_filter() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["protein".to_string(), "organism".to_string()]),
            organism: None,
            format: None,
            pagination: PaginationParams::default(),
        };
        assert!(query.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_handle_searches_organizations(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, description, is_system)
            VALUES ('uniprot', 'UniProt', 'Universal Protein Resource', true),
                   ('ncbi', 'NCBI', 'National Center for Biotechnology Information', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = UnifiedSearchQuery {
            query: "protein".to_string(),
            type_filter: Some(vec!["organization".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(1), Some(10)),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.items.len() > 0);
        assert!(response.items.iter().any(|i| i.slug == "uniprot"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_searches_registry_entries(pool: PgPool) -> sqlx::Result<()> {
        let org_id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org', 'Test Org', true)
            RETURNING id
            "#
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, 'insulin-data', 'Insulin Dataset', 'Insulin protein data', 'data_source'),
                   ($1, 'blast-tool', 'BLAST', 'Basic Local Alignment Search Tool', 'tool')
            "#,
            org_id
        )
        .execute(&pool)
        .await?;

        let query = UnifiedSearchQuery {
            query: "insulin".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(1), Some(10)),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].slug, "insulin-data");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_unified_search(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, description, is_system)
            VALUES ('test-org', 'Test Organization', 'For testing', true)
            "#
        )
        .execute(&pool)
        .await?;

        let org_id = sqlx::query_scalar!(r#"SELECT id FROM organizations WHERE slug = 'test-org'"#)
            .fetch_one(&pool)
            .await?;

        sqlx::query!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, 'test-data', 'Test Dataset', 'Testing search', 'data_source')
            "#,
            org_id
        )
        .execute(&pool)
        .await?;

        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: None,
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(1), Some(10)),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.items.len() >= 2);
        assert!(response.pagination.total >= 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_pagination(pool: PgPool) -> sqlx::Result<()> {
        let org_id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org', 'Test Org', true)
            RETURNING id
            "#
        )
        .fetch_one(&pool)
        .await?;

        for i in 1..=15 {
            sqlx::query!(
                r#"
                INSERT INTO registry_entries (organization_id, slug, name, entry_type)
                VALUES ($1, $2, $3, 'data_source')
                "#,
                org_id,
                format!("entry-{}", i),
                format!("Test Entry {}", i)
            )
            .execute(&pool)
            .await?;
        }

        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
            pagination: PaginationParams::new(Some(1), Some(10)),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 10);
        assert_eq!(response.pagination.page, 1);
        assert_eq!(response.pagination.per_page, 10);
        assert!(response.pagination.has_next);
        assert!(!response.pagination.has_prev);
        Ok(())
    }
}
