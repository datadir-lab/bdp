use chrono::{DateTime, NaiveDate, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVersionQuery {
    pub organization_slug: String,
    pub data_source_slug: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVersionResponse {
    pub id: Uuid,
    pub organization_slug: String,
    pub data_source_slug: String,
    pub data_source_name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub download_count: i64,
    pub files: Vec<FileInfo>,
    pub citations: Vec<CitationInfo>,
    pub has_dependencies: bool,
    pub dependency_count: i32,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: Uuid,
    pub format: String,
    pub s3_key: String,
    pub checksum: String,
    pub size_bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationInfo {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubmed_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetVersionError {
    #[error("Organization slug is required and cannot be empty")]
    OrganizationSlugRequired,
    #[error("Data source slug is required and cannot be empty")]
    DataSourceSlugRequired,
    #[error("Version is required and cannot be empty")]
    VersionRequired,
    #[error("Version '{2}' for data source '{0}/{1}' not found")]
    NotFound(String, String, String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<GetVersionResponse, GetVersionError>> for GetVersionQuery {}

impl crate::cqrs::middleware::Query for GetVersionQuery {}

impl GetVersionQuery {
    pub fn validate(&self) -> Result<(), GetVersionError> {
        if self.organization_slug.is_empty() {
            return Err(GetVersionError::OrganizationSlugRequired);
        }
        if self.data_source_slug.is_empty() {
            return Err(GetVersionError::DataSourceSlugRequired);
        }
        if self.version.is_empty() {
            return Err(GetVersionError::VersionRequired);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetVersionQuery,
) -> Result<GetVersionResponse, GetVersionError> {
    query.validate()?;

    let version_record = sqlx::query_as!(
        VersionRecord,
        r#"
        SELECT
            v.id,
            v.version,
            v.external_version,
            v.release_date,
            v.size_bytes,
            v.download_count as "download_count!",
            v.dependency_count as "dependency_count!",
            v.published_at as "published_at!",
            re.slug as data_source_slug,
            re.name as data_source_name,
            o.slug as organization_slug
        FROM versions v
        JOIN registry_entries re ON v.entry_id = re.id
        JOIN organizations o ON re.organization_id = o.id
        WHERE LOWER(o.slug) = LOWER($1) AND LOWER(re.slug) = LOWER($2) AND v.version = $3
        "#,
        query.organization_slug,
        query.data_source_slug,
        query.version
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| {
        GetVersionError::NotFound(
            query.organization_slug.clone(),
            query.data_source_slug.clone(),
            query.version.clone(),
        )
    })?;

    let files = sqlx::query_as!(
        FileRecord,
        r#"
        SELECT id, format, s3_key, checksum, size_bytes, compression
        FROM version_files
        WHERE version_id = $1
        ORDER BY format
        "#,
        version_record.id
    )
    .fetch_all(&pool)
    .await?;

    let citations = sqlx::query_as!(
        CitationRecord,
        r#"
        SELECT id, citation_type, doi, pubmed_id, title, journal, publication_date, authors
        FROM citations
        WHERE version_id = $1
        ORDER BY citation_type, publication_date DESC
        "#,
        version_record.id
    )
    .fetch_all(&pool)
    .await?;

    let has_dependencies = version_record.dependency_count > 0;

    Ok(GetVersionResponse {
        id: version_record.id,
        organization_slug: version_record.organization_slug,
        data_source_slug: version_record.data_source_slug,
        data_source_name: version_record.data_source_name,
        version: version_record.version,
        external_version: version_record.external_version,
        release_date: version_record.release_date,
        size_bytes: version_record.size_bytes,
        download_count: version_record.download_count,
        files: files
            .into_iter()
            .map(|f| FileInfo {
                id: f.id,
                format: f.format,
                s3_key: f.s3_key,
                checksum: f.checksum,
                size_bytes: f.size_bytes,
                compression: f.compression,
            })
            .collect(),
        citations: citations
            .into_iter()
            .map(|c| CitationInfo {
                id: c.id,
                citation_type: c.citation_type,
                doi: c.doi,
                pubmed_id: c.pubmed_id,
                title: c.title,
                journal: c.journal,
                publication_date: c.publication_date,
                authors: c.authors,
            })
            .collect(),
        has_dependencies,
        dependency_count: version_record.dependency_count,
        published_at: version_record.published_at,
    })
}

#[derive(Debug)]
struct VersionRecord {
    id: Uuid,
    version: String,
    external_version: Option<String>,
    release_date: Option<NaiveDate>,
    size_bytes: Option<i64>,
    download_count: i64,
    dependency_count: i32,
    published_at: DateTime<Utc>,
    data_source_slug: String,
    data_source_name: String,
    organization_slug: String,
}

#[derive(Debug)]
struct FileRecord {
    id: Uuid,
    format: String,
    s3_key: String,
    checksum: String,
    size_bytes: i64,
    compression: Option<String>,
}

#[derive(Debug)]
struct CitationRecord {
    id: Uuid,
    citation_type: Option<String>,
    doi: Option<String>,
    pubmed_id: Option<String>,
    title: Option<String>,
    journal: Option<String>,
    publication_date: Option<NaiveDate>,
    authors: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = GetVersionQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_organization_slug() {
        let query = GetVersionQuery {
            organization_slug: "".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
        };
        assert!(matches!(query.validate(), Err(GetVersionError::OrganizationSlugRequired)));
    }

    #[test]
    fn test_validation_empty_version() {
        let query = GetVersionQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "".to_string(),
        };
        assert!(matches!(query.validate(), Err(GetVersionError::VersionRequired)));
    }

    #[sqlx::test]
    async fn test_handle_gets_version(pool: PgPool) -> sqlx::Result<()> {
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

        let version_id = sqlx::query_scalar!(
            r#"
            INSERT INTO versions (entry_id, version, external_version, release_date, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            entry_id,
            "1.0",
            Some("2025_01"),
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            Some(2048i64)
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes, compression)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            version_id,
            "fasta",
            "proteins/test-org/test-protein/1.0/test.fasta",
            "abc123",
            2048i64,
            Some("gzip")
        )
        .execute(&pool)
        .await?;

        let query = GetVersionQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.version, "1.0");
        assert_eq!(response.external_version, Some("2025_01".to_string()));
        assert_eq!(response.files.len(), 1);
        assert_eq!(response.files[0].format, "fasta");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = GetVersionQuery {
            organization_slug: "nonexistent".to_string(),
            data_source_slug: "nonexistent".to_string(),
            version: "1.0".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(result, Err(GetVersionError::NotFound(_, _, _))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_with_citations(pool: PgPool) -> sqlx::Result<()> {
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

        let version_id = sqlx::query_scalar!(
            "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
            entry_id,
            "1.0"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO citations (version_id, citation_type, doi, title)
            VALUES ($1, $2, $3, $4)
            "#,
            version_id,
            Some("primary"),
            Some("10.1234/test"),
            Some("Test Paper")
        )
        .execute(&pool)
        .await?;

        let query = GetVersionQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.citations.len(), 1);
        assert_eq!(response.citations[0].doi, Some("10.1234/test".to_string()));
        Ok(())
    }
}
