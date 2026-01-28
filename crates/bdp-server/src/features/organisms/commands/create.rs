use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganismCommand {
    pub taxonomy_id: i32,
    pub scientific_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganismResponse {
    pub id: Uuid,
    pub taxonomy_id: i32,
    pub scientific_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateOrganismError {
    #[error("Taxonomy ID must be greater than 0")]
    InvalidTaxonomyId,
    #[error("Scientific name is required and cannot be empty")]
    ScientificNameRequired,
    #[error("Scientific name must be between 1 and 255 characters")]
    ScientificNameLength,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<CreateOrganismResponse, CreateOrganismError>> for CreateOrganismCommand {}

impl crate::cqrs::middleware::Command for CreateOrganismCommand {}

impl CreateOrganismCommand {
    pub fn validate(&self) -> Result<(), CreateOrganismError> {
        if self.taxonomy_id <= 0 {
            return Err(CreateOrganismError::InvalidTaxonomyId);
        }
        if self.scientific_name.trim().is_empty() {
            return Err(CreateOrganismError::ScientificNameRequired);
        }
        if self.scientific_name.len() > 255 {
            return Err(CreateOrganismError::ScientificNameLength);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: CreateOrganismCommand,
) -> Result<CreateOrganismResponse, CreateOrganismError> {
    command.validate()?;

    let result = sqlx::query_as!(
        OrganismRecord,
        r#"
        INSERT INTO organisms (ncbi_taxonomy_id, scientific_name, common_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (ncbi_taxonomy_id)
        DO UPDATE SET
            scientific_name = EXCLUDED.scientific_name,
            common_name = EXCLUDED.common_name
        RETURNING id, ncbi_taxonomy_id, scientific_name, common_name, created_at
        "#,
        command.taxonomy_id,
        command.scientific_name,
        command.common_name
    )
    .fetch_one(&pool)
    .await?;

    Ok(CreateOrganismResponse {
        id: result.id,
        taxonomy_id: result.ncbi_taxonomy_id.unwrap_or(0),
        scientific_name: result.scientific_name,
        common_name: result.common_name,
        created_at: result.created_at.unwrap_or_else(chrono::Utc::now),
    })
}

#[derive(Debug)]
struct OrganismRecord {
    id: Uuid,
    ncbi_taxonomy_id: Option<i32>,
    scientific_name: String,
    common_name: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "Homo sapiens".to_string(),
            common_name: Some("Human".to_string()),
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_success_minimal() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 1,
            scientific_name: "Test organism".to_string(),
            common_name: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_zero_taxonomy_id() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 0,
            scientific_name: "Test".to_string(),
            common_name: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganismError::InvalidTaxonomyId)
        ));
    }

    #[test]
    fn test_validation_negative_taxonomy_id() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: -1,
            scientific_name: "Test".to_string(),
            common_name: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganismError::InvalidTaxonomyId)
        ));
    }

    #[test]
    fn test_validation_empty_scientific_name() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "".to_string(),
            common_name: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganismError::ScientificNameRequired)
        ));
    }

    #[test]
    fn test_validation_whitespace_scientific_name() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "   ".to_string(),
            common_name: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganismError::ScientificNameRequired)
        ));
    }

    #[test]
    fn test_validation_scientific_name_too_long() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "a".repeat(256),
            common_name: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganismError::ScientificNameLength)
        ));
    }

    #[test]
    fn test_validation_scientific_name_max_length() {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "a".repeat(255),
            common_name: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_handle_creates_organism(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "Homo sapiens".to_string(),
            common_name: Some("Human".to_string()),
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.taxonomy_id, 9606);
        assert_eq!(response.scientific_name, "Homo sapiens");
        assert_eq!(response.common_name, Some("Human".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_creates_organism_without_common_name(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 10090,
            scientific_name: "Mus musculus".to_string(),
            common_name: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.taxonomy_id, 10090);
        assert_eq!(response.scientific_name, "Mus musculus");
        assert_eq!(response.common_name, None);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_upsert_updates_existing(pool: PgPool) -> sqlx::Result<()> {
        let cmd1 = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "Homo sapiens".to_string(),
            common_name: Some("Human".to_string()),
        };
        let result1 = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "Homo sapiens updated".to_string(),
            common_name: Some("Human updated".to_string()),
        };
        let result2 = handle(pool.clone(), cmd2).await.unwrap();

        // Should update the existing record
        assert_eq!(result1.id, result2.id);
        assert_eq!(result2.scientific_name, "Homo sapiens updated");
        assert_eq!(result2.common_name, Some("Human updated".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_upsert_preserves_id(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateOrganismCommand {
            taxonomy_id: 7227,
            scientific_name: "Drosophila melanogaster".to_string(),
            common_name: Some("Fruit fly".to_string()),
        };
        let first_result = handle(pool.clone(), cmd.clone()).await.unwrap();
        let first_id = first_result.id;

        // Run the same command again
        let second_result = handle(pool.clone(), cmd).await.unwrap();

        // ID should be the same
        assert_eq!(first_id, second_result.id);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_multiple_different_organisms(pool: PgPool) -> sqlx::Result<()> {
        let cmd1 = CreateOrganismCommand {
            taxonomy_id: 9606,
            scientific_name: "Homo sapiens".to_string(),
            common_name: Some("Human".to_string()),
        };
        let result1 = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = CreateOrganismCommand {
            taxonomy_id: 10090,
            scientific_name: "Mus musculus".to_string(),
            common_name: Some("Mouse".to_string()),
        };
        let result2 = handle(pool.clone(), cmd2).await.unwrap();

        // Should create different records
        assert_ne!(result1.id, result2.id);
        assert_eq!(result1.taxonomy_id, 9606);
        assert_eq!(result2.taxonomy_id, 10090);
        Ok(())
    }
}
