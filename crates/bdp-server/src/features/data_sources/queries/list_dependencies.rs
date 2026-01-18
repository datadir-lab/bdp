use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDependenciesQuery {
    pub organization_slug: String,
    pub data_source_slug: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyItem {
    pub id: Uuid,
    pub organization_slug: String,
    pub entry_slug: String,
    pub entry_name: String,
    pub entry_type: String,
    pub required_version: String,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDependenciesResponse {
    pub source: String,
    pub version: String,
    pub dependency_count: i64,
    pub dependencies: Vec<DependencyItem>,
    pub pagination: PaginationMetadata,
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
pub enum ListDependenciesError {
    #[error("Organization slug is required and cannot be empty")]
    OrganizationSlugRequired,
    #[error("Data source slug is required and cannot be empty")]
    DataSourceSlugRequired,
    #[error("Version is required and cannot be empty")]
    VersionRequired,
    #[error("Page must be greater than 0")]
    InvalidPage,
    #[error("Per page must be between 1 and 1000")]
    InvalidPerPage,
    #[error("Version '{2}' for data source '{0}/{1}' not found")]
    NotFound(String, String, String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<ListDependenciesResponse, ListDependenciesError>> for ListDependenciesQuery {}

impl crate::cqrs::middleware::Query for ListDependenciesQuery {}

impl ListDependenciesQuery {
    pub fn validate(&self) -> Result<(), ListDependenciesError> {
        if self.organization_slug.is_empty() {
            return Err(ListDependenciesError::OrganizationSlugRequired);
        }
        if self.data_source_slug.is_empty() {
            return Err(ListDependenciesError::DataSourceSlugRequired);
        }
        if self.version.is_empty() {
            return Err(ListDependenciesError::VersionRequired);
        }
        if let Some(page) = self.page {
            if page < 1 {
                return Err(ListDependenciesError::InvalidPage);
            }
        }
        if let Some(per_page) = self.per_page {
            if per_page < 1 || per_page > 1000 {
                return Err(ListDependenciesError::InvalidPerPage);
            }
        }
        Ok(())
    }

    fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(100).clamp(1, 1000)
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: ListDependenciesQuery,
) -> Result<ListDependenciesResponse, ListDependenciesError> {
    query.validate()?;

    let version_id = sqlx::query_scalar!(
        r#"
        SELECT v.id
        FROM versions v
        JOIN registry_entries re ON v.entry_id = re.id
        JOIN organizations o ON re.organization_id = o.id
        WHERE o.slug = $1 AND re.slug = $2 AND v.version = $3
        "#,
        query.organization_slug,
        query.data_source_slug,
        query.version
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| {
        ListDependenciesError::NotFound(
            query.organization_slug.clone(),
            query.data_source_slug.clone(),
            query.version.clone(),
        )
    })?;

    let page = query.page();
    let per_page = query.per_page();
    let offset = (page - 1) * per_page;

    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM dependencies d
        WHERE d.version_id = $1
        "#,
        version_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let records = sqlx::query_as!(
        DependencyRecord,
        r#"
        SELECT
            d.id,
            d.depends_on_version as required_version,
            COALESCE(d.dependency_type, 'required') as "dependency_type!",
            re.slug as entry_slug,
            re.name as entry_name,
            re.entry_type,
            o.slug as organization_slug
        FROM dependencies d
        JOIN registry_entries re ON d.depends_on_entry_id = re.id
        JOIN organizations o ON re.organization_id = o.id
        WHERE d.version_id = $1
        ORDER BY re.slug
        LIMIT $2
        OFFSET $3
        "#,
        version_id,
        per_page,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let dependencies = records
        .into_iter()
        .map(|r| DependencyItem {
            id: r.id,
            organization_slug: r.organization_slug,
            entry_slug: r.entry_slug,
            entry_name: r.entry_name,
            entry_type: r.entry_type,
            required_version: r.required_version,
            dependency_type: r.dependency_type,
        })
        .collect();

    let pages = if total == 0 {
        0
    } else {
        ((total as f64) / (per_page as f64)).ceil() as i64
    };

    Ok(ListDependenciesResponse {
        source: format!("{}/{}", query.organization_slug, query.data_source_slug),
        version: query.version.clone(),
        dependency_count: total,
        dependencies,
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

#[derive(Debug)]
struct DependencyRecord {
    id: Uuid,
    organization_slug: String,
    entry_slug: String,
    entry_name: String,
    entry_type: String,
    required_version: String,
    dependency_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = ListDependenciesQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
            page: Some(1),
            per_page: Some(100),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_organization_slug() {
        let query = ListDependenciesQuery {
            organization_slug: "".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
            page: None,
            per_page: None,
        };
        assert!(matches!(
            query.validate(),
            Err(ListDependenciesError::OrganizationSlugRequired)
        ));
    }

    #[test]
    fn test_validation_invalid_per_page() {
        let query = ListDependenciesQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "test-protein".to_string(),
            version: "1.0".to_string(),
            page: Some(1),
            per_page: Some(1001),
        };
        assert!(matches!(
            query.validate(),
            Err(ListDependenciesError::InvalidPerPage)
        ));
    }

    #[sqlx::test]
    async fn test_handle_lists_dependencies(pool: PgPool) -> sqlx::Result<()> {
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

        let version1_id = sqlx::query_scalar!(
            "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
            entry1_id,
            "1.0"
        )
        .fetch_one(&pool)
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

        sqlx::query!(
            r#"
            INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version, dependency_type)
            VALUES ($1, $2, $3, $4)
            "#,
            version1_id,
            entry2_id,
            "2.0",
            "required"
        )
        .execute(&pool)
        .await?;

        let query = ListDependenciesQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "protein-1".to_string(),
            version: "1.0".to_string(),
            page: Some(1),
            per_page: Some(100),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.dependency_count, 1);
        assert_eq!(response.dependencies.len(), 1);
        assert_eq!(response.dependencies[0].entry_slug, "genome-1");
        assert_eq!(response.dependencies[0].required_version, "2.0");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = ListDependenciesQuery {
            organization_slug: "nonexistent".to_string(),
            data_source_slug: "nonexistent".to_string(),
            version: "1.0".to_string(),
            page: None,
            per_page: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(
            result,
            Err(ListDependenciesError::NotFound(_, _, _))
        ));
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

        let entry1_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "main-protein",
            "Main Protein"
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

        let version_id = sqlx::query_scalar!(
            "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
            entry1_id,
            "1.0"
        )
        .fetch_one(&pool)
        .await?;

        for i in 1..=25 {
            let dep_entry_id = sqlx::query_scalar!(
                r#"
                INSERT INTO registry_entries (organization_id, slug, name, entry_type)
                VALUES ($1, $2, $3, 'data_source')
                RETURNING id
                "#,
                org_id,
                format!("dep-{}", i),
                format!("Dependency {}", i)
            )
            .fetch_one(&pool)
            .await?;

            sqlx::query!(
                "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
                dep_entry_id,
                "protein"
            )
            .execute(&pool)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
                VALUES ($1, $2, $3)
                "#,
                version_id,
                dep_entry_id,
                "1.0"
            )
            .execute(&pool)
            .await?;
        }

        let query = ListDependenciesQuery {
            organization_slug: "test-org".to_string(),
            data_source_slug: "main-protein".to_string(),
            version: "1.0".to_string(),
            page: Some(2),
            per_page: Some(10),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.dependencies.len(), 10);
        assert_eq!(response.pagination.page, 2);
        assert_eq!(response.pagination.total, 25);
        assert_eq!(response.pagination.pages, 3);
        assert!(response.pagination.has_prev);
        assert!(response.pagination.has_next);
        Ok(())
    }
}
