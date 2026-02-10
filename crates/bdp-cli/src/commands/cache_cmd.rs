//! `bdp cache` command implementation
//!
//! Manages the project-local cache directory configuration.

use std::path::{Path, PathBuf};

use crate::{
    commands::output::Render,
    error::Result,
    project::{self, ProjectConfig},
};

/// Output from `bdp cache` subcommands.
pub enum CacheOutput {
    Set {
        requested_path: String,
        resolved_path: PathBuf,
    },
    Show {
        configured_path: String,
        resolved_path: PathBuf,
        project_root: PathBuf,
        size: Option<u64>,
    },
    Reset {
        default_path: String,
    },
}

impl Render for CacheOutput {
    fn render(&self) {
        use colored::Colorize;

        match self {
            CacheOutput::Set {
                requested_path,
                resolved_path,
            } => {
                println!("{} Cache directory set to: {}", "✓".green(), requested_path);
                println!("  Resolved path: {}", resolved_path.display());
            },
            CacheOutput::Show {
                configured_path,
                resolved_path,
                project_root,
                size,
            } => {
                println!("{}", "Cache Configuration:".cyan().bold());
                println!("  Configured path: {}", configured_path);
                println!("  Resolved path:   {}", resolved_path.display());
                println!("  Project root:    {}", project_root.display());

                if let Some(size) = size {
                    println!("  Cache size:      {}", crate::progress::format_bytes(*size));
                }
            },
            CacheOutput::Reset { default_path } => {
                println!("{} Cache directory reset to default: {}", "✓".green(), default_path);
            },
        }
    }
}

/// Set the cache directory path
pub async fn set(path: String) -> Result<CacheOutput> {
    let project_root = project::find_project_root()?;

    // Validate the path makes sense
    let resolved = if Path::new(&path).is_absolute() {
        PathBuf::from(&path)
    } else {
        project_root.join(&path)
    };

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&resolved)?;

    // Save config
    let mut config = ProjectConfig::load(&project_root)?;
    config.cache.path = path.clone();
    config.save(&project_root)?;

    Ok(CacheOutput::Set {
        requested_path: path,
        resolved_path: resolved,
    })
}

/// Show the current cache directory
pub async fn show() -> Result<CacheOutput> {
    let project_root = project::find_project_root()?;
    let config = ProjectConfig::load(&project_root)?;
    let resolved = project::resolve_cache_path(&project_root)?;

    // Show size if directory exists
    let size = if resolved.exists() {
        Some(dir_size(&resolved))
    } else {
        None
    };

    Ok(CacheOutput::Show {
        configured_path: config.cache.path,
        resolved_path: resolved,
        project_root,
        size,
    })
}

/// Reset the cache directory to default (.bdp/data)
pub async fn reset() -> Result<CacheOutput> {
    let project_root = project::find_project_root()?;

    let config = ProjectConfig::default();
    config.save(&project_root)?;

    let resolved = project_root.join(&config.cache.path);
    std::fs::create_dir_all(&resolved)?;

    Ok(CacheOutput::Reset {
        default_path: config.cache.path,
    })
}

/// Calculate the total size of a directory (non-recursive for speed)
fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::project::ProjectConfig;

    #[tokio::test]
    async fn test_dir_size_empty() {
        let temp = TempDir::new().unwrap();
        assert_eq!(dir_size(temp.path()), 0);
    }

    #[tokio::test]
    async fn test_dir_size_with_files() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("a.txt"), "hello").unwrap();
        std::fs::write(temp.path().join("b.txt"), "world!").unwrap();
        assert_eq!(dir_size(temp.path()), 11); // 5 + 6
    }

    #[test]
    fn test_config_set_and_load() {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::default();
        config.cache.path = "custom/path".to_string();
        config.save(temp.path()).unwrap();

        let loaded = ProjectConfig::load(temp.path()).unwrap();
        assert_eq!(loaded.cache.path, "custom/path");
    }
}
