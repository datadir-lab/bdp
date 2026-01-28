//! Cache management for downloaded datasets
//!
//! Uses SQLite for tracking cached files and the file system for storage.

pub mod search_cache;

use crate::error::{CliError, Result};
use sqlx::{sqlite::SqlitePool, Row};
use std::fs;
use std::path::PathBuf;

/// Cache manager with SQLite backend
pub struct CacheManager {
    pool: SqlitePool,
    cache_dir: PathBuf,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| CliError::config("Cannot find cache directory"))?
            .join("bdp");

        fs::create_dir_all(&cache_dir)?;

        let db_path = cache_dir.join("bdp.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = SqlitePool::connect(&db_url).await?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| CliError::cache(format!("Migration failed: {}", e)))?;

        Ok(Self { pool, cache_dir })
    }

    /// Store a file in the cache
    pub async fn store(
        &self,
        spec: &str,
        resolved: &str,
        format: &str,
        data: Vec<u8>,
        checksum: &str,
    ) -> Result<()> {
        let size = data.len() as i64;

        // Create directory structure: cache_dir/sources/{org}/{name}/{version}/
        let cache_path = self.get_cache_path(spec, format);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write file
        fs::write(&cache_path, data)?;

        // Insert or update database entry
        sqlx::query(
            r#"
            INSERT INTO cache_entries (spec, resolved, format, checksum, size, cached_at, last_accessed, path)
            VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'), ?6)
            ON CONFLICT(spec) DO UPDATE SET
                resolved = excluded.resolved,
                format = excluded.format,
                checksum = excluded.checksum,
                size = excluded.size,
                cached_at = datetime('now'),
                last_accessed = datetime('now'),
                path = excluded.path
            "#,
        )
        .bind(spec)
        .bind(resolved)
        .bind(format)
        .bind(checksum)
        .bind(size)
        .bind(cache_path.to_string_lossy().to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if a source is cached
    pub async fn is_cached(&self, spec: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM cache_entries WHERE spec = ?1
            "#,
        )
        .bind(spec)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = result.get("count");
        Ok(count > 0)
    }

    /// Get the file path for a cached source
    pub async fn get_path(&self, spec: &str) -> Result<Option<PathBuf>> {
        let result = sqlx::query(
            r#"
            SELECT path FROM cache_entries WHERE spec = ?1
            "#,
        )
        .bind(spec)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                let path: String = row.get("path");
                // Update last_accessed
                let _ = self.update_last_accessed(spec).await;
                Ok(Some(PathBuf::from(path)))
            },
            None => Ok(None),
        }
    }

    /// Get cached entry details
    pub async fn get_entry(&self, spec: &str) -> Result<Option<CacheEntry>> {
        let result = sqlx::query_as::<_, CacheEntry>(
            r#"
            SELECT id, spec, resolved, format, checksum, size, cached_at, last_accessed, path
            FROM cache_entries WHERE spec = ?1
            "#,
        )
        .bind(spec)
        .fetch_optional(&self.pool)
        .await?;

        if result.is_some() {
            let _ = self.update_last_accessed(spec).await;
        }

        Ok(result)
    }

    /// List all cached entries
    pub async fn list_all(&self) -> Result<Vec<CacheEntry>> {
        let entries = sqlx::query_as::<_, CacheEntry>(
            r#"
            SELECT id, spec, resolved, format, checksum, size, cached_at, last_accessed, path
            FROM cache_entries
            ORDER BY cached_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Remove a cached entry
    pub async fn remove(&self, spec: &str) -> Result<bool> {
        // Get the path first
        if let Some(path) = self.get_path(spec).await? {
            // Delete file
            if path.exists() {
                fs::remove_file(&path)?;
            }

            // Clean up empty parent directories
            if let Some(parent) = path.parent() {
                let _ = fs::remove_dir(parent); // Ignore errors if not empty
            }
        }

        // Delete from database
        let result = sqlx::query(
            r#"
            DELETE FROM cache_entries WHERE spec = ?1
            "#,
        )
        .bind(spec)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear all cache
    pub async fn clear_all(&self) -> Result<usize> {
        // Get all entries
        let entries = self.list_all().await?;
        let count = entries.len();

        // Delete all files
        for entry in entries {
            let path = PathBuf::from(&entry.path);
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
        }

        // Clear database
        sqlx::query("DELETE FROM cache_entries")
            .execute(&self.pool)
            .await?;

        // Optionally clean up cache directory structure
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    let _ = fs::remove_dir_all(entry.path());
                }
            }
        }

        Ok(count)
    }

    /// Get total cache size in bytes
    pub async fn total_size(&self) -> Result<i64> {
        let result = sqlx::query(
            r#"
            SELECT COALESCE(SUM(size), 0) as total FROM cache_entries
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = result.get("total");
        Ok(total)
    }

    /// Get cache directory
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Get the cache path for a source specification
    fn get_cache_path(&self, spec: &str, format: &str) -> PathBuf {
        // spec: "uniprot:P01308-fasta@1.0"
        // Extract components
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() != 2 {
            // Fallback for invalid specs
            return self.cache_dir.join("sources").join(spec.replace(':', "_"));
        }

        let org = parts[0];
        let name_version = parts[1];

        let version_parts: Vec<&str> = name_version.split('@').collect();
        if version_parts.len() != 2 {
            return self.cache_dir.join("sources").join(org).join(name_version);
        }

        let name = version_parts[0];
        let version = version_parts[1];

        // Remove format suffix if present in version
        let version_clean = version.split('-').next().unwrap_or(version);

        // Path: cache_dir/sources/org/name/version/name_version.format
        let filename = format!("{}_{}.{}", name, version_clean, format);
        self.cache_dir
            .join("sources")
            .join(org)
            .join(name)
            .join(version_clean)
            .join(filename)
    }

    /// Update last accessed time
    async fn update_last_accessed(&self, spec: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE cache_entries SET last_accessed = datetime('now') WHERE spec = ?1
            "#,
        )
        .bind(spec)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Cache entry record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CacheEntry {
    pub id: i64,
    pub spec: String,
    pub resolved: String,
    pub format: String,
    pub checksum: String,
    pub size: i64,
    pub cached_at: String,
    pub last_accessed: String,
    pub path: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_cache() -> Result<(CacheManager, TempDir)> {
        let temp_dir = TempDir::new()?;
        let cache_dir = temp_dir.path().join("bdp-test-cache");
        fs::create_dir_all(&cache_dir)?;

        // Use in-memory SQLite to avoid Windows permission issues
        let pool = SqlitePool::connect("sqlite::memory:").await?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| CliError::cache(format!("Migration failed: {}", e)))?;

        Ok((CacheManager { pool, cache_dir }, temp_dir))
    }

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let result = create_test_cache().await;
        assert!(result.is_ok());
        let (cache, _temp) = result.unwrap();
        assert!(cache.cache_dir().exists());
    }

    #[tokio::test]
    async fn test_cache_path_generation() {
        let (cache, _temp) = create_test_cache().await.unwrap();
        let path = cache.get_cache_path("uniprot:P01308-fasta@1.0", "fasta");
        assert!(path.to_string_lossy().contains("uniprot"));
        assert!(path.to_string_lossy().contains("P01308"));
        assert!(path.to_string_lossy().contains("1.0"));
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let (cache, _temp) = create_test_cache().await.unwrap();
        let spec = "test:data-txt@1.0";
        let data = b"test data".to_vec();
        let checksum = "abc123";

        cache
            .store(spec, "test:data@1.0", "txt", data.clone(), checksum)
            .await
            .unwrap();

        assert!(cache.is_cached(spec).await.unwrap());

        let path = cache.get_path(spec).await.unwrap();
        assert!(path.is_some());

        let entry = cache.get_entry(spec).await.unwrap();
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.spec, spec);
        assert_eq!(entry.checksum, checksum);

        // Cleanup
        cache.remove(spec).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_all() {
        let (cache, _temp) = create_test_cache().await.unwrap();

        // Store multiple entries
        for i in 1..=3 {
            let spec = format!("test:data{}@1.0-txt", i);
            cache
                .store(&spec, &spec, "txt", vec![0u8; 100], "checksum")
                .await
                .unwrap();
        }

        let entries = cache.list_all().await.unwrap();
        assert_eq!(entries.len(), 3);

        // Cleanup
        for i in 1..=3 {
            let spec = format!("test:data{}@1.0-txt", i);
            cache.remove(&spec).await.unwrap();
        }
    }
}
