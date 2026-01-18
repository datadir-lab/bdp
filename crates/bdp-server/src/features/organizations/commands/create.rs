use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationCommand {
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationResponse {
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

#[derive(Debug, thiserror::Error)]
pub enum CreateOrganizationError {
    #[error("Slug is required and cannot be empty")]
    SlugRequired,
    #[error("Slug must be between 1 and 100 characters")]
    SlugLength,
    #[error("Slug can only contain lowercase letters, numbers, and hyphens")]
    SlugFormat,
    #[error("Name is required and cannot be empty")]
    NameRequired,
    #[error("Name must be between 1 and 256 characters")]
    NameLength,
    #[error("Website URL is invalid: {0}")]
    WebsiteInvalid(String),
    #[error("Logo URL is invalid: {0}")]
    LogoUrlInvalid(String),
    #[error("Organization with slug '{0}' already exists")]
    DuplicateSlug(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<CreateOrganizationResponse, CreateOrganizationError>>
    for CreateOrganizationCommand
{
}

impl crate::cqrs::middleware::Command for CreateOrganizationCommand {}

impl CreateOrganizationCommand {
    pub fn validate(&self) -> Result<(), CreateOrganizationError> {
        if self.slug.is_empty() {
            return Err(CreateOrganizationError::SlugRequired);
        }
        if self.slug.len() > 100 {
            return Err(CreateOrganizationError::SlugLength);
        }
        if !self
            .slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(CreateOrganizationError::SlugFormat);
        }
        if self.slug.starts_with('-') || self.slug.ends_with('-') {
            return Err(CreateOrganizationError::SlugFormat);
        }
        if self.name.trim().is_empty() {
            return Err(CreateOrganizationError::NameRequired);
        }
        if self.name.len() > 256 {
            return Err(CreateOrganizationError::NameLength);
        }
        if let Some(ref website) = self.website {
            if !website.is_empty() && !is_valid_url(website) {
                return Err(CreateOrganizationError::WebsiteInvalid(website.clone()));
            }
        }
        if let Some(ref logo_url) = self.logo_url {
            if !logo_url.is_empty() && !is_valid_url(logo_url) {
                return Err(CreateOrganizationError::LogoUrlInvalid(logo_url.clone()));
            }
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: CreateOrganizationCommand,
) -> Result<CreateOrganizationResponse, CreateOrganizationError> {
    command.validate()?;

    let result = sqlx::query_as!(
        OrganizationRecord,
        r#"
        INSERT INTO organizations (slug, name, website, description, logo_url, is_system)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, slug, name, website, description, logo_url,
                  is_system as "is_system!", created_at as "created_at!", updated_at as "updated_at!"
        "#,
        command.slug,
        command.name,
        command.website,
        command.description,
        command.logo_url,
        command.is_system
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return CreateOrganizationError::DuplicateSlug(command.slug.clone());
            }
        }
        CreateOrganizationError::Database(e)
    })?;

    Ok(CreateOrganizationResponse {
        id: result.id,
        slug: result.slug,
        name: result.name,
        website: result.website,
        description: result.description,
        logo_url: result.logo_url,
        is_system: result.is_system,
        created_at: result.created_at,
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

fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = CreateOrganizationCommand {
            slug: "valid-slug".to_string(),
            name: "Valid Name".to_string(),
            website: Some("https://example.com".to_string()),
            description: Some("Description".to_string()),
            logo_url: None,
            is_system: false,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_slug() {
        let cmd = CreateOrganizationCommand {
            slug: "".to_string(),
            name: "Name".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        assert!(matches!(cmd.validate(), Err(CreateOrganizationError::SlugRequired)));
    }

    #[sqlx::test]
    async fn test_handle_creates_organization(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test Org".to_string(),
            website: Some("https://test.com".to_string()),
            description: Some("Test".to_string()),
            logo_url: None,
            is_system: false,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_duplicate_slug(pool: PgPool) -> sqlx::Result<()> {
        let cmd1 = CreateOrganizationCommand {
            slug: "dup-org".to_string(),
            name: "First".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = CreateOrganizationCommand {
            slug: "dup-org".to_string(),
            name: "Second".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(matches!(result, Err(CreateOrganizationError::DuplicateSlug(_))));
        Ok(())
    }
}
