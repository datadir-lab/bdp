//! NCBI Taxonomy FTP download functionality

use anyhow::{Context, Result};
use chrono::Datelike;
use flate2::read::GzDecoder;
use std::io::{Cursor, Read};
use std::time::Duration;
use suppaftp::FtpStream;
use tar::Archive;
use tracing::{debug, info, warn};
use zip::ZipArchive;

use super::config::NcbiTaxonomyFtpConfig;
use crate::ingest::common::ftp::{MAX_RETRIES, RETRY_DELAY_SECS};

/// FTP client for downloading NCBI Taxonomy data
pub struct NcbiTaxonomyFtp {
    config: NcbiTaxonomyFtpConfig,
}

impl NcbiTaxonomyFtp {
    /// Create a new FTP client
    pub fn new(config: NcbiTaxonomyFtpConfig) -> Self {
        Self { config }
    }

    /// Download and extract taxdump files (current version)
    ///
    /// # Returns
    /// A tuple containing:
    /// - rankedlineage.dmp contents
    /// - merged.dmp contents
    /// - delnodes.dmp contents
    /// - external_version (FTP file modification timestamp)
    pub async fn download_taxdump(&self) -> Result<TaxdumpFiles> {
        self.download_taxdump_version(None).await
    }

    /// Download and extract taxdump files (specific version)
    ///
    /// # Arguments
    /// * `version` - Optional archive date (e.g., "2024-01-01")
    ///   - `None` for current version
    ///   - `Some("2024-01-01")` for historical version from archive
    ///
    /// # Returns
    /// TaxdumpFiles with extracted contents and external_version
    pub async fn download_taxdump_version(&self, version: Option<&str>) -> Result<TaxdumpFiles> {
        let (path, is_zip) = if let Some(ver) = version {
            // Historical archive: taxdump_archive/new_taxdump_YYYY-MM-DD.zip
            (self.config.archive_path(ver), true)
        } else {
            // Current version: new_taxdump/new_taxdump.tar.gz
            (self.config.taxdump_path(), false)
        };

        info!("Downloading taxdump from: {}", path);
        let (compressed, file_timestamp) = self.download_file_with_timestamp(&path).await?;

        // For historical archives, use the version from filename, not file timestamp
        let external_version = if let Some(ver) = version {
            ver.to_string()
        } else {
            file_timestamp
        };

        info!("Downloaded taxdump version {} ({} bytes)", external_version, compressed.len());

        info!("Decompressing and extracting taxdump archive");
        let taxdump_files = if is_zip {
            self.extract_taxdump_zip(&compressed)?
        } else {
            self.extract_taxdump_targz(&compressed)?
        };

        Ok(TaxdumpFiles {
            rankedlineage: taxdump_files.rankedlineage,
            merged: taxdump_files.merged,
            delnodes: taxdump_files.delnodes,
            external_version,
        })
    }

