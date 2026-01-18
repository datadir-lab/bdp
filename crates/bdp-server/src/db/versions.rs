//! Database operations for versions and version files.
//!
//! This module provides comprehensive CRUD operations for versions, which
//! represent specific releases of data sources or tools. Each version can
//! have multiple file formats and dependencies.
//!
//! # Key Operations
//!
//! ## Versions
//! - `create_version()` - Create version with optional files
//! - `get_version()` - Get version details with files
//! - `list_versions_for_entry()` - All versions for a source
//! - `get_latest_version()` - Get latest version by release date
//!
//! ## Version Files
//! - `create_version_file()` - Add file format to version
//! - `get_version_files()` - Get all files for a version
//! - `delete_version_file()` - Remove a file format
//!
//! ## Dependencies
//! - `get_dependencies()` - Paginated dependencies (handles 567k+ rows)
//! - `create_dependencies()` - Bulk insert dependencies
//! - `count_dependencies()` - Count total dependencies
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::db::{versions, create_pool, DbConfig};
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DbConfig::from_env()?;
//!     let pool = create_pool(&config).await?;
//!
//!     let entry_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
//!
//!     // Create version
//!     let version = versions::create_version(
//!         &pool,
//!         entry_id,
//!         "1.0",
//!         Some("2025_01"),
//!         None,
//!         None,
//!     ).await?;
//!
//!     // Add file
//!     versions::create_version_file(
//!         &pool,
//!         version.id,
//!         "fasta",
//!         "proteins/uniprot/P01308/1.0/P01308.fasta",
//!         "abc123...",
//!         1024,
//!         Some("gzip"),
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use super::{DbError, DbResult};
use bdp_common::types::Pagination;

// ============================================================================
// Types
// ============================================================================

/// Represents a version of a data source or tool.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Version {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub version: String,
    pub external_version: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub size_bytes: Option<i64>,
    pub download_count: i64,
    pub additional_metadata: Option<JsonValue>,
    pub dependency_cache: Option<JsonValue>,
    pub dependency_count: i32,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a version with its files.
#[derive(Debug, Clone)]
pub struct VersionWithFiles {
    pub version: Version,
    pub files: Vec<VersionFile>,
}

/// Represents a file format for a version.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VersionFile {
    pub id: Uuid,
    pub version_id: Uuid,
    pub format: String,
    pub s3_key: String,
    pub checksum: String,
    pub size_bytes: i64,
    pub compression: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Represents a dependency relationship between versions.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Dependency {
    pub id: Uuid,
    pub version_id: Uuid,
    pub depends_on_entry_id: Uuid,
    pub depends_on_version: String,
    pub dependency_type: String,
    pub created_at: DateTime<Utc>,
}

/// Input for creating dependencies in bulk.
#[derive(Debug, Clone)]
pub struct DependencyInput {
    pub depends_on_entry_id: Uuid,
    pub depends_on_version: String,
    pub dependency_type: Option<String>,
}

// ============================================================================
// Version Query Operations
// ============================================================================

/// Retrieves a version by entry ID and version string.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `entry_id` - UUID of the registry entry
/// * `version` - Version string (e.g., "1.0")
///
/// # Errors
///
/// Returns `DbError::NotFound` if the version doesn't exist.
///
/// # Examples
///
/// ```rust,ignore
/// let version = versions::get_version(&pool, entry_id, "1.0").await?;
/// ```
pub async fn get_version(pool: &PgPool, entry_id: Uuid, version: &str) -> DbResult<Version> {
    let ver = sqlx::query_as!(
        Version,
        r#"
        SELECT
            id,
            entry_id,
            version,
            external_version,
            release_date,
            size_bytes,
            download_count,
            additional_metadata,
            dependency_cache,
            dependency_count,
            published_at,
            updated_at
        FROM versions
        WHERE entry_id = $1 AND version = $2
        "#,
        entry_id,
        version
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        DbError::NotFound(format!("Version '{}' not found for entry '{}'", version, entry_id))
    })?;

    Ok(ver)
}

