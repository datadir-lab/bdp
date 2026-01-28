//! Create data source command
//!
//! Creates a new data source entry in the registry. Data sources represent
//! biological data files such as protein sequences, genome assemblies, or
//! annotation files.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::features::shared::validation::{
    validate_name, validate_slug, validate_source_type, NameValidationError, SlugValidationError,
};

/// Command to create a new data source
///
/// Creates a registry entry and associated data_source record.
/// Type-specific metadata (protein, organism, etc.) should be added
/// via the appropriate metadata tables after creation.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::data_sources::commands::CreateDataSourceCommand;
/// use uuid::Uuid;
///
/// let command = CreateDataSourceCommand {
///     organization_id: org_id,
///     slug: "human-insulin".to_string(),
///     name: "Human Insulin".to_string(),
///     description: Some("Insulin precursor protein".to_string()),
///     source_type: "protein".to_string(),
///     external_id: Some("P01308".to_string()),
/// };
/// ```
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
    // NOTE: organism_id and additional_metadata have been removed
    // Type-specific metadata should go in *_metadata tables
}

/// Response from creating a data source
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
    // NOTE: organism_id removed - use *_metadata tables for type-specific fields
    pub created_at: DateTime<Utc>,
}

/// Errors that can occur when creating a data source
#[derive(Debug, thiserror::Error)]
pub enum CreateDataSourceError {
    /// Slug validation failed
    #[error("Slug validation failed: {0}")]
    SlugValidation(#[from] SlugValidationError),
    /// Name validation failed
    #[error("Name validation failed: {0}")]
    NameValidation(#[from] NameValidationError),
    /// Source type validation failed
    #[error("Source type validation failed: {0}")]
    SourceTypeValidation(String),
    /// The organization ID does not exist
    #[error("Organization with ID '{0}' not found")]
    OrganizationNotFound(Uuid),
    /// The organism ID does not exist (legacy - organisms are now in metadata tables)
    #[error("Organism with ID '{0}' not found")]
    OrganismNotFound(Uuid),
    /// A data source with this slug already exists
    #[error("Data source with slug '{0}' already exists")]
    DuplicateSlug(String),
    /// A database error occurred
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<CreateDataSourceResponse, CreateDataSourceError>> for CreateDataSourceCommand {}

impl crate::cqrs::middleware::Command for CreateDataSourceCommand {}

impl CreateDataSourceCommand {
    /// Validates the command parameters
    ///
    /// # Errors
    ///
    /// - `SlugValidation` - Slug validation failed (empty, too long, or invalid format)
    /// - `NameValidation` - Name validation failed (empty or too long)
    /// - `SourceTypeValidation` - Source type is not one of the allowed values
    pub fn validate(&self) -> Result<(), CreateDataSourceError> {
        // Validate slug using shared utility (255 char limit for data sources)
        validate_slug(&self.slug, 255)?;

        // Validate name using shared utility
        validate_name(&self.name, 255)?;

        // Validate source type using shared utility
        validate_source_type(&self.source_type)
            .map_err(CreateDataSourceError::SourceTypeValidation)?;

        Ok(())
    }
}

/// Handles the create data source command
///
/// Creates a new data source in a transaction:
/// 1. Validates the organization exists
/// 2. Creates a registry_entries record
/// 3. Creates a data_sources record linked to the entry
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `command` - The create command with data source details
///
/// # Returns
///
/// Returns the created data source details on success.
///
/// # Errors
///
/// - Validation errors if command parameters are invalid
/// - `OrganizationNotFound` - The organization ID doesn't exist
/// - `DuplicateSlug` - A data source with this slug already exists
/// - `Database` - A database error occurred
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
        return Err(CreateDataSourceError::OrganizationNotFound(command.organization_id));
    }

    // NOTE: organism_id validation removed - organisms are now referenced in *_metadata tables

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
        INSERT INTO data_sources (id, source_type, external_id)
        VALUES ($1, $2, $3)
        "#,
        entry_id,
        command.source_type,
        command.external_id
    )
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query_as!(
        DataSourceRecord,
        r#"
        SELECT
            re.id, re.organization_id, re.slug, re.name, re.description,
            ds.source_type, ds.external_id,
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
        created_at: result.created_at,
    })
}

#[derive(Debug)]
#[allow(dead_code)]
struct DataSourceRecord {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    source_type: String,
    external_id: Option<String>,
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
            // organism_id: None,
            // additional_metadata: None,
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
        };
        assert!(matches!(cmd.validate(), Err(CreateDataSourceError::SlugValidation(_))));
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
        };
        assert!(matches!(cmd.validate(), Err(CreateDataSourceError::SourceTypeValidation(_))));
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
            // organism_id: None,
            // additional_metadata: None,
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
            // organism_id: None,
            // additional_metadata: None,
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = CreateDataSourceCommand {
            organization_id: org_id,
            slug: "duplicate".to_string(),
            name: "Second".to_string(),
            description: None,
            source_type: "genome".to_string(),
            external_id: None,
            // organism_id: None,
            // additional_metadata: None,
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(matches!(result, Err(CreateDataSourceError::DuplicateSlug(_))));
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
            // organism_id: None,
            // additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(CreateDataSourceError::OrganizationNotFound(_))));
        Ok(())
    }
}
