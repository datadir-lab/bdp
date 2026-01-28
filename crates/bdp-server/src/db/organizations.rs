//! Database operations for organizations.
//!
//! This module provides comprehensive CRUD operations for the organizations table,
//! implementing best practices with SQLx including:
//!
//! - Type-safe queries using `query_as!` and `query!` macros
//! - Compile-time SQL verification
//! - Comprehensive error handling with custom DbError types
//! - Full-text search capabilities
//! - Statistics and analytics functions
//! - Pagination support
//! - Transaction support
//! - Extensive logging
//!
//! # Architecture
//!
//! The organizations table is the top-level namespace in the BDP registry.
//! Organizations own registry entries (data sources and tools) and provide
//! multi-tenancy support.
//!
//! # Schema Reference
//!
//! ```sql
//! CREATE TABLE organizations (
//!     id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
//!     slug VARCHAR(100) UNIQUE NOT NULL,
//!     name VARCHAR(256) NOT NULL,
//!     website TEXT,
//!     description TEXT,
//!     logo_url TEXT,
//!     is_system BOOLEAN DEFAULT FALSE,
//!     created_at TIMESTAMPTZ DEFAULT NOW(),
//!     updated_at TIMESTAMPTZ DEFAULT NOW()
//! );
//! ```
//!
//! # Examples
//!
//! ## Basic CRUD Operations
//!
//! ```rust,ignore
//! use bdp_server::db::{create_pool, organizations, DbConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DbConfig::from_env()?;
//!     let pool = create_pool(&config).await?;
//!
//!     // Create a new organization
//!     let org = organizations::create_organization(
//!         &pool,
//!         organizations::CreateOrganizationParams {
//!             slug: "acme-corp".to_string(),
//!             name: "ACME Corporation".to_string(),
//!             website: Some("https://acme.com".to_string()),
//!             description: Some("Leading provider of quality products".to_string()),
//!             logo_url: None,
//!             is_system: false,
//!         },
//!     ).await?;
//!
//!     // Get organization by slug
//!     let org = organizations::get_organization_by_slug(&pool, "acme-corp").await?;
//!
//!     // Update organization
//!     let updated = organizations::update_organization(
//!         &pool,
//!         "acme-corp",
//!         organizations::UpdateOrganizationParams {
//!             name: Some("ACME Corp".to_string()),
//!             website: None,
//!             description: Some("Updated description".to_string()),
//!             logo_url: None,
//!             is_system: None,
//!         },
//!     ).await?;
//!
//!     // Search organizations
//!     let results = organizations::search_organizations(
//!         &pool,
//!         "ACME",
//!         Default::default(),
//!     ).await?;
//!
//!     // Get statistics
//!     let stats = organizations::get_organization_statistics(&pool, org.id).await?;
//!
//!     // Delete organization
//!     organizations::delete_organization(&pool, "acme-corp").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Advanced Queries
//!
//! ```rust,ignore
//! use bdp_server::db::organizations::{ListOrganizationsFilter, search_organizations};
//! use bdp_common::types::Pagination;
//!
//! // List with filters
//! let system_orgs = list_organizations(
//!     &pool,
//!     Some(ListOrganizationsFilter {
//!         is_system: Some(true),
//!         name_contains: None,
//!     }),
//!     Pagination::new(20, 0),
//! ).await?;
//!
//! // Full-text search
//! let results = search_organizations(&pool, "protein database", Pagination::default()).await?;
//!
//! // Get statistics for all organizations
//! for org in system_orgs {
//!     let stats = get_organization_statistics(&pool, org.id).await?;
//!     println!("{}: {} entries, {} downloads", org.name, stats.total_entries, stats.total_downloads);
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{DbError, DbResult};

// ============================================================================
// Types
// ============================================================================

/// Represents an organization in the database.
///
/// This is the main organization type returned by query operations.
/// It includes all fields from the organizations table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Organization {
    /// Unique identifier for the organization
    pub id: Uuid,

    /// URL-safe slug used in API paths (e.g., "uniprot", "ncbi")
    pub slug: String,

    /// Display name of the organization
    pub name: String,

    /// Organization website URL
    pub website: Option<String>,

    /// Description of the organization
    pub description: Option<String>,

    /// URL to the organization's logo image
    pub logo_url: Option<String>,

    /// Whether this is a system organization (hardcoded, has scrapers)
    pub is_system: bool,

    /// License type (e.g., "CC-BY-4.0", "MIT", "Custom")
    pub license: Option<String>,

    /// Link to full license text
    pub license_url: Option<String>,

    /// How to cite this organization's data
    pub citation: Option<String>,

    /// Link to citation guidelines
    pub citation_url: Option<String>,

    /// Versioning approach (e.g., "semantic", "date-based", "release-based")
    pub version_strategy: Option<String>,

    /// Description of how versions are managed
    pub version_description: Option<String>,

    /// Link to the original data source
    pub data_source_url: Option<String>,

    /// Link to documentation
    pub documentation_url: Option<String>,

    /// Contact email for questions
    pub contact_email: Option<String>,

    /// Per-organization versioning strategy defining MAJOR vs MINOR bump rules
    /// Stored as JSONB in the database
    pub versioning_strategy: Option<serde_json::Value>,

    /// Timestamp when the organization was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the organization was last updated
    pub updated_at: DateTime<Utc>,
}