/// Retrieves a version by its UUID.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the version
pub async fn get_version_by_id(pool: &PgPool, id: Uuid) -> DbResult<Version> {
    let ver = sqlx::query_as!(
        Version,
        r#"
        SELECT
            id,
            entry_id,
            version,
            external_version,
            release_date,
            size_bytes,
            download_count,
            additional_metadata,
            dependency_cache,
            dependency_count,
            published_at,
            updated_at
        FROM versions
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Version with id '{}' not found", id)))?;

    Ok(ver)
}

/// Retrieves a version with all its files.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `entry_id` - UUID of the registry entry
/// * `version` - Version string
///
/// # Examples
///
/// ```rust,ignore
/// let ver_with_files = versions::get_version_with_files(&pool, entry_id, "1.0").await?;
/// for file in &ver_with_files.files {
///     println!("Format: {}, Size: {}", file.format, file.size_bytes);
/// }
/// ```
pub async fn get_version_with_files(
    pool: &PgPool,
    entry_id: Uuid,
    version: &str,
) -> DbResult<VersionWithFiles> {
    let ver = get_version(pool, entry_id, version).await?;
    let files = get_version_files(pool, ver.id).await?;

    Ok(VersionWithFiles {
        version: ver,
        files,
    })
}

/// Lists all versions for a registry entry.
///
/// Returns versions ordered by release date (newest first).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `entry_id` - UUID of the registry entry
///
/// # Examples
///
/// ```rust,ignore
/// let versions = versions::list_versions_for_entry(&pool, entry_id).await?;
/// for v in versions {
///     println!("Version: {} ({})", v.version, v.external_version.unwrap_or_default());
/// }
/// ```
pub async fn list_versions_for_entry(pool: &PgPool, entry_id: Uuid) -> DbResult<Vec<Version>> {
    let versions = sqlx::query_as!(
        Version,
        r#"
        SELECT
            id,
            entry_id,
            version,
            external_version,
            release_date,
            size_bytes,
            download_count,
            additional_metadata,
            dependency_cache,
            dependency_count,
            published_at,
            updated_at
        FROM versions
        WHERE entry_id = $1
        ORDER BY release_date DESC NULLS LAST, published_at DESC
        "#,
        entry_id
    )
    .fetch_all(pool)
    .await?;

    Ok(versions)
}

/// Gets the latest version for a registry entry.
///
/// The latest version is determined by release_date (if available),
/// otherwise by published_at.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `entry_id` - UUID of the registry entry
///
/// # Errors
///
/// Returns `DbError::NotFound` if no versions exist for the entry.
///
/// # Examples
///
/// ```rust,ignore
/// let latest = versions::get_latest_version(&pool, entry_id).await?;
/// println!("Latest version: {}", latest.version);
/// ```
pub async fn get_latest_version(pool: &PgPool, entry_id: Uuid) -> DbResult<Version> {
    let ver = sqlx::query_as!(
        Version,
        r#"
        SELECT
            id,
            entry_id,
            version,
            external_version,
            release_date,
            size_bytes,
            download_count,
            additional_metadata,
            dependency_cache,
            dependency_count,
            published_at,
            updated_at
        FROM versions
        WHERE entry_id = $1
        ORDER BY release_date DESC NULLS LAST, published_at DESC
        LIMIT 1
        "#,
        entry_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("No versions found for entry '{}'", entry_id)))?;

    Ok(ver)
}

// ============================================================================
// Version Mutation Operations
// ============================================================================

/// Creates a new version for a registry entry.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `entry_id` - UUID of the registry entry
/// * `version` - Version string (e.g., "1.0")
/// * `external_version` - Optional external version (e.g., "2025_01")
/// * `release_date` - Optional release date
/// * `additional_metadata` - Optional JSONB metadata
///
/// # Errors
///
/// Returns `DbError::Duplicate` if the version already exists for this entry.
///
/// # Examples
///
/// ```rust,ignore
/// let version = versions::create_version(
///     &pool,
///     entry_id,
///     "1.0",
///     Some("2025_01"),
///     Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
///     None,
/// ).await?;
/// ```
pub async fn create_version(
    pool: &PgPool,
    entry_id: Uuid,
    version: &str,
    external_version: Option<&str>,
    release_date: Option<NaiveDate>,
    additional_metadata: Option<JsonValue>,
) -> DbResult<Version> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let ver = sqlx::query_as!(
        Version,
        r#"
        INSERT INTO versions (
            id, entry_id, version, external_version, release_date,
            additional_metadata, published_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING
            id,
            entry_id,
            version,
            external_version,
            release_date,
            size_bytes,
            download_count,
            additional_metadata,
            dependency_cache,
            dependency_count,
            published_at,
            updated_at
        "#,
        id,
        entry_id,
        version,
        external_version,
        release_date,
        additional_metadata,
        now,
        now
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return DbError::Duplicate(format!(
                    "Version '{}' already exists for entry '{}'",
                    version, entry_id
                ));
            }
        }
        DbError::from(e)
    })?;

    tracing::info!(
        version_id = %ver.id,
        entry_id = %entry_id,
        version = %version,
        "Created version"
    );

    Ok(ver)
}

