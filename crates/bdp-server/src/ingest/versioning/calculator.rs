//! Version number calculation utilities
//!
//! This module provides utilities for calculating the next version number
//! based on the bump type and managing version information in the database.

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{debug, instrument};
use uuid::Uuid;

use super::types::BumpType;

/// Parsed semantic version components
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticVersion {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}

impl SemanticVersion {
    /// Create a new semantic version
    pub fn new(major: i32, minor: i32, patch: i32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse a version string like "1.5" or "2.0.1"
    ///
    /// Returns None if parsing fails
    pub fn parse(version: &str) -> Option<Self> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let major = parts.first()?.parse().ok()?;
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        Some(Self { major, minor, patch })
    }

    /// Format as a version string (major.minor format by default)
    pub fn to_string_short(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    /// Format as a full version string (major.minor.patch)
    pub fn to_string_full(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Apply a bump to get the next version
    pub fn bump(&self, bump_type: BumpType) -> Self {
        match bump_type {
            BumpType::Major => Self::new(self.major + 1, 0, 0),
            BumpType::Minor => Self::new(self.major, self.minor + 1, 0),
        }
    }

    /// Check if this version is greater than another
    pub fn is_greater_than(&self, other: &Self) -> bool {
        if self.major != other.major {
            return self.major > other.major;
        }
        if self.minor != other.minor {
            return self.minor > other.minor;
        }
        self.patch > other.patch
    }
}

impl Default for SemanticVersion {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Calculate next version based on bump type
///
/// # Arguments
/// * `current` - Current version string (e.g., "1.5" or "2.0")
/// * `bump` - Type of version bump to apply
///
/// # Returns
/// The next version string
///
/// # Examples
/// ```
/// use bdp_server::ingest::versioning::{calculate_next_version, BumpType};
///
/// assert_eq!(calculate_next_version("1.5", BumpType::Major), "2.0");
/// assert_eq!(calculate_next_version("1.5", BumpType::Minor), "1.6");
/// ```
pub fn calculate_next_version(current: &str, bump: BumpType) -> String {
    let version = SemanticVersion::parse(current).unwrap_or_default();
    version.bump(bump).to_string_short()
}

/// Calculate next version with full major.minor.patch format
pub fn calculate_next_version_full(current: &str, bump: BumpType) -> String {
    let version = SemanticVersion::parse(current).unwrap_or_default();
    version.bump(bump).to_string_full()
}

/// Get the current latest version for a data source
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `data_source_id` - The registry entry ID of the data source
///
/// # Returns
/// The latest version string, or None if no versions exist
#[instrument(skip(pool))]
pub async fn get_latest_version(pool: &PgPool, data_source_id: Uuid) -> Result<Option<String>> {
    let result: Option<String> = sqlx::query_scalar(
        r#"
        SELECT version
        FROM versions
        WHERE entry_id = $1
        ORDER BY version_major DESC, version_minor DESC, version_patch DESC
        LIMIT 1
        "#,
    )
    .bind(data_source_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch latest version")?;

    debug!(
        data_source_id = %data_source_id,
        latest_version = ?result,
        "Retrieved latest version"
    );

    Ok(result)
}

/// Get the latest version ID for a data source
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `data_source_id` - The registry entry ID of the data source
///
/// # Returns
/// The latest version UUID, or None if no versions exist
#[instrument(skip(pool))]
pub async fn get_latest_version_id(pool: &PgPool, data_source_id: Uuid) -> Result<Option<Uuid>> {
    let result: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM versions
        WHERE entry_id = $1
        ORDER BY version_major DESC, version_minor DESC, version_patch DESC
        LIMIT 1
        "#,
    )
    .bind(data_source_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch latest version ID")?;

    debug!(
        data_source_id = %data_source_id,
        latest_version_id = ?result,
        "Retrieved latest version ID"
    );

    Ok(result)
}

/// Get version details by ID
#[derive(Debug, Clone)]
pub struct VersionDetails {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub version: String,
    pub external_version: Option<String>,
    pub version_major: i32,
    pub version_minor: i32,
    pub version_patch: i32,
}

/// Get version details by version ID
#[instrument(skip(pool))]
pub async fn get_version_details(pool: &PgPool, version_id: Uuid) -> Result<Option<VersionDetails>> {
    let row = sqlx::query(
        r#"
        SELECT
            id,
            entry_id,
            version,
            external_version,
            version_major,
            version_minor,
            version_patch
        FROM versions
        WHERE id = $1
        "#,
    )
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch version details")?;

    Ok(row.map(|r| VersionDetails {
        id: sqlx::Row::get(&r, "id"),
        entry_id: sqlx::Row::get(&r, "entry_id"),
        version: sqlx::Row::get(&r, "version"),
        external_version: sqlx::Row::get(&r, "external_version"),
        version_major: sqlx::Row::get(&r, "version_major"),
        version_minor: sqlx::Row::get(&r, "version_minor"),
        version_patch: sqlx::Row::get(&r, "version_patch"),
    }))
}

/// Get the previous version before the given version
#[instrument(skip(pool))]
pub async fn get_previous_version_id(
    pool: &PgPool,
    data_source_id: Uuid,
    current_version: &str,
) -> Result<Option<Uuid>> {
    let current = SemanticVersion::parse(current_version).unwrap_or_default();

    let result: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM versions
        WHERE entry_id = $1
        AND (
            version_major < $2
            OR (version_major = $2 AND version_minor < $3)
            OR (version_major = $2 AND version_minor = $3 AND version_patch < $4)
        )
        ORDER BY version_major DESC, version_minor DESC, version_patch DESC
        LIMIT 1
        "#,
    )
    .bind(data_source_id)
    .bind(current.major)
    .bind(current.minor)
    .bind(current.patch)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch previous version")?;

    Ok(result)
}

/// Create a new version record
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `data_source_id` - The registry entry ID of the data source
/// * `version` - The version string (e.g., "1.6")
/// * `external_version` - The external/upstream version (e.g., "2025_01")
///
/// # Returns
/// The new version UUID
#[instrument(skip(pool))]
pub async fn create_version(
    pool: &PgPool,
    data_source_id: Uuid,
    version: &str,
    external_version: Option<&str>,
) -> Result<Uuid> {
    let parsed = SemanticVersion::parse(version).unwrap_or_default();

    let version_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO versions (entry_id, version, external_version, version_major, version_minor, version_patch)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (entry_id, version) DO UPDATE
        SET external_version = EXCLUDED.external_version,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(data_source_id)
    .bind(version)
    .bind(external_version)
    .bind(parsed.major)
    .bind(parsed.minor)
    .bind(parsed.patch)
    .fetch_one(pool)
    .await
    .context("Failed to create version")?;

    debug!(
        data_source_id = %data_source_id,
        version = %version,
        version_id = %version_id,
        "Created new version"
    );

    Ok(version_id)
}

/// Update version changelog field
#[instrument(skip(pool))]
pub async fn update_version_changelog(
    pool: &PgPool,
    version_id: Uuid,
    changelog_text: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE versions
        SET changelog = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(version_id)
    .bind(changelog_text)
    .execute(pool)
    .await
    .context("Failed to update version changelog")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_version_parse() {
        assert_eq!(
            SemanticVersion::parse("1.5"),
            Some(SemanticVersion::new(1, 5, 0))
        );
        assert_eq!(
            SemanticVersion::parse("2.0.1"),
            Some(SemanticVersion::new(2, 0, 1))
        );
        assert_eq!(
            SemanticVersion::parse("3"),
            Some(SemanticVersion::new(3, 0, 0))
        );
        assert_eq!(SemanticVersion::parse(""), None);
        assert_eq!(SemanticVersion::parse("invalid"), None);
    }

    #[test]
    fn test_semantic_version_bump_major() {
        let v = SemanticVersion::new(1, 5, 3);
        let bumped = v.bump(BumpType::Major);
        assert_eq!(bumped, SemanticVersion::new(2, 0, 0));
    }

    #[test]
    fn test_semantic_version_bump_minor() {
        let v = SemanticVersion::new(1, 5, 3);
        let bumped = v.bump(BumpType::Minor);
        assert_eq!(bumped, SemanticVersion::new(1, 6, 0));
    }

    #[test]
    fn test_calculate_next_version() {
        assert_eq!(calculate_next_version("1.5", BumpType::Major), "2.0");
        assert_eq!(calculate_next_version("1.5", BumpType::Minor), "1.6");
        assert_eq!(calculate_next_version("2.0", BumpType::Minor), "2.1");
        assert_eq!(calculate_next_version("0.1", BumpType::Major), "1.0");
    }

    #[test]
    fn test_calculate_next_version_full() {
        assert_eq!(calculate_next_version_full("1.5.2", BumpType::Major), "2.0.0");
        assert_eq!(calculate_next_version_full("1.5.2", BumpType::Minor), "1.6.0");
    }

    #[test]
    fn test_semantic_version_comparison() {
        let v1 = SemanticVersion::new(1, 5, 0);
        let v2 = SemanticVersion::new(2, 0, 0);
        let v3 = SemanticVersion::new(1, 6, 0);
        let v4 = SemanticVersion::new(1, 5, 1);

        assert!(v2.is_greater_than(&v1));
        assert!(v3.is_greater_than(&v1));
        assert!(v4.is_greater_than(&v1));
        assert!(!v1.is_greater_than(&v2));
        assert!(!v1.is_greater_than(&v1));
    }

    #[test]
    fn test_semantic_version_display() {
        let v = SemanticVersion::new(1, 5, 0);
        assert_eq!(v.to_string(), "1.5");
        assert_eq!(v.to_string_short(), "1.5");
        assert_eq!(v.to_string_full(), "1.5.0");
    }

    #[test]
    fn test_semantic_version_default() {
        let v = SemanticVersion::default();
        assert_eq!(v, SemanticVersion::new(1, 0, 0));
    }
}
