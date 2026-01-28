//! Delete organization command
//!
//! Deletes an organization by slug. The deletion will fail if the organization
//! has associated registry entries (data sources or tools).

use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Command to delete an organization
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::organizations::commands::DeleteOrganizationCommand;
///
/// let command = DeleteOrganizationCommand {
///     slug: "old-org".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOrganizationCommand {
    /// URL-safe slug of the organization to delete
    pub slug: String,
}

/// Response from deleting an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOrganizationResponse {
    /// Slug of the deleted organization
    pub slug: String,
    /// Whether the deletion was successful
    pub deleted: bool,
}

/// Errors that can occur when deleting an organization
#[derive(Debug, thiserror::Error)]
pub enum DeleteOrganizationError {
    /// The slug parameter was empty
    #[error("Slug is required and cannot be empty")]
    SlugRequired,
    /// Organization with the given slug was not found
    #[error("Organization with slug '{0}' not found")]
    NotFound(String),
    /// Organization has associated data sources or tools and cannot be deleted
    #[error("Cannot delete organization '{0}': it has associated registry entries")]
    HasDependencies(String),
    /// A database error occurred
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<DeleteOrganizationResponse, DeleteOrganizationError>>
    for DeleteOrganizationCommand
{
}

impl crate::cqrs::middleware::Command for DeleteOrganizationCommand {}

impl DeleteOrganizationCommand {
    /// Validates the command parameters
    ///
    /// # Errors
    ///
    /// Returns `SlugRequired` if the slug is empty.
    pub fn validate(&self) -> Result<(), DeleteOrganizationError> {
        if self.slug.is_empty() {
            return Err(DeleteOrganizationError::SlugRequired);
        }
        Ok(())
    }
}

/// Handles the delete organization command
///
/// Deletes an organization by slug. The operation will fail with `HasDependencies`
/// if the organization has any associated registry entries (data sources or tools)
/// due to foreign key constraints.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `command` - The delete command with the slug to delete
///
/// # Returns
///
/// Returns confirmation of the deletion on success.
///
/// # Errors
///
/// - `SlugRequired` - Slug is empty
/// - `NotFound` - No organization with the given slug exists
/// - `HasDependencies` - Organization has associated entries and cannot be deleted
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: DeleteOrganizationCommand,
) -> Result<DeleteOrganizationResponse, DeleteOrganizationError> {
    command.validate()?;

    let result = sqlx::query!(
        r#"
        DELETE FROM organizations
        WHERE slug = $1
        RETURNING slug
        "#,
        command.slug
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_foreign_key_violation() {
                return DeleteOrganizationError::HasDependencies(command.slug.clone());
            }
        }
        DeleteOrganizationError::Database(e)
    })?;

    match result {
        Some(_) => Ok(DeleteOrganizationResponse {
            slug: command.slug,
            deleted: true,
        }),
        None => Err(DeleteOrganizationError::NotFound(command.slug)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = DeleteOrganizationCommand {
            slug: "valid-slug".to_string(),
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_slug() {
        let cmd = DeleteOrganizationCommand {
            slug: "".to_string(),
        };
        assert!(matches!(cmd.validate(), Err(DeleteOrganizationError::SlugRequired)));
    }

    #[sqlx::test]
    async fn test_handle_deletes_organization(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (slug, name, is_system)
            VALUES ($1, $2, $3)
            "#,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let cmd = DeleteOrganizationCommand {
            slug: "test-org".to_string(),
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        assert!(response.deleted);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = DeleteOrganizationCommand {
            slug: "nonexistent".to_string(),
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(DeleteOrganizationError::NotFound(_))));
        Ok(())
    }
}
