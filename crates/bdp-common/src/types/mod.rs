//! Common types used across BDP

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a dataset version
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Represents a dataset entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub version: Version,
    pub checksum: String,
    pub size: u64,
    pub created_at: String,
}

/// Represents metadata for a dataset file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub checksum: String,
    pub size: u64,
    pub modified_at: String,
}

/// Checksum algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    Sha256,
    Sha512,
}

impl std::fmt::Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecksumAlgorithm::Sha256 => write!(f, "sha256"),
            ChecksumAlgorithm::Sha512 => write!(f, "sha512"),
        }
    }
}

// ============================================================================
// Database Types
// ============================================================================

/// Represents an organization in the registry.
///
/// Organizations are the top-level namespace for datasets and provide
/// multi-tenancy support in the registry.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_common::types::Organization;
/// use uuid::Uuid;
///
/// let org = Organization {
///     id: Uuid::new_v4(),
///     slug: "acme-corp".to_string(),
///     name: "ACME Corporation".to_string(),
///     description: Some("Leading provider of datasets".to_string()),
///     created_at: chrono::Utc::now(),
///     updated_at: chrono::Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Organization {
    /// Unique identifier for the organization
    pub id: Uuid,

    /// URL-safe slug used in API paths (e.g., "acme-corp")
    pub slug: String,

    /// Display name of the organization
    pub name: String,

    /// Optional description of the organization
    pub description: Option<String>,

    /// Timestamp when the organization was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the organization was last updated
    pub updated_at: DateTime<Utc>,
}

/// Represents a registry entry (dataset) within an organization.
///
/// A registry entry contains metadata about a dataset, including its name,
/// visibility, and relationships to versions and files.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_common::types::RegistryEntry;
/// use uuid::Uuid;
///
/// let entry = RegistryEntry {
///     id: Uuid::new_v4(),
///     organization_id: Uuid::new_v4(),
///     slug: "sales-data".to_string(),
///     name: "Sales Data".to_string(),
///     description: Some("Historical sales records".to_string()),
///     is_public: false,
///     created_at: chrono::Utc::now(),
///     updated_at: chrono::Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryEntry {
    /// Unique identifier for the registry entry
    pub id: Uuid,

    /// ID of the organization that owns this dataset
    pub organization_id: Uuid,

    /// URL-safe slug for the dataset (e.g., "sales-data")
    pub slug: String,

    /// Display name of the dataset
    pub name: String,

    /// Optional description of the dataset
    pub description: Option<String>,

    /// Whether this dataset is publicly accessible
    pub is_public: bool,

    /// Timestamp when the entry was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the entry was last updated
    pub updated_at: DateTime<Utc>,
}

/// Represents a specific version of a dataset.
///
/// Versions track the evolution of datasets over time, with each version
/// having its own set of files and metadata.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_common::types::DatasetVersion;
/// use uuid::Uuid;
///
/// let version = DatasetVersion {
///     id: Uuid::new_v4(),
///     registry_entry_id: Uuid::new_v4(),
///     version_number: "1.0.0".to_string(),
///     description: Some("Initial release".to_string()),
///     metadata: None,
///     created_at: chrono::Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetVersion {
    /// Unique identifier for this version
    pub id: Uuid,

    /// ID of the registry entry this version belongs to
    pub registry_entry_id: Uuid,

    /// Semantic version number (e.g., "1.0.0")
    pub version_number: String,

    /// Optional description of changes in this version
    pub description: Option<String>,

    /// Optional JSON metadata for the version
    pub metadata: Option<serde_json::Value>,

    /// Timestamp when the version was created
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Result Types
// ============================================================================

/// Common result type for database operations.
///
/// This type alias provides a consistent error handling pattern across
/// database operations, wrapping results with a boxed dynamic error.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_common::types::DbResult;
///
/// fn get_user(id: i32) -> DbResult<User> {
///     // Database query here
///     Ok(user)
/// }
/// ```
pub type DbResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Pagination parameters for list queries.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_common::types::Pagination;
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
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use bdp_common::types::Pagination;
    ///
    /// let pagination = Pagination::new(10, 20);
    /// assert_eq!(pagination.limit, 10);
    /// assert_eq!(pagination.offset, 20);
    /// ```
    pub fn new(limit: i64, offset: i64) -> Self {
        Self { limit, offset }
    }

    /// Creates pagination for a specific page with a given page size.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use bdp_common::types::Pagination;
    ///
    /// let page_2 = Pagination::page(2, 20); // page 2, 20 items per page
    /// assert_eq!(page_2.offset, 20);
    /// assert_eq!(page_2.limit, 20);
    /// ```
    pub fn page(page: i64, page_size: i64) -> Self {
        Self {
            limit: page_size,
            offset: page * page_size,
        }
    }
}
