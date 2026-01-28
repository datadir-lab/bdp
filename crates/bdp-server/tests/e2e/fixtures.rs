//! Test data management for E2E tests
//!
//! Handles both CI fixtures (committed) and real UniProt data (downloaded, cached).

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Test data mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestDataMode {
    /// Use small CI sample (fast, always available)
    CI,
    /// Use real downloaded UniProt data (cached, realistic)
    Real,
}

impl TestDataMode {
    /// Detect mode from environment
    pub fn from_env() -> Self {
        match std::env::var("BDP_E2E_MODE").as_deref() {
            Ok("real") => Self::Real,
            Ok("ci") | Ok(_) | Err(_) => Self::CI,
        }
    }
}

/// Test data manager
pub struct TestDataManager {
    mode: TestDataMode,
    fixtures_dir: PathBuf,
}

impl TestDataManager {
    /// Create new test data manager
    pub fn new(mode: TestDataMode) -> Self {
        let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");

        Self { mode, fixtures_dir }
    }

    /// Get the UniProt DAT file path
    pub fn get_uniprot_dat_path(&self) -> Result<PathBuf> {
        match self.mode {
            TestDataMode::CI => {
                let path = self.fixtures_dir.join("uniprot_ci_sample.dat");
                if !path.exists() {
                    anyhow::bail!("CI sample data not found at {:?}", path);
                }
                Ok(path)
            },
            TestDataMode::Real => {
                let real_dir = self.fixtures_dir.join("real");

                // Check if data exists
                if !real_dir.exists() {
                    fs::create_dir_all(&real_dir)
                        .context("Failed to create real data directory")?;
                }

                // Look for any .dat or .dat.gz file
                let entries =
                    fs::read_dir(&real_dir).context("Failed to read real data directory")?;

                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "dat" || path.to_str().unwrap_or("").ends_with(".dat.gz") {
                            info!("Using real UniProt data: {:?}", path);
                            return Ok(path);
                        }
                    }
                }

                warn!("No real UniProt data found in {:?}", real_dir);
                warn!("Run `just e2e-download-data` to download real data");
                warn!("Falling back to CI sample");

                // Fallback to CI mode
                self.fixtures_dir
                    .join("uniprot_ci_sample.dat")
                    .canonicalize()
                    .context("Failed to find CI sample data")
            },
        }
    }

    /// Get test data info
    pub fn get_info(&self) -> TestDataInfo {
        let dat_path = self.get_uniprot_dat_path().ok();
        let size = dat_path
            .as_ref()
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len());

        TestDataInfo {
            mode: self.mode,
            path: dat_path,
            size_bytes: size,
        }
    }

    /// Check if real data is available
    pub fn has_real_data(&self) -> bool {
        let real_dir = self.fixtures_dir.join("real");
        if !real_dir.exists() {
            return false;
        }

        fs::read_dir(&real_dir)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|e| {
                        let path = e.path();
                        path.extension().map(|ext| ext == "dat").unwrap_or(false)
                            || path
                                .to_str()
                                .map(|s| s.ends_with(".dat.gz"))
                                .unwrap_or(false)
                    })
                    .then_some(())
            })
            .is_some()
    }
}

/// Test data info
#[derive(Debug, Clone)]
pub struct TestDataInfo {
    pub mode: TestDataMode,
    pub path: Option<PathBuf>,
    pub size_bytes: Option<u64>,
}

impl TestDataInfo {
    /// Format size as human-readable string
    pub fn size_human(&self) -> String {
        match self.size_bytes {
            Some(bytes) => {
                if bytes < 1024 {
                    format!("{} B", bytes)
                } else if bytes < 1024 * 1024 {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                }
            },
            None => "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_mode_has_data() {
        let manager = TestDataManager::new(TestDataMode::CI);
        let path = manager.get_uniprot_dat_path();
        assert!(path.is_ok(), "CI sample data should exist");
    }

    #[test]
    fn test_data_info() {
        let manager = TestDataManager::new(TestDataMode::CI);
        let info = manager.get_info();
        assert_eq!(info.mode, TestDataMode::CI);
        assert!(info.path.is_some());
        assert!(info.size_bytes.is_some());
    }

    #[test]
    fn test_mode_from_env() {
        std::env::set_var("BDP_E2E_MODE", "ci");
        assert_eq!(TestDataMode::from_env(), TestDataMode::CI);

        std::env::set_var("BDP_E2E_MODE", "real");
        assert_eq!(TestDataMode::from_env(), TestDataMode::Real);

        std::env::remove_var("BDP_E2E_MODE");
        assert_eq!(TestDataMode::from_env(), TestDataMode::CI);
    }
}
