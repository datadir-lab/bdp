use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDataSourceCommand {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDataSourceResponse {
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
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateDataSourceError {
    #[error("At least one field must be provided for update")]
    NoFieldsToUpdate,
    #[error("Name must be between 1 and 255 characters")]
    NameLength,
    #[error("Name cannot be empty or only whitespace")]
    NameEmpty,
    #[error("Data source with ID '{0}' not found")]
    NotFound(Uuid),
    #[error("Organism with ID '{0}' not found")]
    OrganismNotFound(Uuid),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<UpdateDataSourceResponse, UpdateDataSourceError>> for UpdateDataSourceCommand {}

impl crate::cqrs::middleware::Command for UpdateDataSourceCommand {}

impl UpdateDataSourceCommand {
    pub fn validate(&self) -> Result<(), UpdateDataSourceError> {
        if self.name.is_none()
            && self.description.is_none()
            && self.external_id.is_none()
            && self.organism_id.is_none()
            && self.additional_metadata.is_none()
        {
            return Err(UpdateDataSourceError::NoFieldsToUpdate);
        }
        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                return Err(UpdateDataSourceError::NameEmpty);
            }
            if name.len() > 255 {
                return Err(UpdateDataSourceError::NameLength);
            }
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: UpdateDataSourceCommand,
) -> Result<UpdateDataSourceResponse, UpdateDataSourceError> {
    command.validate()?;

    if let Some(organism_id) = command.organism_id {
        let organism_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM organisms WHERE id = $1)",
            organism_id
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(false);

        if !organism_exists {
            return Err(UpdateDataSourceError::OrganismNotFound(organism_id));
        }
    }

    let mut tx = pool.begin().await?;

    let current = sqlx::query_as!(
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
        command.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| UpdateDataSourceError::NotFound(command.id))?;

    let new_name = command.name.as_ref().unwrap_or(&current.name);
    let new_description = command.description.as_ref().or(current.description.as_ref());

    sqlx::query!(
        r#"
        UPDATE registry_entries
        SET name = $2, description = $3, updated_at = NOW()
        WHERE id = $1
        "#,
        command.id,
        new_name,
        new_description
    )
    .execute(&mut *tx)
    .await?;

    let new_external_id = command.external_id.as_ref().or(current.external_id.as_ref());
    let new_organism_id = command.organism_id.or(current.organism_id);

    if command.additional_metadata.is_some() {
        sqlx::query!(
            r#"
            UPDATE data_sources
            SET external_id = $2, organism_id = $3, additional_metadata = $4
            WHERE id = $1
            "#,
            command.id,
            new_external_id,
            new_organism_id,
            command.additional_metadata
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            r#"
            UPDATE data_sources
            SET external_id = $2, organism_id = $3
            WHERE id = $1
            "#,
            command.id,
            new_external_id,
            new_organism_id
        )
        .execute(&mut *tx)
        .await?;
    }

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
        command.id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(UpdateDataSourceResponse {
        id: result.id,
        organization_id: result.organization_id,
        slug: result.slug,
        name: result.name,
        description: result.description,
        source_type: result.source_type,
        external_id: result.external_id,
        organism_id: result.organism_id,
        updated_at: result.updated_at,
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
        let cmd = UpdateDataSourceCommand {
            id: Uuid::new_v4(),
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_no_fields() {
        let cmd = UpdateDataSourceCommand {
            id: Uuid::new_v4(),
            name: None,
            description: None,
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(UpdateDataSourceError::NoFieldsToUpdate)
        ));
    }

    #[test]
    fn test_validation_empty_name() {
        let cmd = UpdateDataSourceCommand {
            id: Uuid::new_v4(),
            name: Some("   ".to_string()),
            description: None,
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(UpdateDataSourceError::NameEmpty)
        ));
    }

    #[sqlx::test]
    async fn test_handle_updates_data_source(pool: PgPool) -> sqlx::Result<()> {
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
            "Original Name"
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

        let cmd = UpdateDataSourceCommand {
            id: entry_id,
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            external_id: Some("P12345".to_string()),
            organism_id: None,
            additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.name, "Updated Name");
        assert_eq!(
            response.description,
            Some("Updated description".to_string())
        );
        assert_eq!(response.external_id, Some("P12345".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = UpdateDataSourceCommand {
            id: Uuid::new_v4(),
            name: Some("Name".to_string()),
            description: None,
            external_id: None,
            organism_id: None,
            additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(UpdateDataSourceError::NotFound(_))));
        Ok(())
    }
}
