//! Shared FTP utilities for data ingestion
//!
//! Provides common FTP operations with retry logic used across ingestion pipelines.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::ingest::common::ftp::{FtpConfig, FtpClient};
//!
//! let config = FtpConfig {
//!     host: "ftp.example.com".to_string(),
//!     port: 21,
//!     username: "anonymous".to_string(),
//!     password: "user@example.com".to_string(),
//! };
//!
//! let client = FtpClient::new(config);
//! let data = client.download_file("/pub/data.txt").await?;
//! ```

use anyhow::{Context, Result};
use std::io::Read;
use std::time::Duration;
use suppaftp::FtpStream;
use tracing::{debug, info, warn};

/// Maximum number of retry attempts for FTP operations
pub const MAX_RETRIES: u32 = 3;

/// Base delay between retry attempts (in seconds)
/// Actual delay is this value multiplied by attempt number (exponential backoff)
pub const RETRY_DELAY_SECS: u64 = 5;

/// Configuration for FTP connection
#[derive(Debug, Clone)]
pub struct FtpConfig {
    /// FTP server hostname
    pub host: String,

    /// FTP server port (usually 21)
    pub port: u16,

    /// FTP username (typically "anonymous" for public servers)
    pub username: String,

    /// FTP password (typically email for anonymous access)
    pub password: String,
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 21,
            username: "anonymous".to_string(),
            password: "user@example.com".to_string(),
        }
    }
}

/// Result type for FTP download operations with timestamp
pub struct DownloadResult {
    /// Downloaded file data
    pub data: Vec<u8>,

    /// File modification timestamp from FTP server (if available)
    pub timestamp: Option<String>,
}

/// Generic FTP client with retry logic
///
/// Provides common FTP operations used across different ingestion pipelines.
/// All operations use Extended Passive Mode (EPSV) for better NAT/firewall compatibility.
pub struct FtpClient {
    config: FtpConfig,
}

impl FtpClient {
    /// Create a new FTP client with the given configuration
    pub fn new(config: FtpConfig) -> Self {
        Self { config }
    }

    /// Download a file from the FTP server with retry logic
    ///
    /// # Arguments
    /// * `path` - Full path to the file on the FTP server
    ///
    /// # Returns
    /// The file contents as a byte vector
    pub async fn download_file(&self, path: &str) -> Result<Vec<u8>> {
        let result = self.download_file_with_metadata(path).await?;
        Ok(result.data)
    }

