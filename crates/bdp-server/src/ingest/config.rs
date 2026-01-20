//! Ingestion configuration
//!
//! Configuration for data ingestion jobs including UniProt sync.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Ingestion mode configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum IngestionMode {
    /// Latest mode: only ingest newest available version
    Latest(LatestConfig),
    /// Historical mode: backfill multiple versions
    Historical(HistoricalConfig),
}

/// Configuration for latest mode ingestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LatestConfig {
    /// Check interval in seconds (default: 86400 = daily)
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
    /// Auto-ingest when newer version detected (default: false)
    #[serde(default)]
    pub auto_ingest: bool,
    /// Ignore versions before this date (format: YYYY_MM)
    #[serde(default)]
    pub ignore_before: Option<String>,
}

/// Configuration for historical mode ingestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoricalConfig {
    /// Start version (e.g., "2020_01")
    pub start_version: String,
    /// End version (None = all available)
    #[serde(default)]
    pub end_version: Option<String>,
    /// Batch size for processing multiple versions (default: 3)
    #[serde(default = "default_historical_batch_size")]
    pub batch_size: usize,
    /// Skip versions that already exist in database (default: true)
    #[serde(default = "default_true")]
    pub skip_existing: bool,
}

fn default_check_interval() -> u64 {
    86400 // 1 day
}

fn default_historical_batch_size() -> usize {
    3
}

fn default_true() -> bool {
    true
}

impl Default for LatestConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval(),
            auto_ingest: false,
            ignore_before: None,
        }
    }
}

impl Default for HistoricalConfig {
    fn default() -> Self {
        Self {
            start_version: "2020_01".to_string(),
            end_version: None,
            batch_size: default_historical_batch_size(),
            skip_existing: true,
        }
    }
}

impl Default for IngestionMode {
    fn default() -> Self {
        IngestionMode::Latest(LatestConfig::default())
    }
}

/// Main ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Whether ingestion is enabled
    pub enabled: bool,
    /// Number of worker threads for job processing
    pub worker_threads: usize,
    /// Maximum retries for failed jobs
    pub max_retries: u32,
    /// Job timeout in seconds
    pub job_timeout_secs: u64,
    /// UniProt-specific configuration
    pub uniprot: UniProtConfig,
}

/// UniProt-specific ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtConfig {
    /// FTP host for UniProt data
    pub ftp_host: String,
    /// FTP path to UniProt data
    pub ftp_path: String,
    /// Oldest version to consider for ingestion (format: YYYY_MM)
    pub oldest_version: String,
    /// Cron schedule for automatic ingestion (e.g., "0 2 * * *" for daily at 2 AM)
    pub ingestion_schedule: String,
    /// Whether to enable automatic scheduled ingestion
    pub auto_ingest_enabled: bool,
    /// Batch size for processing entries
    pub batch_size: usize,
    /// Connection timeout for FTP in seconds
    pub ftp_timeout_secs: u64,
    /// Ingestion mode (latest or historical)
    pub ingestion_mode: IngestionMode,
    /// Simple start-from version - ingest all versions >= this (SIMPLIFIED API)
    /// Format: YYYY_MM (e.g., "2018_01")
    /// If set, this takes precedence over mode configuration
    pub start_from_version: String,
    /// Cache directory for decompressed DAT files
    /// Default: /tmp/bdp-ingest-cache (Linux/macOS) or %TEMP%\bdp-ingest-cache (Windows)
    pub cache_dir: std::path::PathBuf,
}

impl IngestConfig {
    /// Load ingestion configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let config = Self {
            enabled: std::env::var("INGEST_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            worker_threads: std::env::var("INGEST_WORKER_THREADS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4),
            max_retries: std::env::var("INGEST_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            job_timeout_secs: std::env::var("INGEST_JOB_TIMEOUT_SECS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
            uniprot: UniProtConfig::from_env()?,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.enabled {
            if self.worker_threads == 0 {
                anyhow::bail!("INGEST_WORKER_THREADS must be greater than 0");
            }
            if self.job_timeout_secs == 0 {
                anyhow::bail!("INGEST_JOB_TIMEOUT_SECS must be greater than 0");
            }
        }
        self.uniprot.validate()?;
        Ok(())
    }

    /// Get job timeout as Duration
    pub fn job_timeout(&self) -> Duration {
        Duration::from_secs(self.job_timeout_secs)
    }
}

impl UniProtConfig {
    /// Load UniProt configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        // Parse ingestion mode
        let mode_str = std::env::var("INGEST_UNIPROT_MODE")
            .unwrap_or_else(|_| "latest".to_string());

