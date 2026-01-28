//! UniProt FTP download functionality

use anyhow::{Context, Result};
use chrono::NaiveDate;
use flate2::read::GzDecoder;
use std::io::Read;
use std::time::Duration;
use suppaftp::FtpStream;
use tracing::{debug, info, warn};

use super::config::UniProtFtpConfig;
use super::models::ReleaseInfo;
use crate::ingest::common::ftp::{MAX_RETRIES, RETRY_DELAY_SECS};

/// FTP client for downloading UniProt data
pub struct UniProtFtp {
    config: UniProtFtpConfig,
}

impl UniProtFtp {
    /// Create a new FTP client
    pub fn new(config: UniProtFtpConfig) -> Self {
        Self { config }
    }

    /// Download release notes
    ///
    /// # Arguments
    /// * `version` - Optional release version (e.g., "2024_01")
    ///   - `None` for current release
    ///   - `Some("2024_01")` for specific previous release
    ///
    /// # Returns
    /// The contents of the relnotes.txt file
    pub async fn download_release_notes(&self, version: Option<&str>) -> Result<String> {
        let path = self.config.release_notes_path(version)
            .context("Failed to build release notes path")?;
        let data = self.download_file(&path).await?;
        String::from_utf8(data).context("Release notes are not valid UTF-8")
    }

