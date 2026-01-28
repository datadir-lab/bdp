// Gene Ontology Downloader (HTTP and FTP)

use crate::ingest::gene_ontology::{GoHttpConfig, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use std::io::Read;
use std::time::Duration;
use suppaftp::{FtpStream, Mode};
use tracing::{info, warn};

/// HTTP client for downloading GO files
pub struct GoDownloader {
    client: Client,
    config: GoHttpConfig,
}

impl GoDownloader {
    /// Create new downloader with configuration
    pub fn new(config: GoHttpConfig) -> Result<Self> {
        config
            .validate()
            .map_err(|e| crate::ingest::gene_ontology::GoError::Validation(e))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("BDP-Gene-Ontology-Ingester/1.0")
            .build()?;

        Ok(GoDownloader { client, config })
    }

    /// Download GO ontology OBO file for a specific version
    ///
    /// # Arguments
    /// * `version` - Optional version override (e.g., "2025-01-01")
    ///
    /// If version is None, uses the configured go_release_version.
    pub async fn download_ontology_version(&self, version: Option<&str>) -> Result<String> {
        // Check if local file is configured
        if let Some(local_path) = &self.config.local_ontology_path {
            info!("Loading GO ontology from local file: {}", local_path);
            // Use tokio::fs for async file I/O to avoid blocking the async runtime
            let text = tokio::fs::read_to_string(local_path).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Failed to read local ontology file '{}': {}", local_path, e),
                )
            })?;

            info!(
                "Loaded GO ontology from local file: {} bytes ({} KB)",
                text.len(),
                text.len() / 1024
            );

            return Ok(text);
        }

        // Otherwise download from URL
        let url = self.config.ontology_url_for_version(version);
        info!("Downloading GO ontology from: {}", url);

        let content = self.download_with_retry(&url).await?;
        let text = String::from_utf8(content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        info!("Downloaded GO ontology: {} bytes ({} KB)", text.len(), text.len() / 1024);

        Ok(text)
    }

    /// Download GO ontology OBO file (or load from local file if configured)
    pub async fn download_ontology(&self) -> Result<String> {
        self.download_ontology_version(None).await
    }

    /// List all available versions from the GO release archive
    ///
    /// Returns a list of version strings in YYYY-MM-DD format
    pub async fn list_available_versions(&self) -> Result<Vec<String>> {
        use super::version_discovery::VersionDiscovery;

        let discovery = VersionDiscovery::new(self.config.clone())?;
        let versions = discovery.discover_all_versions().await?;

        Ok(versions.into_iter().map(|v| v.external_version).collect())
    }

    /// Download GOA UniProt annotations (gzipped GAF file)
    pub async fn download_goa_uniprot(&self) -> Result<String> {
        let url = self.config.goa_uniprot_url();
        info!("Downloading GOA UniProt annotations from: {}", url);

        let compressed = self.download_with_retry(&url).await?;
        info!(
            "Downloaded compressed GAF: {} bytes ({} MB)",
            compressed.len(),
            compressed.len() / (1024 * 1024)
        );

        // Decompress gzip
        let decompressed = self.decompress_gzip(&compressed)?;
        info!(
            "Decompressed GAF: {} bytes ({} GB)",
            decompressed.len(),
            decompressed.len() / (1024 * 1024 * 1024)
        );

        Ok(decompressed)
    }

    /// Download GOA file for specific organism
    pub async fn download_goa_organism(&self, organism: &str) -> Result<String> {
        let url = self.config.goa_organism_url(organism);
        info!("Downloading GOA {} annotations from: {}", organism, url);

        let compressed = self.download_with_retry(&url).await?;
        info!(
            "Downloaded compressed GAF: {} bytes ({} KB)",
            compressed.len(),
            compressed.len() / 1024
        );

        // Decompress gzip
        let decompressed = self.decompress_gzip(&compressed)?;
        info!(
            "Decompressed GAF: {} bytes ({} MB)",
            decompressed.len(),
            decompressed.len() / (1024 * 1024)
        );

        Ok(decompressed)
    }

    /// Download URL with retry logic (supports HTTP and FTP)
    async fn download_with_retry(&self, url: &str) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 1..=self.config.max_retries {
            let result = if url.starts_with("ftp://") {
                self.download_ftp(url).await
            } else {
                self.download_url(url).await
            };

            match result {
                Ok(content) => return Ok(content),
                Err(e) => {
                    warn!("Download attempt {}/{} failed: {}", attempt, self.config.max_retries, e);
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        // Exponential backoff: 2^attempt seconds
                        let backoff_secs = 2u64.pow(attempt);
                        info!("Retrying in {} seconds...", backoff_secs);
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    }
                },
            }
        }

        // last_error is guaranteed to be Some since we always set it on failure
        // and exit early on success. This is safe because max_retries >= 1.
        match last_error {
            Some(err) => Err(err),
            None => Err(crate::ingest::gene_ontology::GoError::Validation(format!(
                "Download failed after {} retries with no error captured (this should never happen)",
                self.config.max_retries
            ))),
        }
    }

    /// Download URL without retry
    async fn download_url(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP error: {}", response.status()),
            )
            .into());
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Download from FTP server
    async fn download_ftp(&self, url: &str) -> Result<Vec<u8>> {
        // Parse FTP URL: ftp://server/path/to/file
        let url_without_protocol = url.strip_prefix("ftp://").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid FTP URL")
        })?;

        let (server_and_path, _) = url_without_protocol
            .split_once('/')
            .unwrap_or((url_without_protocol, ""));
        let server = server_and_path.split('/').next().unwrap_or(server_and_path);
        let path = url.strip_prefix(&format!("ftp://{}", server)).unwrap_or("");

        info!("Connecting to FTP server: {}", server);

        // Run FTP operations in blocking thread
        let server = server.to_string();
        let path = path.to_string();

        tokio::task::spawn_blocking(move || {
            let mut ftp_stream = FtpStream::connect(format!("{}:21", server))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            ftp_stream
                .login("anonymous", "anonymous@")
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            // Enable passive mode for firewall/NAT compatibility
            ftp_stream.set_mode(Mode::Passive);

            info!("Downloading file: {}", path);

            let cursor = ftp_stream
                .retr_as_buffer(&path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            ftp_stream
                .quit()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            Ok::<Vec<u8>, std::io::Error>(cursor.into_inner())
        })
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
        .map_err(|e| crate::ingest::gene_ontology::GoError::Io(e))
    }

    /// Decompress gzip data
    fn decompress_gzip(&self, compressed: &[u8]) -> Result<String> {
        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = String::new();

        decoder
            .read_to_string(&mut decompressed)
            .map_err(|e| crate::ingest::gene_ontology::GoError::Decompression(e.to_string()))?;

        Ok(decompressed)
    }

    /// Get configuration
    pub fn config(&self) -> &GoHttpConfig {
        &self.config
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
        let config = GoHttpConfig::default();
        let downloader = GoDownloader::new(config);
        assert!(downloader.is_ok());
    }

    #[test]
    fn test_invalid_config() {
        let mut config = GoHttpConfig::default();
        config.ontology_base_url = "".to_string();

        let downloader = GoDownloader::new(config);
        assert!(downloader.is_err());
    }

    #[tokio::test]
    #[ignore] // Ignore by default (requires network)
    async fn test_download_ontology() {
        let config = GoHttpConfig::test_config();
        let downloader = GoDownloader::new(config).unwrap();

        let result = downloader.download_ontology().await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert!(content.contains("format-version:"));
        assert!(content.contains("[Term]"));
    }

    #[tokio::test]
    #[ignore] // Ignore by default (requires network)
    async fn test_download_goa_organism() {
        let config = GoHttpConfig::test_config();
        let downloader = GoDownloader::new(config).unwrap();

        // Download small organism file for testing
        let result = downloader.download_goa_organism("human").await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert!(content.contains("!gaf-version: 2.2"));
        assert!(content.contains("UniProtKB"));
    }

    #[test]
    fn test_decompress_gzip() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let config = GoHttpConfig::default();
        let downloader = GoDownloader::new(config).unwrap();

        // Create test gzip data
        let test_data = "Hello, GO!";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(test_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        // Decompress
        let decompressed = downloader.decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, test_data);
    }
}
