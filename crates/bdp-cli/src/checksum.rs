//! Checksum computation and verification
//!
//! Uses SHA-256 for file integrity verification.

use crate::error::{CliError, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// Compute SHA-256 checksum of bytes
pub fn compute_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute SHA-256 checksum of a file
pub fn compute_file_checksum(path: impl AsRef<Path>) -> Result<String> {
    let mut file = std::fs::File::open(path.as_ref())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// Verify that data matches the expected checksum
pub fn verify_checksum(data: &[u8], expected: &str) -> Result<()> {
    let actual = compute_checksum(data);
    if actual == expected {
        Ok(())
    } else {
        Err(CliError::checksum_mismatch(
            "data",
            expected.to_string(),
            actual,
        ))
    }
}

/// Verify that a file matches the expected checksum
pub fn verify_file_checksum(path: impl AsRef<Path>, expected: &str) -> Result<()> {
    let path = path.as_ref();
    let actual = compute_file_checksum(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(CliError::checksum_mismatch(
            path.display().to_string(),
            expected.to_string(),
            actual,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_checksum() {
        let data = b"hello world";
        let checksum = compute_checksum(data);
        // SHA-256 of "hello world"
        assert_eq!(
            checksum,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_checksum_empty() {
        let data = b"";
        let checksum = compute_checksum(data);
        // SHA-256 of empty string
        assert_eq!(
            checksum,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_compute_file_checksum() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test data").unwrap();
        temp_file.flush().unwrap();

        let checksum = compute_file_checksum(temp_file.path()).unwrap();
        assert_eq!(
            checksum,
            "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9"
        );
    }

    #[test]
    fn test_verify_checksum_success() {
        let data = b"hello world";
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_checksum(data, expected).is_ok());
    }

    #[test]
    fn test_verify_checksum_failure() {
        let data = b"hello world";
        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_checksum(data, wrong_checksum);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::ChecksumMismatch { .. }));
    }

    #[test]
    fn test_verify_file_checksum_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test data").unwrap();
        temp_file.flush().unwrap();

        let expected = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";
        assert!(verify_file_checksum(temp_file.path(), expected).is_ok());
    }

    #[test]
    fn test_verify_file_checksum_failure() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test data").unwrap();
        temp_file.flush().unwrap();

        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_file_checksum(temp_file.path(), wrong_checksum);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::ChecksumMismatch { .. }));
    }

    #[test]
    fn test_compute_checksum_large_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Write 1MB of data
        let data = vec![0u8; 1024 * 1024];
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        let checksum = compute_file_checksum(temp_file.path()).unwrap();

        // Verify it computed a checksum (64 hex characters for SHA-256)
        assert_eq!(checksum.len(), 64);

        // Verify it's consistent - compute again and compare
        let checksum2 = compute_file_checksum(temp_file.path()).unwrap();
        assert_eq!(checksum, checksum2);

        // Also verify using the byte checksum matches
        let byte_checksum = compute_checksum(&data);
        assert_eq!(checksum, byte_checksum);
    }
}
