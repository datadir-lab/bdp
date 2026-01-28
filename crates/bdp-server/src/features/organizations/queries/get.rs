//! Get organization query
//!
//! Retrieves a single organization by slug or ID. Organizations represent
//! data publishers such as UniProt, NCBI, or custom user organizations.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Query to retrieve an organization by slug or ID
///
/// At least one of `slug` or `id` must be provided. If both are provided,
/// the slug takes precedence.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::organizations::queries::GetOrganizationQuery;
///
/// // Query by slug
/// let query = GetOrganizationQuery {
///     slug: Some("uniprot".to_string()),
///     id: None,
/// };
///
/// // Query by ID
/// let query = GetOrganizationQuery {
///     slug: None,
///     id: Some(org_id),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrganizationQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
}

/// Response containing full organization details
///
/// Includes all organization metadata including licensing, citation information,
/// versioning strategy, and contact details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrganizationResponse {
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
    // Licensing and citation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_url: Option<String>,
    // Versioning strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_description: Option<String>,
    // Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
    // Per-organization versioning strategy (JSONB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versioning_rules: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Errors that can occur when getting an organization
#[derive(Debug, thiserror::Error)]
pub enum GetOrganizationError {
    /// Neither slug nor id was provided
    #[error("Either slug or id must be provided to look up an organization")]
    SlugOrIdRequired,
    /// The organization was not found in the database
    #[error("Organization '{identifier}' not found. Use the organizations list endpoint to see available organizations.")]
    NotFound { identifier: String },
    /// A database error occurred
    #[error("Failed to retrieve organization: {0}")]
    Database(#[from] sqlx::Error),
}

impl GetOrganizationError {
    /// Create a not found error with the identifier used in the query
    pub fn not_found(identifier: impl Into<String>) -> Self {
        Self::NotFound {
            identifier: identifier.into(),
        }
    }
}

impl Request<Result<GetOrganizationResponse, GetOrganizationError>> for GetOrganizationQuery {}

impl crate::cqrs::middleware::Query for GetOrganizationQuery {}

impl GetOrganizationQuery {
    /// Validates the query parameters
    ///
    /// # Errors
    ///
    /// Returns `SlugOrIdRequired` if neither `slug` nor `id` is provided.
    pub fn validate(&self) -> Result<(), GetOrganizationError> {
        if self.slug.is_none() && self.id.is_none() {
            return Err(GetOrganizationError::SlugOrIdRequired);
        }
        Ok(())
    }
}

/// Handles the get organization query
///
/// Retrieves organization details by slug or ID from the database.
/// The slug lookup is case-insensitive.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - The query containing slug or ID to look up
///
/// # Returns
///
/// Returns the organization details on success.
///
/// # Errors
///
/// - `SlugOrIdRequired` - Neither slug nor ID was provided
/// - `NotFound` - No organization matches the given slug or ID
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetOrganizationQuery,
) -> Result<GetOrganizationResponse, GetOrganizationError> {
    query.validate()?;

    let (result, identifier) = if let Some(ref slug) = query.slug {
        let record = sqlx::query_as!(
            OrganizationRecord,
            r#"
            SELECT id, slug, name, website, description, logo_url,
                   is_system as "is_system!",
                   license, license_url, citation, citation_url,
                   version_strategy, version_description,
                   data_source_url, documentation_url, contact_email,
                   versioning_rules,
                   created_at as "created_at!", updated_at as "updated_at!"
            FROM organizations
            WHERE LOWER(slug) = LOWER($1)
            "#,
            slug
        )
        .fetch_optional(&pool)
        .await?;
        (record, slug.clone())
    } else if let Some(id) = query.id {
        let record = sqlx::query_as!(
            OrganizationRecord,
            r#"
            SELECT id, slug, name, website, description, logo_url,
                   is_system as "is_system!",
                   license, license_url, citation, citation_url,
                   version_strategy, version_description,
                   data_source_url, documentation_url, contact_email,
                   versioning_rules,
                   created_at as "created_at!", updated_at as "updated_at!"
            FROM organizations
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&pool)
        .await?;
        (record, id.to_string())
    } else {
        (None, String::new())
    };

    let org = result.ok_or_else(|| GetOrganizationError::not_found(&identifier))?;

    Ok(GetOrganizationResponse {
        id: org.id,
        slug: org.slug,
        name: org.name,
        website: org.website,
        description: org.description,
        logo_url: org.logo_url,
        is_system: org.is_system,
        license: org.license,
        license_url: org.license_url,
        citation: org.citation,
        citation_url: org.citation_url,
        version_strategy: org.version_strategy,
        version_description: org.version_description,
        data_source_url: org.data_source_url,
        documentation_url: org.documentation_url,
        contact_email: org.contact_email,
        versioning_rules: org.versioning_rules,
        created_at: org.created_at,
        updated_at: org.updated_at,
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
    license: Option<String>,
    license_url: Option<String>,
    citation: Option<String>,
    citation_url: Option<String>,
    version_strategy: Option<String>,
    version_description: Option<String>,
    data_source_url: Option<String>,
    documentation_url: Option<String>,
    contact_email: Option<String>,
    versioning_rules: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success_with_slug() {
        let query = GetOrganizationQuery {
            slug: Some("test-org".to_string()),
            id: None,
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_success_with_id() {
        let query = GetOrganizationQuery {
            slug: None,
            id: Some(Uuid::new_v4()),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_failure_no_slug_or_id() {
        let query = GetOrganizationQuery {
            slug: None,
            id: None,
        };
        assert!(matches!(
            query.validate(),
            Err(GetOrganizationError::SlugOrIdRequired)
        ));
    }

    #[sqlx::test]
    async fn test_handle_get_by_slug(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::new_v4(),
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let query = GetOrganizationQuery {
            slug: Some("test-org".to_string()),
            id: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");
        assert_eq!(response.name, "Test Org");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_get_by_id(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org_id,
            "test-org-2",
            "Test Org 2",
            false
        )
        .execute(&pool)
        .await?;

        let query = GetOrganizationQuery {
            slug: None,
            id: Some(org_id),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, org_id);
        assert_eq!(response.slug, "test-org-2");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = GetOrganizationQuery {
            slug: Some("nonexistent".to_string()),
            id: None,
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(result, Err(GetOrganizationError::NotFound { .. })));
        Ok(())
    }
}
