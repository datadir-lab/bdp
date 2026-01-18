//! UniProt data source configuration

use serde::{Deserialize, Serialize};

/// Release type for FTP path selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseType {
    /// Current release (always latest)
    /// Path: /pub/databases/uniprot/current_release/
    Current,
    /// Previous release (by version)
    /// Path: /pub/databases/uniprot/previous_releases/release-YYYY_MM/
    Previous,
}

/// Configuration for UniProt data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtFtpConfig {
    /// FTP server hostname
    pub ftp_host: String,
    /// FTP port (default: 21)
    pub ftp_port: u16,
    /// FTP username (default: "anonymous")
    pub ftp_username: String,
    /// FTP password (default: "anonymous")
    pub ftp_password: String,
    /// Base path for UniProt data on FTP server
    pub ftp_base_path: String,
    /// Connection timeout in seconds (default: 30)
    pub connection_timeout_secs: u64,
    /// Read timeout in seconds (default: 300 = 5 minutes)
    pub read_timeout_secs: u64,
    /// Maximum number of entries to parse (None for unlimited)
    pub parse_limit: Option<usize>,
    /// Release type (Current or Previous)
    pub release_type: ReleaseType,
}

impl Default for UniProtFtpConfig {
    fn default() -> Self {
        Self {
            ftp_host: "ftp.uniprot.org".to_string(),
            ftp_port: 21,
            ftp_username: "anonymous".to_string(),
            ftp_password: "anonymous".to_string(),
            ftp_base_path: "/pub/databases/uniprot".to_string(),
            connection_timeout_secs: 30,
            read_timeout_secs: 300, // 5 minutes for large files
            parse_limit: None,
            release_type: ReleaseType::Current,
        }
    }
}

impl UniProtFtpConfig {
    /// Create a new UniProt configuration
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

    /// Set release type
    pub fn with_release_type(mut self, release_type: ReleaseType) -> Self {
        self.release_type = release_type;
        self
    }

    /// Get the base release path based on release type
    ///
    /// Returns:
    /// - Current: `/pub/databases/uniprot/current_release`
    /// - Previous: `/pub/databases/uniprot/previous_releases/release-{version}`
    pub fn release_base_path(&self, version: Option<&str>) -> String {
        match self.release_type {
            ReleaseType::Current => {
                format!("{}/current_release", self.ftp_base_path)
            }
            ReleaseType::Previous => {
                let version = version.expect("Version required for previous releases");
                format!(
                    "{}/previous_releases/release-{}",
                    self.ftp_base_path, version
                )
            }
        }
    }

    /// Get the full FTP path for release notes
    ///
    /// # Arguments
    /// * `version` - Optional for current release, required for previous (e.g., "2024_01")
    pub fn release_notes_path(&self, version: Option<&str>) -> String {
        let base = self.release_base_path(version);
        format!("{}/knowledgebase/relnotes.txt", base)
    }

    /// Get the full FTP path for the DAT file
    ///
    /// # Arguments
    /// * `version` - Optional for current release, required for previous (e.g., "2024_01")
    /// * `dataset` - "sprot" for Swiss-Prot or "trembl" for TrEMBL (default: "sprot")
    pub fn dat_file_path(&self, version: Option<&str>, dataset: Option<&str>) -> String {
        let base = self.release_base_path(version);
        let dataset = dataset.unwrap_or("sprot");
        format!(
            "{}/knowledgebase/complete/uniprot_{}.dat.gz",
            base, dataset
        )
    }

    /// Get the full FTP path for FASTA file
    pub fn fasta_file_path(&self, version: Option<&str>, dataset: Option<&str>) -> String {
        let base = self.release_base_path(version);
        let dataset = dataset.unwrap_or("sprot");
        format!(
            "{}/knowledgebase/complete/uniprot_{}.fasta.gz",
            base, dataset
        )
    }

    /// Get the full FTP path for XML file
    pub fn xml_file_path(&self, version: Option<&str>, dataset: Option<&str>) -> String {
        let base = self.release_base_path(version);
        let dataset = dataset.unwrap_or("sprot");
        format!(
            "{}/knowledgebase/complete/uniprot_{}.xml.gz",
            base, dataset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = UniProtFtpConfig::default();
        assert_eq!(config.ftp_host, "ftp.uniprot.org");
        assert_eq!(config.ftp_port, 21);
        assert_eq!(config.ftp_username, "anonymous");
        assert_eq!(config.parse_limit, None);
    }

    #[test]
    fn test_release_notes_path_current() {
        let config = UniProtFtpConfig::default().with_release_type(ReleaseType::Current);
        let path = config.release_notes_path(None);
        assert_eq!(
            path,
            "/pub/databases/uniprot/current_release/knowledgebase/relnotes.txt"
        );
    }

    #[test]
    fn test_release_notes_path_previous() {
        let config = UniProtFtpConfig::default().with_release_type(ReleaseType::Previous);
        let path = config.release_notes_path(Some("2024_01"));
        assert_eq!(
            path,
            "/pub/databases/uniprot/previous_releases/release-2024_01/knowledgebase/relnotes.txt"
        );
    }

    #[test]
    fn test_dat_file_path_current() {
        let config = UniProtFtpConfig::default().with_release_type(ReleaseType::Current);
        let path = config.dat_file_path(None, None);
        assert_eq!(
            path,
            "/pub/databases/uniprot/current_release/knowledgebase/complete/uniprot_sprot.dat.gz"
        );
    }

    #[test]
    fn test_dat_file_path_previous() {
        let config = UniProtFtpConfig::default().with_release_type(ReleaseType::Previous);
        let path = config.dat_file_path(Some("2024_01"), None);
        assert_eq!(
            path,
            "/pub/databases/uniprot/previous_releases/release-2024_01/knowledgebase/complete/uniprot_sprot.dat.gz"
        );
    }

    #[test]
    fn test_dat_file_path_trembl() {
        let config = UniProtFtpConfig::default().with_release_type(ReleaseType::Current);
        let path = config.dat_file_path(None, Some("trembl"));
        assert_eq!(
            path,
            "/pub/databases/uniprot/current_release/knowledgebase/complete/uniprot_trembl.dat.gz"
        );
    }

    #[test]
    fn test_fasta_and_xml_paths() {
        let config = UniProtFtpConfig::default().with_release_type(ReleaseType::Current);

        let fasta = config.fasta_file_path(None, None);
        assert_eq!(
            fasta,
            "/pub/databases/uniprot/current_release/knowledgebase/complete/uniprot_sprot.fasta.gz"
        );

        let xml = config.xml_file_path(None, None);
        assert_eq!(
            xml,
            "/pub/databases/uniprot/current_release/knowledgebase/complete/uniprot_sprot.xml.gz"
        );
    }

    #[test]
    fn test_with_parse_limit() {
        let config = UniProtFtpConfig::new().with_parse_limit(100);
        assert_eq!(config.parse_limit, Some(100));
    }
}