    /// List all available taxdump archive versions
    ///
    /// Returns a sorted list of archive dates (oldest to newest)
    /// e.g., ["2024-01-01", "2024-02-01", "2024-03-01", ...]
    pub async fn list_available_versions(&self) -> Result<Vec<String>> {
        let archive_dir = "/pub/taxonomy/taxdump_archive";

        info!("Listing available taxdump archives from: {}", archive_dir);
        let files = self.list_directory_files(archive_dir).await?;

        // Parse archive filenames: new_taxdump_YYYY-MM-DD.zip
        let mut versions: Vec<String> = files
            .iter()
            .filter_map(|filename| {
                if filename.starts_with("new_taxdump_") && filename.ends_with(".zip") {
                    // Extract date: new_taxdump_2024-01-01.zip -> 2024-01-01
                    let date = filename
                        .strip_prefix("new_taxdump_")?
                        .strip_suffix(".zip")?;
                    Some(date.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Sort chronologically (oldest first)
        versions.sort();

        info!("Found {} taxdump archive versions", versions.len());
        debug!("Available versions: {:?}", versions);

        Ok(versions)
    }

    /// List files in an FTP directory
    ///
    /// # Arguments
    /// * `path` - Directory path to list
    ///
    /// # Returns
    /// Vector of filenames (not full paths)
    async fn list_directory_files(&self, path: &str) -> Result<Vec<String>> {
        let config = self.config.clone();
        let path = path.to_string();

        // Retry loop for FTP listing
        for attempt in 1..=MAX_RETRIES {
            match tokio::task::spawn_blocking({
                let config = config.clone();
                let path = path.clone();
                move || Self::list_directory_sync(&config, &path)
            })
            .await
            {
                Ok(Ok(files)) => {
                    info!("Successfully listed directory {} ({} files)", path, files.len());
                    return Ok(files);
                },
                Ok(Err(e)) => {
                    if attempt < MAX_RETRIES {
                        warn!(
                            "List attempt {}/{} failed: {}. Retrying in {}s...",
                            attempt, MAX_RETRIES, e, RETRY_DELAY_SECS
                        );
                        tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS * attempt as u64))
                            .await;
                    } else {
                        return Err(e).with_context(|| {
                            format!("Failed to list {} after {} attempts", path, MAX_RETRIES)
                        });
                    }
                },
                Err(e) => {
                    return Err(anyhow::anyhow!("FTP list task panicked: {}", e));
                },
            }
        }

        unreachable!("Retry loop should always return")
    }

    /// Synchronous FTP directory listing
    fn list_directory_sync(config: &NcbiTaxonomyFtpConfig, path: &str) -> Result<Vec<String>> {
        debug!("Connecting to FTP server: {}:{}", config.ftp_host, config.ftp_port);

        // Connect to FTP server
        let mut ftp_stream = FtpStream::connect(format!("{}:{}", config.ftp_host, config.ftp_port))
            .context("Failed to connect to FTP server")?;

        // Use Extended Passive Mode (EPSV)
        ftp_stream.set_mode(suppaftp::Mode::ExtendedPassive);

        debug!("Logging in as: {}", config.ftp_username);

        // Login
        ftp_stream
            .login(&config.ftp_username, &config.ftp_password)
            .context("FTP login failed")?;

        debug!("Listing directory: {}", path);

        // List directory
        let entries = ftp_stream
            .list(Some(path))
            .with_context(|| format!("Failed to list directory: {}", path))?;

        // Parse filenames from listing
        // FTP LIST format: "-rw-r--r--   1 ftp  anonymous  134217728 Jan 01 12:00 taxdmp_2024-01-01.tar.gz"
        let files: Vec<String> = entries
            .iter()
            .filter_map(|entry| {
                // Split by whitespace and take last field (filename)
                entry.split_whitespace().last().map(|s| s.to_string())
            })
            .collect();

        // Logout
        let _ = ftp_stream.quit();

        debug!("Listed {} files from {}", files.len(), path);

        Ok(files)
    }

    /// Extract required files from taxdump tar.gz archive
    fn extract_taxdump_targz(&self, compressed: &[u8]) -> Result<ExtractedFiles> {
        // Decompress gzip
        let mut decoder = GzDecoder::new(compressed);

        // Extract tar archive
        let mut archive = Archive::new(&mut decoder);

        let mut rankedlineage = None;
        let mut merged = None;
        let mut delnodes = None;

        for entry in archive.entries().context("Failed to read tar archive")? {
            let mut entry = entry.context("Failed to read tar entry")?;
            let path = entry.path().context("Failed to get entry path")?;
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Invalid filename in archive")?;

            match filename {
                "rankedlineage.dmp" => {
                    let mut content = String::new();
                    entry
                        .read_to_string(&mut content)
                        .context("Failed to read rankedlineage.dmp")?;
                    let len = content.len();
                    rankedlineage = Some(content);
                    debug!("Extracted rankedlineage.dmp ({} bytes)", len);
                },
                "merged.dmp" => {
                    let mut content = String::new();
                    entry
                        .read_to_string(&mut content)
                        .context("Failed to read merged.dmp")?;
                    let len = content.len();
                    merged = Some(content);
                    debug!("Extracted merged.dmp ({} bytes)", len);
                },
                "delnodes.dmp" => {
                    let mut content = String::new();
                    entry
                        .read_to_string(&mut content)
                        .context("Failed to read delnodes.dmp")?;
                    let len = content.len();
                    delnodes = Some(content);
                    debug!("Extracted delnodes.dmp ({} bytes)", len);
                },
                _ => {
                    // Skip other files
                    debug!("Skipping file: {}", filename);
                },
            }

            // Early exit if we have all required files
            if rankedlineage.is_some() && merged.is_some() && delnodes.is_some() {
                break;
            }
        }

        Ok(ExtractedFiles {
            rankedlineage: rankedlineage.context("rankedlineage.dmp not found in archive")?,
            merged: merged.context("merged.dmp not found in archive")?,
            delnodes: delnodes.context("delnodes.dmp not found in archive")?,
        })
    }

    /// Extract required files from taxdump .zip archive
    fn extract_taxdump_zip(&self, compressed: &[u8]) -> Result<ExtractedFiles> {
        // Create a cursor from the compressed data
        let cursor = Cursor::new(compressed);

        // Open zip archive
        let mut archive = ZipArchive::new(cursor).context("Failed to open zip archive")?;

        let mut rankedlineage = None;
        let mut merged = None;
        let mut delnodes = None;

        // Iterate through files in zip
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("Failed to read zip entry")?;

            let filename = file.name().to_string();

            // Extract just the filename (handle paths like "taxdump/rankedlineage.dmp")
            let basename = filename.split('/').next_back().unwrap_or(&filename);

            match basename {
                "rankedlineage.dmp" => {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .context("Failed to read rankedlineage.dmp")?;
                    let len = content.len();
                    rankedlineage = Some(content);
                    debug!("Extracted rankedlineage.dmp ({} bytes)", len);
                },
                "merged.dmp" => {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .context("Failed to read merged.dmp")?;
                    let len = content.len();
                    merged = Some(content);
                    debug!("Extracted merged.dmp ({} bytes)", len);
                },
                "delnodes.dmp" => {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .context("Failed to read delnodes.dmp")?;
                    let len = content.len();
                    delnodes = Some(content);
                    debug!("Extracted delnodes.dmp ({} bytes)", len);
                },
                _ => {
                    // Skip other files
                    debug!("Skipping file: {}", basename);
                },
            }

            // Early exit if we have all required files
            if rankedlineage.is_some() && merged.is_some() && delnodes.is_some() {
                break;
            }
        }

        Ok(ExtractedFiles {
            rankedlineage: rankedlineage.context("rankedlineage.dmp not found in archive")?,
            merged: merged.context("merged.dmp not found in archive")?,
            delnodes: delnodes.context("delnodes.dmp not found in archive")?,
        })
    }

    /// Download a file and get its modification timestamp
    async fn download_file_with_timestamp(&self, path: &str) -> Result<(Vec<u8>, String)> {
        let config = self.config.clone();
        let path = path.to_string();

        // Retry loop
        for attempt in 1..=MAX_RETRIES {
            debug!("Download attempt {}/{} for: {}", attempt, MAX_RETRIES, path);

            match tokio::task::spawn_blocking({
                let config = config.clone();
                let path = path.clone();
                move || Self::download_file_with_timestamp_sync(&config, &path)
            })
            .await
            {
                Ok(Ok(result)) => {
                    info!("Successfully downloaded {} ({} bytes)", path, result.0.len());
                    return Ok(result);
                },
                Ok(Err(e)) => {
                    if attempt < MAX_RETRIES {
                        warn!(
                            "Download attempt {}/{} failed: {}. Retrying in {}s...",
                            attempt, MAX_RETRIES, e, RETRY_DELAY_SECS
                        );
                        tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS * attempt as u64))
                            .await;
                    } else {
                        return Err(e).with_context(|| {
                            format!("Failed to download {} after {} attempts", path, MAX_RETRIES)
                        });
                    }
                },
                Err(e) => {
                    return Err(anyhow::anyhow!("FTP download task panicked: {}", e));
                },
            }
        }

