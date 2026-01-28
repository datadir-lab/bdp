//! Shared decompression utilities for data ingestion
//!
//! Provides common decompression operations for various archive formats.
//!
//! # Supported Formats
//!
//! - **Gzip** (.gz): Using flate2
//! - **Tar** (.tar): Using tar crate
//! - **Tar.gz** (.tar.gz, .tgz): Combined gzip + tar
//! - **Zip** (.zip): Using zip crate
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::ingest::common::decompression::{decompress_gzip, extract_tar_gz};
//!
//! // Decompress a gzip file
//! let decompressed = decompress_gzip(&compressed_data)?;
//!
//! // Extract specific files from a tar.gz archive
//! let files = extract_tar_gz(&archive_data, &["data.txt", "metadata.json"])?;
//! ```

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use tracing::debug;

/// Decompress gzip-compressed data
///
/// # Arguments
/// * `data` - Gzip-compressed bytes
///
/// # Returns
/// Decompressed bytes
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .context("Failed to decompress gzip data")?;
    debug!("Decompressed {} -> {} bytes", data.len(), decompressed.len());
    Ok(decompressed)
}

/// Extract files from a tar archive
///
/// # Arguments
/// * `data` - Tar archive bytes (uncompressed)
/// * `filenames` - List of filenames to extract (empty = extract all)
///
/// # Returns
/// HashMap of filename -> file contents
pub fn extract_tar(data: &[u8], filenames: &[&str]) -> Result<HashMap<String, Vec<u8>>> {
    let cursor = Cursor::new(data);
    let mut archive = tar::Archive::new(cursor);
    let mut result = HashMap::new();

    let extract_all = filenames.is_empty();
    let filenames_set: std::collections::HashSet<&str> = filenames.iter().copied().collect();

    for entry_result in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry_result.context("Failed to read tar entry")?;
        let path = entry
            .path()
            .context("Failed to get entry path")?
            .to_string_lossy()
            .to_string();

        // Extract the filename part only
        let filename = path
            .split('/')
            .last()
            .unwrap_or(&path)
            .to_string();

        if extract_all || filenames_set.contains(filename.as_str()) {
            let mut contents = Vec::new();
            entry
                .read_to_end(&mut contents)
                .with_context(|| format!("Failed to read tar entry: {}", filename))?;
            debug!("Extracted {} ({} bytes)", filename, contents.len());
            result.insert(filename, contents);
        }
    }

    if !extract_all && result.len() < filenames.len() {
        let missing: Vec<_> = filenames
            .iter()
            .filter(|f| !result.contains_key(**f))
            .collect();
        anyhow::bail!("Missing files in tar archive: {:?}", missing);
    }

    Ok(result)
}

/// Extract files from a tar.gz archive (gzip-compressed tar)
///
/// # Arguments
/// * `data` - Gzip-compressed tar archive bytes
/// * `filenames` - List of filenames to extract (empty = extract all)
///
/// # Returns
/// HashMap of filename -> file contents
pub fn extract_tar_gz(data: &[u8], filenames: &[&str]) -> Result<HashMap<String, Vec<u8>>> {
    let decompressed = decompress_gzip(data)?;
    extract_tar(&decompressed, filenames)
}

/// Extract files from a zip archive
///
/// # Arguments
/// * `data` - Zip archive bytes
/// * `filenames` - List of filenames to extract (empty = extract all)
///
/// # Returns
/// HashMap of filename -> file contents
pub fn extract_zip(data: &[u8], filenames: &[&str]) -> Result<HashMap<String, Vec<u8>>> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to read zip archive")?;
    let mut result = HashMap::new();

    let extract_all = filenames.is_empty();
    let filenames_set: std::collections::HashSet<&str> = filenames.iter().copied().collect();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .with_context(|| format!("Failed to read zip entry at index {}", i))?;

        if file.is_dir() {
            continue;
        }

        let name = file.name().to_string();
        let filename = name
            .split('/')
            .last()
            .unwrap_or(&name)
            .to_string();

        if extract_all || filenames_set.contains(filename.as_str()) {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .with_context(|| format!("Failed to read zip entry: {}", filename))?;
            debug!("Extracted {} ({} bytes)", filename, contents.len());
            result.insert(filename, contents);
        }
    }

    if !extract_all && result.len() < filenames.len() {
        let missing: Vec<_> = filenames
            .iter()
            .filter(|f| !result.contains_key(**f))
            .collect();
        anyhow::bail!("Missing files in zip archive: {:?}", missing);
    }

    Ok(result)
}

/// Extract a single file from a tar.gz archive
///
/// # Arguments
/// * `data` - Gzip-compressed tar archive bytes
/// * `filename` - Name of the file to extract
///
/// # Returns
/// File contents as bytes
pub fn extract_single_tar_gz(data: &[u8], filename: &str) -> Result<Vec<u8>> {
    let files = extract_tar_gz(data, &[filename])?;
    files
        .into_iter()
        .next()
        .map(|(_, contents)| contents)
        .with_context(|| format!("File not found in archive: {}", filename))
}

/// Extract a single file from a zip archive
///
/// # Arguments
/// * `data` - Zip archive bytes
/// * `filename` - Name of the file to extract
///
/// # Returns
/// File contents as bytes
pub fn extract_single_zip(data: &[u8], filename: &str) -> Result<Vec<u8>> {
    let files = extract_zip(data, &[filename])?;
    files
        .into_iter()
        .next()
        .map(|(_, contents)| contents)
        .with_context(|| format!("File not found in archive: {}", filename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn create_gzip_data(content: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn test_decompress_gzip() {
        let original = b"Hello, World!";
        let compressed = create_gzip_data(original);
        let decompressed = decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_gzip_invalid() {
        let invalid = b"not gzip data";
        assert!(decompress_gzip(invalid).is_err());
    }
}
