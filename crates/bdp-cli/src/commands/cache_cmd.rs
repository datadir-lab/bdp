//! `bdp cache` command implementation
//!
//! Manages the project-local cache directory configuration.

use std::path::Path;

use colored::Colorize;

use crate::{
    error::Result,
    project::{self, ProjectConfig},
};

/// Set the cache directory path
pub async fn set(path: String) -> Result<()> {
    let project_root = project::find_project_root()?;

    // Validate the path makes sense
    let resolved = if Path::new(&path).is_absolute() {
        std::path::PathBuf::from(&path)
    } else {
        project_root.join(&path)
    };

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&resolved)?;

    // Save config
    let mut config = ProjectConfig::load(&project_root)?;
    config.cache.path = path.clone();
    config.save(&project_root)?;

    println!("{} Cache directory set to: {}", "✓".green(), path);
    println!("  Resolved path: {}", resolved.display());

    Ok(())
}

/// Show the current cache directory
pub async fn show() -> Result<()> {
    let project_root = project::find_project_root()?;
    let config = ProjectConfig::load(&project_root)?;
    let resolved = project::resolve_cache_path(&project_root)?;

    println!("{}", "Cache Configuration:".cyan().bold());
    println!("  Configured path: {}", config.cache.path);
    println!("  Resolved path:   {}", resolved.display());
    println!("  Project root:    {}", project_root.display());

    // Show size if directory exists
    if resolved.exists() {
        let size = dir_size(&resolved);
        println!("  Cache size:      {}", crate::progress::format_bytes(size));
    }

    Ok(())
}

/// Reset the cache directory to default (.bdp/data)
pub async fn reset() -> Result<()> {
    let project_root = project::find_project_root()?;

    let config = ProjectConfig::default();
    config.save(&project_root)?;

    let resolved = project_root.join(&config.cache.path);
    std::fs::create_dir_all(&resolved)?;

    println!("{} Cache directory reset to default: {}", "✓".green(), config.cache.path);

    Ok(())
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