/// Parameters for creating a new organization.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::CreateOrganizationParams;
///
/// let params = CreateOrganizationParams {
///     slug: "uniprot".to_string(),
///     name: "Universal Protein Resource".to_string(),
///     website: Some("https://www.uniprot.org".to_string()),
///     description: Some("Comprehensive protein database".to_string()),
///     logo_url: None,
///     is_system: true,
///     license: Some("CC-BY-4.0".to_string()),
///     license_url: Some("https://creativecommons.org/licenses/by/4.0/".to_string()),
///     citation: Some("Cite UniProt...".to_string()),
///     citation_url: Some("https://www.uniprot.org/help/publications".to_string()),
///     version_strategy: Some("date-based".to_string()),
///     version_description: Some("Releases follow YYYY_MM format".to_string()),
///     data_source_url: Some("https://ftp.uniprot.org/".to_string()),
///     documentation_url: Some("https://www.uniprot.org/help".to_string()),
///     contact_email: Some("help@uniprot.org".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationParams {
    /// URL-safe slug (must be unique)
    pub slug: String,

    /// Display name
    pub name: String,

    /// Optional website URL
    pub website: Option<String>,

    /// Optional description
    pub description: Option<String>,

    /// Optional logo URL
    pub logo_url: Option<String>,

    /// Whether this is a system organization
    pub is_system: bool,

    /// Optional license type
    pub license: Option<String>,

    /// Optional license URL
    pub license_url: Option<String>,

    /// Optional citation information
    pub citation: Option<String>,

    /// Optional citation URL
    pub citation_url: Option<String>,

    /// Optional version strategy
    pub version_strategy: Option<String>,

    /// Optional version description
    pub version_description: Option<String>,

    /// Optional data source URL
    pub data_source_url: Option<String>,

    /// Optional documentation URL
    pub documentation_url: Option<String>,

    /// Optional contact email
    pub contact_email: Option<String>,

    /// Optional versioning strategy (JSONB)
    pub versioning_strategy: Option<serde_json::Value>,
}

/// Parameters for updating an organization.
///
/// All fields are optional. Only provided fields will be updated.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::UpdateOrganizationParams;
///
/// // Update only the name
/// let params = UpdateOrganizationParams {
///     name: Some("New Name".to_string()),
///     website: None,
///     description: None,
///     logo_url: None,
///     is_system: None,
///     license: None,
///     license_url: None,
///     citation: None,
///     citation_url: None,
///     version_strategy: None,
///     version_description: None,
///     data_source_url: None,
///     documentation_url: None,
///     contact_email: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateOrganizationParams {
    /// Optional new name
    pub name: Option<String>,

    /// Optional new website (use Some(None) to clear)
    pub website: Option<String>,

    /// Optional new description (use Some(None) to clear)
    pub description: Option<String>,

    /// Optional new logo URL (use Some(None) to clear)
    pub logo_url: Option<String>,

    /// Optional new is_system value
    pub is_system: Option<bool>,

    /// Optional new license
    pub license: Option<String>,

    /// Optional new license URL
    pub license_url: Option<String>,

    /// Optional new citation
    pub citation: Option<String>,

    /// Optional new citation URL
    pub citation_url: Option<String>,

    /// Optional new version strategy
    pub version_strategy: Option<String>,

    /// Optional new version description
    pub version_description: Option<String>,

    /// Optional new data source URL
    pub data_source_url: Option<String>,

    /// Optional new documentation URL
    pub documentation_url: Option<String>,

    /// Optional new contact email
    pub contact_email: Option<String>,

    /// Optional new versioning strategy
    pub versioning_strategy: Option<serde_json::Value>,
}

/// Filter parameters for listing organizations.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::ListOrganizationsFilter;
///
/// // Filter for system organizations with "protein" in the name
/// let filter = ListOrganizationsFilter {
///     is_system: Some(true),
///     name_contains: Some("protein".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListOrganizationsFilter {
    /// Filter by is_system flag
    pub is_system: Option<bool>,

    /// Filter by name (case-insensitive partial match)
    pub name_contains: Option<String>,
}

/// Statistics for an organization.
///
/// Provides aggregated metrics about an organization's content and usage.
///
/// # Examples
///
/// ```rust,ignore
/// let stats = get_organization_statistics(&pool, org_id).await?;
/// println!("Total entries: {}", stats.total_entries);
/// println!("Total versions: {}", stats.total_versions);
/// println!("Total size: {} bytes", stats.total_size_bytes);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationStatistics {
    /// Organization ID
    pub organization_id: Uuid,

    /// Total number of registry entries (data sources + tools)
    pub total_entries: i64,

    /// Number of data source entries
    pub data_source_count: i64,

    /// Number of tool entries
    pub tool_count: i64,

    /// Total number of versions across all entries
    pub total_versions: i64,

    /// Total size of all version files (in bytes)
    pub total_size_bytes: i64,

    /// Total number of downloads
    pub total_downloads: i64,

    /// Most recent version release date
    pub latest_release_date: Option<DateTime<Utc>>,
}

