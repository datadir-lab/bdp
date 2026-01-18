use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDataSourceCommand {
    pub organization_id: Uuid,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDataSourceResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateDataSourceError {
    #[error("Slug is required and cannot be empty")]
    SlugRequired,
    #[error("Slug must be between 1 and 255 characters")]
    SlugLength,
    #[error("Name is required and cannot be empty")]
    NameRequired,
    #[error("Name must be between 1 and 255 characters")]
    NameLength,
    #[error("Source type is required")]
    SourceTypeRequired,
    #[error("Invalid source type: {0}. Must be one of: protein, genome, annotation, structure, other")]
    InvalidSourceType(String),
    #[error("Organization with ID '{0}' not found")]
    OrganizationNotFound(Uuid),
    #[error("Organism with ID '{0}' not found")]
    OrganismNotFound(Uuid),
    #[error("Data source with slug '{0}' already exists")]
    DuplicateSlug(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<CreateDataSourceResponse, CreateDataSourceError>> for CreateDataSourceCommand {}

impl crate::cqrs::middleware::Command for CreateDataSourceCommand {}

impl CreateDataSourceCommand {
    pub fn validate(&self) -> Result<(), CreateDataSourceError> {
        if self.slug.is_empty() {
            return Err(CreateDataSourceError::SlugRequired);
        }
        if self.slug.len() > 255 {
            return Err(CreateDataSourceError::SlugLength);
        }
        if self.name.trim().is_empty() {
            return Err(CreateDataSourceError::NameRequired);
        }
        if self.name.len() > 255 {
            return Err(CreateDataSourceError::NameLength);
        }
        if self.source_type.is_empty() {
            return Err(CreateDataSourceError::SourceTypeRequired);
        }
        if !matches!(
            self.source_type.as_str(),
            "protein" | "genome" | "annotation" | "structure" | "other"
        ) {
            return Err(CreateDataSourceError::InvalidSourceType(
                self.source_type.clone(),
            ));
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: CreateDataSourceCommand,
) -> Result<CreateDataSourceResponse, CreateDataSourceError> {
    command.validate()?;

    let org_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)",
        command.organization_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(false);

    if !org_exists {
        return Err(CreateDataSourceError::OrganizationNotFound(
            command.organization_id,
        ));
    }

    if let Some(organism_id) = command.organism_id {
        let organism_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM organisms WHERE id = $1)",
            organism_id
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(false);

        if !organism_exists {
            return Err(CreateDataSourceError::OrganismNotFound(organism_id));
        }
    }

    let mut tx = pool.begin().await?;

    let entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, $2, $3, $4, 'data_source')
        RETURNING id
        "#,
        command.organization_id,
        command.slug,
        command.name,
        command.description
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return CreateDataSourceError::DuplicateSlug(command.slug.clone());
            }
        }
        CreateDataSourceError::Database(e)
    })?;

    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type, external_id, organism_id, additional_metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        entry_id,
        command.source_type,
        command.external_id,
        command.organism_id,
        command.additional_metadata
    )
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query_as!(
        DataSourceRecord,
        r#"
        SELECT
            re.id, re.organization_id, re.slug, re.name, re.description,
            ds.source_type, ds.external_id, ds.organism_id,
            re.created_at as "created_at!", re.updated_at as "updated_at!"
        FROM registry_entries re
        JOIN data_sources ds ON re.id = ds.id
        WHERE re.id = $1
        "#,
        entry_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(CreateDataSourceResponse {
        id: result.id,
        organization_id: result.organization_id,
        slug: result.slug,
        name: result.name,
        description: result.description,
        source_type: result.source_type,
        external_id: result.external_id,
        organism_id: result.organism_id,
        created_at: result.created_at,
    })
}

#[derive(Debug)]
struct DataSourceRecord {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    source_type: String,
    external_id: Option<String>,
    organism_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = CreateDataSourceCommand {
            organization_id: Uuid::new_v4(),
            slug: "uniprot-p01308".to_string(),
            name: "Insulin precursor".to_string(),
            description: Some("Human insulin".to_string()),
            source_type: "protein".to_string(),
            external_id: Some("P01308".to_string()),
            organism_id: None,
            additional_metadata: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_slug() {
        let cmd = CreateDataSourceCommand {
            organization_id: Uuid::new_v4(),
            slug: "".to_string(),
            name: "Name".to_string(),
            description: None,
            source_type: "protein".to_string(),
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateDataSourceError::SlugRequired)
        ));
    }

    #[test]
    fn test_validation_invalid_source_type() {
        let cmd = CreateDataSourceCommand {
            organization_id: Uuid::new_v4(),
            slug: "test".to_string(),
            name: "Name".to_string(),
            description: None,
            source_type: "invalid".to_string(),
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateDataSourceError::InvalidSourceType(_))
        ));
    }

    #[sqlx::test]
    async fn test_handle_creates_data_source(pool: PgPool) -> sqlx::Result<()> {
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

        let cmd = CreateDataSourceCommand {
            organization_id: org_id,
            slug: "test-protein".to_string(),
            name: "Test Protein".to_string(),
            description: Some("Test description".to_string()),
            source_type: "protein".to_string(),
            external_id: Some("P12345".to_string()),
            organism_id: None,
            additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-protein");
        assert_eq!(response.source_type, "protein");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_duplicate_slug(pool: PgPool) -> sqlx::Result<()> {
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

        let cmd1 = CreateDataSourceCommand {
            organization_id: org_id,
            slug: "duplicate".to_string(),
            name: "First".to_string(),
            description: None,
            source_type: "protein".to_string(),
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = CreateDataSourceCommand {
            organization_id: org_id,
            slug: "duplicate".to_string(),
            name: "Second".to_string(),
            description: None,
            source_type: "genome".to_string(),
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(matches!(
            result,
            Err(CreateDataSourceError::DuplicateSlug(_))
        ));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_organization_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateDataSourceCommand {
            organization_id: Uuid::new_v4(),
            slug: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            source_type: "protein".to_string(),
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(
            result,
            Err(CreateDataSourceError::OrganizationNotFound(_))
        ));
        Ok(())
    }
}
