//! Search suggestions query
//!
//! Provides autocomplete suggestions for search queries using PostgreSQL's
//! trigram similarity (pg_trgm extension) for fuzzy matching.

use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Query for autocomplete search suggestions
///
/// Provides fast fuzzy-matching suggestions as the user types,
/// using trigram similarity for partial and misspelled queries.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::search::queries::SearchSuggestionsQuery;
///
/// // Get suggestions for partial input
/// let query = SearchSuggestionsQuery {
///     q: "ins".to_string(),
///     limit: Some(5),
///     type_filter: Some(vec!["data_source".to_string()]),
///     source_type_filter: Some(vec!["protein".to_string()]),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionsQuery {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type_filter: Option<Vec<String>>,
}

/// Response containing autocomplete suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionsResponse {
    /// Matching suggestions ranked by similarity score
    pub suggestions: Vec<SearchSuggestionItem>,
}

/// A single autocomplete suggestion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionItem {
    /// Unique identifier
    pub id: Uuid,
    /// Organization slug (for building URLs)
    pub organization_slug: String,
    /// Entry slug
    pub slug: String,
    /// Display name
    pub name: String,
    /// Type: "data_source", "tool", or "organization"
    pub entry_type: String,
    /// Source type (for data sources): protein, genome, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// Latest version (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    /// Trigram similarity score (0-1, higher is more similar)
    pub match_score: f32,
}

/// Errors that can occur during search suggestions
#[derive(Debug, thiserror::Error)]
pub enum SearchSuggestionsError {
    /// Query must be at least 2 characters
    #[error("Query is required and must be at least 2 characters")]
    QueryTooShort,
    /// Limit was outside valid range (1-20)
    #[error("Limit must be between 1 and 20")]
    InvalidLimit,
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

impl Request<Result<SearchSuggestionsResponse, SearchSuggestionsError>> for SearchSuggestionsQuery {}

impl crate::cqrs::middleware::Query for SearchSuggestionsQuery {}

impl SearchSuggestionsQuery {
    /// Validates the suggestions query parameters
    ///
    /// # Errors
    ///
    /// - `QueryTooShort` - Query is less than 2 characters
    /// - `InvalidLimit` - Limit is less than 1 or greater than 20
    /// - `InvalidTypeFilter` - Type filter contains an invalid value
    /// - `InvalidSourceTypeFilter` - Source type filter contains an invalid value
    pub fn validate(&self) -> Result<(), SearchSuggestionsError> {
        if self.q.trim().len() < 2 {
            return Err(SearchSuggestionsError::QueryTooShort);
        }

        let limit = self.limit();
        if !(1..=20).contains(&limit) {
            return Err(SearchSuggestionsError::InvalidLimit);
        }

        if let Some(ref types) = self.type_filter {
            for t in types {
                if t != "data_source" && t != "tool" && t != "organization" {
                    return Err(SearchSuggestionsError::InvalidTypeFilter(t.clone()));
                }
            }
        }

        if let Some(ref source_types) = self.source_type_filter {
            for st in source_types {
                if !matches!(
                    st.as_str(),
                    "protein"
                        | "genome"
                        | "organism"
                        | "taxonomy"
                        | "bundle"
                        | "transcript"
                        | "annotation"
                        | "structure"
                        | "pathway"
                        | "other"
                ) {
                    return Err(SearchSuggestionsError::InvalidSourceTypeFilter(st.clone()));
                }
            }
        }

        Ok(())
    }

    fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).clamp(1, 20)
    }
}

