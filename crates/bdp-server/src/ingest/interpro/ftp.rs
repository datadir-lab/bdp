// InterPro FTP Downloader
//
// Downloads protein2ipr.dat.gz and entry.list files from InterPro FTP server

use crate::error::Error;
use crate::ingest::interpro::config::InterProConfig;
use std::path::{Path, PathBuf};
use suppaftp::FtpStream;
use tracing::{debug, info};

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a string matches the version format (e.g., "96.0", "100.0")
fn is_version_format(s: &str) -> bool {
    // Must have exactly one dot
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        return false;
    }

    // Both parts must be numeric
    parts[0].chars().all(|c| c.is_ascii_digit())
        && parts[1].chars().all(|c| c.is_ascii_digit())
}

// ============================================================================
// FTP Downloader
// ============================================================================

/// InterPro FTP downloader
pub struct InterProFtpDownloader {
    config: InterProConfig,
    ftp: Option<FtpStream>,
}

impl InterProFtpDownloader {
    /// Create a new FTP downloader
    pub fn new(config: InterProConfig) -> Self {
        Self { config, ftp: None }
    }

    /// Connect to FTP server
    pub fn connect(&mut self) -> Result<(), Error> {
        info!("Connecting to InterPro FTP server: {}", self.config.ftp_host);

        let mut ftp = FtpStream::connect(format!("{}:21", self.config.ftp_host))
            .map_err(|e| Error::Other(format!("FTP connection failed: {}", e)))?;

        // Anonymous login
        ftp.login("anonymous", "anonymous")
            .map_err(|e| Error::Other(format!("FTP login failed: {}", e)))?;

        info!("Successfully connected to InterPro FTP");

        self.ftp = Some(ftp);
        Ok(())
    }

    /// Disconnect from FTP server
    pub fn disconnect(&mut self) -> Result<(), Error> {
        if let Some(mut ftp) = self.ftp.take() {
            ftp.quit()
                .map_err(|e| Error::Other(format!("FTP disconnect failed: {}", e)))?;
            info!("Disconnected from InterPro FTP");
        }
        Ok(())
    }

    /// Get current release version from FTP
    pub fn get_current_version(&mut self) -> Result<String, Error> {
        let ftp = self
            .ftp
            .as_mut()
            .ok_or_else(|| Error::Other("Not connected to FTP".to_string()))?;

        // Read current release version from release_notes.txt or directory listing
        let release_path = self.config.get_current_release_path();

        ftp.cwd(&release_path)
            .map_err(|e| Error::Other(format!("Failed to change to release directory: {}", e)))?;

        // List files to find version
        let list = ftp
            .list(None)
            .map_err(|e| Error::Other(format!("Failed to list files: {}", e)))?;

        debug!("Release directory contents: {:?}", list);

        // For now, return a placeholder - real implementation would parse release_notes.txt
        Ok("95.0".to_string())
    }

    /// Download protein2ipr.dat.gz file
    pub fn download_protein2ipr(
        &mut self,
        version: &str,
        output_dir: &Path,
    ) -> Result<PathBuf, Error> {
        let ftp = self
            .ftp
            .as_mut()
            .ok_or_else(|| Error::Other("Not connected to FTP".to_string()))?;

        let remote_path = self.config.get_protein2ipr_path(version);
        let local_path = output_dir.join(format!("protein2ipr_{}.dat.gz", version));

        info!(
            "Downloading protein2ipr.dat.gz from {} to {:?}",
            remote_path, local_path
        );

        // Change to directory
        let dir_path = format!("{}{}", self.config.ftp_path, version);
        ftp.cwd(&dir_path)
            .map_err(|e| Error::Other(format!("Failed to change directory: {}", e)))?;

        // Download file
        let mut reader = ftp
            .retr_as_buffer("protein2ipr.dat.gz")
            .map_err(|e| Error::Other(format!("Failed to download file: {}", e)))?;

        // Write to local file
        let mut file = std::fs::File::create(&local_path)
            .map_err(|e| Error::Other(format!("Failed to create output file: {}", e)))?;

        std::io::copy(&mut reader, &mut file)
            .map_err(|e| Error::Other(format!("Failed to write file: {}", e)))?;

        info!(
            "Successfully downloaded protein2ipr.dat.gz ({} bytes)",
            file.metadata()
                .map(|m| m.len())
                .unwrap_or(0)
        );

        Ok(local_path)
    }