// ============================================================================
// Pagination Helper
// ============================================================================

/// Pagination parameters for list queries.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::Pagination;
///
/// let pagination = Pagination {
///     limit: 20,
///     offset: 0,
/// };
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Pagination {
    /// Maximum number of items to return
    pub limit: i64,

    /// Number of items to skip
    pub offset: i64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}

impl Pagination {
    /// Creates a new pagination instance with custom values.
    pub fn new(limit: i64, offset: i64) -> Self {
        Self { limit, offset }
    }

    /// Creates pagination for a specific page with a given page size.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let page_2 = Pagination::page(2, 20); // page 2, 20 items per page
    /// assert_eq!(page_2.offset, 40);
    /// assert_eq!(page_2.limit, 20);
    /// ```
    pub fn page(page: i64, page_size: i64) -> Self {
        Self {
            limit: page_size,
            offset: page * page_size,
        }
    }
}

// ============================================================================
// Create Operations
// ============================================================================

/// Creates a new organization with duplicate checking.
///
/// This function validates the slug for uniqueness before insertion. If an
/// organization with the same slug already exists, it returns a `DbError::Duplicate`.
///
/// The function automatically:
/// - Generates a new UUID for the organization
/// - Sets created_at and updated_at timestamps
/// - Logs the creation event
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `params` - Organization creation parameters
///
/// # Errors
///
/// Returns:
/// - `DbError::Duplicate` if an organization with the same slug exists
/// - `DbError::Sqlx` for other database errors
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::{create_organization, CreateOrganizationParams};
///
/// let org = create_organization(
///     &pool,
///     CreateOrganizationParams {
///         slug: "uniprot".to_string(),
///         name: "Universal Protein Resource".to_string(),
///         website: Some("https://www.uniprot.org".to_string()),
///         description: Some("Comprehensive protein database".to_string()),
///         logo_url: None,
///         is_system: true,
///     },
/// ).await?;
///
/// println!("Created organization with ID: {}", org.id);
/// ```
pub async fn create_organization(
    pool: &PgPool,
    params: CreateOrganizationParams,
) -> DbResult<Organization> {
    // Check for duplicate slug
    if get_organization_by_slug(pool, &params.slug).await.is_ok() {
        return Err(DbError::Duplicate(format!(
            "Organization with slug '{}' already exists",
            params.slug
        )));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    let org = sqlx::query_as!(
        Organization,
        r#"
        INSERT INTO organizations (
            id, slug, name, website, description, logo_url, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email,
            versioning_strategy,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        RETURNING
            id, slug, name, website, description, logo_url, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email,
            versioning_strategy,
            created_at, updated_at
        "#,
        id,
        params.slug,
        params.name,
        params.website,
        params.description,
        params.logo_url,
        params.is_system,
        params.license,
        params.license_url,
        params.citation,
        params.citation_url,
        params.version_strategy,
        params.version_description,
        params.data_source_url,
        params.documentation_url,
        params.contact_email,
        params.versioning_strategy,
        now,
        now
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        // Handle unique constraint violations
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return DbError::Duplicate(format!(
                    "Organization with slug '{}' already exists",
                    params.slug
                ));
            }
        }
        DbError::from(e)
    })?;

    tracing::info!(
        org_id = %org.id,
        org_slug = %org.slug,
        is_system = org.is_system,
        "Created new organization"
    );

    Ok(org)
}

// ============================================================================
// Query Operations
// ============================================================================

