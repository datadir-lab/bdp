//! Create organization command
//!
//! This module implements the command for creating new organizations with
//! comprehensive validation, error handling, and audit logging.

use crate::db::{organizations, DbError};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

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
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Validation errors for create organization command
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
    Database(#[from] DbError),
}

impl CreateOrganizationCommand {
    /// Validates the command parameters
    ///
    /// # Errors
    ///
    /// Returns a validation error if any field fails validation:
    /// - Slug must be 1-100 characters, lowercase letters, numbers, hyphens only
    /// - Name must be 1-256 characters
    /// - URLs must be valid if provided
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let command = CreateOrganizationCommand { /* ... */ };
    /// command.validate()?;
    /// ```
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

/// Handler for the CreateOrganization command
///
/// This implements the command handler pattern for creating organizations.
/// It provides validation, execution, and audit logging.
pub struct CreateOrganizationHandler<'a> {
    pool: &'a PgPool,
}

impl<'a> CreateOrganizationHandler<'a> {
    /// Creates a new handler instance
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Executes the create organization command
    ///
    /// # Arguments
    ///
    /// * `command` - The create organization command
    ///
    /// # Returns
    ///
    /// Returns the created organization details or an error.
    ///
    /// # Errors
    ///
    /// - Validation errors if command parameters are invalid
    /// - Database errors if the operation fails
    /// - Duplicate error if an organization with the same slug exists
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use bdp_server::features::organizations::commands::{
    ///     CreateOrganizationCommand, CreateOrganizationHandler
    /// };
    ///
    /// let handler = CreateOrganizationHandler::new(&pool);
    /// let command = CreateOrganizationCommand { /* ... */ };
    /// let response = handler.handle(command).await?;
    /// println!("Created organization: {} ({})", response.name, response.id);
    /// ```
    #[tracing::instrument(
        skip(self, command),
        fields(
            slug = %command.slug,
            name = %command.name,
            is_system = command.is_system
        )
    )]
    pub async fn handle(
        &self,
        command: CreateOrganizationCommand,
    ) -> Result<CreateOrganizationResponse, CreateOrganizationError> {
        // Validate command
        command.validate()?;

        tracing::info!("Creating organization");

        // Convert command to database parameters
        let params = organizations::CreateOrganizationParams {
            slug: command.slug.clone(),
            name: command.name,
            website: command.website,
            description: command.description,
            logo_url: command.logo_url,
            is_system: command.is_system,
        };

        // Execute database operation
        let org = organizations::create_organization(self.pool, params)
            .await
            .map_err(|e| match e {
                DbError::Duplicate(_) => {
                    CreateOrganizationError::DuplicateSlug(command.slug.clone())
                },
                other => CreateOrganizationError::Database(other),
            })?;

        tracing::info!(
            org_id = %org.id,
            org_slug = %org.slug,
            "Organization created successfully"
        );

        // Convert to response
        Ok(CreateOrganizationResponse {
            id: org.id,
            slug: org.slug,
            name: org.name,
            website: org.website,
            description: org.description,
            logo_url: org.logo_url,
            is_system: org.is_system,
            created_at: org.created_at,
        })
    }
}

/// Validates if a string is a valid URL
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_command() {
        let command = CreateOrganizationCommand {
            slug: "acme-corp".to_string(),
            name: "ACME Corporation".to_string(),
            website: Some("https://acme.com".to_string()),
            description: Some("Test description".to_string()),
            logo_url: None,
            is_system: false,
        };

        assert!(command.validate().is_ok());
    }

    #[test]
    fn test_empty_slug() {
        let command = CreateOrganizationCommand {
            slug: "".to_string(),
            name: "Test".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(command.validate(), Err(CreateOrganizationError::SlugRequired)));
    }

    #[test]
    fn test_slug_too_long() {
        let command = CreateOrganizationCommand {
            slug: "a".repeat(101),
            name: "Test".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(command.validate(), Err(CreateOrganizationError::SlugLength)));
    }

    #[test]
    fn test_invalid_slug_format() {
        let invalid_slugs = vec![
            "UPPERCASE",
            "has spaces",
            "has_underscore",
            "-starts-with-hyphen",
            "ends-with-hyphen-",
            "has@special",
        ];

        for slug in invalid_slugs {
            let command = CreateOrganizationCommand {
                slug: slug.to_string(),
                name: "Test".to_string(),
                website: None,
                description: None,
                logo_url: None,
                is_system: false,
            };

            assert!(
                matches!(command.validate(), Err(CreateOrganizationError::SlugFormat)),
                "Slug '{}' should be invalid",
                slug
            );
        }
    }

    #[test]
    fn test_valid_slug_formats() {
        let valid_slugs = vec!["acme", "acme-corp", "acme-corp-123", "a", "123", "my-org-2024"];

        for slug in valid_slugs {
            let command = CreateOrganizationCommand {
                slug: slug.to_string(),
                name: "Test".to_string(),
                website: None,
                description: None,
                logo_url: None,
                is_system: false,
            };

            assert!(command.validate().is_ok(), "Slug '{}' should be valid", slug);
        }
    }

    #[test]
    fn test_empty_name() {
        let command = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "   ".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(command.validate(), Err(CreateOrganizationError::NameRequired)));
    }

    #[test]
    fn test_name_too_long() {
        let command = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "a".repeat(257),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(command.validate(), Err(CreateOrganizationError::NameLength)));
    }

    #[test]
    fn test_invalid_website_url() {
        let command = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "Test".to_string(),
            website: Some("not-a-url".to_string()),
            description: None,
            logo_url: None,
            is_system: false,
        };

        assert!(matches!(command.validate(), Err(CreateOrganizationError::WebsiteInvalid(_))));
    }

    #[test]
    fn test_invalid_logo_url() {
        let command = CreateOrganizationCommand {
            slug: "test".to_string(),
            name: "Test".to_string(),
            website: None,
            description: None,
            logo_url: Some("not-a-url".to_string()),
            is_system: false,
        };

        assert!(matches!(command.validate(), Err(CreateOrganizationError::LogoUrlInvalid(_))));
    }

    #[test]
    fn test_is_valid_url() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://example.com"));
        assert!(is_valid_url("https://example.com/path?query=1"));

        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url("example.com"));
        assert!(!is_valid_url("not a url"));
    }
}
