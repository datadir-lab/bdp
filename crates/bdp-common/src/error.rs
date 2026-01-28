//! Error types for BDP
//!
//! This module provides user-friendly error types with actionable messages
//! that help diagnose and resolve issues.

use thiserror::Error;

/// Result type alias for BDP operations
pub type Result<T> = std::result::Result<T, BdpError>;

/// Main error type for BDP
///
/// All errors include contextual information to help users understand
/// what went wrong and how to fix it.
#[derive(Error, Debug)]
pub enum BdpError {
    /// File system operations failed (read, write, create directory, etc.)
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization failed
    #[error("Failed to process JSON data: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Downloaded file checksum doesn't match expected value
    #[error("Checksum verification failed for '{file}': expected '{expected}', got '{actual}'. The file may be corrupted or incomplete. Try re-downloading.")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    /// Requested dataset doesn't exist in the registry
    #[error("Dataset '{name}' not found in registry '{registry}'. Run 'bdp search {name}' to find available datasets.")]
    DatasetNotFound { registry: String, name: String },

    /// Requested version doesn't exist for the dataset
    #[error("Version '{version}' not found for dataset '{dataset}'. Available versions can be listed with 'bdp info {dataset}'.")]
    VersionNotFound { dataset: String, version: String },

    /// Version string doesn't follow the expected format
    #[error("Invalid version format '{version}': {reason}. Expected format: major.minor.patch (e.g., '1.0.0') or date-based (e.g., '2024.01').")]
    InvalidVersion { version: String, reason: String },

    /// Configuration is missing or invalid
    #[error("Configuration error: {message}. {suggestion}")]
    Config { message: String, suggestion: String },

    /// Network request failed
    #[error("Network request to '{url}' failed: {reason}. Check your internet connection and try again.")]
    Network { url: String, reason: String },

    /// Database operation failed
    #[error("Database operation failed: {operation} - {reason}")]
    Database { operation: String, reason: String },

    /// Failed to parse input data
    #[error("Failed to parse {data_type}: {reason}")]
    Parse { data_type: String, reason: String },

    /// Unexpected error with details
    #[error("Unexpected error: {message}. Please report this issue at https://github.com/your-org/bdp/issues")]
    Unknown { message: String },
}

impl BdpError {
    /// Create a dataset not found error
    pub fn dataset_not_found(registry: impl Into<String>, name: impl Into<String>) -> Self {
        Self::DatasetNotFound {
            registry: registry.into(),
            name: name.into(),
        }
    }

    /// Create a version not found error
    pub fn version_not_found(dataset: impl Into<String>, version: impl Into<String>) -> Self {
        Self::VersionNotFound {
            dataset: dataset.into(),
            version: version.into(),
        }
    }

    /// Create an invalid version error
    pub fn invalid_version(version: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidVersion {
            version: version.into(),
            reason: reason.into(),
        }
    }

    /// Create a configuration error with suggestion
    pub fn config(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create a network error
    pub fn network(url: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Network {
            url: url.into(),
            reason: reason.into(),
        }
    }

    /// Create a database error
    pub fn database(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Database {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a parse error
    pub fn parse(data_type: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Parse {
            data_type: data_type.into(),
            reason: reason.into(),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown {
            message: message.into(),
        }
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
}
