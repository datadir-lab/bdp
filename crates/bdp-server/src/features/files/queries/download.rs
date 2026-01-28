//! Download file query
//!
//! Generates a presigned URL for downloading a file from S3-compatible storage.

use crate::storage::Storage;
use mediator::Request;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Query to generate a presigned download URL
///
/// Verifies the file exists and generates a presigned URL valid for 1 hour.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::files::queries::DownloadFileQuery;
///
/// let query = DownloadFileQuery {
///     org: "uniprot".to_string(),
///     name: "human-insulin".to_string(),
///     version: "1.0.0".to_string(),
///     filename: "P01308.fasta".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileQuery {
    pub org: String,
    pub name: String,
    pub version: String,
    pub filename: String,
}

/// Response containing the presigned download URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileResponse {
    /// Presigned URL for downloading the file
    pub presigned_url: String,
    /// URL expiration time in seconds (currently 3600 = 1 hour)
    pub expires_in: u64,
}

/// Errors that can occur when downloading a file
#[derive(Debug, thiserror::Error)]
pub enum DownloadFileError {
    /// Organization name was empty
    #[error("Organization name is required and cannot be empty")]
    OrgRequired,
    /// Data source name was empty
    #[error("Data source name is required and cannot be empty")]
    NameRequired,
    /// Version was empty
    #[error("Version is required and cannot be empty")]
    VersionRequired,
    /// Filename was empty
    #[error("Filename is required and cannot be empty")]
    FilenameRequired,
    /// The requested file does not exist in storage
    #[error("File not found")]
    NotFound,
    /// An error occurred in the storage backend
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

impl Request<Result<DownloadFileResponse, DownloadFileError>> for DownloadFileQuery {}

impl crate::cqrs::middleware::Query for DownloadFileQuery {}

impl DownloadFileQuery {
    /// Validates the query parameters
    ///
    /// # Errors
    ///
    /// - `OrgRequired` - Organization name is empty
    /// - `NameRequired` - Data source name is empty
    /// - `VersionRequired` - Version is empty
    /// - `FilenameRequired` - Filename is empty
    pub fn validate(&self) -> Result<(), DownloadFileError> {
        if self.org.trim().is_empty() {
            return Err(DownloadFileError::OrgRequired);
        }
        if self.name.trim().is_empty() {
            return Err(DownloadFileError::NameRequired);
        }
        if self.version.trim().is_empty() {
            return Err(DownloadFileError::VersionRequired);
        }
        if self.filename.trim().is_empty() {
            return Err(DownloadFileError::FilenameRequired);
        }
        Ok(())
    }
}

/// Handles the download file query
///
/// Checks if the file exists in storage and generates a presigned
/// download URL valid for 1 hour. The object key is constructed as:
/// `{org}/{name}/{version}/{filename}`.
///
/// # Arguments
///
/// * `storage` - S3-compatible storage backend
/// * `query` - Download query with file location parameters
///
/// # Returns
///
/// Returns the presigned URL and expiration time on success.
///
/// # Errors
///
/// - Validation errors if query parameters are invalid
/// - `NotFound` - The file does not exist in storage
/// - `Storage` - An error occurred in the storage backend
#[tracing::instrument(skip(storage))]
pub async fn handle(
    storage: Storage,
    query: DownloadFileQuery,
) -> Result<DownloadFileResponse, DownloadFileError> {
    query.validate()?;

    let key = storage.build_key(&query.org, &query.name, &query.version, &query.filename);

    let exists = storage.exists(&key).await?;
    if !exists {
        return Err(DownloadFileError::NotFound);
    }

    let expires_in = 3600u64;
    let presigned_url = storage
        .generate_presigned_url(&key, Duration::from_secs(expires_in))
        .await?;

    Ok(DownloadFileResponse {
        presigned_url,
        expires_in,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_org() {
        let query = DownloadFileQuery {
            org: "".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::OrgRequired)
        ));
    }

    #[test]
    fn test_validation_empty_name() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::NameRequired)
        ));
    }

    #[test]
    fn test_validation_empty_version() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::VersionRequired)
        ));
    }

    #[test]
    fn test_validation_empty_filename() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::FilenameRequired)
        ));
    }
}