/// Updates a version's metadata.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the version
/// * `external_version` - Optional new external version
/// * `release_date` - Optional new release date
/// * `additional_metadata` - Optional new metadata
///
/// # Examples
///
/// ```rust,ignore
/// versions::update_version(
///     &pool,
///     version_id,
///     Some(Some("2025_02")),
///     None,
///     None,
/// ).await?;
/// ```
pub async fn update_version(
    pool: &PgPool,
    id: Uuid,
    external_version: Option<Option<&str>>,
    release_date: Option<Option<NaiveDate>>,
    additional_metadata: Option<Option<JsonValue>>,
) -> DbResult<Version> {
    let current = get_version_by_id(pool, id).await?;

    let updated_external_version = match external_version {
        Some(Some(v)) => Some(v),
        Some(None) => None,
        None => current.external_version.as_deref(),
    };
    let updated_release_date = match release_date {
        Some(d) => d,
        None => current.release_date,
    };
    let updated_metadata = match additional_metadata {
        Some(Some(m)) => Some(m),
        Some(None) => None,
        None => current.additional_metadata,
    };

    let now = Utc::now();

    let ver = sqlx::query_as!(
        Version,
        r#"
        UPDATE versions
        SET
            external_version = $2,
            release_date = $3,
            additional_metadata = $4,
            updated_at = $5
        WHERE id = $1
        RETURNING
            id,
            entry_id,
            version,
            external_version,
            release_date,
            size_bytes,
            download_count,
            additional_metadata,
            dependency_cache,
            dependency_count,
            published_at,
            updated_at
        "#,
        id,
        updated_external_version,
        updated_release_date,
        updated_metadata,
        now
    )
    .fetch_one(pool)
    .await?;

    tracing::info!(version_id = %id, "Updated version");

    Ok(ver)
}

/// Deletes a version.
///
/// This cascades to delete all version files and dependencies.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the version
pub async fn delete_version(pool: &PgPool, id: Uuid) -> DbResult<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM versions
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Version with id '{}' not found", id)));
    }

    tracing::info!(version_id = %id, "Deleted version");

    Ok(())
}

/// Increments the download count for a version.
///
/// This is a separate function to avoid locking issues during updates.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - UUID of the version
pub async fn increment_download_count(pool: &PgPool, id: Uuid) -> DbResult<()> {
    sqlx::query!(
        r#"
        UPDATE versions
        SET download_count = download_count + 1
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// Version File Operations
// ============================================================================

/// Retrieves all files for a version.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
pub async fn get_version_files(pool: &PgPool, version_id: Uuid) -> DbResult<Vec<VersionFile>> {
    let files = sqlx::query_as!(
        VersionFile,
        r#"
        SELECT id, version_id, format, s3_key, checksum, size_bytes, compression, created_at
        FROM version_files
        WHERE version_id = $1
        ORDER BY format
        "#,
        version_id
    )
    .fetch_all(pool)
    .await?;

    Ok(files)
}

/// Retrieves a specific file format for a version.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
/// * `format` - File format (e.g., "fasta", "xml")
pub async fn get_version_file(
    pool: &PgPool,
    version_id: Uuid,
    format: &str,
) -> DbResult<VersionFile> {
    let file = sqlx::query_as!(
        VersionFile,
        r#"
        SELECT id, version_id, format, s3_key, checksum, size_bytes, compression, created_at
        FROM version_files
        WHERE version_id = $1 AND format = $2
        "#,
        version_id,
        format
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        DbError::NotFound(format!(
            "File format '{}' not found for version '{}'",
            format, version_id
        ))
    })?;

    Ok(file)
}

/// Creates a new version file (adds a format to a version).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
/// * `format` - File format (e.g., "fasta", "xml")
/// * `s3_key` - S3 storage key
/// * `checksum` - SHA-256 checksum
/// * `size_bytes` - File size in bytes
/// * `compression` - Optional compression (e.g., "gzip", "bzip2")
///
/// # Errors
///
/// Returns `DbError::Duplicate` if this format already exists for the version.
///
/// # Examples
///
/// ```rust,ignore
/// let file = versions::create_version_file(
///     &pool,
///     version_id,
///     "fasta",
///     "proteins/uniprot/P01308/1.0/P01308.fasta",
///     "abc123def456...",
///     1024,
///     Some("gzip"),
/// ).await?;
/// ```
pub async fn create_version_file(
    pool: &PgPool,
    version_id: Uuid,
    format: &str,
    s3_key: &str,
    checksum: &str,
    size_bytes: i64,
    compression: Option<&str>,
) -> DbResult<VersionFile> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let file = sqlx::query_as!(
        VersionFile,
        r#"
        INSERT INTO version_files (id, version_id, format, s3_key, checksum, size_bytes, compression, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, version_id, format, s3_key, checksum, size_bytes, compression, created_at
        "#,
        id,
        version_id,
        format,
        s3_key,
        checksum,
        size_bytes,
        compression,
        now
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return DbError::Duplicate(format!(
                    "File format '{}' already exists for version '{}'",
                    format, version_id
                ));
            }
        }
        DbError::from(e)
    })?;

    tracing::info!(
        version_file_id = %file.id,
        version_id = %version_id,
        format = %format,
        "Created version file"
    );

    Ok(file)
}

