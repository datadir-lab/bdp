// GenBank/RefSeq FTP download functionality

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::io::{Cursor, Read};
use std::time::Duration;
use suppaftp::FtpStream;
use tracing::{debug, info, warn};

use super::config::GenbankFtpConfig;
use super::models::Division;

/// Maximum number of retry attempts for FTP operations
const MAX_RETRIES: u32 = 3;

/// Delay between retry attempts (in seconds)
const RETRY_DELAY_SECS: u64 = 5;

/// FTP client for downloading GenBank/RefSeq data
pub struct GenbankFtp {
    config: GenbankFtpConfig,
}

impl GenbankFtp {
    /// Create a new FTP client
    pub fn new(config: GenbankFtpConfig) -> Self {
        Self { config }
    }

    /// Get current release number
    pub async fn get_current_release(&self) -> Result<String> {
        let path = self.config.get_release_number_path();
        info!("Fetching current release number from: {}", path);

        let data = self.download_file(&path).await?;
        let release = String::from_utf8(data)
            .context("Failed to parse release number")?
            .trim()
            .to_string();

        info!("Current release: {}", release);
        Ok(release)
    }

    /// List all files for a division
    /// Returns list of (filename, size_bytes) tuples
    pub async fn list_division_files(&self, division: &Division) -> Result<Vec<(String, u64)>> {
        let base_path = self.config.get_base_path();
        let pattern = self.config.get_division_file_pattern(division);

        info!(
            "Listing files for division {} (pattern: {})",
            division.as_str(),
            pattern
        );

        let mut ftp = self.connect().await?;
        ftp.cwd(base_path)
            .context("Failed to change to GenBank directory")?;

        let list = ftp.list(None).context("Failed to list files")?;
        let mut files = Vec::new();

        // Parse FTP LIST output
        // Format: "-rw-r--r--   1 ftp anonymous 12345678 Jan 15 12:00 gbvrl1.seq.gz"
        for line in list {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 9 {
                continue;
            }

            let filename = parts[8];
            let size_str = parts[4];

            // Match files for this division
            if filename.starts_with(division.file_prefix()) && filename.ends_with(".seq.gz") {
                if let Ok(size) = size_str.parse::<u64>() {
                    files.push((filename.to_string(), size));
                }
            }
        }

        info!(
            "Found {} files for division {}",
            files.len(),
            division.as_str()
        );

        Ok(files)
    }

    /// Download a single GenBank file
    pub async fn download_division_file(&self, filename: &str) -> Result<Vec<u8>> {
        let base_path = self.config.get_base_path();
        let path = format!("{}/{}", base_path, filename);

        info!("Downloading: {}", filename);
        self.download_file(&path).await
    }

    /// Download and decompress a GenBank file
    pub async fn download_and_decompress(&self, filename: &str) -> Result<Vec<u8>> {
        let compressed = self.download_division_file(filename).await?;
        info!(
            "Decompressing {} ({} bytes compressed)",
            filename,
            compressed.len()
        );

        let cursor = Cursor::new(compressed);
        let mut decoder = GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .context("Failed to decompress file")?;

        info!(
            "Decompressed {} ({} bytes decompressed)",
            filename,
            decompressed.len()
        );

        Ok(decompressed)
    }

    /// Download all files for a division
    pub async fn download_division(&self, division: &Division) -> Result<Vec<(String, Vec<u8>)>> {
        let files = self.list_division_files(division).await?;
        let mut results = Vec::new();

        for (filename, size) in files {
            info!(
                "Downloading {} for division {} ({} bytes)",
                filename,
                division.as_str(),
                size
            );

            match self.download_and_decompress(&filename).await {
                Ok(data) => {
                    results.push((filename, data));
                }
                Err(e) => {
                    warn!("Failed to download {}: {}", filename, e);
                }
            }
        }

        Ok(results)
    }

    /// Download a file from FTP server (internal helper)
    async fn download_file(&self, path: &str) -> Result<Vec<u8>> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            match self.try_download_file(path).await {
                Ok(data) => return Ok(data),
                Err(e) if attempts < MAX_RETRIES => {
                    warn!(
                        "Download attempt {}/{} failed for {}: {}",
                        attempts, MAX_RETRIES, path, e
                    );
                    tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
                }
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to download {} after {} attempts",
                        path, MAX_RETRIES
                    ))
                }
            }
        }
    }

    /// Single attempt to download a file
    async fn try_download_file(&self, path: &str) -> Result<Vec<u8>> {
        let mut ftp = self.connect().await?;

        // Set binary mode
        ftp.transfer_type(suppaftp::types::FileType::Binary)
            .context("Failed to set binary mode")?;

        // Download file
        debug!("Retrieving file: {}", path);
        let cursor = ftp
            .retr_as_buffer(path)
            .context(format!("Failed to retrieve file: {}", path))?;

        Ok(cursor.into_inner())
    }

    /// Connect to FTP server
    async fn connect(&self) -> Result<FtpStream> {
        debug!(
            "Connecting to FTP server: {}:{}",
            self.config.host, self.config.port
        );

        let mut ftp = FtpStream::connect(format!("{}:{}", self.config.host, self.config.port))
            .context("Failed to connect to FTP server")?;

        // Note: Timeout is configured at the TCP socket level via FTP library defaults
        // set_read_timeout is not available in current suppaftp version

        // Login anonymously
        ftp.login("anonymous", "anonymous")
            .context("Failed to login")?;

        debug!("Successfully connected to FTP server");
        Ok(ftp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_paths() {
        let config = GenbankFtpConfig::new().with_genbank();
        assert_eq!(config.get_base_path(), "/genbank");

        let config = GenbankFtpConfig::new().with_refseq();
        assert_eq!(config.get_base_path(), "/refseq/release");
    }

    #[test]
    fn test_division_pattern() {
        let config = GenbankFtpConfig::new();
        assert_eq!(
            config.get_division_file_pattern(&Division::Viral),
            "gbvrl*.seq.gz"
        );
        assert_eq!(
            config.get_division_file_pattern(&Division::Phage),
            "gbphg*.seq.gz"
        );
    }
}
