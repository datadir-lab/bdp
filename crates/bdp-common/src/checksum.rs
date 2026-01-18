//! Checksum utilities for file verification

use crate::error::{BdpError, Result};
use crate::types::ChecksumAlgorithm;
use sha2::{Digest, Sha256, Sha512};
use std::io::Read;
use std::path::Path;

/// Compute checksum for a file
pub fn compute_file_checksum(
    path: impl AsRef<Path>,
    algorithm: ChecksumAlgorithm,
) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    compute_checksum(&mut file, algorithm)
}

/// Compute checksum for any readable source
pub fn compute_checksum<R: Read>(reader: &mut R, algorithm: ChecksumAlgorithm) -> Result<String> {
    match algorithm {
        ChecksumAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 8192];

            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }

            Ok(hex::encode(hasher.finalize()))
        },
        ChecksumAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            let mut buffer = [0u8; 8192];

            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }

            Ok(hex::encode(hasher.finalize()))
        },
    }
}

/// Verify checksum for a file
pub fn verify_file_checksum(
    path: impl AsRef<Path>,
    expected: &str,
    algorithm: ChecksumAlgorithm,
) -> Result<bool> {
    let actual = compute_file_checksum(path, algorithm)?;
    if actual == expected {
        Ok(true)
    } else {
        Err(BdpError::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_compute_checksum_sha256() {
        let data = b"hello world";
        let mut cursor = Cursor::new(data);
        let checksum = compute_checksum(&mut cursor, ChecksumAlgorithm::Sha256).unwrap();
        assert_eq!(checksum, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_compute_checksum_sha512() {
        let data = b"hello world";
        let mut cursor = Cursor::new(data);
        let checksum = compute_checksum(&mut cursor, ChecksumAlgorithm::Sha512).unwrap();
        assert_eq!(
            checksum,
            "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f"
        );
    }
}