/// Handles the search suggestions query
///
/// Provides autocomplete suggestions using PostgreSQL's trigram similarity
/// (pg_trgm extension). Searches both organizations and registry entries
/// with optional type and source type filtering.
///
/// Results are ranked by trigram similarity score using `word_similarity`,
/// which measures how similar the query is to words in the name field.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Suggestions query with search term and filters
///
/// # Returns
///
/// Returns up to `limit` suggestions ranked by similarity.
///
/// # Errors
///
/// - Validation errors if query parameters are invalid
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: SearchSuggestionsQuery,
) -> Result<SearchSuggestionsResponse, SearchSuggestionsError> {
    query.validate()?;

    let limit = query.limit();
    let search_term = query.q.trim();

    let type_filter = query.type_filter.as_ref();
    let has_org_filter =
        type_filter.is_some_and(|types| types.contains(&"organization".to_string()));
    let has_entry_filter = type_filter.is_none_or(|types| {
        types.contains(&"data_source".to_string()) || types.contains(&"tool".to_string())
    });

    let mut all_suggestions = Vec::new();

    // Search organizations using trigram similarity
    if has_org_filter {
        let org_suggestions = search_organizations_autocomplete(&pool, search_term, limit).await?;
        all_suggestions.extend(org_suggestions);
    }

    // Search registry entries using trigram similarity
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

        let entry_suggestions = search_entries_autocomplete(
            &pool,
            search_term,
            entry_types,
            query.source_type_filter.clone(),
            limit,
        )
        .await?;
        all_suggestions.extend(entry_suggestions);
    }

    // Sort by match score descending
    all_suggestions.sort_by(|a, b| {
        b.match_score
            .partial_cmp(&a.match_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit to requested number
    all_suggestions.truncate(limit as usize);

    Ok(SearchSuggestionsResponse {
        suggestions: all_suggestions,
    })
}

async fn search_organizations_autocomplete(
    pool: &PgPool,
    search_term: &str,
    limit: i64,
) -> Result<Vec<SearchSuggestionItem>, sqlx::Error> {
    let records: Vec<OrganizationSuggestionRow> = sqlx::query_as(
        r#"
        SELECT
            id,
            slug,
            name,
            word_similarity($1, name) as match_score
        FROM organizations
        WHERE $1 <% name OR name ILIKE '%' || $1 || '%'
        ORDER BY match_score DESC, name
        LIMIT $2
        "#,
    )
    .bind(search_term)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| SearchSuggestionItem {
            id: r.id,
            organization_slug: r.slug.clone(),
            slug: r.slug,
            name: r.name,
            entry_type: "organization".to_string(),
            source_type: None,
            latest_version: None,
            match_score: r.match_score,
        })
        .collect())
}

async fn search_entries_autocomplete(
    pool: &PgPool,
    search_term: &str,
    entry_types: Option<Vec<String>>,
    source_type_filter: Option<Vec<String>>,
    limit: i64,
) -> Result<Vec<SearchSuggestionItem>, sqlx::Error> {
    // Use materialized view for autocomplete to avoid scalar subquery for latest_version
    // This eliminates N+1 query problem and uses pre-computed data
    let records: Vec<RegistryEntrySuggestionRow> = sqlx::query_as(
        r#"
        SELECT
            mv.id,
            mv.organization_slug,
            mv.slug,
            mv.name,
            mv.entry_type,
            mv.source_type,
            mv.latest_version as latest_version,
            word_similarity($1, mv.name) as match_score
        FROM search_registry_entries_mv mv
        WHERE ($1 <% mv.name OR mv.name ILIKE '%' || $1 || '%')
          AND ($2::VARCHAR[] IS NULL OR mv.entry_type = ANY($2))
          AND ($4::VARCHAR[] IS NULL OR mv.source_type = ANY($4))
        ORDER BY match_score DESC, mv.name
        LIMIT $3
        "#,
    )
    .bind(search_term)
    .bind(entry_types.as_deref())
    .bind(limit)
    .bind(source_type_filter.as_deref())
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| SearchSuggestionItem {
            id: r.id,
            organization_slug: r.organization_slug,
            slug: r.slug,
            name: r.name,
            entry_type: r.entry_type,
            source_type: r.source_type,
            latest_version: r.latest_version,
            match_score: r.match_score,
        })
        .collect())
}

#[derive(Debug, sqlx::FromRow)]
struct OrganizationSuggestionRow {
    id: Uuid,
    slug: String,
    name: String,
    match_score: f32,
}

