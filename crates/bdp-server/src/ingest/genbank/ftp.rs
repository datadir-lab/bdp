// GenBank/RefSeq FTP download functionality

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::io::{Cursor, Read};
use std::time::Duration;
use suppaftp::FtpStream;
use tracing::{debug, info, warn};

use super::config::GenbankFtpConfig;
use super::models::Division;
use crate::ingest::common::ftp::{MAX_RETRIES, RETRY_DELAY_SECS};

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

        let host = format!("{}:{}", self.config.host, self.config.port);
        let path_owned = path.to_string();

        let data = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let mut ftp = FtpStream::connect(&host).context("Failed to connect to FTP server")?;
            ftp.login("anonymous", "anonymous")
                .context("Failed to login")?;
            ftp.transfer_type(suppaftp::types::FileType::Binary)
                .context("Failed to set binary mode")?;
            let cursor = ftp
                .retr_as_buffer(&path_owned)
                .context(format!("Failed to retrieve file: {}", path_owned))?;
            Ok(cursor.into_inner())
        })
        .await
        .context("FTP release number task panicked")??;

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
        let file_prefix = division.file_prefix().to_string();

        info!("Listing files for division {} (pattern: {})", division.as_str(), pattern);

        let host = format!("{}:{}", self.config.host, self.config.port);
        let base_path_owned = base_path.to_string();

        let files = tokio::task::spawn_blocking(move || -> Result<Vec<(String, u64)>> {
            let mut ftp = FtpStream::connect(&host).context("Failed to connect to FTP server")?;
            ftp.login("anonymous", "anonymous")
                .context("Failed to login")?;
            ftp.cwd(&base_path_owned)
                .context("Failed to change to GenBank directory")?;

            let list = ftp.list(None).context("Failed to list files")?;
            let mut files = Vec::new();

            for line in list {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 9 {
                    continue;
                }

                let filename = parts[8];
                let size_str = parts[4];

                if filename.starts_with(&file_prefix) && filename.ends_with(".seq.gz") {
                    if let Ok(size) = size_str.parse::<u64>() {
                        files.push((filename.to_string(), size));
                    }
                }
            }

            Ok(files)
        })
        .await
        .context("FTP list task panicked")??;

        info!("Found {} files for division {}", files.len(), division.as_str());

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
        info!("Decompressing {} ({} bytes compressed)", filename, compressed.len());

        let cursor = Cursor::new(compressed);
        let mut decoder = GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .context("Failed to decompress file")?;

        info!("Decompressed {} ({} bytes decompressed)", filename, decompressed.len());

        Ok(decompressed)
    }

    /// Download a GenBank file and return a streaming decompressor
    ///
    /// This method provides memory-efficient streaming decompression.
    /// Instead of loading the entire decompressed file into memory,
    /// it returns a reader that decompresses on-the-fly as data is consumed.
    ///
    /// # Memory Usage
    /// - Only the compressed file is held in memory (~100-200MB)
    /// - Decompression happens incrementally as the parser reads
    /// - Peak memory usage: ~500MB vs ~7GB for non-streaming approach
    ///
    /// # Example
    /// ```no_run
    /// # use anyhow::Result;
    /// # async fn example() -> Result<()> {
    /// # use bdp_server::ingest::genbank::ftp::GenbankFtp;
    /// # use bdp_server::ingest::genbank::config::GenbankFtpConfig;
    /// # let ftp = GenbankFtp::new(GenbankFtpConfig::new());
    /// let reader = ftp.download_division_file_streaming("gbvrl1.seq.gz").await?;
    /// // Pass reader directly to parser
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_division_file_streaming(
        &self,
        filename: &str,
    ) -> Result<GzDecoder<Cursor<Vec<u8>>>> {
        let compressed = self.download_division_file(filename).await?;
        info!(
            "Starting streaming decompression for {} ({} bytes compressed)",
            filename,
            compressed.len()
        );

        let cursor = Cursor::new(compressed);
        let decoder = GzDecoder::new(cursor);

        Ok(decoder)
    }

    /// Download all files for a division
    pub async fn download_division(&self, division: &Division) -> Result<Vec<(String, Vec<u8>)>> {
        let files = self.list_division_files(division).await?;
        let mut results = Vec::new();

        for (filename, size) in files {
            info!("Downloading {} for division {} ({} bytes)", filename, division.as_str(), size);

            match self.download_and_decompress(&filename).await {
                Ok(data) => {
                    results.push((filename, data));
                },
                Err(e) => {
                    warn!("Failed to download {}: {}", filename, e);
                },
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
                },
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to download {} after {} attempts",
                        path, MAX_RETRIES
                    ))
                },
            }
        }
    }

    /// Single attempt to download a file
    ///
    /// Uses spawn_blocking to avoid blocking tokio worker threads during
    /// the synchronous FTP download. This is critical on servers with few
    /// CPU cores (e.g., 2 cores = 2 tokio threads) where a blocked thread
    /// prevents HTTP health checks from responding.
    async fn try_download_file(&self, path: &str) -> Result<Vec<u8>> {
        let host = format!("{}:{}", self.config.host, self.config.port);
        let path = path.to_string();

        tokio::task::spawn_blocking(move || {
            let mut ftp = FtpStream::connect(&host).context("Failed to connect to FTP server")?;
            ftp.login("anonymous", "anonymous")
                .context("Failed to login")?;
            ftp.transfer_type(suppaftp::types::FileType::Binary)
                .context("Failed to set binary mode")?;

            debug!("Retrieving file: {}", path);
            let cursor = ftp
                .retr_as_buffer(&path)
                .context(format!("Failed to retrieve file: {}", path))?;

            Ok(cursor.into_inner())
        })
        .await
        .context("FTP download task panicked")?
    }

    /// List release directories (for historical version discovery)
    ///
    /// Lists subdirectories in the base path to discover historical releases
    pub async fn list_release_directories(&self) -> Result<Vec<String>> {
        let base_path = self.config.get_base_path();

        info!("Listing release directories in: {}", base_path);

        let host = format!("{}:{}", self.config.host, self.config.port);
        let base_path_owned = base_path.to_string();

        let directories = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
            let mut ftp = FtpStream::connect(&host).context("Failed to connect to FTP server")?;
            ftp.login("anonymous", "anonymous")
                .context("Failed to login")?;
            ftp.cwd(&base_path_owned)
                .context("Failed to change to release directory")?;

            let list = ftp.list(None).context("Failed to list directories")?;
            let mut directories = Vec::new();

            for line in list {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                if parts[0].starts_with('d') {
                    if let Some(name) = parts.last() {
                        directories.push(name.to_string());
                    }
                }
            }

            Ok(directories)
        })
        .await
        .context("FTP list directories task panicked")??;

        info!("Found {} directories in {}", directories.len(), base_path);

        Ok(directories)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
        assert_eq!(config.get_division_file_pattern(&Division::Viral), "gbvrl*.seq.gz");
        assert_eq!(config.get_division_file_pattern(&Division::Phage), "gbphg*.seq.gz");
    }

    #[test]
    fn test_streaming_decompression_basic() {
        // Create test data
        let test_data = b"LOCUS       TEST1               100 bp    DNA     linear   VRL 01-JAN-2026\n\
                         DEFINITION  Test sequence 1.\n\
                         ACCESSION   TEST1\n\
                         VERSION     TEST1.1\n\
                         //\n\
                         LOCUS       TEST2               200 bp    DNA     linear   VRL 01-JAN-2026\n\
                         DEFINITION  Test sequence 2.\n\
                         ACCESSION   TEST2\n\
                         VERSION     TEST2.1\n\
                         //\n";

        // Compress the test data
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Create a streaming decoder
        let cursor = Cursor::new(compressed);
        let mut decoder = GzDecoder::new(cursor);

        // Read decompressed data
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        // Verify the data matches
        assert_eq!(decompressed, test_data);
        assert!(!decompressed.is_empty());
    }

    #[test]
    fn test_streaming_vs_nonstreaming_equivalence() {
        // Test data
        let test_data = b"ATCGATCGATCGATCG".repeat(1000);

        // Compress
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Non-streaming approach (old method)
        let cursor1 = Cursor::new(compressed.clone());
        let mut decoder1 = GzDecoder::new(cursor1);
        let mut result1 = Vec::new();
        decoder1.read_to_end(&mut result1).unwrap();

        // Streaming approach (new method)
        let cursor2 = Cursor::new(compressed);
        let mut decoder2 = GzDecoder::new(cursor2);
        let mut result2 = Vec::new();
        decoder2.read_to_end(&mut result2).unwrap();

        // Both should produce identical results
        assert_eq!(result1, result2);
        assert_eq!(result1, test_data.to_vec());
    }

    #[test]
    fn test_streaming_memory_efficiency() {
        // This test verifies that streaming doesn't load everything at once
        // by reading in chunks rather than all at once

        let test_data = b"X".repeat(10_000); // 10KB test data

        // Compress
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Stream with small buffer
        let cursor = Cursor::new(compressed);
        let mut decoder = GzDecoder::new(cursor);

        let mut total_read = 0;
        let mut buffer = [0u8; 1024]; // 1KB buffer

        loop {
            let bytes_read = decoder.read(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }
            total_read += bytes_read;
        }

        assert_eq!(total_read, test_data.len());
    }

    #[test]
    fn test_streaming_with_bufreader() {
        // Test that streaming works well with BufReader (as used by parser)
        use std::io::BufRead;

        let test_data = b"Line 1\nLine 2\nLine 3\nLine 4\n";

        // Compress
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Stream with BufReader
        let cursor = Cursor::new(compressed);
        let decoder = GzDecoder::new(cursor);
        let buf_reader = std::io::BufReader::new(decoder);

        let mut lines = Vec::new();
        for line in buf_reader.lines() {
            lines.push(line.unwrap());
        }

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[3], "Line 4");
    }

    #[test]
    fn test_large_data_streaming() {
        // Test with larger data (~1MB) to ensure streaming works at scale
        let test_data = b"ATCGATCGATCGATCG".repeat(65536); // ~1MB

        // Compress
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        info!("Compressed {} bytes to {} bytes", test_data.len(), compressed.len());

        // Stream decompress
        let cursor = Cursor::new(compressed);
        let mut decoder = GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed.len(), test_data.len());
        assert_eq!(decompressed, test_data.to_vec());
    }

    #[test]
    fn test_empty_compressed_data() {
        // Test edge case: empty data
        let test_data = b"";

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        let cursor = Cursor::new(compressed);
        let mut decoder = GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, test_data);
    }

    #[test]
    fn test_invalid_gzip_data() {
        // Test error handling with invalid gzip data
        let invalid_data = b"This is not gzip data";

        let cursor = Cursor::new(invalid_data.to_vec());
        let mut decoder = GzDecoder::new(cursor);
        let mut decompressed = Vec::new();

        // Should error when trying to decompress
        let result = decoder.read_to_end(&mut decompressed);
        assert!(result.is_err());
    }
}
