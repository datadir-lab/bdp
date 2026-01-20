use chrono::{DateTime, NaiveDate, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataSourceQuery {
    pub organization_slug: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataSourceResponse {
    pub id: Uuid,
    pub organization: OrganizationInfo,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism: Option<OrganismInfo>,
    pub versions: Vec<VersionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub total_downloads: i64,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismInfo {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_taxonomy_id: Option<i32>,
    pub scientific_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: Uuid,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub download_count: i64,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetDataSourceError {
    #[error("Organization slug is required and cannot be empty")]
    OrganizationSlugRequired,
    #[error("Data source slug is required and cannot be empty")]
    SlugRequired,
    #[error("Data source '{0}/{1}' not found")]
    NotFound(String, String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<GetDataSourceResponse, GetDataSourceError>> for GetDataSourceQuery {}

impl crate::cqrs::middleware::Query for GetDataSourceQuery {}

impl GetDataSourceQuery {
    pub fn validate(&self) -> Result<(), GetDataSourceError> {
        if self.organization_slug.is_empty() {
            return Err(GetDataSourceError::OrganizationSlugRequired);
        }
        if self.slug.is_empty() {
            return Err(GetDataSourceError::SlugRequired);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetDataSourceQuery,
) -> Result<GetDataSourceResponse, GetDataSourceError> {
    query.validate()?;

    let result = sqlx::query_as!(
        DataSourceRecord,
        r#"
        SELECT
            re.id,
            re.organization_id,
            o.slug as organization_slug,
            o.name as organization_name,
            re.slug,
            re.name,
            re.description,
            ds.source_type,
            ds.external_id,
            COALESCE(pm.taxonomy_id, CASE WHEN ds.source_type = 'organism' THEN ds.id ELSE NULL END) as organism_id,
            COALESCE(om_ref.taxonomy_id, om_direct.taxonomy_id) as ncbi_taxonomy_id,
            COALESCE(om_ref.scientific_name, om_direct.scientific_name) as scientific_name,
            COALESCE(om_ref.common_name, om_direct.common_name) as common_name,
            re.created_at as "created_at!",
            re.updated_at as "updated_at!"
        FROM registry_entries re
        JOIN data_sources ds ON re.id = ds.id
        JOIN organizations o ON re.organization_id = o.id
        LEFT JOIN protein_metadata pm ON ds.id = pm.data_source_id
        LEFT JOIN taxonomy_metadata om_ref ON pm.taxonomy_id = om_ref.data_source_id
        LEFT JOIN taxonomy_metadata om_direct ON ds.id = om_direct.data_source_id AND ds.source_type = 'organism'
        WHERE o.slug = $1 AND re.slug = $2
        "#,
        query.organization_slug,
        query.slug
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| {
        GetDataSourceError::NotFound(query.organization_slug.clone(), query.slug.clone())
    })?;

    let versions = sqlx::query_as!(
        VersionRecord,
        r#"
        SELECT id, version, external_version, release_date, size_bytes,
               download_count as "download_count!", published_at as "published_at!"
        FROM versions
        WHERE entry_id = $1
        ORDER BY published_at DESC
        "#,
        result.id
    )
    .fetch_all(&pool)
    .await?;

    let total_downloads: i64 = versions.iter().map(|v| v.download_count).sum();

    let latest_version = versions.first().map(|v| v.version.clone());

    let tags = sqlx::query_scalar!(
        r#"
        SELECT t.name
        FROM tags t
        JOIN entry_tags et ON t.id = et.tag_id
        WHERE et.entry_id = $1
        ORDER BY t.name
        "#,
        result.id
    )
    .fetch_all(&pool)
    .await?;

    Ok(GetDataSourceResponse {
        id: result.id,
        organization: OrganizationInfo {
            id: result.organization_id,
            slug: result.organization_slug,
            name: result.organization_name,
        },
        slug: result.slug,
        name: result.name,
        description: result.description,
        source_type: result.source_type,
        external_id: result.external_id,
        organism: result.organism_id.map(|id| OrganismInfo {
            id,
            ncbi_taxonomy_id: result.ncbi_taxonomy_id,
            scientific_name: result.scientific_name.unwrap_or_default(),
            common_name: result.common_name,
        }),
        versions: versions
            .into_iter()
            .map(|v| VersionInfo {
                id: v.id,
                version: v.version,
                external_version: v.external_version,
                release_date: v.release_date,
                size_bytes: v.size_bytes,
                download_count: v.download_count,
                published_at: v.published_at,
            })
            .collect(),
        latest_version,
        total_downloads,
        tags,
        created_at: result.created_at,
        updated_at: result.updated_at,
    })
}

#[derive(Debug)]
struct DataSourceRecord {
    id: Uuid,
    organization_id: Uuid,
    organization_slug: String,
    organization_name: String,
    slug: String,
    name: String,
    description: Option<String>,
    source_type: String,
    external_id: Option<String>,
    organism_id: Option<Uuid>,
    ncbi_taxonomy_id: Option<i32>,
    scientific_name: Option<String>,
    common_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
struct VersionRecord {
    id: Uuid,
    version: String,
    external_version: Option<String>,
    release_date: Option<NaiveDate>,
    size_bytes: Option<i64>,
    download_count: i64,
    published_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "test-protein".to_string(),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_organization_slug() {
        let query = GetDataSourceQuery {
            organization_slug: "".to_string(),
            slug: "test-protein".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(GetDataSourceError::OrganizationSlugRequired)
        ));
    }

    #[test]
    fn test_validation_empty_slug() {
        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(GetDataSourceError::SlugRequired)
        ));
    }

    #[sqlx::test]
    async fn test_handle_gets_data_source(pool: PgPool) -> sqlx::Result<()> {
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
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein",
            Some("Test description")
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type, external_id) VALUES ($1, $2, $3)",
            entry_id,
            "protein",
            Some("P12345")
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO versions (entry_id, version, external_version) VALUES ($1, $2, $3)",
            entry_id,
            "1.0",
            Some("2025_01")
        )
        .execute(&pool)
        .await?;

        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "test-protein".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-protein");
        assert_eq!(response.name, "Test Protein");
        assert_eq!(response.source_type, "protein");
        assert_eq!(response.external_id, Some("P12345".to_string()));
        assert_eq!(response.versions.len(), 1);
        assert_eq!(response.versions[0].version, "1.0");
        assert_eq!(response.latest_version, Some("1.0".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = GetDataSourceQuery {
            organization_slug: "nonexistent".to_string(),
            slug: "nonexistent".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(result, Err(GetDataSourceError::NotFound(_, _))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_with_tags(pool: PgPool) -> sqlx::Result<()> {
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

        let tag_id = sqlx::query_scalar!(
            "INSERT INTO tags (name, category) VALUES ($1, $2) RETURNING id",
            "human",
            Some("organism")
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO entry_tags (entry_id, tag_id) VALUES ($1, $2)",
            entry_id,
            tag_id
        )
        .execute(&pool)
        .await?;

        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "test-protein".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.tags.len(), 1);
        assert_eq!(response.tags[0], "human");
        Ok(())
    }
}