/// Deletes a version file.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
/// * `format` - File format to delete
pub async fn delete_version_file(pool: &PgPool, version_id: Uuid, format: &str) -> DbResult<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM version_files
        WHERE version_id = $1 AND format = $2
        "#,
        version_id,
        format
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!(
            "File format '{}' not found for version '{}'",
            format, version_id
        )));
    }

    tracing::info!(
        version_id = %version_id,
        format = %format,
        "Deleted version file"
    );

    Ok(())
}

// ============================================================================
// Dependency Operations
// ============================================================================

/// Gets dependencies for a version with pagination.
///
/// This function is optimized for handling large dependency sets (e.g., 567k+ rows).
/// It uses efficient indexing and pagination to avoid loading all dependencies
/// into memory at once.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
/// * `pagination` - Pagination parameters (recommended: limit ≤ 1000)
///
/// # Examples
///
/// ```rust,ignore
/// // Fetch first page
/// let deps = versions::get_dependencies(&pool, version_id, Pagination::new(1000, 0)).await?;
///
/// // Fetch next page
/// let more_deps = versions::get_dependencies(&pool, version_id, Pagination::new(1000, 1000)).await?;
/// ```
pub async fn get_dependencies(
    pool: &PgPool,
    version_id: Uuid,
    pagination: Pagination,
) -> DbResult<Vec<Dependency>> {
    let deps = sqlx::query_as!(
        Dependency,
        r#"
        SELECT id, version_id, depends_on_entry_id, depends_on_version, dependency_type, created_at
        FROM dependencies
        WHERE version_id = $1
        ORDER BY depends_on_entry_id
        LIMIT $2 OFFSET $3
        "#,
        version_id,
        pagination.limit,
        pagination.offset
    )
    .fetch_all(pool)
    .await?;

    Ok(deps)
}

/// Counts total dependencies for a version.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
///
/// # Examples
///
/// ```rust,ignore
/// let total = versions::count_dependencies(&pool, version_id).await?;
/// let pages = (total as f64 / 1000.0).ceil() as i64;
/// println!("Total dependencies: {}, Pages: {}", total, pages);
/// ```
pub async fn count_dependencies(pool: &PgPool, version_id: Uuid) -> DbResult<i64> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM dependencies
        WHERE version_id = $1
        "#,
        version_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.count)
}