/// Retrieves an organization by its slug.
///
/// This is the primary method for looking up organizations by their
/// URL-friendly identifier.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `slug` - URL-safe slug of the organization
///
/// # Errors
///
/// Returns:
/// - `DbError::NotFound` if the organization doesn't exist
/// - `DbError::Sqlx` for other database errors
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::get_organization_by_slug;
///
/// let org = get_organization_by_slug(&pool, "uniprot").await?;
/// println!("Found organization: {} ({})", org.name, org.slug);
/// ```
pub async fn get_organization_by_slug(pool: &PgPool, slug: &str) -> DbResult<Organization> {
    let org = sqlx::query_as!(
        Organization,
        r#"
        SELECT
            id, slug, name, website, description, logo_url, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email,
            versioning_strategy,
            created_at, updated_at
        FROM organizations
        WHERE slug = $1
        "#,
        slug
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Organization '{}' not found", slug)))?;

    tracing::debug!(org_id = %org.id, org_slug = %org.slug, "Retrieved organization by slug");

    Ok(org)
}

/// Retrieves an organization by its UUID.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the organization
///
/// # Errors
///
/// Returns:
/// - `DbError::NotFound` if the organization doesn't exist
/// - `DbError::Sqlx` for other database errors
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::get_organization_by_id;
/// use uuid::Uuid;
///
/// let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
/// let org = get_organization_by_id(&pool, id).await?;
/// println!("Found organization: {}", org.name);
/// ```
pub async fn get_organization_by_id(pool: &PgPool, id: Uuid) -> DbResult<Organization> {
    let org = sqlx::query_as!(
        Organization,
        r#"
        SELECT
            id, slug, name, website, description, logo_url, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email,
            versioning_strategy,
            created_at, updated_at
        FROM organizations
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Organization with id '{}' not found", id)))?;

    tracing::debug!(org_id = %org.id, org_slug = %org.slug, "Retrieved organization by ID");

    Ok(org)
}

/// Lists organizations with pagination and optional filtering.
///
/// This function supports filtering by is_system flag and name matching.
/// Results are ordered by creation date (newest first).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `filter` - Optional filter parameters
/// * `pagination` - Pagination parameters (limit and offset)
///
/// # Errors
///
/// Returns `DbError::Sqlx` for database errors.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::{list_organizations, ListOrganizationsFilter, Pagination};
///
/// // List all organizations
/// let all_orgs = list_organizations(&pool, None, Pagination::default()).await?;
///
/// // List only system organizations
/// let system_orgs = list_organizations(
///     &pool,
///     Some(ListOrganizationsFilter {
///         is_system: Some(true),
///         name_contains: None,
///     }),
///     Pagination::default(),
/// ).await?;
///
/// // Search by name with pagination
/// let page_2 = list_organizations(
///     &pool,
///     Some(ListOrganizationsFilter {
///         is_system: None,
///         name_contains: Some("protein".to_string()),
///     }),
///     Pagination::page(1, 20),
/// ).await?;
/// ```
pub async fn list_organizations(
    pool: &PgPool,
    filter: Option<ListOrganizationsFilter>,
    pagination: Pagination,
) -> DbResult<Vec<Organization>> {
    let filter = filter.unwrap_or_default();

    // Build dynamic query based on filters
    let orgs = if let Some(is_system) = filter.is_system {
        if let Some(name_pattern) = filter.name_contains {
            let pattern = format!("%{}%", name_pattern);
            sqlx::query_as!(
                Organization,
                r#"
                SELECT
                    id, slug, name, website, description, logo_url, is_system,
                    license, license_url, citation, citation_url,
                    version_strategy, version_description,
                    data_source_url, documentation_url, contact_email,
                    versioning_strategy,
                    created_at, updated_at
                FROM organizations
                WHERE is_system = $1 AND name ILIKE $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
                is_system,
                pattern,
                pagination.limit,
                pagination.offset
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                Organization,
                r#"
                SELECT
                    id, slug, name, website, description, logo_url, is_system,
                    license, license_url, citation, citation_url,
                    version_strategy, version_description,
                    data_source_url, documentation_url, contact_email,
                    versioning_strategy,
                    created_at, updated_at
                FROM organizations
                WHERE is_system = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
                is_system,
                pagination.limit,
                pagination.offset
            )
            .fetch_all(pool)
            .await?
        }
    } else if let Some(name_pattern) = filter.name_contains {
        let pattern = format!("%{}%", name_pattern);
        sqlx::query_as!(
            Organization,
            r#"
            SELECT
                id, slug, name, website, description, logo_url, is_system,
                license, license_url, citation, citation_url,
                version_strategy, version_description,
                data_source_url, documentation_url, contact_email,
                versioning_strategy,
                created_at, updated_at
            FROM organizations
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            pattern,
            pagination.limit,
            pagination.offset
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            Organization,
            r#"
            SELECT
                id, slug, name, website, description, logo_url, is_system,
                license, license_url, citation, citation_url,
                version_strategy, version_description,
                data_source_url, documentation_url, contact_email,
                versioning_strategy,
                created_at, updated_at
            FROM organizations
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            pagination.limit,
            pagination.offset
        )
        .fetch_all(pool)
        .await?
    };

    tracing::debug!(
        count = orgs.len(),
        limit = pagination.limit,
        offset = pagination.offset,
        "Listed organizations"
    );

    Ok(orgs)
}

/// Counts the total number of organizations matching the filter criteria.
///
/// This is useful for pagination to calculate the total number of pages.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `filter` - Optional filter parameters
///
/// # Errors
///
/// Returns `DbError::Sqlx` for database errors.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::{count_organizations, ListOrganizationsFilter};
///
/// // Count all organizations
/// let total = count_organizations(&pool, None).await?;
/// let total_pages = (total as f64 / 50.0).ceil() as i64;
///
/// // Count system organizations
/// let system_count = count_organizations(
///     &pool,
///     Some(ListOrganizationsFilter {
///         is_system: Some(true),
///         name_contains: None,
///     }),
/// ).await?;
/// ```
pub async fn count_organizations(
    pool: &PgPool,
    filter: Option<ListOrganizationsFilter>,
) -> DbResult<i64> {
    let filter = filter.unwrap_or_default();

    let count = if let Some(is_system) = filter.is_system {
        if let Some(name_pattern) = filter.name_contains {
            let pattern = format!("%{}%", name_pattern);
            sqlx::query!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM organizations
                WHERE is_system = $1 AND name ILIKE $2
                "#,
                is_system,
                pattern
            )
            .fetch_one(pool)
            .await?
            .count
        } else {
            sqlx::query!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM organizations
                WHERE is_system = $1
                "#,
                is_system
            )
            .fetch_one(pool)
            .await?
            .count
        }
    } else if let Some(name_pattern) = filter.name_contains {
        let pattern = format!("%{}%", name_pattern);
        sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM organizations
            WHERE name ILIKE $1
            "#,
            pattern
        )
        .fetch_one(pool)
        .await?
        .count
    } else {
        sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM organizations
            "#
        )
        .fetch_one(pool)
        .await?
        .count
    };

    Ok(count)
}

