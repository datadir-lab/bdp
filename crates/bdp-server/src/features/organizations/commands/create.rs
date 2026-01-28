//! Create organization command
//!
//! This module implements the command for creating new organizations using
//! the mediator pattern with function-based handlers and inline SQL queries.
//!
//! # Architecture
//!
//! - Command: Pure data structure (no behavior except validation)
//! - Handler: Standalone async function with all business logic and DB operations
//! - SQL queries are inline in the handler for simplicity and compile-time verification

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::features::shared::validation::{
    validate_name, validate_slug, validate_url, NameValidationError, SlugValidationError,
    UrlValidationError,
};

/// Command to create a new organization
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::organizations::commands::CreateOrganizationCommand;
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
    #[error("Slug validation failed: {0}")]
    SlugValidation(#[from] SlugValidationError),

    #[error("Name validation failed: {0}")]
    NameValidation(#[from] NameValidationError),

    #[error("URL validation failed: {0}")]
    UrlValidation(#[from] UrlValidationError),

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
    /// Returns a validation error if any field fails validation:
    /// - Slug must be 1-100 characters, lowercase letters, numbers, hyphens only
    /// - Slug cannot start or end with a hyphen
    /// - Name must be 1-256 characters
    /// - URLs must be valid if provided
    #[tracing::instrument(skip(self), fields(slug = %self.slug, name = %self.name))]
    pub fn validate(&self) -> Result<(), CreateOrganizationError> {
        // Validate slug using shared utility
        validate_slug(&self.slug, 100)?;

        // Validate name using shared utility
        validate_name(&self.name, 256)?;

        // Validate website URL if provided
        if let Some(ref website) = self.website {
            validate_url(website, "website")?;
        }

        // Validate logo URL if provided
        if let Some(ref logo_url) = self.logo_url {
            validate_url(logo_url, "logo_url")?;
        }

        tracing::debug!("Command validation passed");
        Ok(())
    }
}

/// Handler function for creating organizations
///
/// This is a standalone async function that contains all business logic
/// and database operations. SQL queries are inline for compile-time verification.
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

    // Execute inline SQL query
    let result = sqlx::query_as!(
        OrganizationRecord,
        r#"
        INSERT INTO organizations (slug, name, website, description, logo_url, is_system)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, slug, name, website, description, logo_url,
                  is_system as "is_system!", created_at as "created_at!"
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
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganizationError::SlugValidation(_))
        ));
    }

    #[test]
    fn test_validation_slug_too_long() {
        let cmd = CreateOrganizationCommand {
            slug: "a".repeat(101),
            name: "Test".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganizationError::SlugValidation(_))
        ));
    }

    #[test]
    fn test_validation_invalid_slug_format() {
        let invalid_slugs = vec![
            "UPPERCASE",
            "has spaces",
            "has_underscore",
            "-starts-with-hyphen",
            "ends-with-hyphen-",
            "has@special",
        ];

        for slug in invalid_slugs {
            let cmd = CreateOrganizationCommand {
                slug: slug.to_string(),
                name: "Test".to_string(),
                website: None,
                description: None,
                logo_url: None,
                is_system: false,
            };
            assert!(
                matches!(cmd.validate(), Err(CreateOrganizationError::SlugValidation(_))),
                "Slug '{}' should be invalid",
                slug
            );
        }
    }

    #[test]
    fn test_validation_valid_slug_formats() {
        let valid_slugs = vec!["acme", "acme-corp", "acme-corp-123", "a", "123", "my-org-2024"];

        for slug in valid_slugs {
            let cmd = CreateOrganizationCommand {
                slug: slug.to_string(),
                name: "Test".to_string(),
                website: None,
                description: None,
                logo_url: None,
                is_system: false,
            };
            assert!(cmd.validate().is_ok(), "Slug '{}' should be valid", slug);
        }
    }

    #[test]
    fn test_validation_empty_name() {
        let cmd = CreateOrganizationCommand {
            slug: "valid-slug".to_string(),
            name: "   ".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganizationError::NameValidation(_))
        ));
    }

    #[test]
    fn test_validation_name_too_long() {
        let cmd = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "a".repeat(257),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganizationError::NameValidation(_))
        ));
    }

    #[test]
    fn test_validation_invalid_website_url() {
        let cmd = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "Test".to_string(),
            website: Some("not-a-url".to_string()),
            description: None,
            logo_url: None,
            is_system: false,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganizationError::UrlValidation(_))
        ));
    }

    #[test]
    fn test_validation_invalid_logo_url() {
        let cmd = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "Test".to_string(),
            website: None,
            description: None,
            logo_url: Some("not-a-url".to_string()),
            is_system: false,
        };
        assert!(matches!(
            cmd.validate(),
            Err(CreateOrganizationError::UrlValidation(_))
        ));
    }

    #[sqlx::test]
    async fn test_handle_creates_organization(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test Organization".to_string(),
            website: Some("https://test.com".to_string()),
            description: Some("Test description".to_string()),
            logo_url: None,
            is_system: false,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        assert_eq!(response.name, "Test Organization");
        assert_eq!(response.website, Some("https://test.com".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_duplicate_slug(pool: PgPool) -> sqlx::Result<()> {
        let cmd1 = CreateOrganizationCommand {
            slug: "dup-org".to_string(),
            name: "First Organization".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = CreateOrganizationCommand {
            slug: "dup-org".to_string(),
            name: "Second Organization".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(matches!(
            result,
            Err(CreateOrganizationError::DuplicateSlug(_))
        ));
        Ok(())
    }
}
