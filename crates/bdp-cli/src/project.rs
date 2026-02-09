//! Project root discovery and configuration
//!
//! Finds the project root by walking up from CWD looking for `bdp.yml`,
//! and manages project-local configuration in `.bdp/.config` (TOML format).

use crate::error::{CliError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Project-local configuration stored in `.bdp/.config`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,
}

/// Cache directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Path to the cache directory (relative to project root, or absolute)
    #[serde(default = "CacheConfig::default_path")]
    pub path: String,
}

impl CacheConfig {
    fn default_path() -> String {
        ".bdp/data".to_string()
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            path: Self::default_path(),
        }
    }
}

impl ProjectConfig {
    /// Load project config from `.bdp/.config` in the given project root.
    /// Returns default config if file doesn't exist.
    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join(".bdp").join(".config");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: ProjectConfig = toml::from_str(&content)
            .map_err(|e| CliError::config(format!("Failed to parse .bdp/.config: {}", e)))?;
        Ok(config)
    }

    /// Save project config to `.bdp/.config` in the given project root.
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let bdp_dir = project_root.join(".bdp");
        std::fs::create_dir_all(&bdp_dir)?;

        let config_path = bdp_dir.join(".config");
        let content = toml::to_string_pretty(self)
            .map_err(|e| CliError::config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

/// Find the project root by walking up from CWD looking for `bdp.yml`.
pub fn find_project_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    find_project_root_from(&cwd)
}

/// Find the project root by walking up from the given directory.
fn find_project_root_from(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("bdp.yml").exists() {
            return Ok(current);
        }

        if !current.pop() {
            return Err(CliError::NotInitialized(
                "No bdp.yml found in current or parent directories. Run 'bdp init' first."
                    .to_string(),
            ));
        }
    }
}

/// Resolve the cache path from project config.
/// If the configured path is relative, it's resolved relative to the project root.
/// If absolute, it's used as-is.
pub fn resolve_cache_path(project_root: &Path) -> Result<PathBuf> {
    let config = ProjectConfig::load(project_root)?;
    let cache_path = Path::new(&config.cache.path);

    if cache_path.is_absolute() {
        Ok(cache_path.to_path_buf())
    } else {
        Ok(project_root.join(cache_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert_eq!(config.cache.path, ".bdp/data");
    }

    #[test]
    fn test_config_save_load() {
        let temp = TempDir::new().unwrap();
        let config = ProjectConfig::default();
        config.save(temp.path()).unwrap();

        let loaded = ProjectConfig::load(temp.path()).unwrap();
        assert_eq!(loaded.cache.path, ".bdp/data");
    }

    #[test]
    fn test_config_load_missing_file() {
        let temp = TempDir::new().unwrap();
        let config = ProjectConfig::load(temp.path()).unwrap();
        assert_eq!(config.cache.path, ".bdp/data");
    }

    #[test]
    fn test_find_project_root() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("bdp.yml"), "project:\n  name: test\n  version: '0.1.0'\n")
            .unwrap();

        let result = find_project_root_from(temp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp.path().to_path_buf());
    }

    #[test]
    fn test_find_project_root_subdirectory() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("bdp.yml"), "project:\n  name: test\n  version: '0.1.0'\n")
            .unwrap();
        let sub = temp.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();

        let result = find_project_root_from(&sub);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp.path().to_path_buf());
    }

    #[test]
    fn test_find_project_root_not_found() {
        let temp = TempDir::new().unwrap();
        let result = find_project_root_from(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_cache_path_relative() {
        let temp = TempDir::new().unwrap();
        let config = ProjectConfig::default();
        config.save(temp.path()).unwrap();

        let path = resolve_cache_path(temp.path()).unwrap();
        assert_eq!(path, temp.path().join(".bdp/data"));
    }

    #[test]
    fn test_resolve_cache_path_absolute() {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::default();

        // Use platform-appropriate absolute path
        let abs_path = if cfg!(windows) {
            "C:\\shared\\bdp-data".to_string()
        } else {
            "/shared/bdp-data".to_string()
        };
        config.cache.path = abs_path.clone();
        config.save(temp.path()).unwrap();

        let path = resolve_cache_path(temp.path()).unwrap();
        assert_eq!(path, PathBuf::from(&abs_path));
    }

    #[test]
    fn test_config_roundtrip_custom_path() {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::default();
        config.cache.path = "custom/cache".to_string();
        config.save(temp.path()).unwrap();

        let loaded = ProjectConfig::load(temp.path()).unwrap();
        assert_eq!(loaded.cache.path, "custom/cache");
    }
}