#[derive(Debug, sqlx::FromRow)]
struct RegistryEntrySuggestionRow {
    id: Uuid,
    organization_slug: String,
    slug: String,
    name: String,
    entry_type: String,
    source_type: Option<String>,
    latest_version: Option<String>,
    match_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = SearchSuggestionsQuery {
            q: "protein".to_string(),
            limit: Some(10),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: None,
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_query_too_short() {
        let query = SearchSuggestionsQuery {
            q: "a".to_string(),
            limit: None,
            type_filter: None,
            source_type_filter: None,
        };
        assert!(matches!(query.validate(), Err(SearchSuggestionsError::QueryTooShort)));
    }

    #[test]
    fn test_validation_invalid_limit() {
        let query = SearchSuggestionsQuery {
            q: "protein".to_string(),
            limit: Some(25),
            type_filter: None,
            source_type_filter: None,
        };
        assert!(matches!(query.validate(), Err(SearchSuggestionsError::InvalidLimit)));
    }

    #[test]
    fn test_validation_invalid_type_filter() {
        let query = SearchSuggestionsQuery {
            q: "protein".to_string(),
            limit: None,
            type_filter: Some(vec!["invalid".to_string()]),
            source_type_filter: None,
        };
        assert!(matches!(query.validate(), Err(SearchSuggestionsError::InvalidTypeFilter(_))));
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

        let query = SearchSuggestionsQuery {
            q: "uni".to_string(),
            limit: Some(10),
            type_filter: Some(vec!["organization".to_string()]),
            source_type_filter: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() > 0);
        assert!(response.suggestions.iter().any(|s| s.slug == "uniprot"));
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

        let query = SearchSuggestionsQuery {
            q: "insu".to_string(),
            limit: Some(10),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() > 0);
        assert!(response
            .suggestions
            .iter()
            .any(|s| s.slug == "insulin-data"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_respects_limit(pool: PgPool) -> sqlx::Result<()> {
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
                format!("test-entry-{}", i),
                format!("Test Entry {}", i)
            )
            .execute(&pool)
            .await?;
        }

        let query = SearchSuggestionsQuery {
            q: "test".to_string(),
            limit: Some(5),
            type_filter: None,
            source_type_filter: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() <= 5);
        Ok(())
    }

    #[test]
    fn test_validation_invalid_source_type_filter() {
        let query = SearchSuggestionsQuery {
            q: "test".to_string(),
            limit: None,
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["invalid_type".to_string()]),
        };
        assert!(matches!(
            query.validate(),
            Err(SearchSuggestionsError::InvalidSourceTypeFilter(_))
        ));
    }

    #[test]
    fn test_validation_valid_source_type_filter() {
        let query = SearchSuggestionsQuery {
            q: "test".to_string(),
            limit: None,
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["protein".to_string(), "organism".to_string()]),
        };
        assert!(query.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_source_type_filter_protein_only(pool: PgPool) -> sqlx::Result<()> {
        let org_id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org', 'Test Org', true)
            RETURNING id
            "#
        )
        .fetch_one(&pool)
        .await?;

        // Create protein data source
        let protein_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'protein-data', 'Protein Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'protein')
            "#,
            protein_id
        )
        .execute(&pool)
        .await?;

        // Create organism data source
        let organism_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'organism-data', 'Organism Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'organism')
            "#,
            organism_id
        )
        .execute(&pool)
        .await?;

        // Test filter for protein only
        let query = SearchSuggestionsQuery {
            q: "data".to_string(),
            limit: Some(10),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["protein".to_string()]),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() > 0);
        assert!(response
            .suggestions
            .iter()
            .all(|s| s.source_type.as_deref() == Some("protein")));
        assert!(!response
            .suggestions
            .iter()
            .any(|s| s.slug == "organism-data"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_source_type_filter_organism_only(pool: PgPool) -> sqlx::Result<()> {
        let org_id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org', 'Test Org', true)
            RETURNING id
            "#
        )
        .fetch_one(&pool)
        .await?;

        // Create protein data source
        let protein_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'protein-data', 'Protein Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'protein')
            "#,
            protein_id
        )
        .execute(&pool)
        .await?;

        // Create organism data source
        let organism_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'organism-data', 'Organism Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'organism')
            "#,
            organism_id
        )
        .execute(&pool)
        .await?;

        // Test filter for organism only
        let query = SearchSuggestionsQuery {
            q: "data".to_string(),
            limit: Some(10),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["organism".to_string()]),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() > 0);
        assert!(response
            .suggestions
            .iter()
            .all(|s| s.source_type.as_deref() == Some("organism")));
        assert!(!response
            .suggestions
            .iter()
            .any(|s| s.slug == "protein-data"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_source_type_filter_multiple_types(pool: PgPool) -> sqlx::Result<()> {
        let org_id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org', 'Test Org', true)
            RETURNING id
            "#
        )
        .fetch_one(&pool)
        .await?;

        // Create protein, organism, and genome data sources
        let protein_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'protein-data', 'Protein Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'protein')
            "#,
            protein_id
        )
        .execute(&pool)
        .await?;

        let organism_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'organism-data', 'Organism Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'organism')
            "#,
            organism_id
        )
        .execute(&pool)
        .await?;

        let genome_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'genome-data', 'Genome Dataset', 'data_source')
            RETURNING id
            "#,
            org_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'genome')
            "#,
            genome_id
        )
        .execute(&pool)
        .await?;

        // Test filter for protein and organism only (should exclude genome)
        let query = SearchSuggestionsQuery {
            q: "data".to_string(),
            limit: Some(10),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["protein".to_string(), "organism".to_string()]),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() >= 2);
        assert!(response
            .suggestions
            .iter()
            .all(|s| s.source_type.as_deref() == Some("protein")
                || s.source_type.as_deref() == Some("organism")));
        assert!(!response.suggestions.iter().any(|s| s.slug == "genome-data"));
        Ok(())
    }
}