    /// Parse release information from relnotes.txt content
    ///
    /// # Format
    /// Release notes contain lines like:
    /// - "Swiss-Prot Release 2024_01 of 15-Jan-2024"
    /// - "UniProtKB/Swiss-Prot Release 2024_01 consists of 571609 sequence entries"
    pub fn parse_release_notes(&self, content: &str) -> Result<ReleaseInfo> {
        let mut external_version = None;
        let mut release_date = None;
        let mut swissprot_count = None;

        for line in content.lines() {
            // Parse release version and date
            // Example: "Swiss-Prot Release 2024_01 of 15-Jan-2024"
            if line.contains("Swiss-Prot Release") && line.contains(" of ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for i in 0..parts.len() {
                    if parts[i] == "Release" && i + 1 < parts.len() {
                        external_version = Some(parts[i + 1].to_string());
                    }
                    if parts[i] == "of" && i + 1 < parts.len() {
                        let date_str = parts[i + 1].trim_end_matches(&['.', ',', ';'][..]);
                        release_date = Some(self.parse_release_date(date_str)?);
                    }
                }
            }

            // Parse SwissProt count
            // Example: "UniProtKB/Swiss-Prot Release 2024_01 consists of 571609 sequence entries"
            if line.contains("consists of") && line.contains("sequence entries") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for i in 0..parts.len() {
                    if parts[i] == "of" && i + 1 < parts.len() {
                        if let Ok(count) = parts[i + 1].parse::<u64>() {
                            swissprot_count = Some(count);
                            break;
                        }
                    }
                }
            }
        }

        let external_version = external_version
            .context("Could not find release version in release notes")?;
        let release_date = release_date
            .context("Could not find release date in release notes")?;
        let swissprot_count = swissprot_count
            .context("Could not find SwissProt count in release notes")?;

        Ok(ReleaseInfo::new(external_version, release_date, swissprot_count))
    }

    /// Download and decompress a DAT file
    ///
    /// # Arguments
    /// * `version` - Optional release version (e.g., "2024_01")
    ///   - `None` for current release
    ///   - `Some("2024_01")` for specific previous release
    /// * `dataset` - Optional dataset type (default: "sprot")
    ///   - `None` or `Some("sprot")` for Swiss-Prot (curated)
    ///   - `Some("trembl")` for TrEMBL (unreviewed)
    ///
    /// # Returns
    /// Decompressed DAT file contents
    pub async fn download_dat_file(&self, version: Option<&str>, dataset: Option<&str>) -> Result<Vec<u8>> {
        let path = self.config.dat_file_path(version, dataset)
            .context("Failed to build DAT file path")?;
        let compressed = self.download_file(&path).await?;

        // Decompress gzip
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .context("Failed to decompress DAT file")?;

        Ok(decompressed)
    }

    /// Check if a release exists on the FTP server
    ///
    /// # Arguments
    /// * `version` - Optional release version (e.g., "2024_01")
    ///   - `None` for current release (always exists)
    ///   - `Some("2024_01")` to check if specific previous release exists
    pub async fn check_version_exists(&self, version: Option<&str>) -> Result<bool> {
        let path = self.config.release_notes_path(version)
            .context("Failed to build release notes path")?;

        // Try to download release notes - if it exists, version exists
        match self.download_file(&path).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Download a file from the FTP server using synchronous FTP with retry logic
    async fn download_file(&self, path: &str) -> Result<Vec<u8>> {
        let config = self.config.clone();
        let path = path.to_string();

        // Retry loop
        for attempt in 1..=MAX_RETRIES {
            debug!("Download attempt {}/{} for: {}", attempt, MAX_RETRIES, path);

            match tokio::task::spawn_blocking({
                let config = config.clone();
                let path = path.clone();
                move || Self::download_file_sync(&config, &path)
            })
            .await
            {
                Ok(Ok(data)) => {
                    info!("Successfully downloaded {} ({} bytes)", path, data.len());
                    return Ok(data);
                }
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
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("FTP download task panicked: {}", e));
                }
            }
        }

        unreachable!("Retry loop should always return")
    }

    /// Synchronous FTP download implementation
    ///
    /// Note: suppaftp doesn't support timeout configuration directly.
    /// The tokio task timeout wrapper in download_file() provides overall timeout protection.
    fn download_file_sync(config: &UniProtFtpConfig, path: &str) -> Result<Vec<u8>> {
        debug!("Connecting to FTP server: {}:{}", config.ftp_host, config.ftp_port);

        // Connect to FTP server
        let mut ftp_stream = FtpStream::connect(format!("{}:{}", config.ftp_host, config.ftp_port))
            .context("Failed to connect to FTP server")?;

        // Use Extended Passive Mode (EPSV) - better for NAT/Docker environments
        // EPSV is defined in RFC 2428 and works better through firewalls/NAT than standard PASV
        ftp_stream.set_mode(suppaftp::Mode::ExtendedPassive);

        debug!("FTP connection established with Extended Passive Mode (EPSV)");
        debug!("Logging in as: {}", config.ftp_username);

        // Login
        ftp_stream
            .login(&config.ftp_username, &config.ftp_password)
            .context("Failed to login to FTP server")?;

        debug!("Setting binary transfer mode");

        // Set binary mode
        ftp_stream
            .transfer_type(suppaftp::types::FileType::Binary)
            .context("Failed to set binary mode")?;

        debug!("Downloading file: {}", path);

        // Download file
        let mut reader = ftp_stream
            .retr_as_buffer(path)
            .with_context(|| format!("Failed to download file: {}", path))?;

        let mut data = Vec::new();
        let bytes_read = reader
            .read_to_end(&mut data)
            .context("Failed to read file data")?;

        debug!("Downloaded {} bytes from {}", bytes_read, path);

        // Quit gracefully
        if let Err(e) = ftp_stream.quit() {
            warn!("Failed to quit FTP session gracefully: {}", e);
        }

        Ok(data)
    }

    /// List directories in a given FTP path
    ///
    /// # Arguments
    /// * `path` - The FTP directory path to list
    ///
    /// # Returns
    /// Vector of directory names (not full paths, just the names)
    pub async fn list_directories(&self, path: &str) -> Result<Vec<String>> {
        let config = self.config.clone();
        let path = path.to_string();

        // Retry loop for FTP listing
        for attempt in 1..=MAX_RETRIES {
            debug!("FTP LIST attempt {}/{} for: {}", attempt, MAX_RETRIES, path);

            match tokio::task::spawn_blocking({
                let config = config.clone();
                let path = path.clone();
                move || Self::list_directories_sync(&config, &path)
            })
            .await
            {
                Ok(Ok(dirs)) => {
                    info!("Successfully listed {} directories in {}", dirs.len(), path);
                    return Ok(dirs);
                }
                Ok(Err(e)) => {
                    if attempt < MAX_RETRIES {
                        warn!(
                            "FTP LIST attempt {}/{} failed: {}. Retrying in {}s...",
                            attempt, MAX_RETRIES, e, RETRY_DELAY_SECS
                        );
                        tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS * attempt as u64))
                            .await;
                    } else {
                        return Err(e).with_context(|| {
                            format!("Failed to list {} after {} attempts", path, MAX_RETRIES)
                        });
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("FTP LIST task panicked: {}", e));
                }
            }
        }

        unreachable!("Retry loop should always return")
    }

    /// Synchronous FTP directory listing implementation
    fn list_directories_sync(config: &UniProtFtpConfig, path: &str) -> Result<Vec<String>> {
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
            .context("Failed to login to FTP server")?;

        debug!("Listing directory: {}", path);

        // List directory contents
        let entries = ftp_stream
            .list(Some(path))
            .with_context(|| format!("Failed to list directory: {}", path))?;

        debug!("Received {} entries from FTP LIST", entries.len());

        // Parse directory names from LIST output
        let mut directories = Vec::new();
        for entry in entries {
            debug!("Parsing FTP LIST entry: {}", entry);

            // FTP LIST format varies, but typically:
            // drwxr-xr-x   2 ftp      ftp          4096 Jan 15 12:00 release-2025_01
            // We need to extract directory names (entries starting with 'd')
            let parts: Vec<&str> = entry.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            // Check if this is a directory (first char is 'd')
            if parts[0].starts_with('d') {
                // Directory name is typically the last part
                if let Some(name) = parts.last() {
                    directories.push(name.to_string());
                }
            }
        }

        debug!("Extracted {} directories from listing", directories.len());

        // Quit gracefully
        if let Err(e) = ftp_stream.quit() {
            warn!("Failed to quit FTP session gracefully: {}", e);
        }

        Ok(directories)
    }

    /// Parse release date in format "15-Jan-2024"
    fn parse_release_date(&self, date_str: &str) -> Result<NaiveDate> {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid date format: {}", date_str);
        }

        let day: u32 = parts[0].parse().context("Failed to parse day")?;
        let month = match parts[1] {
            "Jan" => 1,
            "Feb" => 2,
            "Mar" => 3,
            "Apr" => 4,
            "May" => 5,
            "Jun" => 6,
            "Jul" => 7,
            "Aug" => 8,
            "Sep" => 9,
            "Oct" => 10,
            "Nov" => 11,
            "Dec" => 12,
            _ => anyhow::bail!("Invalid month: {}", parts[1]),
        };
        let year: i32 = parts[2].parse().context("Failed to parse year")?;

        NaiveDate::from_ymd_opt(year, month, day)
            .with_context(|| format!("Invalid date: {}", date_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_release_date() {
        let ftp = UniProtFtp::new(UniProtFtpConfig::default());

        let date = ftp.parse_release_date("15-Jan-2024").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);

        let date = ftp.parse_release_date("31-Dec-2023").unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 31);
    }

    #[test]
    fn test_parse_release_date_invalid() {
        let ftp = UniProtFtp::new(UniProtFtpConfig::default());
        assert!(ftp.parse_release_date("invalid").is_err());
        assert!(ftp.parse_release_date("15-XXX-2024").is_err());
    }

    #[test]
    fn test_parse_release_notes() {
        let ftp = UniProtFtp::new(UniProtFtpConfig::default());
        let content = r#"
                          UniProt Knowledgebase Release 2024_01

        Swiss-Prot Release 2024_01 of 15-Jan-2024

UniProtKB/Swiss-Prot Release 2024_01 consists of 571609 sequence entries,
comprising 206391219 amino acids abstracted from 284352 references.
        "#;

        let info = ftp.parse_release_notes(content).unwrap();
        assert_eq!(info.external_version, "2024_01");
        assert_eq!(info.release_date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(info.swissprot_count, 571609);
    }

    #[test]
    fn test_parse_release_notes_missing_version() {
        let ftp = UniProtFtp::new(UniProtFtpConfig::default());
        let content = "No version information here";
        assert!(ftp.parse_release_notes(content).is_err());
    }

    #[test]
    fn test_parse_release_notes_missing_count() {
        let ftp = UniProtFtp::new(UniProtFtpConfig::default());
        let content = "Swiss-Prot Release 2024_01 of 15-Jan-2024";
        assert!(ftp.parse_release_notes(content).is_err());
    }

    #[test]
    fn test_parse_ftp_list_entries() {
        // Test parsing FTP LIST output format
        let entries = vec![
            "drwxr-xr-x   2 ftp      ftp          4096 Jan 15 12:00 release-2025_01".to_string(),
            "drwxr-xr-x   2 ftp      ftp          4096 Dec 15 12:00 release-2024_12".to_string(),
            "-rw-r--r--   1 ftp      ftp          1234 Jan 15 12:00 README.txt".to_string(),
            "drwxr-xr-x   2 ftp      ftp          4096 Nov 15 12:00 release-2024_11".to_string(),
        ];

        let mut directories = Vec::new();
        for entry in entries {
            let parts: Vec<&str> = entry.split_whitespace().collect();
            if !parts.is_empty() && parts[0].starts_with('d') {
                if let Some(name) = parts.last() {
                    directories.push(name.to_string());
                }
            }
        }

        assert_eq!(directories.len(), 3);
        assert!(directories.contains(&"release-2025_01".to_string()));
        assert!(directories.contains(&"release-2024_12".to_string()));
        assert!(directories.contains(&"release-2024_11".to_string()));
        assert!(!directories.contains(&"README.txt".to_string()));
    }

    #[test]
    fn test_parse_empty_ftp_list() {
        let entries: Vec<String> = vec![];
        let mut directories = Vec::new();

        for entry in entries {
            let parts: Vec<&str> = entry.split_whitespace().collect();
            if !parts.is_empty() && parts[0].starts_with('d') {
                if let Some(name) = parts.last() {
                    directories.push(name.to_string());
                }
            }
        }

        assert_eq!(directories.len(), 0);
    }
}
