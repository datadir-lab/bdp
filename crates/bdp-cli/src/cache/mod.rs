//! Cache management for downloaded datasets
//!
//! Uses the filesystem for storage. The lockfile (bdp.lock) tracks metadata;
//! this module only manages the on-disk file layout under `sources/`.

pub mod search_cache;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{error::Result, manifest::parse_source_spec, project};

/// Cache manager backed by the filesystem (no database)
pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    /// Create a cache manager for a specific project.
    /// Reads `.bdp/.config` to determine cache directory, defaults to
    /// `.bdp/data/`.
    pub fn for_project(project_root: &Path) -> Result<Self> {
        let cache_dir = project::resolve_cache_path(project_root)?;
        Self::new_with_dir(cache_dir)
    }

    /// Core constructor: create a cache manager with a specific directory.
    pub fn new_with_dir(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Check if a source file exists on disk
    pub fn is_cached(&self, spec: &str, format: &str) -> bool {
        self.get_cache_path(spec, format).exists()
    }

    /// Write file to cache directory and return the path written
    pub fn store(&self, spec: &str, format: &str, data: &[u8]) -> Result<PathBuf> {
        let cache_path = self.get_cache_path(spec, format);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cache_path, data)?;
        Ok(cache_path)
    }

    /// Walk `sources/` and sum file sizes
    pub fn total_size(&self) -> Result<u64> {
        let sources_dir = self.cache_dir.join("sources");
        if !sources_dir.exists() {
            return Ok(0);
        }
        let mut total: u64 = 0;
        for entry in walkdir::WalkDir::new(&sources_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        Ok(total)
    }

    /// Delete all files under `sources/` and return the number of files removed
    pub fn clear_all(&self) -> Result<usize> {
        let sources_dir = self.cache_dir.join("sources");
        if !sources_dir.exists() {
            return Ok(0);
        }

        // Count files first
        let count = walkdir::WalkDir::new(&sources_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        // Remove the entire sources directory tree
        fs::remove_dir_all(&sources_dir)?;

        // Also remove cache.db if it exists (leftover from previous versions)
        let legacy_db = self.cache_dir.join("cache.db");
        if legacy_db.exists() {
            let _ = fs::remove_file(&legacy_db);
        }
        let legacy_shm = self.cache_dir.join("cache.db-shm");
        if legacy_shm.exists() {
            let _ = fs::remove_file(&legacy_shm);
        }
        let legacy_wal = self.cache_dir.join("cache.db-wal");
        if legacy_wal.exists() {
            let _ = fs::remove_file(&legacy_wal);
        }

        Ok(count)
    }

    /// Get the cache path for a source specification
    ///
    /// Produces: `cache_dir/sources/{org}/{identifier}/{version}/{identifier}_{version}.{format}`
    /// Example: `uniprot:P01308-fasta@1.0` -> `.bdp/data/sources/uniprot/P01308/1.0/P01308_1.0.fasta`
    pub fn get_cache_path(&self, spec: &str, format: &str) -> PathBuf {
        // Use parse_source_spec to correctly separate identifier from format
        if let Ok((org, identifier, version, _spec_format)) = parse_source_spec(spec) {
            let filename = format!("{}_{}.{}", identifier, version, format);
            self.cache_dir
                .join("sources")
                .join(org)
                .join(&identifier)
                .join(&version)
                .join(filename)
        } else {
            // Fallback for unparseable specs
            self.cache_dir
                .join("sources")
                .join(spec.replace([':', '@'], "_"))
        }
    }

    /// Get cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn create_test_cache() -> Result<(CacheManager, TempDir)> {
        let temp_dir = TempDir::new()?;
        let cache_dir = temp_dir.path().join("bdp-test-cache");
        fs::create_dir_all(&cache_dir)?;
        Ok((CacheManager { cache_dir }, temp_dir))
    }

    #[test]
    fn test_cache_manager_creation() {
        let result = create_test_cache();
        assert!(result.is_ok());
        let (cache, _temp) = result.unwrap();
        assert!(cache.cache_dir().exists());
    }

    #[test]
    fn test_cache_path_generation() {
        let (cache, _temp) = create_test_cache().unwrap();

        // uniprot:P01308-fasta@1.0 -> sources/uniprot/P01308/1.0/P01308_1.0.fasta
        let path = cache.get_cache_path("uniprot:P01308-fasta@1.0", "fasta");
        let path_str = path.to_string_lossy().replace('\\', "/");
        assert!(
            path_str.ends_with("sources/uniprot/P01308/1.0/P01308_1.0.fasta"),
            "Expected .../sources/uniprot/P01308/1.0/P01308_1.0.fasta, got: {}",
            path_str
        );
        // Must NOT contain the format suffix in the directory name
        assert!(
            !path_str.contains("P01308-fasta"),
            "Directory should be P01308, not P01308-fasta: {}",
            path_str
        );
    }

    #[test]
    fn test_cache_path_without_format() {
        let (cache, _temp) = create_test_cache().unwrap();

        // ncbi:blast@2.14.0 (no format suffix) -> sources/ncbi/blast/2.14.0/blast_2.14.0.tar
        let path = cache.get_cache_path("ncbi:blast@2.14.0", "tar");
        let path_str = path.to_string_lossy().replace('\\', "/");
        assert!(
            path_str.ends_with("sources/ncbi/blast/2.14.0/blast_2.14.0.tar"),
            "Expected .../sources/ncbi/blast/2.14.0/blast_2.14.0.tar, got: {}",
            path_str
        );
    }

    #[test]
    fn test_cache_path_matches_generate() {
        let (cache, _temp) = create_test_cache().unwrap();

        // Verify cache path matches what generate.rs produces
        let path = cache.get_cache_path("uniprot:G4V4F9-fasta@1.0", "fasta");
        let path_str = path.to_string_lossy().replace('\\', "/");
        assert!(
            path_str.ends_with("sources/uniprot/G4V4F9/1.0/G4V4F9_1.0.fasta"),
            "Cache path must match generate output: {}",
            path_str
        );
    }

    #[test]
    fn test_store_and_retrieve() {
        let (cache, _temp) = create_test_cache().unwrap();
        let spec = "test:data-txt@1.0";
        let data = b"test data";

        let path = cache.store(spec, "txt", data).unwrap();
        assert!(path.exists());

        assert!(cache.is_cached(spec, "txt"));
        assert!(!cache.is_cached(spec, "fasta"));
    }

    #[test]
    fn test_total_size() {
        let (cache, _temp) = create_test_cache().unwrap();

        // Empty cache
        assert_eq!(cache.total_size().unwrap(), 0);

        // Store some data
        cache.store("test:a-txt@1.0", "txt", &[0u8; 100]).unwrap();
        cache.store("test:b-txt@1.0", "txt", &[0u8; 200]).unwrap();

        assert_eq!(cache.total_size().unwrap(), 300);
    }

    #[test]
    fn test_clear_all() {
        let (cache, _temp) = create_test_cache().unwrap();

        cache.store("test:a-txt@1.0", "txt", &[0u8; 100]).unwrap();
        cache.store("test:b-txt@1.0", "txt", &[0u8; 200]).unwrap();

        let count = cache.clear_all().unwrap();
        assert_eq!(count, 2);
        assert_eq!(cache.total_size().unwrap(), 0);
    }
}
