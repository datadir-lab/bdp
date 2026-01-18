//! Error types for BDP

use thiserror::Error;

/// Result type alias for BDP operations
pub type Result<T> = std::result::Result<T, BdpError>;

/// Main error type for BDP
#[derive(Error, Debug)]
pub enum BdpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Dataset not found: {0}")]
    DatasetNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
