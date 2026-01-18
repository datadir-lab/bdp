//! Error types for BDP CLI

use thiserror::Error;

/// Result type alias for CLI operations
pub type Result<T> = std::result::Result<T, CliError>;

/// Comprehensive error type for CLI operations
#[derive(Error, Debug)]
pub enum CliError {
    /// API-related errors
    #[error("API error: {0}")]
    Api(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Invalid manifest format or content
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    /// Invalid lockfile format or content
    #[error("Invalid lockfile: {0}")]
    InvalidLockfile(String),

    /// Cache-related errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// Checksum verification failure
    #[error("Checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    /// Database errors (SQLx)
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Audit database errors (rusqlite)
    #[error("Audit error: {0}")]
    AuditDb(#[from] rusqlite::Error),

    /// Audit trail errors
    #[error("Audit error: {0}")]
    Audit(String),

    /// I/O errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP client errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// YAML parsing errors
    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// JSON parsing errors
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Invalid source specification
    #[error("Invalid source specification: {0}")]
    InvalidSourceSpec(String),

    /// Project already initialized
    #[error("Project already initialized: {0}")]
    AlreadyInitialized(String),

    /// Project not initialized
    #[error("Project not initialized: {0}")]
    NotInitialized(String),

    /// Source already exists
    #[error("Source already exists: {0}")]
    SourceExists(String),

    /// Source not found
    #[error("Source not found: {0}")]
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
