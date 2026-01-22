use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrganizationQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrganizationResponse {
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
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetOrganizationError {
    #[error("Either slug or id is required")]
    SlugOrIdRequired,
    #[error("Organization not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<GetOrganizationResponse, GetOrganizationError>> for GetOrganizationQuery {}

impl crate::cqrs::middleware::Query for GetOrganizationQuery {}

impl GetOrganizationQuery {
    pub fn validate(&self) -> Result<(), GetOrganizationError> {
        if self.slug.is_none() && self.id.is_none() {
            return Err(GetOrganizationError::SlugOrIdRequired);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetOrganizationQuery,
) -> Result<GetOrganizationResponse, GetOrganizationError> {
    query.validate()?;

    let result = if let Some(slug) = query.slug {
        sqlx::query_as!(
            OrganizationRecord,
            r#"
            SELECT id, slug, name, website, description, logo_url,
                   is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
            FROM organizations
            WHERE LOWER(slug) = LOWER($1)
            "#,
            slug
        )
        .fetch_optional(&pool)
        .await?
    } else if let Some(id) = query.id {
        sqlx::query_as!(
            OrganizationRecord,
            r#"
            SELECT id, slug, name, website, description, logo_url,
                   is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
            FROM organizations
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&pool)
        .await?
    } else {
        None
    };

    let org = result.ok_or(GetOrganizationError::NotFound)?;

    Ok(GetOrganizationResponse {
        id: org.id,
        slug: org.slug,
        name: org.name,
        website: org.website,
        description: org.description,
        logo_url: org.logo_url,
        is_system: org.is_system,
        created_at: org.created_at,
        updated_at: org.updated_at,
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
    fn test_validation_success_with_slug() {
        let query = GetOrganizationQuery {
            slug: Some("test-org".to_string()),
            id: None,
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_success_with_id() {
        let query = GetOrganizationQuery {
            slug: None,
            id: Some(Uuid::new_v4()),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_failure_no_slug_or_id() {
        let query = GetOrganizationQuery {
            slug: None,
            id: None,
        };
        assert!(matches!(
            query.validate(),
            Err(GetOrganizationError::SlugOrIdRequired)
        ));
    }

    #[sqlx::test]
    async fn test_handle_get_by_slug(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::new_v4(),
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let query = GetOrganizationQuery {
            slug: Some("test-org".to_string()),
            id: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        assert_eq!(response.name, "Test Org");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_get_by_id(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org_id,
            "test-org-2",
            "Test Org 2",
            false
        )
        .execute(&pool)
        .await?;

        let query = GetOrganizationQuery {
            slug: None,
            id: Some(org_id),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, org_id);
        assert_eq!(response.slug, "test-org-2");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = GetOrganizationQuery {
            slug: Some("nonexistent".to_string()),
            id: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(result, Err(GetOrganizationError::NotFound)));
        Ok(())
    }
}