    /// Download entry.list file
    pub fn download_entry_list(
        &mut self,
        version: &str,
        output_dir: &Path,
    ) -> Result<PathBuf, Error> {
        let ftp = self
            .ftp
            .as_mut()
            .ok_or_else(|| Error::Other("Not connected to FTP".to_string()))?;

        let remote_path = self.config.get_entry_list_path(version);
        let local_path = output_dir.join(format!("entry_list_{}.txt", version));

        info!(
            "Downloading entry.list from {} to {:?}",
            remote_path, local_path
        );

        // Change to directory
        let dir_path = format!("{}{}", self.config.ftp_path, version);
        ftp.cwd(&dir_path)
            .map_err(|e| Error::Other(format!("Failed to change directory: {}", e)))?;

        // Download file
        let mut reader = ftp
            .retr_as_buffer("entry.list")
            .map_err(|e| Error::Other(format!("Failed to download file: {}", e)))?;

        // Write to local file
        let mut file = std::fs::File::create(&local_path)
            .map_err(|e| Error::Other(format!("Failed to create output file: {}", e)))?;

        std::io::copy(&mut reader, &mut file)
            .map_err(|e| Error::Other(format!("Failed to write file: {}", e)))?;

        info!(
            "Successfully downloaded entry.list ({} bytes)",
            file.metadata()
                .map(|m| m.len())
                .unwrap_or(0)
        );

        Ok(local_path)
    }

    /// Download both required files for a version
    pub fn download_all(
        &mut self,
        version: &str,
        output_dir: &Path,
    ) -> Result<(PathBuf, PathBuf), Error> {
        info!("Downloading all InterPro files for version {}", version);

        // Ensure output directory exists
        std::fs::create_dir_all(output_dir)
            .map_err(|e| Error::Other(format!("Failed to create output directory: {}", e)))?;

        let protein2ipr_path = self.download_protein2ipr(version, output_dir)?;
        let entry_list_path = self.download_entry_list(version, output_dir)?;

        info!("Successfully downloaded all files for version {}", version);

        Ok((protein2ipr_path, entry_list_path))
    }

    /// List available versions on FTP
    ///
    /// Returns a list of version directory names (e.g., ["96.0", "97.0", "98.0"]).
    /// Filters out non-version directories like "current", "tools", etc.
    pub fn list_versions(&mut self) -> Result<Vec<String>, Error> {
        let ftp = self
            .ftp
            .as_mut()
            .ok_or_else(|| Error::Other("Not connected to FTP".to_string()))?;

        // Change to base InterPro directory
        ftp.cwd(&self.config.ftp_path)
            .map_err(|e| Error::Other(format!("Failed to change directory: {}", e)))?;

        // Use LIST command to get detailed directory listing
        let list = ftp
            .list(None)
            .map_err(|e| Error::Other(format!("Failed to list directories: {}", e)))?;

        debug!("FTP LIST returned {} entries", list.len());

        // Parse directory names from LIST output
        let mut versions = Vec::new();

        for entry in list {
            // FTP LIST format: drwxr-xr-x   2 ftp      ftp          4096 Jan 15 12:00 96.0
            // We need to extract directory names (entries starting with 'd')
            let parts: Vec<&str> = entry.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            // Check if this is a directory (first char is 'd')
            if parts[0].starts_with('d') {
                // Directory name is typically the last part
                if let Some(name) = parts.last() {
                    // Filter for version directories only (e.g., "96.0", "97.0")
                    // Simple check: must be all digits and dots
                    if is_version_format(name) {
                        versions.push(name.to_string());
                        debug!("Found version directory: {}", name);
                    } else {
                        debug!("Skipping non-version directory: {}", name);
                    }
                }
            }
        }

        // Sort versions chronologically
        versions.sort();

        info!("Found {} InterPro versions on FTP", versions.len());

        Ok(versions)
    }
}

impl Drop for InterProFtpDownloader {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downloader_creation() {
        let config = InterProConfig::default();
        let downloader = InterProFtpDownloader::new(config);

        assert!(downloader.ftp.is_none());
    }

    // Note: Connection tests are commented out as they require actual FTP access
    // They can be enabled for manual testing

    // #[test]
    // fn test_connect_to_ftp() {
    //     let config = InterProConfig::default();
    //     let mut downloader = InterProFtpDownloader::new(config);
    //
    //     let result = downloader.connect();
    //     assert!(result.is_ok());
    //
    //     let disconnect_result = downloader.disconnect();
    //     assert!(disconnect_result.is_ok());
    // }
}
