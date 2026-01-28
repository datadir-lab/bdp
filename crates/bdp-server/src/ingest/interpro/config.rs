// InterPro Configuration
//
// Environment-based configuration for InterPro ingestion

use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for InterPro FTP connection and ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterProConfig {
    /// FTP host (default: ftp.ebi.ac.uk)
    pub ftp_host: String,

    /// FTP base path (default: /pub/databases/interpro/)
    pub ftp_path: String,

    /// FTP connection timeout in seconds
    pub ftp_timeout_secs: u64,

    /// Batch size for processing entries
    pub batch_size: usize,

    /// Whether to enable automatic ingestion
    pub auto_enabled: bool,

    /// Cron schedule for automatic ingestion (e.g., "0 2 * * *" for daily at 2 AM)
    pub schedule: String,
}

impl Default for InterProConfig {
    fn default() -> Self {
        Self {
            ftp_host: "ftp.ebi.ac.uk".to_string(),
            ftp_path: "/pub/databases/interpro/".to_string(),
            ftp_timeout_secs: 300,
            batch_size: 500,
            auto_enabled: false,
            schedule: "0 2 * * *".to_string(), // Daily at 2 AM
        }
    }
}

impl InterProConfig {
    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - INGEST_INTERPRO_FTP_HOST
    /// - INGEST_INTERPRO_FTP_PATH
    /// - INGEST_INTERPRO_FTP_TIMEOUT_SECS
    /// - INGEST_INTERPRO_BATCH_SIZE
    /// - INGEST_INTERPRO_AUTO_ENABLED
    /// - INGEST_INTERPRO_SCHEDULE
    pub fn from_env() -> Self {
        Self {
            ftp_host: env::var("INGEST_INTERPRO_FTP_HOST")
                .unwrap_or_else(|_| "ftp.ebi.ac.uk".to_string()),

            ftp_path: env::var("INGEST_INTERPRO_FTP_PATH")
                .unwrap_or_else(|_| "/pub/databases/interpro/".to_string()),

            ftp_timeout_secs: env::var("INGEST_INTERPRO_FTP_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            batch_size: env::var("INGEST_INTERPRO_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500),

            auto_enabled: env::var("INGEST_INTERPRO_AUTO_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),

            schedule: env::var("INGEST_INTERPRO_SCHEDULE")
                .unwrap_or_else(|_| "0 2 * * *".to_string()),
        }
    }

    /// Get protein2ipr.dat.gz file path for current release
    pub fn get_protein2ipr_path(&self, version: &str) -> String {
        format!("{}{}/protein2ipr.dat.gz", self.ftp_path, version)
    }

    /// Get entry.list file path for current release
    pub fn get_entry_list_path(&self, version: &str) -> String {
        format!("{}{}/entry.list", self.ftp_path, version)
    }

    /// Get current release version path
    pub fn get_current_release_path(&self) -> String {
        format!("{}current_release/", self.ftp_path)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.ftp_host.is_empty() {
            return Err("FTP host cannot be empty".to_string());
        }

        if self.ftp_path.is_empty() {
            return Err("FTP path cannot be empty".to_string());
        }

        if self.batch_size == 0 {
            return Err("Batch size must be greater than 0".to_string());
        }

        if self.batch_size > 10000 {
            return Err("Batch size too large (max 10000)".to_string());
        }

        if self.ftp_timeout_secs == 0 {
            return Err("FTP timeout must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = InterProConfig::default();

        assert_eq!(config.ftp_host, "ftp.ebi.ac.uk");
        assert_eq!(config.ftp_path, "/pub/databases/interpro/");
        assert_eq!(config.ftp_timeout_secs, 300);
        assert_eq!(config.batch_size, 500);
        assert!(!config.auto_enabled);
    }

    #[test]
    fn test_get_protein2ipr_path() {
        let config = InterProConfig::default();
        let path = config.get_protein2ipr_path("95.0");

        assert_eq!(path, "/pub/databases/interpro/95.0/protein2ipr.dat.gz");
    }

    #[test]
    fn test_get_entry_list_path() {
        let config = InterProConfig::default();
        let path = config.get_entry_list_path("95.0");

        assert_eq!(path, "/pub/databases/interpro/95.0/entry.list");
    }

    #[test]
    fn test_get_current_release_path() {
        let config = InterProConfig::default();
        let path = config.get_current_release_path();

        assert_eq!(path, "/pub/databases/interpro/current_release/");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = InterProConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_host() {
        let mut config = InterProConfig::default();
        config.ftp_host = String::new();

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_batch_size() {
        let mut config = InterProConfig::default();
        config.batch_size = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_batch_size_too_large() {
        let mut config = InterProConfig::default();
        config.batch_size = 20000;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_timeout() {
        let mut config = InterProConfig::default();
        config.ftp_timeout_secs = 0;

        assert!(config.validate().is_err());
    }
}
