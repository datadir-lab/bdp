//! Error types for BDP CLI
//!
//! This module provides user-friendly error types with clear, actionable messages
//! that help users understand what went wrong and how to fix it.

use thiserror::Error;

/// Result type alias for CLI operations
pub type Result<T> = std::result::Result<T, CliError>;

/// Comprehensive error type for CLI operations
///
/// All errors are designed to be user-facing with clear messages and suggestions.
#[derive(Error, Debug)]
pub enum CliError {
    /// API server communication failed
    #[error("Server error: {0}. Ensure the BDP server is running (check with 'bdp status') and accessible.")]
    Api(String),

    /// Required file is missing
    #[error("File not found: '{0}'. Verify the file path exists and you have read permissions.")]
    FileNotFound(String),

    /// Manifest file (bdp.yml) has invalid format or content
    #[error("Invalid manifest (bdp.yml): {0}. Run 'bdp init' to create a valid manifest.")]
    InvalidManifest(String),

    /// Lockfile (bdl.lock) has invalid format or content
    #[error("Invalid lockfile (bdl.lock): {0}. Delete the lockfile and run 'bdp pull' to regenerate it.")]
    InvalidLockfile(String),

    /// Cache operation failed
    #[error("Cache error: {0}. Try running 'bdp clean --cache' to clear the cache.")]
    Cache(String),

    /// Downloaded file checksum verification failed
    #[error("Checksum verification failed for '{file}': expected '{expected}', got '{actual}'. The file may be corrupted. Run 'bdp pull --force' to re-download.")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    /// Database operation failed (SQLx)
    #[error("Database error: {0}. Check your database connection settings.")]
    Database(#[from] sqlx::Error),

    /// Audit database operation failed (rusqlite)
    #[error("Audit database error: {0}")]
    AuditDb(#[from] rusqlite::Error),

    /// Audit trail operation failed
    #[error("Audit trail error: {0}")]
    Audit(String),

    /// File system operation failed
    #[error("File operation failed: {0}. Check file permissions and disk space.")]
    Io(#[from] std::io::Error),

    /// HTTP request failed
    #[error("Network request failed: {0}. Check your internet connection and server URL.")]
    Http(#[from] reqwest::Error),

    /// Configuration is missing or invalid
    #[error("Configuration error: {0}. Check your environment variables or config file.")]
    Config(String),

    /// YAML parsing failed
    #[error("Failed to parse YAML: {0}. Check the file syntax at the indicated line/column.")]
    YamlParse(#[from] serde_yaml::Error),

    /// JSON parsing failed
    #[error("Failed to parse JSON: {0}. Check the file syntax.")]
    JsonParse(#[from] serde_json::Error),

    /// Source specification doesn't follow expected format
    #[error("Invalid source specification: {0}. Expected format: 'registry:identifier-format@version' (e.g., 'uniprot:P01308-fasta@1.0').")]
    InvalidSourceSpec(String),

    /// Project directory already has a bdp.yml
    #[error("Project already initialized: {0}. Use --force to reinitialize.")]
    AlreadyInitialized(String),

    /// Project directory doesn't have a bdp.yml
    #[error("Not a BDP project: {0}. Run 'bdp init' first to initialize this directory.")]
    NotInitialized(String),

    /// Source is already in the manifest
    #[error("Source '{0}' already exists in manifest. Use 'bdp source list' to see current sources.")]
    SourceExists(String),

    /// Source not found in manifest or registry
    #[error("Source '{0}' not found. Run 'bdp search' to find available sources.")]
    SourceNotFound(String),

    /// Generic anyhow error wrapper
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl CliError {
    /// Create an API error
    pub fn api(msg: impl Into<String>) -> Self {
        Self::Api(msg.into())
    }

    /// Create a cache error
    pub fn cache(msg: impl Into<String>) -> Self {
        Self::Cache(msg.into())
    }

    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an invalid manifest error
    pub fn invalid_manifest(msg: impl Into<String>) -> Self {
        Self::InvalidManifest(msg.into())
    }

    /// Create an invalid source spec error
    pub fn invalid_source_spec(msg: impl Into<String>) -> Self {
        Self::InvalidSourceSpec(msg.into())
    }

    /// Create a checksum mismatch error
    pub fn checksum_mismatch(
        file: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::ChecksumMismatch {
            file: file.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create an audit error
    pub fn audit(msg: impl Into<String>) -> Self {
        Self::Audit(msg.into())
    }
}