// ============================================================================
// Update Operations
// ============================================================================

/// Updates an existing organization with partial field updates.
///
/// Only the fields provided in `params` will be updated. Fields set to `None`
/// in the params will retain their current values. The updated_at timestamp
/// is automatically updated.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `slug` - URL-safe slug of the organization to update
/// * `params` - Update parameters (only provided fields are updated)
///
/// # Errors
///
/// Returns:
/// - `DbError::NotFound` if the organization doesn't exist
/// - `DbError::Sqlx` for other database errors
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::{update_organization, UpdateOrganizationParams};
///
/// // Update only the name
/// let org = update_organization(
///     &pool,
///     "uniprot",
///     UpdateOrganizationParams {
///         name: Some("UniProt".to_string()),
///         ..Default::default()
///     },
/// ).await?;
///
/// // Update multiple fields
/// let org = update_organization(
///     &pool,
///     "uniprot",
///     UpdateOrganizationParams {
///         name: Some("UniProt".to_string()),
///         description: Some("Updated description".to_string()),
///         website: Some("https://uniprot.org".to_string()),
///         ..Default::default()
///     },
/// ).await?;
/// ```
pub async fn update_organization(
    pool: &PgPool,
    slug: &str,
    params: UpdateOrganizationParams,
) -> DbResult<Organization> {
    // First, get the current organization
    let mut org = get_organization_by_slug(pool, slug).await?;

    // Update fields if provided
    if let Some(name) = params.name {
        org.name = name;
    }
    if let Some(website) = params.website {
        org.website = Some(website);
    }
    if let Some(description) = params.description {
        org.description = Some(description);
    }
    if let Some(logo_url) = params.logo_url {
        org.logo_url = Some(logo_url);
    }
    if let Some(is_system) = params.is_system {
        org.is_system = is_system;
    }
    if let Some(license) = params.license {
        org.license = Some(license);
    }
    if let Some(license_url) = params.license_url {
        org.license_url = Some(license_url);
    }
    if let Some(citation) = params.citation {
        org.citation = Some(citation);
    }
    if let Some(citation_url) = params.citation_url {
        org.citation_url = Some(citation_url);
    }
    if let Some(version_strategy) = params.version_strategy {
        org.version_strategy = Some(version_strategy);
    }
    if let Some(version_description) = params.version_description {
        org.version_description = Some(version_description);
    }
    if let Some(data_source_url) = params.data_source_url {
        org.data_source_url = Some(data_source_url);
    }
    if let Some(documentation_url) = params.documentation_url {
        org.documentation_url = Some(documentation_url);
    }
    if let Some(contact_email) = params.contact_email {
        org.contact_email = Some(contact_email);
    }
    if let Some(versioning_strategy) = params.versioning_strategy {
        org.versioning_strategy = Some(versioning_strategy);
    }

    let now = Utc::now();

    let org = sqlx::query_as!(
        Organization,
        r#"
        UPDATE organizations
        SET
            name = $2,
            website = $3,
            description = $4,
            logo_url = $5,
            is_system = $6,
            license = $7,
            license_url = $8,
            citation = $9,
            citation_url = $10,
            version_strategy = $11,
            version_description = $12,
            data_source_url = $13,
            documentation_url = $14,
            contact_email = $15,
            versioning_strategy = $16,
            updated_at = $17
        WHERE slug = $1
        RETURNING
            id, slug, name, website, description, logo_url, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email,
            versioning_strategy,
            created_at, updated_at
        "#,
        slug,
        org.name,
        org.website,
        org.description,
        org.logo_url,
        org.is_system,
        org.license,
        org.license_url,
        org.citation,
        org.citation_url,
        org.version_strategy,
        org.version_description,
        org.data_source_url,
        org.documentation_url,
        org.contact_email,
        org.versioning_strategy,
        now
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Organization '{}' not found", slug)))?;

    tracing::info!(
        org_id = %org.id,
        org_slug = %org.slug,
        "Updated organization"
    );

    Ok(org)
}

// ============================================================================
// Delete Operations
// ============================================================================

/// Safely deletes an organization by its slug.
///
/// This function permanently removes an organization from the database.
/// Note that deletion may fail if there are foreign key constraints,
/// such as registry entries belonging to this organization. The database
/// schema uses ON DELETE RESTRICT for the organization_id foreign key
/// in registry_entries to prevent orphaned data.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `slug` - URL-safe slug of the organization to delete
///
/// # Errors
///
/// Returns:
/// - `DbError::NotFound` if the organization doesn't exist
/// - `DbError::Sqlx` for other database errors, including:
///   - Foreign key violations (organization has registry entries)
///   - Other constraint violations
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::delete_organization;
///
/// // Delete an organization (will fail if it has registry entries)
/// match delete_organization(&pool, "old-org").await {
///     Ok(()) => println!("Organization deleted successfully"),
///     Err(DbError::Sqlx(e)) => println!("Cannot delete: {}", e),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub async fn delete_organization(pool: &PgPool, slug: &str) -> DbResult<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM organizations
        WHERE slug = $1
        "#,
        slug
    )
    .execute(pool)
    .await
    .map_err(|e| {
        // Provide helpful error messages for common constraints
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_foreign_key_violation() {
                return DbError::Sqlx(sqlx::Error::Database(Box::new(
                    sqlx::postgres::PgDatabaseError::new(
                        "23503".to_string(),
                        format!(
                            "Cannot delete organization '{}': it has associated registry entries",
                            slug
                        ),
                    ),
                )));
            }
        }
        DbError::from(e)
    })?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Organization '{}' not found", slug)));
    }

    tracing::info!(org_slug = %slug, "Deleted organization");

    Ok(())
}