        unreachable!("Retry loop should always return")
    }

    /// Synchronous FTP download with timestamp retrieval
    fn download_file_with_timestamp_sync(
        config: &NcbiTaxonomyFtpConfig,
        path: &str,
    ) -> Result<(Vec<u8>, String)> {
        debug!("Connecting to FTP server: {}:{}", config.ftp_host, config.ftp_port);

        // Connect to FTP server
        let mut ftp_stream = FtpStream::connect(format!("{}:{}", config.ftp_host, config.ftp_port))
            .context("Failed to connect to FTP server")?;

        // Use Extended Passive Mode (EPSV) - better for NAT/Docker environments
        ftp_stream.set_mode(suppaftp::Mode::ExtendedPassive);

        debug!("FTP connection established with Extended Passive Mode (EPSV)");
        debug!("Logging in as: {}", config.ftp_username);

        // Login
        ftp_stream
            .login(&config.ftp_username, &config.ftp_password)
            .context("FTP login failed")?;

        debug!("FTP login successful");

        // Get file modification timestamp (MDTM command)
        let timestamp = ftp_stream
            .mdtm(path)
            .context("Failed to get file modification time")?;

        // Format timestamp as YYYY-MM-DD
        let external_version =
            format!("{:04}-{:02}-{:02}", timestamp.year(), timestamp.month(), timestamp.day());

        debug!("File modification time: {}", external_version);

        // Download file
        debug!("Downloading file: {}", path);
        let cursor = ftp_stream
            .retr_as_buffer(path)
            .with_context(|| format!("Failed to download file: {}", path))?;

        let data = cursor.into_inner();
        debug!("Downloaded {} bytes from {}", data.len(), path);

        // Logout
        let _ = ftp_stream.quit();

        Ok((data, external_version))
    }
}

/// Extracted taxdump files with external version
#[derive(Debug)]
pub struct TaxdumpFiles {
    pub rankedlineage: String,
    pub merged: String,
    pub delnodes: String,
    pub external_version: String,
}

/// Internal struct for extracted files during processing
#[derive(Debug)]
struct ExtractedFiles {
    rankedlineage: String,
    merged: String,
    delnodes: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_taxdump_format() {
        // Test that we can create the config and paths
        let config = NcbiTaxonomyFtpConfig::new();
        let _ftp = NcbiTaxonomyFtp::new(config.clone());

        // Verify paths are constructed correctly
        assert!(config.taxdump_path().contains("new_taxdump.tar.gz"));
    }
}
