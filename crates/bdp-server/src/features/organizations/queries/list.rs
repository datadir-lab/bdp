use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrganizationsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_system: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationListItem {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrganizationsResponse {
    pub items: Vec<OrganizationListItem>,
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
pub enum ListOrganizationsError {
    #[error("Page must be greater than 0")]
    InvalidPage,
    #[error("Per page must be between 1 and 100")]
    InvalidPerPage,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<ListOrganizationsResponse, ListOrganizationsError>>
    for ListOrganizationsQuery
{
}

impl crate::cqrs::middleware::Query for ListOrganizationsQuery {}

impl ListOrganizationsQuery {
    pub fn validate(&self) -> Result<(), ListOrganizationsError> {
        if let Some(page) = self.page {
            if page < 1 {
                return Err(ListOrganizationsError::InvalidPage);
            }
        }
        if let Some(per_page) = self.per_page {
            if per_page < 1 || per_page > 100 {
                return Err(ListOrganizationsError::InvalidPerPage);
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
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: ListOrganizationsQuery,
) -> Result<ListOrganizationsResponse, ListOrganizationsError> {
    query.validate()?;

    let page = query.page();
    let per_page = query.per_page();
    let offset = (page - 1) * per_page;

    let total = if query.is_system.is_some() || query.name_contains.is_some() {
        let name_pattern = query
            .name_contains
            .as_ref()
            .map(|s| format!("%{}%", s.to_lowercase()));

        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM organizations
            WHERE ($1::BOOLEAN IS NULL OR is_system = $1)
              AND ($2::TEXT IS NULL OR LOWER(name) LIKE $2)
            "#,
            query.is_system,
            name_pattern.as_deref()
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(0)
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM organizations
            "#
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(0)
    };

    let name_pattern = query
        .name_contains
        .as_ref()
        .map(|s| format!("%{}%", s.to_lowercase()));

    let records = sqlx::query_as!(
        OrganizationRecord,
        r#"
        SELECT id, slug, name, website, description, logo_url,
               is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
        FROM organizations
        WHERE ($1::BOOLEAN IS NULL OR is_system = $1)
          AND ($2::TEXT IS NULL OR LOWER(name) LIKE $2)
        ORDER BY created_at DESC
        LIMIT $3
        OFFSET $4
        "#,
        query.is_system,
        name_pattern.as_deref(),
        per_page,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let items = records
        .into_iter()
        .map(|r| OrganizationListItem {
            id: r.id,
            slug: r.slug,
            name: r.name,
            website: r.website,
            description: r.description,
            logo_url: r.logo_url,
            is_system: r.is_system,
            created_at: r.created_at,
        })
        .collect();

    let pages = if total == 0 {
        0
    } else {
        ((total as f64) / (per_page as f64)).ceil() as i64
    };

    Ok(ListOrganizationsResponse {
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

#[derive(Debug)]
struct OrganizationRecord {
    id: Uuid,
    slug: String,
    name: String,
    website: Option<String>,
    description: Option<String>,
    logo_url: Option<String>,
    is_system: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = ListOrganizationsQuery {
            page: Some(1),
            per_page: Some(20),
            is_system: None,
            name_contains: None,
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_invalid_page() {
        let query = ListOrganizationsQuery {
            page: Some(0),
            per_page: Some(20),
            is_system: None,
            name_contains: None,
        };
        assert!(matches!(
            query.validate(),
            Err(ListOrganizationsError::InvalidPage)
        ));
    }

    #[test]
    fn test_validation_invalid_per_page() {
        let query = ListOrganizationsQuery {
            page: Some(1),
            per_page: Some(101),
            is_system: None,
            name_contains: None,
        };
        assert!(matches!(
            query.validate(),
            Err(ListOrganizationsError::InvalidPerPage)
        ));
    }

    #[sqlx::test]
    async fn test_handle_lists_organizations(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org-1', 'Test Org 1', false),
                   ('test-org-2', 'Test Org 2', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = ListOrganizationsQuery {
            page: Some(1),
            per_page: Some(10),
            is_system: None,
            name_contains: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.pagination.total, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_filters_by_is_system(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('test-org-1', 'Test Org 1', false),
                   ('test-org-2', 'Test Org 2', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = ListOrganizationsQuery {
            page: Some(1),
            per_page: Some(10),
            is_system: Some(true),
            name_contains: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].slug, "test-org-2");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_filters_by_name(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ('uniprot', 'UniProt', true),
                   ('ncbi', 'NCBI', true)
            "#
        )
        .execute(&pool)
        .await?;

        let query = ListOrganizationsQuery {
            page: Some(1),
            per_page: Some(10),
            is_system: None,
            name_contains: Some("uni".to_string()),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].slug, "uniprot");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_pagination(pool: PgPool) -> sqlx::Result<()> {
        for i in 1..=25 {
            sqlx::query!(
                r#"
                INSERT INTO organizations (slug, name, is_system)
                VALUES ($1, $2, false)
                "#,
                format!("org-{}", i),
                format!("Org {}", i)
            )
            .execute(&pool)
            .await?;
        }

        let query = ListOrganizationsQuery {
            page: Some(2),
            per_page: Some(10),
            is_system: None,
            name_contains: None,
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