// ============================================================================
// Search Operations
// ============================================================================

/// Performs full-text search on organizations using PostgreSQL's text search.
///
/// This function searches across the name, description, and slug fields using
/// PostgreSQL's full-text search capabilities. Results are ranked by relevance.
///
/// For simple name matching, consider using `list_organizations` with a
/// `name_contains` filter instead.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `search_term` - Search query (supports multiple words, PostgreSQL tsquery syntax)
/// * `pagination` - Pagination parameters
///
/// # Errors
///
/// Returns `DbError::Sqlx` for database errors.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::{search_organizations, Pagination};
///
/// // Simple search
/// let results = search_organizations(&pool, "protein", Pagination::default()).await?;
///
/// // Multi-word search (searches for documents containing both words)
/// let results = search_organizations(&pool, "protein database", Pagination::default()).await?;
///
/// // With pagination
/// let page_1 = search_organizations(&pool, "genome", Pagination::page(0, 10)).await?;
/// let page_2 = search_organizations(&pool, "genome", Pagination::page(1, 10)).await?;
/// ```
pub async fn search_organizations(
    pool: &PgPool,
    search_term: &str,
    pagination: Pagination,
) -> DbResult<Vec<Organization>> {
    // Note: This uses PostgreSQL's full-text search with ts_rank for relevance scoring
    // Requires: CREATE INDEX organizations_search_idx ON organizations
    //           USING GIN (to_tsvector('english', name || ' ' || COALESCE(description, '') || ' ' || slug));
    let orgs = sqlx::query_as!(
        Organization,
        r#"
        SELECT
            id, slug, name, website, description, logo_url, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email,
            versioning_strategy,
            created_at, updated_at
        FROM organizations
        WHERE
            to_tsvector('english', name || ' ' || COALESCE(description, '') || ' ' || slug)
            @@ plainto_tsquery('english', $1)
        ORDER BY
            ts_rank(
                to_tsvector('english', name || ' ' || COALESCE(description, '') || ' ' || slug),
                plainto_tsquery('english', $1)
            ) DESC,
            created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        search_term,
        pagination.limit,
        pagination.offset
    )
    .fetch_all(pool)
    .await?;

    tracing::debug!(
        search_term = %search_term,
        results_count = orgs.len(),
        "Performed full-text search on organizations"
    );

    Ok(orgs)
}

// ============================================================================
// Statistics Operations
// ============================================================================