/// Creates dependencies in bulk.
///
/// This function efficiently inserts multiple dependencies using a single
/// query with multiple VALUES clauses. It's optimized for large datasets
/// (e.g., creating 567k dependencies for uniprot:all@1.0).
///
/// # Performance Considerations
///
/// For very large dependency sets (>10k), consider batching into chunks:
///
/// ```rust,ignore
/// let chunk_size = 5000;
/// for chunk in deps.chunks(chunk_size) {
///     versions::create_dependencies(&pool, version_id, chunk).await?;
/// }
/// ```
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
/// * `dependencies` - Slice of dependency inputs
///
/// # Examples
///
/// ```rust,ignore
/// let deps = vec![
///     DependencyInput {
///         depends_on_entry_id: protein1_id,
///         depends_on_version: "1.0".to_string(),
///         dependency_type: None,
///     },
///     DependencyInput {
///         depends_on_entry_id: protein2_id,
///         depends_on_version: "1.0".to_string(),
///         dependency_type: None,
///     },
/// ];
/// versions::create_dependencies(&pool, version_id, &deps).await?;
/// ```
pub async fn create_dependencies(
    pool: &PgPool,
    version_id: Uuid,
    dependencies: &[DependencyInput],
) -> DbResult<u64> {
    if dependencies.is_empty() {
        return Ok(0);
    }

    // Build bulk insert query
    let mut query_str = String::from(
        "INSERT INTO dependencies (id, version_id, depends_on_entry_id, depends_on_version, dependency_type, created_at) VALUES ",
    );

    let now = Utc::now();
    let mut values = Vec::new();

    for (idx, dep) in dependencies.iter().enumerate() {
        let base = idx * 6;
        values.push(format!(
            "(${}, ${}, ${}, ${}, ${}, ${})",
            base + 1,
            base + 2,
            base + 3,
            base + 4,
            base + 5,
            base + 6
        ));
    }

    query_str.push_str(&values.join(", "));
    query_str.push_str(" ON CONFLICT (version_id, depends_on_entry_id) DO NOTHING");

    let mut query = sqlx::query(&query_str);

    for dep in dependencies {
        let id = Uuid::new_v4();
        let dep_type = dep.dependency_type.as_deref().unwrap_or("required");
        query = query
            .bind(id)
            .bind(version_id)
            .bind(dep.depends_on_entry_id)
            .bind(&dep.depends_on_version)
            .bind(dep_type)
            .bind(now);
    }

    let result = query.execute(pool).await?;

    tracing::info!(
        version_id = %version_id,
        count = dependencies.len(),
        inserted = result.rows_affected(),
        "Created dependencies"
    );

    Ok(result.rows_affected())
}

/// Deletes a specific dependency.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
/// * `depends_on_entry_id` - UUID of the dependency entry
pub async fn delete_dependency(
    pool: &PgPool,
    version_id: Uuid,
    depends_on_entry_id: Uuid,
) -> DbResult<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM dependencies
        WHERE version_id = $1 AND depends_on_entry_id = $2
        "#,
        version_id,
        depends_on_entry_id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!(
            "Dependency not found for version '{}' and entry '{}'",
            version_id, depends_on_entry_id
        )));
    }

    tracing::info!(
        version_id = %version_id,
        depends_on_entry_id = %depends_on_entry_id,
        "Deleted dependency"
    );

    Ok(())
}