        let ingestion_mode = match mode_str.as_str() {
            "latest" => {
                let check_interval_secs = std::env::var("INGEST_UNIPROT_CHECK_INTERVAL_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_check_interval);

                let auto_ingest = std::env::var("INGEST_UNIPROT_AUTO_INGEST")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(false);

                let ignore_before = std::env::var("INGEST_UNIPROT_IGNORE_BEFORE")
                    .ok();

                IngestionMode::Latest(LatestConfig {
                    check_interval_secs,
                    auto_ingest,
                    ignore_before,
                })
            }
            "historical" => {
                let start_version = std::env::var("INGEST_UNIPROT_HISTORICAL_START")
                    .unwrap_or_else(|_| "2020_01".to_string());

                let end_version = std::env::var("INGEST_UNIPROT_HISTORICAL_END")
                    .ok();

                let batch_size = std::env::var("INGEST_UNIPROT_HISTORICAL_BATCH_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_historical_batch_size);

                let skip_existing = std::env::var("INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);

                IngestionMode::Historical(HistoricalConfig {
                    start_version,
                    end_version,
                    batch_size,
                    skip_existing,
                })
            }
            _ => {
                anyhow::bail!("Invalid INGEST_UNIPROT_MODE: {}. Must be 'latest' or 'historical'", mode_str);
            }
        };

        // Get cache directory from env or use platform-specific default
        let cache_dir = std::env::var("INGEST_CACHE_DIR")
            .ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                if cfg!(windows) {
                    std::env::temp_dir().join("bdp-ingest-cache")
                } else {
                    std::path::PathBuf::from("/tmp/bdp-ingest-cache")
                }
            });

        let config = Self {
            ftp_host: std::env::var("INGEST_UNIPROT_FTP_HOST")
                .unwrap_or_else(|_| "ftp.uniprot.org".to_string()),
            ftp_path: std::env::var("INGEST_UNIPROT_FTP_PATH")
                .unwrap_or_else(|_| "/pub/databases/uniprot/current_release/knowledgebase/complete".to_string()),
            oldest_version: std::env::var("INGEST_UNIPROT_OLDEST_VERSION")
                .unwrap_or_else(|_| "2020_01".to_string()),
            ingestion_schedule: std::env::var("INGEST_UNIPROT_SCHEDULE")
                .unwrap_or_else(|_| "0 2 * * *".to_string()),
            auto_ingest_enabled: std::env::var("INGEST_UNIPROT_AUTO_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            batch_size: std::env::var("INGEST_UNIPROT_BATCH_SIZE")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .unwrap_or(5000),
            ftp_timeout_secs: std::env::var("INGEST_UNIPROT_FTP_TIMEOUT_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            ingestion_mode,
            start_from_version: std::env::var("INGEST_START_FROM_VERSION")
                .unwrap_or_else(|_| "".to_string()),
            cache_dir,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate UniProt configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.ftp_host.is_empty() {
            anyhow::bail!("INGEST_UNIPROT_FTP_HOST cannot be empty");
        }
        if self.ftp_path.is_empty() {
            anyhow::bail!("INGEST_UNIPROT_FTP_PATH cannot be empty");
        }
        if self.oldest_version.is_empty() {
            anyhow::bail!("INGEST_UNIPROT_OLDEST_VERSION cannot be empty");
        }
        // Validate version format (YYYY_MM)
        if !self.oldest_version.contains('_') || self.oldest_version.len() != 7 {
            anyhow::bail!(
                "INGEST_UNIPROT_OLDEST_VERSION must be in format YYYY_MM, got: {}",
                self.oldest_version
            );
        }
        if self.batch_size == 0 {
            anyhow::bail!("INGEST_UNIPROT_BATCH_SIZE must be greater than 0");
        }
        if self.ftp_timeout_secs == 0 {
            anyhow::bail!("INGEST_UNIPROT_FTP_TIMEOUT_SECS must be greater than 0");
        }
        Ok(())
    }

    /// Get FTP timeout as Duration
    pub fn ftp_timeout(&self) -> Duration {
        Duration::from_secs(self.ftp_timeout_secs)
    }
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            worker_threads: 4,
            max_retries: 3,
            job_timeout_secs: 3600,
            uniprot: UniProtConfig::default(),
        }
    }
}

impl Default for UniProtConfig {
    fn default() -> Self {
        let cache_dir = if cfg!(windows) {
            std::env::temp_dir().join("bdp-ingest-cache")
        } else {
            std::path::PathBuf::from("/tmp/bdp-ingest-cache")
        };

        Self {
            ftp_host: "ftp.uniprot.org".to_string(),
            ftp_path: "/pub/databases/uniprot/current_release/knowledgebase/complete".to_string(),
            oldest_version: "2020_01".to_string(),
            ingestion_schedule: "0 2 * * *".to_string(),
            auto_ingest_enabled: false,
            batch_size: 5000,
            ftp_timeout_secs: 300,
            ingestion_mode: IngestionMode::default(),
            start_from_version: "".to_string(),
            cache_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniprot_config_default() {
        let config = UniProtConfig::default();
        assert_eq!(config.ftp_host, "ftp.uniprot.org");
        assert_eq!(config.oldest_version, "2020_01");
        assert_eq!(config.batch_size, 1000);
    }

    #[test]
    fn test_uniprot_config_validation_valid() {
        let config = UniProtConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_uniprot_config_validation_invalid_version() {
        let mut config = UniProtConfig::default();
        config.oldest_version = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_uniprot_config_validation_empty_host() {
        let mut config = UniProtConfig::default();
        config.ftp_host = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_uniprot_config_validation_zero_batch_size() {
        let mut config = UniProtConfig::default();
        config.batch_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ingest_config_default() {
        let config = IngestConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.worker_threads, 4);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_ingest_config_validation_valid() {
        let config = IngestConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ingest_config_validation_zero_workers() {
        let mut config = IngestConfig::default();
        config.enabled = true;
        config.worker_threads = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_job_timeout_duration() {
        let config = IngestConfig {
            job_timeout_secs: 1800,
            ..Default::default()
        };
        assert_eq!(config.job_timeout(), Duration::from_secs(1800));
    }

    #[test]
    fn test_ftp_timeout_duration() {
        let config = UniProtConfig {
            ftp_timeout_secs: 600,
            ..Default::default()
        };
        assert_eq!(config.ftp_timeout(), Duration::from_secs(600));
    }
}
