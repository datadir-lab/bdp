use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrCreateOrganismQuery {
    pub slug: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GetOrCreateOrganismError {
    #[error("Slug is required and cannot be empty")]
    SlugRequired,
    #[error("Organization with slug '{0}' not found")]
    NotFound(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<Uuid, GetOrCreateOrganismError>> for GetOrCreateOrganismQuery {}

impl crate::cqrs::middleware::Query for GetOrCreateOrganismQuery {}

impl GetOrCreateOrganismQuery {
    pub fn validate(&self) -> Result<(), GetOrCreateOrganismError> {
        if self.slug.trim().is_empty() {
            return Err(GetOrCreateOrganismError::SlugRequired);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetOrCreateOrganismQuery,
) -> Result<Uuid, GetOrCreateOrganismError> {
    query.validate()?;

    let result = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM organizations
        WHERE slug = $1
        "#,
        query.slug
    )
    .fetch_optional(&pool)
    .await?;

    result.ok_or_else(|| GetOrCreateOrganismError::NotFound(query.slug.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = GetOrCreateOrganismQuery {
            slug: "uniprot".to_string(),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_slug() {
        let query = GetOrCreateOrganismQuery {
            slug: "".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(GetOrCreateOrganismError::SlugRequired)
        ));
    }

    #[test]
    fn test_validation_whitespace_slug() {
        let query = GetOrCreateOrganismQuery {
            slug: "   ".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(GetOrCreateOrganismError::SlugRequired)
        ));
    }

    #[sqlx::test]
    async fn test_handle_finds_existing_organization(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org_id,
            "test-org",
            "Test Organization",
            false
        )
        .execute(&pool)
        .await?;

        let query = GetOrCreateOrganismQuery {
            slug: "test-org".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), org_id);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_returns_error_when_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = GetOrCreateOrganismQuery {
            slug: "nonexistent-org".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(
            result,
            Err(GetOrCreateOrganismError::NotFound(_))
        ));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_case_sensitive_slug(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org_id,
            "lowercase-slug",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let query = GetOrCreateOrganismQuery {
            slug: "LOWERCASE-SLUG".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        // Should not find it since slug is case-sensitive
        assert!(matches!(
            result,
            Err(GetOrCreateOrganismError::NotFound(_))
        ));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_multiple_organizations(pool: PgPool) -> sqlx::Result<()> {
        let org1_id = Uuid::new_v4();
        let org2_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org1_id,
            "org-one",
            "Organization One",
            false
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org2_id,
            "org-two",
            "Organization Two",
            false
        )
        .execute(&pool)
        .await?;

        let query1 = GetOrCreateOrganismQuery {
            slug: "org-one".to_string(),
        };
        let result1 = handle(pool.clone(), query1).await.unwrap();

        let query2 = GetOrCreateOrganismQuery {
            slug: "org-two".to_string(),
        };
        let result2 = handle(pool.clone(), query2).await.unwrap();

        assert_eq!(result1, org1_id);
        assert_eq!(result2, org2_id);
        assert_ne!(result1, result2);
        Ok(())
    }
}
