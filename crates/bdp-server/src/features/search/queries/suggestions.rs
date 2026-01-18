use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionsQuery {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_filter: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionsResponse {
    pub suggestions: Vec<SearchSuggestionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionItem {
    pub id: Uuid,
    pub organization_slug: String,
    pub slug: String,
    pub name: String,
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub match_score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum SearchSuggestionsError {
    #[error("Query is required and must be at least 2 characters")]
    QueryTooShort,
    #[error("Limit must be between 1 and 20")]
    InvalidLimit,
    #[error("Invalid type filter: {0}. Must be 'data_source', 'tool', or 'organization'")]
    InvalidTypeFilter(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<SearchSuggestionsResponse, SearchSuggestionsError>> for SearchSuggestionsQuery {}

impl crate::cqrs::middleware::Query for SearchSuggestionsQuery {}

impl SearchSuggestionsQuery {
    pub fn validate(&self) -> Result<(), SearchSuggestionsError> {
        if self.q.trim().len() < 2 {
            return Err(SearchSuggestionsError::QueryTooShort);
        }

        let limit = self.limit();
        if limit < 1 || limit > 20 {
            return Err(SearchSuggestionsError::InvalidLimit);
        }

        if let Some(ref types) = self.type_filter {
            for t in types {
                if t != "data_source" && t != "tool" && t != "organization" {
                    return Err(SearchSuggestionsError::InvalidTypeFilter(t.clone()));
                }
            }
        }

        Ok(())
    }

    fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).clamp(1, 20)
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: SearchSuggestionsQuery,
) -> Result<SearchSuggestionsResponse, SearchSuggestionsError> {
    query.validate()?;

    let limit = query.limit();
    let search_term = query.q.trim();

    let type_filter = query.type_filter.as_ref();
    let has_org_filter = type_filter.map_or(false, |types| types.contains(&"organization".to_string()));
    let has_entry_filter = type_filter.map_or(true, |types| {
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

        let entry_suggestions = search_entries_autocomplete(&pool, search_term, entry_types, limit).await?;
        all_suggestions.extend(entry_suggestions);
    }

    // Sort by match score descending
    all_suggestions.sort_by(|a, b| {
        b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal)
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
            similarity(name, $1) as match_score
        FROM organizations
        WHERE name % $1
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
    limit: i64,
) -> Result<Vec<SearchSuggestionItem>, sqlx::Error> {
    let records: Vec<RegistryEntrySuggestionRow> = sqlx::query_as(
        r#"
        SELECT
            re.id,
            o.slug as organization_slug,
            re.slug,
            re.name,
            re.entry_type,
            ds.source_type,
            (
                SELECT v.version
                FROM versions v
                WHERE v.entry_id = re.id
                ORDER BY v.published_at DESC
                LIMIT 1
            ) as latest_version,
            similarity(re.name, $1) as match_score
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        WHERE re.name % $1
          AND ($2::VARCHAR[] IS NULL OR re.entry_type = ANY($2))
        ORDER BY match_score DESC, re.name
        LIMIT $3
        "#,
    )
    .bind(search_term)
    .bind(entry_types.as_deref())
    .bind(limit)
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
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_query_too_short() {
        let query = SearchSuggestionsQuery {
            q: "a".to_string(),
            limit: None,
            type_filter: None,
        };
        assert!(matches!(
            query.validate(),
            Err(SearchSuggestionsError::QueryTooShort)
        ));
    }

    #[test]
    fn test_validation_invalid_limit() {
        let query = SearchSuggestionsQuery {
            q: "protein".to_string(),
            limit: Some(25),
            type_filter: None,
        };
        assert!(matches!(
            query.validate(),
            Err(SearchSuggestionsError::InvalidLimit)
        ));
    }

    #[test]
    fn test_validation_invalid_type_filter() {
        let query = SearchSuggestionsQuery {
            q: "protein".to_string(),
            limit: None,
            type_filter: Some(vec!["invalid".to_string()]),
        };
        assert!(matches!(
            query.validate(),
            Err(SearchSuggestionsError::InvalidTypeFilter(_))
        ));
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
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() > 0);
        assert!(response.suggestions.iter().any(|s| s.slug == "insulin-data"));
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
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.suggestions.len() <= 5);
        Ok(())
    }
}
