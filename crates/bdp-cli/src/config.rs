//! Configuration management for BDP CLI
//!
//! Handles CLI settings like server URL, cache path, etc.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// CLI Configuration Constants
// ============================================================================

/// Default BDP server URL when not specified via environment variable.
pub const DEFAULT_SERVER_URL: &str = "http://localhost:8000";

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// BDP server URL
    pub server_url: String,

    /// Cache directory path
    pub cache_dir: PathBuf,

    /// Enable verbose output
    #[serde(default)]
    pub verbose: bool,
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| crate::error::CliError::config("Could not determine cache directory"))?
            .join("bdp");

        Ok(Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            cache_dir,
            verbose: false,
        })
    }

    /// Load config from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::new()?;

        if let Ok(url) = std::env::var("BDP_SERVER_URL") {
            config.server_url = url;
        }

        if let Ok(cache) = std::env::var("BDP_CACHE_DIR") {
            config.cache_dir = PathBuf::from(cache);
        }

        Ok(config)
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Get the server URL
    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    /// Set the server URL
    pub fn set_server_url(&mut self, url: String) {
        self.server_url = url;
    }

    /// Set the cache directory
    pub fn set_cache_dir(&mut self, dir: PathBuf) {
        self.cache_dir = dir;
    }

    /// Enable verbose output
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Check if verbose output is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}

impl Default for Config {
    fn default() -> Self {
        // If we can't determine the cache directory, fall back to a local directory
        Self::new().unwrap_or_else(|_| Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            cache_dir: std::path::PathBuf::from(".bdp-cache"),
            verbose: false,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::new().unwrap();
        assert_eq!(config.server_url, DEFAULT_SERVER_URL);
        assert!(config.cache_dir.to_string_lossy().contains("bdp"));
        assert!(!config.verbose);
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("BDP_SERVER_URL", "http://example.com");
        std::env::set_var("BDP_CACHE_DIR", "/tmp/test-cache");

        let config = Config::from_env().unwrap();
        assert_eq!(config.server_url, "http://example.com");
        assert_eq!(config.cache_dir, PathBuf::from("/tmp/test-cache"));

        std::env::remove_var("BDP_SERVER_URL");
        std::env::remove_var("BDP_CACHE_DIR");
    }

    #[test]
    fn test_config_setters() {
        let mut config = Config::new().unwrap();

        config.set_server_url("https://production.example.com".to_string());
        assert_eq!(config.server_url(), "https://production.example.com");

        config.set_cache_dir(PathBuf::from("/custom/cache"));
        assert_eq!(config.cache_dir(), &PathBuf::from("/custom/cache"));

        config.set_verbose(true);
        assert!(config.is_verbose());
    }
}
