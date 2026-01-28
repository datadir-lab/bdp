//! Upload file command
//!
//! Uploads a file to S3-compatible storage and returns a presigned URL
//! for accessing the uploaded file.

use crate::storage::Storage;
use mediator::Request;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Command to upload a file to storage
///
/// Uploads the file content to an S3-compatible storage backend
/// using a structured key path: `{org}/{name}/{version}/{filename}`.
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::files::commands::UploadFileCommand;
///
/// let command = UploadFileCommand {
///     org: "uniprot".to_string(),
///     name: "human-insulin".to_string(),
///     version: "1.0.0".to_string(),
///     filename: "P01308.fasta".to_string(),
///     content: fasta_bytes,
///     content_type: Some("text/plain".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileCommand {
    pub org: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    #[serde(skip)]
    pub content: Vec<u8>,
    pub content_type: Option<String>,
}

/// Response from uploading a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileResponse {
    /// S3 object key
    pub key: String,
    /// MD5 checksum of the uploaded content
    pub checksum: String,
    /// Size of the uploaded file in bytes
    pub size: i64,
    /// Presigned URL for downloading (valid for 1 hour)
    pub presigned_url: String,
}

/// Errors that can occur when uploading a file
#[derive(Debug, thiserror::Error)]
pub enum UploadFileError {
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
    /// Filename exceeded maximum length
    #[error("Filename must not exceed 255 characters")]
    FilenameLength,
    /// Content was empty (zero bytes)
    #[error("Content is required and cannot be empty")]
    ContentRequired,
    /// An error occurred in the storage backend
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

impl Request<Result<UploadFileResponse, UploadFileError>> for UploadFileCommand {}

impl crate::cqrs::middleware::Command for UploadFileCommand {}

impl UploadFileCommand {
    /// Validates the command parameters
    ///
    /// # Errors
    ///
    /// - `OrgRequired` - Organization name is empty
    /// - `NameRequired` - Data source name is empty
    /// - `VersionRequired` - Version is empty
    /// - `FilenameRequired` - Filename is empty
    /// - `FilenameLength` - Filename exceeds 255 characters
    /// - `ContentRequired` - Content is empty
    pub fn validate(&self) -> Result<(), UploadFileError> {
        if self.org.trim().is_empty() {
            return Err(UploadFileError::OrgRequired);
        }
        if self.name.trim().is_empty() {
            return Err(UploadFileError::NameRequired);
        }
        if self.version.trim().is_empty() {
            return Err(UploadFileError::VersionRequired);
        }
        if self.filename.trim().is_empty() {
            return Err(UploadFileError::FilenameRequired);
        }
        if self.filename.len() > 255 {
            return Err(UploadFileError::FilenameLength);
        }
        if self.content.is_empty() {
            return Err(UploadFileError::ContentRequired);
        }
        Ok(())
    }
}

/// Handles the upload file command
///
/// Uploads the file to S3-compatible storage and generates a presigned
/// download URL valid for 1 hour. The object key is constructed as:
/// `{org}/{name}/{version}/{filename}`.
///
/// # Arguments
///
/// * `storage` - S3-compatible storage backend
/// * `command` - Upload command with file content and metadata
///
/// # Returns
///
/// Returns the upload result including key, checksum, size, and presigned URL.
///
/// # Errors
///
/// - Validation errors if command parameters are invalid
/// - `Storage` - An error occurred in the storage backend
#[tracing::instrument(skip(storage, command))]
pub async fn handle(
    storage: Storage,
    command: UploadFileCommand,
) -> Result<UploadFileResponse, UploadFileError> {
    command.validate()?;

    let key = storage.build_key(&command.org, &command.name, &command.version, &command.filename);

    let upload_result = storage
        .upload(&key, command.content, command.content_type)
        .await?;

    let presigned_url = storage
        .generate_presigned_url(&key, Duration::from_secs(3600))
        .await?;

    Ok(UploadFileResponse {
        key: upload_result.key,
        checksum: upload_result.checksum,
        size: upload_result.size,
        presigned_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: Some("application/octet-stream".to_string()),
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_org() {
        let cmd = UploadFileCommand {
            org: "".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::OrgRequired)));
    }

    #[test]
    fn test_validation_empty_name() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::NameRequired)));
    }

    #[test]
    fn test_validation_empty_version() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::VersionRequired)));
    }

    #[test]
    fn test_validation_empty_filename() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::FilenameRequired)));
    }

    #[test]
    fn test_validation_filename_too_long() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "a".repeat(256),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::FilenameLength)));
    }

    #[test]
    fn test_validation_empty_content() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::ContentRequired)));
    }
}
