use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResponse {
    pub items: Vec<SearchResultItem>,
    pub pagination: PaginationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub id: Uuid,
    pub organization_slug: String,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism: Option<OrganismInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    pub available_formats: Vec<String>,
    pub total_downloads: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub rank: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismInfo {
    pub scientific_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_taxonomy_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum UnifiedSearchError {
    #[error("Query is required and cannot be empty")]
    QueryRequired,
    #[error("Per page must be between 1 and 100")]
    InvalidPerPage,
    #[error("Page must be greater than 0")]
    InvalidPage,
    #[error("Invalid type filter: {0}. Must be 'data_source', 'tool', or 'organization'")]
    InvalidTypeFilter(String),
    #[error("Invalid source type filter: {0}. Must be one of: protein, genome, organism, taxonomy, bundle, transcript, annotation, structure, pathway, other")]
    InvalidSourceTypeFilter(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<UnifiedSearchResponse, UnifiedSearchError>> for UnifiedSearchQuery {}

impl crate::cqrs::middleware::Query for UnifiedSearchQuery {}

impl UnifiedSearchQuery {
    pub fn validate(&self) -> Result<(), UnifiedSearchError> {
        if self.query.trim().is_empty() {
            return Err(UnifiedSearchError::QueryRequired);
        }

        let per_page = self.per_page();
        if per_page < 1 || per_page > 100 {
            return Err(UnifiedSearchError::InvalidPerPage);
        }

        let page = self.page();
        if page < 1 {
            return Err(UnifiedSearchError::InvalidPage);
        }

        if let Some(ref types) = self.type_filter {
            for t in types {
                if t != "data_source" && t != "tool" && t != "organization" {
                    return Err(UnifiedSearchError::InvalidTypeFilter(t.clone()));
                }
            }
        }

        if let Some(ref source_types) = self.source_type_filter {
            for st in source_types {
                if !matches!(st.as_str(), "protein" | "genome" | "organism" | "taxonomy" | "bundle" | "transcript" | "annotation" | "structure" | "pathway" | "other") {
                    return Err(UnifiedSearchError::InvalidSourceTypeFilter(st.clone()));
                }
            }
        }

        Ok(())
    }

    fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

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
    let has_org_filter = type_filter.map_or(false, |types| types.contains(&"organization".to_string()));
    let has_entry_filter = type_filter.map_or(true, |types| {
        types.contains(&"data_source".to_string()) || types.contains(&"tool".to_string())
    });

    let mut all_results = Vec::new();

    if has_org_filter {
        let org_results = search_organizations(&pool, &query).await?;
        all_results.extend(org_results);
    }

    if has_entry_filter {
        let entry_results = search_registry_entries(&pool, &query).await?;
        all_results.extend(entry_results);
    }

    all_results.sort_by(|a, b| b.rank.partial_cmp(&a.rank).unwrap_or(std::cmp::Ordering::Equal));

    let total = count_search_results(&pool, &query).await?;
    let items: Vec<SearchResultItem> = all_results.into_iter().skip(offset as usize).take(per_page as usize).collect();

    let pages = if total == 0 {
        0
    } else {
        ((total as f64) / (per_page as f64)).ceil() as i64
    };

    Ok(UnifiedSearchResponse {
        items,
        pagination: PaginationMetadata {
            page,
            per_page,
            total,
            pages,
            has_next: page < pages,
            has_prev: page > 1,
        },
    })
}

async fn search_organizations(
    pool: &PgPool,
    query: &UnifiedSearchQuery,
) -> Result<Vec<SearchResultItem>, sqlx::Error> {
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
        LIMIT $2
        "#,
        query.query,
        query.per_page() + query.offset()
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

    let records: Vec<RegistryEntrySearchRow> = sqlx::query_as!(
        RegistryEntrySearchRow,
        r#"
        SELECT
            re.id,
            o.slug as organization_slug,
            re.slug,
            re.name,
            re.description,
            re.entry_type,
            ds.source_type as "source_type?",
            t.tool_type as "tool_type?",
            COALESCE(org_ref.scientific_name, org_direct.scientific_name) as "scientific_name?",
            COALESCE(org_ref.common_name, org_direct.common_name) as "common_name?",
            COALESCE(org_ref.taxonomy_id, org_direct.taxonomy_id) as "ncbi_taxonomy_id?",
            ds.external_id as "external_id?",
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
            ) as "available_formats!",
            COALESCE(
                (
                    SELECT SUM(v.download_count)::bigint
                    FROM versions v
                    WHERE v.entry_id = re.id
                ),
                0
            ) as "total_downloads!",
            ts_rank(
                to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')),
                plainto_tsquery('english', $1)
            ) as "rank!"
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        LEFT JOIN tools t ON t.id = re.id
        LEFT JOIN protein_metadata pm ON pm.data_source_id = ds.id
        LEFT JOIN taxonomy_metadata org_ref ON org_ref.data_source_id = pm.taxonomy_id
        LEFT JOIN taxonomy_metadata org_direct ON org_direct.data_source_id = ds.id AND ds.source_type = 'organism'
        WHERE to_tsvector('english', re.name || ' ' || COALESCE(re.description, ''))
            @@ plainto_tsquery('english', $1)
          AND ($2::VARCHAR[] IS NULL OR re.entry_type = ANY($2))
          AND ($3::TEXT IS NULL OR org_ref.scientific_name ILIKE $3 OR org_ref.common_name ILIKE $3 OR org_direct.scientific_name ILIKE $3 OR org_direct.common_name ILIKE $3)
          AND ($4::TEXT IS NULL OR EXISTS (
              SELECT 1
              FROM versions v
              JOIN version_files vf ON vf.version_id = v.id
              WHERE v.entry_id = re.id AND vf.format = $4
          ))
          AND ($6::VARCHAR[] IS NULL OR ds.source_type = ANY($6))
          AND re.slug IS NOT NULL
          AND o.slug IS NOT NULL
        ORDER BY 17 DESC, 16 DESC, re.created_at DESC
        LIMIT $5
        "#,
        query.query,
        entry_types.as_deref(),
        organism_pattern.as_deref(),
        query.format,
        query.per_page() + query.offset(),
        query.source_type_filter.as_deref()
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
            organism: if r.scientific_name.is_some() {
                Some(OrganismInfo {
                    scientific_name: r.scientific_name.unwrap(),
                    common_name: r.common_name,
                    ncbi_taxonomy_id: r.ncbi_taxonomy_id,
                })
            } else {
                None
            },
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
    let has_org_filter = type_filter.map_or(false, |types| types.contains(&"organization".to_string()));
    let has_entry_filter = type_filter.map_or(true, |types| {
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

        let entry_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(DISTINCT re.id) as "count!"
            FROM registry_entries re
            JOIN organizations o ON o.id = re.organization_id
            LEFT JOIN data_sources ds ON ds.id = re.id
            LEFT JOIN tools t ON t.id = re.id
            LEFT JOIN protein_metadata pm ON pm.data_source_id = ds.id
            LEFT JOIN taxonomy_metadata org_ref ON org_ref.data_source_id = pm.taxonomy_id
            LEFT JOIN taxonomy_metadata org_direct ON org_direct.data_source_id = ds.id AND ds.source_type = 'organism'
            WHERE to_tsvector('english', re.name || ' ' || COALESCE(re.description, ''))
                @@ plainto_tsquery('english', $1)
              AND ($2::VARCHAR[] IS NULL OR re.entry_type = ANY($2))
              AND ($3::TEXT IS NULL OR org_ref.scientific_name ILIKE $3 OR org_ref.common_name ILIKE $3 OR org_direct.scientific_name ILIKE $3 OR org_direct.common_name ILIKE $3)
              AND ($4::TEXT IS NULL OR EXISTS (
                  SELECT 1
                  FROM versions v
                  JOIN version_files vf ON vf.version_id = v.id
                  WHERE v.entry_id = re.id AND vf.format = $4
              ))
              AND ($5::VARCHAR[] IS NULL OR ds.source_type = ANY($5))
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
            page: Some(1),
            per_page: Some(20),
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
            page: None,
            per_page: None,
        };
        assert!(matches!(
            query.validate(),
            Err(UnifiedSearchError::QueryRequired)
        ));
    }

    #[test]
    fn test_validation_invalid_per_page() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: None,
            source_type_filter: None,
            organism: None,
            format: None,
            page: Some(1),
            per_page: Some(101),
        };
        assert!(matches!(
            query.validate(),
            Err(UnifiedSearchError::InvalidPerPage)
        ));
    }

    #[test]
    fn test_validation_invalid_page() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: None,
            source_type_filter: None,
            organism: None,
            format: None,
            page: Some(0),
            per_page: Some(20),
        };
        assert!(matches!(
            query.validate(),
            Err(UnifiedSearchError::InvalidPage)
        ));
    }

    #[test]
    fn test_validation_invalid_type_filter() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["invalid".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
            page: None,
            per_page: None,
        };
        assert!(matches!(
            query.validate(),
            Err(UnifiedSearchError::InvalidTypeFilter(_))
        ));
    }

    #[test]
    fn test_validation_invalid_source_type_filter() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["invalid_type".to_string()]),
            organism: None,
            format: None,
            page: None,
            per_page: None,
        };
        assert!(matches!(
            query.validate(),
            Err(UnifiedSearchError::InvalidSourceTypeFilter(_))
        ));
    }

    #[test]
    fn test_validation_valid_source_type_filter() {
        let query = UnifiedSearchQuery {
            query: "test".to_string(),
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: Some(vec!["protein".to_string(), "organism".to_string()]),
            organism: None,
            format: None,
            page: None,
            per_page: None,
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
            page: Some(1),
            per_page: Some(10),
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
            page: Some(1),
            per_page: Some(10),
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

        let org_id = sqlx::query_scalar!(
            r#"SELECT id FROM organizations WHERE slug = 'test-org'"#
        )
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
            page: Some(1),
            per_page: Some(10),
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
            page: Some(1),
            per_page: Some(10),
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