/// Deletes all dependencies for a version.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `version_id` - UUID of the version
pub async fn delete_all_dependencies(pool: &PgPool, version_id: Uuid) -> DbResult<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM dependencies
        WHERE version_id = $1
        "#,
        version_id
    )
    .execute(pool)
    .await?;

    tracing::info!(
        version_id = %version_id,
        count = result.rows_affected(),
        "Deleted all dependencies"
    );

    Ok(result.rows_affected())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create test pool
    #[allow(dead_code)]
    async fn create_test_pool() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://postgres:postgres@localhost:5432/bdp_test".to_string()
        });
        PgPool::connect(&url).await.unwrap()
    }

    /// Helper to create test entry
    #[allow(dead_code)]
    async fn create_test_entry(pool: &PgPool) -> (Uuid, Uuid) {
        let org_id = Uuid::new_v4();
        let entry_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            org_id,
            format!("test-org-{}", org_id),
            "Test Org",
            now,
            now
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!(
            "INSERT INTO registry_entries (id, organization_id, slug, name, entry_type, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            entry_id,
            org_id,
            format!("test-entry-{}", entry_id),
            "Test Entry",
            "data_source",
            now,
            now
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(pool)
        .await
        .unwrap();

        (org_id, entry_id)
    }

    /// Helper to cleanup test entry
    #[allow(dead_code)]
    async fn cleanup_test_entry(pool: &PgPool, org_id: Uuid) {
        sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
            .execute(pool)
            .await
            .unwrap();
    }

    #[sqlx::test]
    async fn test_create_and_get_version(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;

        let version = create_version(&pool, entry_id, "1.0", Some("2025_01"), None, None)
            .await
            .unwrap();

        assert_eq!(version.version, "1.0");
        assert_eq!(version.external_version.as_deref(), Some("2025_01"));

        let fetched = get_version(&pool, entry_id, "1.0").await.unwrap();
        assert_eq!(fetched.id, version.id);

        cleanup_test_entry(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_create_duplicate_version(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;

        create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        let result = create_version(&pool, entry_id, "1.0", None, None, None).await;
        assert!(matches!(result, Err(DbError::Duplicate(_))));

        cleanup_test_entry(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_list_versions_for_entry(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;

        create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();
        create_version(&pool, entry_id, "1.1", None, None, None)
            .await
            .unwrap();
        create_version(&pool, entry_id, "2.0", None, None, None)
            .await
            .unwrap();

        let versions = list_versions_for_entry(&pool, entry_id).await.unwrap();
        assert_eq!(versions.len(), 3);

        cleanup_test_entry(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_get_latest_version(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;

        create_version(
            &pool,
            entry_id,
            "1.0",
            None,
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            None,
        )
        .await
        .unwrap();

        create_version(
            &pool,
            entry_id,
            "1.1",
            None,
            Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
            None,
        )
        .await
        .unwrap();

        let latest = get_latest_version(&pool, entry_id).await.unwrap();
        assert_eq!(latest.version, "1.1");

        cleanup_test_entry(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_create_and_get_version_file(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;
        let version = create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        let file = create_version_file(
            &pool,
            version.id,
            "fasta",
            "test/path.fasta",
            "abc123",
            1024,
            Some("gzip"),
        )
        .await
        .unwrap();

        assert_eq!(file.format, "fasta");
        assert_eq!(file.size_bytes, 1024);

        let fetched = get_version_file(&pool, version.id, "fasta").await.unwrap();
        assert_eq!(fetched.id, file.id);

        cleanup_test_entry(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_get_version_files(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;
        let version = create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        create_version_file(&pool, version.id, "fasta", "test.fasta", "abc", 100, None)
            .await
            .unwrap();
        create_version_file(&pool, version.id, "xml", "test.xml", "def", 200, None)
            .await
            .unwrap();

        let files = get_version_files(&pool, version.id).await.unwrap();
        assert_eq!(files.len(), 2);

        cleanup_test_entry(&pool, org_id).await;
    }

    #[sqlx::test]
    async fn test_create_dependencies(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;
        let (org_id2, entry_id2) = create_test_entry(&pool).await;
        let (org_id3, entry_id3) = create_test_entry(&pool).await;

        let version = create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        let deps = vec![
            DependencyInput {
                depends_on_entry_id: entry_id2,
                depends_on_version: "1.0".to_string(),
                dependency_type: None,
            },
            DependencyInput {
                depends_on_entry_id: entry_id3,
                depends_on_version: "1.0".to_string(),
                dependency_type: None,
            },
        ];

        let count = create_dependencies(&pool, version.id, &deps).await.unwrap();
        assert_eq!(count, 2);

        let fetched = get_dependencies(&pool, version.id, Pagination::default())
            .await
            .unwrap();
        assert_eq!(fetched.len(), 2);

        cleanup_test_entry(&pool, org_id).await;
        cleanup_test_entry(&pool, org_id2).await;
        cleanup_test_entry(&pool, org_id3).await;
    }

    #[sqlx::test]
    async fn test_count_dependencies(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;
        let (org_id2, entry_id2) = create_test_entry(&pool).await;

        let version = create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        let deps = vec![DependencyInput {
            depends_on_entry_id: entry_id2,
            depends_on_version: "1.0".to_string(),
            dependency_type: None,
        }];

        create_dependencies(&pool, version.id, &deps).await.unwrap();

        let count = count_dependencies(&pool, version.id).await.unwrap();
        assert_eq!(count, 1);

        cleanup_test_entry(&pool, org_id).await;
        cleanup_test_entry(&pool, org_id2).await;
    }

    #[sqlx::test]
    async fn test_delete_dependency(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;
        let (org_id2, entry_id2) = create_test_entry(&pool).await;

        let version = create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        let deps = vec![DependencyInput {
            depends_on_entry_id: entry_id2,
            depends_on_version: "1.0".to_string(),
            dependency_type: None,
        }];

        create_dependencies(&pool, version.id, &deps).await.unwrap();

        delete_dependency(&pool, version.id, entry_id2)
            .await
            .unwrap();

        let count = count_dependencies(&pool, version.id).await.unwrap();
        assert_eq!(count, 0);

        cleanup_test_entry(&pool, org_id).await;
        cleanup_test_entry(&pool, org_id2).await;
    }

    #[sqlx::test]
    async fn test_increment_download_count(pool: PgPool) {
        let (org_id, entry_id) = create_test_entry(&pool).await;
        let version = create_version(&pool, entry_id, "1.0", None, None, None)
            .await
            .unwrap();

        increment_download_count(&pool, version.id).await.unwrap();
        increment_download_count(&pool, version.id).await.unwrap();

        let updated = get_version_by_id(&pool, version.id).await.unwrap();
        assert_eq!(updated.download_count, 2);

        cleanup_test_entry(&pool, org_id).await;
    }
}
