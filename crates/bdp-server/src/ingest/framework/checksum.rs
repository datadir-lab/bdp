//! MD5 checksum utilities for file verification

use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::AsyncReadExt;

/// Compute MD5 checksum of bytes
pub fn compute_md5(data: &[u8]) -> String {
    let digest = md5::compute(data);
    format!("{:x}", digest)
}

/// Compute MD5 checksum of a file
pub async fn compute_file_md5(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .await
        .context("Failed to read file")?;

    Ok(compute_md5(&buffer))
}

/// Verify MD5 checksum matches expected value
pub fn verify_md5(data: &[u8], expected_md5: &str) -> Result<bool> {
    let computed_md5 = compute_md5(data);
    Ok(computed_md5.eq_ignore_ascii_case(expected_md5))
}

/// Verify file MD5 checksum
pub async fn verify_file_md5(path: &Path, expected_md5: &str) -> Result<bool> {
    let computed_md5 = compute_file_md5(path).await?;
    Ok(computed_md5.eq_ignore_ascii_case(expected_md5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_md5() {
        let data = b"Hello, world!";
        let md5 = compute_md5(data);
        // MD5 of "Hello, world!"
        assert_eq!(md5, "6cd3556deb0da54bca060b4c39479839");
    }

    #[test]
    fn test_verify_md5() {
        let data = b"test data";
        let expected = "eb733a00c0c9d336e65691a37ab54293";
        assert!(verify_md5(data, expected).unwrap());

        let wrong = "wrong_md5_hash";
        assert!(!verify_md5(data, wrong).unwrap());
    }

    #[test]
    fn test_case_insensitive() {
        let data = b"test";
        let lowercase = "098f6bcd4621d373cade4e832627b4f6";
        let uppercase = "098F6BCD4621D373CADE4E832627B4F6";

        assert!(verify_md5(data, lowercase).unwrap());
        assert!(verify_md5(data, uppercase).unwrap());
    }
}
