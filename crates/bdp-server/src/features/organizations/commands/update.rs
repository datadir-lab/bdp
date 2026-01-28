//! Update organization command
//!
//! Partially updates an existing organization. Only the fields that are
//! provided will be updated; others remain unchanged.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Command to update an existing organization
///
/// At least one field besides `slug` must be provided for update.
/// The `slug` identifies which organization to update and cannot itself
/// be changed.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::organizations::commands::UpdateOrganizationCommand;
///
/// let command = UpdateOrganizationCommand {
///     slug: "acme-corp".to_string(),
///     name: Some("ACME Corporation Inc.".to_string()),
///     website: Some("https://acme.io".to_string()),
///     description: None,  // Keep existing description
///     logo_url: None,
///     is_system: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationCommand {
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_system: Option<bool>,
}

/// Response from updating an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationResponse {
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
    pub updated_at: DateTime<Utc>,
}

/// Errors that can occur when updating an organization
#[derive(Debug, thiserror::Error)]
pub enum UpdateOrganizationError {
    /// The slug parameter was empty
    #[error("Slug is required and cannot be empty")]
    SlugRequired,
    /// No fields were provided for update
    #[error("At least one field must be provided for update")]
    NoFieldsToUpdate,
    /// Name exceeds maximum length
    #[error("Name must be between 1 and 256 characters")]
    NameLength,
    /// Name was empty or only whitespace
    #[error("Name cannot be empty or only whitespace")]
    NameEmpty,
    /// Website URL failed validation
    #[error("Website URL is invalid: {0}")]
    WebsiteInvalid(String),
    /// Logo URL failed validation
    #[error("Logo URL is invalid: {0}")]
    LogoUrlInvalid(String),
    /// Organization with the given slug was not found
    #[error("Organization with slug '{0}' not found")]
    NotFound(String),
    /// A database error occurred
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<UpdateOrganizationResponse, UpdateOrganizationError>>
    for UpdateOrganizationCommand
{
}

impl crate::cqrs::middleware::Command for UpdateOrganizationCommand {}

impl UpdateOrganizationCommand {
    /// Validates the command parameters
    ///
    /// # Errors
    ///
    /// - `SlugRequired` - Slug is empty
    /// - `NoFieldsToUpdate` - No fields provided for update
    /// - `NameLength` - Name exceeds 256 characters
    /// - `NameEmpty` - Name is empty or whitespace-only
    /// - `WebsiteInvalid` - Website URL is not a valid HTTP(S) URL
    /// - `LogoUrlInvalid` - Logo URL is not a valid HTTP(S) URL
    pub fn validate(&self) -> Result<(), UpdateOrganizationError> {
        if self.slug.is_empty() {
            return Err(UpdateOrganizationError::SlugRequired);
        }
        if self.name.is_none()
            && self.website.is_none()
            && self.description.is_none()
            && self.logo_url.is_none()
            && self.is_system.is_none()
        {
            return Err(UpdateOrganizationError::NoFieldsToUpdate);
        }
        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                return Err(UpdateOrganizationError::NameEmpty);
            }
            if name.len() > 256 {
                return Err(UpdateOrganizationError::NameLength);
            }
        }
        if let Some(ref website) = self.website {
            if !website.is_empty() && !is_valid_url(website) {
                return Err(UpdateOrganizationError::WebsiteInvalid(website.clone()));
            }
        }
        if let Some(ref logo_url) = self.logo_url {
            if !logo_url.is_empty() && !is_valid_url(logo_url) {
                return Err(UpdateOrganizationError::LogoUrlInvalid(logo_url.clone()));
            }
        }
        Ok(())
    }
}

/// Handles the update organization command
///
/// Updates an existing organization with the provided fields. Fields that
/// are `None` are not changed (existing values are preserved).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `command` - The update command with fields to modify
///
/// # Returns
///
/// Returns the updated organization details on success.
///
/// # Errors
///
/// - Validation errors if command parameters are invalid
/// - `NotFound` - No organization with the given slug exists
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: UpdateOrganizationCommand,
) -> Result<UpdateOrganizationResponse, UpdateOrganizationError> {
    command.validate()?;

    let slug = &command.slug;
    let org = sqlx::query_as!(
        OrganizationRecord,
        r#"
        SELECT id, slug, name, website, description, logo_url,
               is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
        FROM organizations
        WHERE slug = $1
        "#,
        slug
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| UpdateOrganizationError::NotFound(command.slug.clone()))?;

    let new_name = command.name.as_ref().unwrap_or(&org.name);
    let new_website = command.website.as_ref().or(org.website.as_ref());
    let new_description = command.description.as_ref().or(org.description.as_ref());
    let new_logo_url = command.logo_url.as_ref().or(org.logo_url.as_ref());
    let new_is_system = command.is_system.unwrap_or(org.is_system);

    let result = sqlx::query_as!(
        OrganizationRecord,
        r#"
        UPDATE organizations
        SET name = $2, website = $3, description = $4, logo_url = $5, is_system = $6, updated_at = NOW()
        WHERE slug = $1
        RETURNING id, slug, name, website, description, logo_url,
                  is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        slug,
        new_name,
        new_website,
        new_description,
        new_logo_url,
        new_is_system
    )
    .fetch_one(&pool)
    .await?;

    Ok(UpdateOrganizationResponse {
        id: result.id,
        slug: result.slug,
        name: result.name,
        website: result.website,
        description: result.description,
        logo_url: result.logo_url,
        is_system: result.is_system,
        updated_at: result.updated_at,
    })
}

#[derive(Debug)]
#[allow(dead_code)]
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

fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = UpdateOrganizationCommand {
            slug: "test-org".to_string(),
            name: Some("Updated Name".to_string()),
            website: Some("https://example.com".to_string()),
            description: Some("Updated description".to_string()),
            logo_url: None,
            is_system: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_slug() {
        let cmd = UpdateOrganizationCommand {
            slug: "".to_string(),
            name: Some("Name".to_string()),
            website: None,
            description: None,
            logo_url: None,
            is_system: None,
        };
        assert!(matches!(cmd.validate(), Err(UpdateOrganizationError::SlugRequired)));
    }

    #[test]
    fn test_validation_no_fields() {
        let cmd = UpdateOrganizationCommand {
            slug: "test-org".to_string(),
            name: None,
            website: None,
            description: None,
            logo_url: None,
            is_system: None,
        };
        assert!(matches!(cmd.validate(), Err(UpdateOrganizationError::NoFieldsToUpdate)));
    }

    #[sqlx::test]
    async fn test_handle_updates_organization(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO organizations (slug, name, is_system) VALUES ($1, $2, $3)",
            "test-org",
            "Original Name",
            false
        )
        .execute(&pool)
        .await?;

        let cmd = UpdateOrganizationCommand {
            slug: "test-org".to_string(),
            name: Some("Updated Name".to_string()),
            website: Some("https://test.com".to_string()),
            description: None,
            logo_url: None,
            is_system: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        assert_eq!(response.name, "Updated Name");
        assert_eq!(response.website, Some("https://test.com".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = UpdateOrganizationCommand {
            slug: "nonexistent".to_string(),
            name: Some("Name".to_string()),
            website: None,
            description: None,
            logo_url: None,
            is_system: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(UpdateOrganizationError::NotFound(_))));
        Ok(())
    }
}
