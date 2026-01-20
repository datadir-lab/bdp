//! NCBI Taxonomy data source configuration

use serde::{Deserialize, Serialize};

/// Configuration for NCBI Taxonomy data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcbiTaxonomyFtpConfig {
    /// FTP server hostname
    pub ftp_host: String,
    /// FTP port (default: 21)
    pub ftp_port: u16,
    /// FTP username (default: "anonymous")
    pub ftp_username: String,
    /// FTP password (default: "anonymous")
    pub ftp_password: String,
    /// Base path for NCBI taxonomy data on FTP server
    pub ftp_base_path: String,
    /// Connection timeout in seconds (default: 30)
    pub connection_timeout_secs: u64,
    /// Read timeout in seconds (default: 1800 = 30 minutes)
    pub read_timeout_secs: u64,
    /// Maximum number of entries to parse (None for unlimited)
    pub parse_limit: Option<usize>,
}

impl Default for NcbiTaxonomyFtpConfig {
    fn default() -> Self {
        Self {
            ftp_host: "ftp.ncbi.nlm.nih.gov".to_string(),
            ftp_port: 21,
            ftp_username: "anonymous".to_string(),
            ftp_password: "anonymous".to_string(),
            ftp_base_path: "/pub/taxonomy/new_taxdump".to_string(),
            connection_timeout_secs: 30,
            read_timeout_secs: 1800, // 30 minutes for large files (140 MB tar.gz archive)
            parse_limit: None,
        }
    }
}

impl NcbiTaxonomyFtpConfig {
    /// Create a new NCBI Taxonomy configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set parse limit
    pub fn with_parse_limit(mut self, limit: usize) -> Self {
        self.parse_limit = Some(limit);
        self
    }

    /// Set FTP host
    pub fn with_ftp_host(mut self, host: impl Into<String>) -> Self {
        self.ftp_host = host.into();
        self
    }

    /// Set connection timeout
    pub fn with_connection_timeout(mut self, timeout_secs: u64) -> Self {
        self.connection_timeout_secs = timeout_secs;
        self
    }

    /// Set read timeout
    pub fn with_read_timeout(mut self, timeout_secs: u64) -> Self {
        self.read_timeout_secs = timeout_secs;
        self
    }

    /// Get the full FTP path for new_taxdump.tar.gz (current version)
    pub fn taxdump_path(&self) -> String {
        format!("{}/new_taxdump.tar.gz", self.ftp_base_path)
    }

    /// Get the full FTP path for historical archive
    ///
    /// # Arguments
    /// * `date` - Archive date in format "YYYY-MM-DD" (e.g., "2024-01-01")
    ///
    /// # Returns
    /// FTP path like "/pub/taxonomy/taxdump_archive/new_taxdump_2024-01-01.zip"
    pub fn archive_path(&self, date: &str) -> String {
        format!("/pub/taxonomy/taxdump_archive/new_taxdump_{}.zip", date)
    }

    /// Get the full FTP path for taxdump_readme.txt
    pub fn readme_path(&self) -> String {
        format!("{}/taxdump_readme.txt", self.ftp_base_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NcbiTaxonomyFtpConfig::default();
        assert_eq!(config.ftp_host, "ftp.ncbi.nlm.nih.gov");
        assert_eq!(config.ftp_port, 21);
        assert_eq!(config.ftp_username, "anonymous");
        assert_eq!(config.parse_limit, None);
    }

    #[test]
    fn test_taxdump_path() {
        let config = NcbiTaxonomyFtpConfig::default();
        let path = config.taxdump_path();
        assert_eq!(path, "/pub/taxonomy/new_taxdump/new_taxdump.tar.gz");
    }

    #[test]
    fn test_readme_path() {
        let config = NcbiTaxonomyFtpConfig::default();
        let path = config.readme_path();
        assert_eq!(path, "/pub/taxonomy/new_taxdump/taxdump_readme.txt");
    }

    #[test]
    fn test_with_parse_limit() {
        let config = NcbiTaxonomyFtpConfig::new().with_parse_limit(100);
        assert_eq!(config.parse_limit, Some(100));
    }

    #[test]
    fn test_builder_pattern() {
        let config = NcbiTaxonomyFtpConfig::new()
            .with_ftp_host("test.example.com")
            .with_connection_timeout(60)
            .with_read_timeout(3600)
            .with_parse_limit(500);

        assert_eq!(config.ftp_host, "test.example.com");
        assert_eq!(config.connection_timeout_secs, 60);
        assert_eq!(config.read_timeout_secs, 3600);
        assert_eq!(config.parse_limit, Some(500));
    }
}