    /// Download a file with metadata (including timestamp)
    ///
    /// # Arguments
    /// * `path` - Full path to the file on the FTP server
    ///
    /// # Returns
    /// DownloadResult containing file data and optional timestamp
    pub async fn download_file_with_metadata(&self, path: &str) -> Result<DownloadResult> {
        let config = self.config.clone();
        let path = path.to_string();

        for attempt in 1..=MAX_RETRIES {
            debug!("Download attempt {}/{} for: {}", attempt, MAX_RETRIES, path);

            match tokio::task::spawn_blocking({
                let config = config.clone();
                let path = path.clone();
                move || Self::download_file_sync(&config, &path)
            })
            .await
            {
                Ok(Ok(result)) => {
                    info!("Successfully downloaded {} ({} bytes)", path, result.data.len());
                    return Ok(result);
                },
                Ok(Err(e)) => {
                    if attempt < MAX_RETRIES {
                        let delay = RETRY_DELAY_SECS * attempt as u64;
                        warn!(
                            "Download attempt {}/{} failed: {}. Retrying in {}s...",
                            attempt, MAX_RETRIES, e, delay
                        );
                        tokio::time::sleep(Duration::from_secs(delay)).await;
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

    /// List directory contents on the FTP server
    ///
    /// # Arguments
    /// * `path` - Directory path to list
    ///
    /// # Returns
    /// Vector of parsed directory entries
    pub async fn list_directory(&self, path: &str) -> Result<Vec<FtpEntry>> {
        let config = self.config.clone();
        let path = path.to_string();

        for attempt in 1..=MAX_RETRIES {
            debug!("LIST attempt {}/{} for: {}", attempt, MAX_RETRIES, path);

            match tokio::task::spawn_blocking({
                let config = config.clone();
                let path = path.clone();
                move || Self::list_directory_sync(&config, &path)
            })
            .await
            {
                Ok(Ok(entries)) => {
                    info!("Successfully listed {} ({} entries)", path, entries.len());
                    return Ok(entries);
                },
                Ok(Err(e)) => {
                    if attempt < MAX_RETRIES {
                        let delay = RETRY_DELAY_SECS * attempt as u64;
                        warn!(
                            "LIST attempt {}/{} failed: {}. Retrying in {}s...",
                            attempt, MAX_RETRIES, e, delay
                        );
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                    } else {
                        return Err(e).with_context(|| {
                            format!("Failed to list {} after {} attempts", path, MAX_RETRIES)
                        });
                    }
                },
                Err(e) => {
                    return Err(anyhow::anyhow!("FTP LIST task panicked: {}", e));
                },
            }
        }

        unreachable!("Retry loop should always return")
    }

    /// List only directories in a path
    pub async fn list_directories(&self, path: &str) -> Result<Vec<String>> {
        let entries = self.list_directory(path).await?;
        Ok(entries
            .into_iter()
            .filter(|e| e.is_directory)
            .map(|e| e.name)
            .collect())
    }

    /// List only files in a path
    pub async fn list_files(&self, path: &str) -> Result<Vec<String>> {
        let entries = self.list_directory(path).await?;
        Ok(entries
            .into_iter()
            .filter(|e| !e.is_directory)
            .map(|e| e.name)
            .collect())
    }

    /// Synchronous FTP download implementation
    fn download_file_sync(config: &FtpConfig, path: &str) -> Result<DownloadResult> {
        debug!("Connecting to FTP server: {}:{}", config.host, config.port);

        let mut ftp_stream = FtpStream::connect(format!("{}:{}", config.host, config.port))
            .context("Failed to connect to FTP server")?;

        // Use Extended Passive Mode - better for NAT/Docker environments
        ftp_stream.set_mode(suppaftp::Mode::ExtendedPassive);

        debug!("Logging in as: {}", config.username);
        ftp_stream
            .login(&config.username, &config.password)
            .context("Failed to login to FTP server")?;

        ftp_stream
            .transfer_type(suppaftp::types::FileType::Binary)
            .context("Failed to set binary mode")?;

        // Try to get modification time
        let timestamp = ftp_stream.mdtm(path).ok().map(|dt| dt.to_string());

        debug!("Downloading file: {}", path);
        let mut reader = ftp_stream
            .retr_as_buffer(path)
            .with_context(|| format!("Failed to download file: {}", path))?;

        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .context("Failed to read file data")?;

        debug!("Downloaded {} bytes from {}", data.len(), path);

        if let Err(e) = ftp_stream.quit() {
            warn!("Failed to quit FTP session gracefully: {}", e);
        }

        Ok(DownloadResult { data, timestamp })
    }

    /// Synchronous FTP directory listing
    fn list_directory_sync(config: &FtpConfig, path: &str) -> Result<Vec<FtpEntry>> {
        debug!("Connecting to FTP server: {}:{}", config.host, config.port);

        let mut ftp_stream = FtpStream::connect(format!("{}:{}", config.host, config.port))
            .context("Failed to connect to FTP server")?;

        ftp_stream.set_mode(suppaftp::Mode::ExtendedPassive);

        debug!("Logging in as: {}", config.username);
        ftp_stream
            .login(&config.username, &config.password)
            .context("FTP login failed")?;

        debug!("Listing directory: {}", path);
        let entries = ftp_stream
            .list(Some(path))
            .with_context(|| format!("Failed to list directory: {}", path))?;

        let parsed = entries
            .iter()
            .filter_map(|line| FtpEntry::parse(line))
            .collect();

        if let Err(e) = ftp_stream.quit() {
            warn!("Failed to quit FTP session gracefully: {}", e);
        }

        Ok(parsed)
    }
}

/// Parsed FTP directory entry
#[derive(Debug, Clone)]
pub struct FtpEntry {
    /// Entry name (filename or directory name)
    pub name: String,

    /// Whether this is a directory
    pub is_directory: bool,

    /// File size in bytes (if available)
    pub size: Option<u64>,
}

impl FtpEntry {
    /// Parse an FTP LIST line into an entry
    ///
    /// FTP LIST format varies, but typically:
    /// `drwxr-xr-x   2 ftp ftp  4096 Jan 15 12:00 dirname`
    /// `-rw-r--r--   1 ftp ftp  1234 Jan 15 12:00 filename.txt`
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let is_directory = parts[0].starts_with('d');
        let name = parts.last()?.to_string();

        // Try to parse size (usually the 5th field for Unix-style listings)
        let size = if parts.len() >= 5 {
            parts[4].parse().ok()
        } else {
            None
        };

        Some(Self {
            name,
            is_directory,
            size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directory_entry() {
        let entry =
            FtpEntry::parse("drwxr-xr-x   2 ftp ftp  4096 Jan 15 12:00 release-2025_01").unwrap();
        assert_eq!(entry.name, "release-2025_01");
        assert!(entry.is_directory);
        assert_eq!(entry.size, Some(4096));
    }

    #[test]
    fn test_parse_file_entry() {
        let entry =
            FtpEntry::parse("-rw-r--r--   1 ftp ftp  123456 Jan 15 12:00 data.txt").unwrap();
        assert_eq!(entry.name, "data.txt");
        assert!(!entry.is_directory);
        assert_eq!(entry.size, Some(123456));
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(FtpEntry::parse("").is_none());
        assert!(FtpEntry::parse("   ").is_none());
    }

    #[test]
    fn test_ftp_config_default() {
        let config = FtpConfig::default();
        assert_eq!(config.port, 21);
        assert_eq!(config.username, "anonymous");
    }
}
