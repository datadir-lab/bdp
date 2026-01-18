//! Create organization command
//!
//! This module implements the command for creating new organizations using
//! the mediator pattern with function-based handlers and inline SQL queries.
//!
//! # Architecture
//!
//! - Command: Pure data structure (no behavior)
//! - Handler: Standalone async function with all business logic and DB operations
//! - No shared DB layer: SQL queries are inline in the handler

use chrono::{DateTime, Utc};
use mediator::{Request, RequestHandler};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Command to create a new organization
///
/// This is a pure data structure with no behavior except validation helpers.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::organizations::CreateOrganizationCommand;
///
/// let command = CreateOrganizationCommand {
///     slug: "acme-corp".to_string(),
///     name: "ACME Corporation".to_string(),
///     website: Some("https://acme.com".to_string()),
///     description: Some("Leading provider of quality products".to_string()),
///     logo_url: None,
///     is_system: false,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationCommand {
    /// URL-safe slug (must be unique)
    pub slug: String,

    /// Display name of the organization
    pub name: String,

    /// Optional website URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional logo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,

    /// Whether this is a system organization
    #[serde(default)]
    pub is_system: bool,
}

/// Response from creating an organization
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

/// Errors that can occur when creating an organization
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

// Implement mediator Request trait for the command
impl Request<Result<CreateOrganizationResponse, CreateOrganizationError>>
    for CreateOrganizationCommand
{
}

// Mark as Command for CQRS middleware
impl crate::cqrs::middleware::Command for CreateOrganizationCommand {}

impl CreateOrganizationCommand {
    /// Validates the command parameters
    ///
    /// # Errors
    ///
    /// Returns a validation error if any field fails validation
    #[tracing::instrument(skip(self), fields(slug = %self.slug, name = %self.name))]
    pub fn validate(&self) -> Result<(), CreateOrganizationError> {
        // Validate slug
        if self.slug.is_empty() {
            return Err(CreateOrganizationError::SlugRequired);
        }

        if self.slug.len() > 100 {
            return Err(CreateOrganizationError::SlugLength);
        }

        // Slug must be lowercase alphanumeric with hyphens
        if !self
            .slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(CreateOrganizationError::SlugFormat);
        }

        // Slug should not start or end with hyphen
        if self.slug.starts_with('-') || self.slug.ends_with('-') {
            return Err(CreateOrganizationError::SlugFormat);
        }

        // Validate name
        if self.name.trim().is_empty() {
            return Err(CreateOrganizationError::NameRequired);
        }

        if self.name.len() > 256 {
            return Err(CreateOrganizationError::NameLength);
        }

        // Validate website URL if provided
        if let Some(ref website) = self.website {
            if !website.is_empty() && !is_valid_url(website) {
                return Err(CreateOrganizationError::WebsiteInvalid(website.clone()));
            }
        }

        // Validate logo URL if provided
        if let Some(ref logo_url) = self.logo_url {
            if !logo_url.is_empty() && !is_valid_url(logo_url) {
                return Err(CreateOrganizationError::LogoUrlInvalid(logo_url.clone()));
            }
        }

        tracing::debug!("Command validation passed");
        Ok(())
    }
}

/// Handler function for creating organizations
///
/// This is a standalone async function that contains all business logic
/// and database operations. No shared DB layer is used - SQL is inline.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `command` - The create organization command
///
/// # Returns
///
/// Returns the created organization details or an error
///
/// # Errors
///
/// - Validation errors if command parameters are invalid
/// - Database errors if the operation fails
/// - Duplicate error if an organization with the same slug exists
#[tracing::instrument(
    skip(pool, command),
    fields(
        slug = %command.slug,
        name = %command.name,
        is_system = command.is_system
    )
)]
pub async fn handle(
    pool: PgPool,
    command: CreateOrganizationCommand,
) -> Result<CreateOrganizationResponse, CreateOrganizationError> {
    // Validate command
    command.validate()?;

    tracing::info!("Creating organization");

    // Execute inline SQL query - no shared DB layer
    let result = sqlx::query_as!(
        OrganizationRecord,
        r#"
        INSERT INTO organizations (slug, name, website, description, logo_url, is_system)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, slug, name, website, description, logo_url, is_system, created_at, updated_at
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
        // Check for unique constraint violation
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return CreateOrganizationError::DuplicateSlug(command.slug.clone());
            }
        }
        CreateOrganizationError::Database(e)
    })?;

    tracing::info!(
        org_id = %result.id,
        org_slug = %result.slug,
        "Organization created successfully"
    );

    // Convert database record to response
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

// Database record structure for sqlx query
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
    #[allow(dead_code)]
    updated_at: DateTime<Utc>,
}

/// Basic URL validation helper
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_validation_success() {
        let command = CreateOrganizationCommand {
            slug: "valid-slug".to_string(),
            name: "Valid Name".to_string(),
            website: Some("https://example.com".to_string()),
            description: Some("Description".to_string()),
            logo_url: None,
            is_system: false,
        };

        assert!(command.validate().is_ok());
    }

    #[test]
    fn test_command_validation_empty_slug() {
        let command = CreateOrganizationCommand {
            slug: "".to_string(),
            name: "Valid Name".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(
            command.validate(),
            Err(CreateOrganizationError::SlugRequired)
        ));
    }

    #[test]
    fn test_command_validation_invalid_slug_format() {
        let command = CreateOrganizationCommand {
            slug: "Invalid_Slug".to_string(),
            name: "Valid Name".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(
            command.validate(),
            Err(CreateOrganizationError::SlugFormat)
        ));
    }

    #[test]
    fn test_command_validation_slug_starts_with_hyphen() {
        let command = CreateOrganizationCommand {
            slug: "-invalid".to_string(),
            name: "Valid Name".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(
            command.validate(),
            Err(CreateOrganizationError::SlugFormat)
        ));
    }

    #[test]
    fn test_command_validation_empty_name() {
        let command = CreateOrganizationCommand {
            slug: "valid-slug".to_string(),
            name: "   ".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(
            command.validate(),
            Err(CreateOrganizationError::NameRequired)
        ));
    }

    #[test]
    fn test_command_validation_invalid_website() {
        let command = CreateOrganizationCommand {
            slug: "valid-slug".to_string(),
            name: "Valid Name".to_string(),
            website: Some("not-a-url".to_string()),
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(
            command.validate(),
            Err(CreateOrganizationError::WebsiteInvalid(_))
        ));
    }

    #[sqlx::test]
    async fn test_handle_creates_organization(pool: PgPool) -> sqlx::Result<()> {
        let command = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test Organization".to_string(),
            website: Some("https://test.com".to_string()),
            description: Some("Test description".to_string()),
            logo_url: None,
            is_system: false,
        };

        let result = handle(pool.clone(), command).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        assert_eq!(response.name, "Test Organization");
        assert_eq!(response.website, Some("https://test.com".to_string()));

        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_duplicate_slug_error(pool: PgPool) -> sqlx::Result<()> {
        let command1 = CreateOrganizationCommand {
            slug: "duplicate-org".to_string(),
            name: "First Organization".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        // Create first organization
        let _ = handle(pool.clone(), command1).await.unwrap();

        // Try to create with same slug
        let command2 = CreateOrganizationCommand {
            slug: "duplicate-org".to_string(),
            name: "Second Organization".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        let result = handle(pool.clone(), command2).await;

        assert!(matches!(
            result,
            Err(CreateOrganizationError::DuplicateSlug(_))
        ));

        Ok(())
    }
}