/// Retrieves comprehensive statistics for an organization.
///
/// This function calculates aggregate statistics including:
/// - Total number of registry entries (data sources + tools)
/// - Breakdown by entry type (data_source vs tool)
/// - Total number of versions
/// - Total size of all files
/// - Total download count
/// - Latest release date
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `organization_id` - UUID of the organization
///
/// # Errors
///
/// Returns `DbError::Sqlx` for database errors. Returns statistics with zero
/// counts if the organization exists but has no entries.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::db::organizations::{get_organization_by_slug, get_organization_statistics};
///
/// let org = get_organization_by_slug(&pool, "uniprot").await?;
/// let stats = get_organization_statistics(&pool, org.id).await?;
///
/// println!("Organization: {}", org.name);
/// println!("  Total entries: {}", stats.total_entries);
/// println!("  Data sources: {}", stats.data_source_count);
/// println!("  Tools: {}", stats.tool_count);
/// println!("  Total versions: {}", stats.total_versions);
/// println!("  Total size: {} bytes", stats.total_size_bytes);
/// println!("  Total downloads: {}", stats.total_downloads);
/// if let Some(date) = stats.latest_release_date {
///     println!("  Latest release: {}", date);
/// }
/// ```
pub async fn get_organization_statistics(
    pool: &PgPool,
    organization_id: Uuid,
) -> DbResult<OrganizationStatistics> {
    // Complex aggregation query joining multiple tables
    let result = sqlx::query!(
        r#"
        SELECT
            COUNT(DISTINCT re.id)::bigint as "total_entries!",
            COUNT(DISTINCT CASE WHEN re.entry_type = 'data_source' THEN re.id END)::bigint as "data_source_count!",
            COUNT(DISTINCT CASE WHEN re.entry_type = 'tool' THEN re.id END)::bigint as "tool_count!",
            COUNT(DISTINCT v.id)::bigint as "total_versions!",
            COALESCE(SUM(v.size_bytes), 0)::bigint as "total_size_bytes!",
            COALESCE(SUM(v.download_count), 0)::bigint as "total_downloads!",
            MAX(v.release_date) as "latest_release_date?"
        FROM organizations o
        LEFT JOIN registry_entries re ON re.organization_id = o.id
        LEFT JOIN versions v ON v.entry_id = re.id
        WHERE o.id = $1
        GROUP BY o.id
        "#,
        organization_id
    )
    .fetch_optional(pool)
    .await?;

    let stats = if let Some(row) = result {
        OrganizationStatistics {
            organization_id,
            total_entries: row.total_entries,
            data_source_count: row.data_source_count,
            tool_count: row.tool_count,
            total_versions: row.total_versions,
            total_size_bytes: row.total_size_bytes,
            total_downloads: row.total_downloads,
            latest_release_date: row.latest_release_date.and_then(|d| {
                d.and_hms_opt(0, 0, 0)
                    .map(|naive_dt| DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc))
            }),
        }
    } else {
        // Organization doesn't exist, return error
        return Err(DbError::NotFound(format!(
            "Organization with id '{}' not found",
            organization_id
        )));
    };

    tracing::debug!(
        org_id = %organization_id,
        total_entries = stats.total_entries,
        total_versions = stats.total_versions,
        "Retrieved organization statistics"
    );

    Ok(stats)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test organization with random slug.
    async fn create_test_organization(pool: &PgPool, suffix: &str) -> Organization {
        let params = CreateOrganizationParams {
            slug: format!("test-org-{}", suffix),
            name: format!("Test Organization {}", suffix),
            website: Some(format!("https://test-{}.com", suffix)),
            description: Some(format!("Test description for {}", suffix)),
            logo_url: None,
            is_system: false,
            license: None,
            license_url: None,
            citation: None,
            citation_url: None,
            version_strategy: None,
            version_description: None,
            data_source_url: None,
            documentation_url: None,
            contact_email: None,
            versioning_strategy: None,
        };

        create_organization(pool, params)
            .await
            .expect("Failed to create test organization")
    }

    #[sqlx::test]
    #[ignore] // Remove this to run tests with a database
    async fn test_create_and_get_organization(pool: PgPool) {
        let params = CreateOrganizationParams {
            slug: "test-org".to_string(),
            name: "Test Organization".to_string(),
            website: Some("https://test.com".to_string()),
            description: Some("Test description".to_string()),
            logo_url: None,
            is_system: false,
            license: None,
            license_url: None,
            citation: None,
            citation_url: None,
            version_strategy: None,
            version_description: None,
            data_source_url: None,
            documentation_url: None,
            contact_email: None,
            versioning_strategy: None,
        };

        let org = create_organization(&pool, params).await.unwrap();

        assert_eq!(org.slug, "test-org");
        assert_eq!(org.name, "Test Organization");
        assert_eq!(org.website.as_deref(), Some("https://test.com"));
        assert!(!org.is_system);

        // Get by slug
        let fetched = get_organization_by_slug(&pool, "test-org").await.unwrap();
        assert_eq!(fetched.id, org.id);
        assert_eq!(fetched.name, org.name);

        // Get by ID
        let fetched_by_id = get_organization_by_id(&pool, org.id).await.unwrap();
        assert_eq!(fetched_by_id.slug, org.slug);

        // Cleanup
        delete_organization(&pool, "test-org").await.unwrap();
    }

    #[sqlx::test]
    #[ignore]
    async fn test_duplicate_organization(pool: PgPool) {
        let params = CreateOrganizationParams {
            slug: "test-org".to_string(),
            name: "Test".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
            license: None,
            license_url: None,
            citation: None,
            citation_url: None,
            version_strategy: None,
            version_description: None,
            data_source_url: None,
            documentation_url: None,
            contact_email: None,
            versioning_strategy: None,
        };

        create_organization(&pool, params.clone()).await.unwrap();

        let result = create_organization(&pool, params).await;
        assert!(matches!(result, Err(DbError::Duplicate(_))));

        // Cleanup
        delete_organization(&pool, "test-org").await.unwrap();
    }

    #[sqlx::test]
    #[ignore]
    async fn test_update_organization(pool: PgPool) {
        let org = create_test_organization(&pool, "update-test").await;

        let update_params = UpdateOrganizationParams {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            website: None,
            logo_url: None,
            is_system: None,
            license: None,
            license_url: None,
            citation: None,
            citation_url: None,
            version_strategy: None,
            version_description: None,
            data_source_url: None,
            documentation_url: None,
            contact_email: None,
            versioning_strategy: None,
        };

        let updated = update_organization(&pool, &org.slug, update_params)
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.website, org.website);

        // Cleanup
        delete_organization(&pool, &org.slug).await.unwrap();
    }

    #[sqlx::test]
    #[ignore]
    async fn test_list_organizations(pool: PgPool) {
        // Create test organizations
        let _org1 = create_test_organization(&pool, "list-1").await;
        let _org2 = create_test_organization(&pool, "list-2").await;
        let _org3 = create_test_organization(&pool, "list-3").await;

        // List all
        let orgs = list_organizations(&pool, None, Pagination::new(10, 0))
            .await
            .unwrap();
        assert!(orgs.len() >= 3);

        // List with pagination
        let page1 = list_organizations(&pool, None, Pagination::new(2, 0))
            .await
            .unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = list_organizations(&pool, None, Pagination::new(2, 2))
            .await
            .unwrap();
        assert!(page2.len() >= 1);

        // Count
        let count = count_organizations(&pool, None).await.unwrap();
        assert!(count >= 3);

        // Cleanup
        delete_organization(&pool, "test-org-list-1").await.unwrap();
        delete_organization(&pool, "test-org-list-2").await.unwrap();
        delete_organization(&pool, "test-org-list-3").await.unwrap();
    }

    #[sqlx::test]
    #[ignore]
    async fn test_list_with_filters(pool: PgPool) {
        // Create test organizations with different attributes
        let params_system = CreateOrganizationParams {
            slug: "test-system-org".to_string(),
            name: "System Test Org".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: true,
            license: None,
            license_url: None,
            citation: None,
            citation_url: None,
            version_strategy: None,
            version_description: None,
            data_source_url: None,
            documentation_url: None,
            contact_email: None,
            versioning_strategy: None,
        };
        create_organization(&pool, params_system).await.unwrap();

        let params_regular = CreateOrganizationParams {
            slug: "test-regular-org".to_string(),
            name: "Regular Test Org".to_string(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
            license: None,
            license_url: None,
            citation: None,
            citation_url: None,
            version_strategy: None,
            version_description: None,
            data_source_url: None,
            documentation_url: None,
            contact_email: None,
            versioning_strategy: None,
        };
        create_organization(&pool, params_regular).await.unwrap();

        // Filter by is_system
        let system_orgs = list_organizations(
            &pool,
            Some(ListOrganizationsFilter {
                is_system: Some(true),
                name_contains: None,
            }),
            Pagination::default(),
        )
        .await
        .unwrap();
        assert!(system_orgs.iter().any(|o| o.slug == "test-system-org"));

        // Filter by name
        let system_named = list_organizations(
            &pool,
            Some(ListOrganizationsFilter {
                is_system: Some(true),
                name_contains: Some("System".to_string()),
            }),
            Pagination::default(),
        )
        .await
        .unwrap();
        assert!(system_named.iter().any(|o| o.slug == "test-system-org"));

        // Cleanup
        delete_organization(&pool, "test-system-org").await.unwrap();
        delete_organization(&pool, "test-regular-org")
            .await
            .unwrap();
    }

    #[sqlx::test]
    #[ignore]
    async fn test_search_organizations(pool: PgPool) {
        let org = create_test_organization(&pool, "search-test").await;

        // Search should find the organization
        let results = search_organizations(&pool, "Test Organization", Pagination::default())
            .await
            .unwrap();
        assert!(results.iter().any(|o| o.id == org.id));

        // Cleanup
        delete_organization(&pool, &org.slug).await.unwrap();
    }

    #[sqlx::test]
    #[ignore]
    async fn test_delete_organization(pool: PgPool) {
        let org = create_test_organization(&pool, "delete-test").await;

        // Delete should succeed
        delete_organization(&pool, &org.slug).await.unwrap();

        // Get should fail
        let result = get_organization_by_slug(&pool, &org.slug).await;
        assert!(matches!(result, Err(DbError::NotFound(_))));
    }

    #[sqlx::test]
    #[ignore]
    async fn test_get_organization_statistics(pool: PgPool) {
        let org = create_test_organization(&pool, "stats-test").await;

        // Get statistics (should be empty)
        let stats = get_organization_statistics(&pool, org.id).await.unwrap();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.data_source_count, 0);
        assert_eq!(stats.tool_count, 0);
        assert_eq!(stats.total_versions, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.total_downloads, 0);

        // Cleanup
        delete_organization(&pool, &org.slug).await.unwrap();
    }
}
