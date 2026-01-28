//! Search result caching
//!
//! Caches search results in SQLite to reduce API calls and improve performance.

use crate::api::types::SearchResponse;
use crate::error::{CliError, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tracing::{debug, info};

/// Default cache TTL in minutes
const DEFAULT_CACHE_TTL_MINUTES: i64 = 5;

/// Search cache manager
pub struct SearchCache {
    db_path: PathBuf,
    ttl_minutes: i64,
}

impl SearchCache {
    /// Create a new search cache
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let ttl_minutes = std::env::var("BDP_SEARCH_CACHE_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_CACHE_TTL_MINUTES);

        Ok(Self {
            db_path,
            ttl_minutes,
        })
    }

    /// Open a connection to the cache database
    fn open_connection(&self) -> Result<Connection> {
        // Ensure parent directory exists
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(Connection::open(&self.db_path)?)
    }

    /// Initialize the cache schema
    pub fn init(&self) -> Result<()> {
        {
            let conn = self.open_connection()?;

            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS search_cache (
                    query_hash TEXT PRIMARY KEY,
                    query_text TEXT NOT NULL,
                    filters_json TEXT,
                    results_json TEXT NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    expires_at TIMESTAMP NOT NULL
                )
                "#,
                [],
            )?;

            // Create index on expires_at for efficient cleanup
            conn.execute(
                r#"
                CREATE INDEX IF NOT EXISTS idx_search_cache_expires_at
                ON search_cache(expires_at)
                "#,
                [],
            )?;

            // Explicitly close connection to ensure changes are flushed
        } // conn is dropped here

        debug!("Search cache schema initialized");
        Ok(())
    }

    /// Get cached search results
    pub fn get(
        &self,
        query: &str,
        filters: &SearchFilters,
    ) -> Result<Option<SearchResponse>> {
        let hash = self.hash_query(query, filters);
        let conn = self.open_connection()?;

        let result: rusqlite::Result<(String, String)> = conn.query_row(
            r#"
            SELECT results_json, expires_at
            FROM search_cache
            WHERE query_hash = ?1
            "#,
            params![hash],
            |row| {
                Ok((
                    row.get(0)?,  // results_json
                    row.get(1)?,  // expires_at
                ))
            },
        );

        match result {
            Ok((results_json, expires_at)) => {
                // Check if cache is expired
                let expires_at: DateTime<Utc> = expires_at.parse().map_err(|e| {
                    CliError::cache(format!("Failed to parse expiration date: {}", e))
                })?;

                if Utc::now() > expires_at {
                    debug!(query = %query, "Cache entry expired");
                    // Clean up expired entry
                    let _ = self.delete(&hash);
                    return Ok(None);
                }

                // Deserialize cached response
                let response: SearchResponse = serde_json::from_str(&results_json)?;
                info!(query = %query, results = response.results.len(), "Cache hit");
                Ok(Some(response))
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(query = %query, "Cache miss");
                Ok(None)
            },
            Err(e) => Err(CliError::from(e)),
        }
    }

    /// Store search results in cache
    pub fn set(
        &self,
        query: &str,
        filters: &SearchFilters,
        response: &SearchResponse,
    ) -> Result<()> {
        let hash = self.hash_query(query, filters);
        let filters_json = serde_json::to_string(filters)?;
        let results_json = serde_json::to_string(response)?;

        let expires_at = Utc::now() + Duration::minutes(self.ttl_minutes);
        let expires_at_str = expires_at.to_rfc3339();

        let conn = self.open_connection()?;
        conn.execute(
            r#"
            INSERT OR REPLACE INTO search_cache
            (query_hash, query_text, filters_json, results_json, created_at, expires_at)
            VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, ?5)
            "#,
            params![hash, query, filters_json, results_json, expires_at_str],
        )?;

        debug!(query = %query, ttl_minutes = self.ttl_minutes, "Cached search results");
        Ok(())
    }

    /// Delete a specific cache entry
    fn delete(&self, hash: &str) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "DELETE FROM search_cache WHERE query_hash = ?1",
            params![hash],
        )?;
        Ok(())
    }

    /// Clear all search cache
    pub fn clear(&self) -> Result<usize> {
        let conn = self.open_connection()?;
        let count = conn.execute("DELETE FROM search_cache", [])?;
        info!(count = count, "Cleared search cache");
        Ok(count)
    }

    /// Clean up expired cache entries
    pub fn cleanup_expired(&self) -> Result<usize> {
        let conn = self.open_connection()?;
        let now = Utc::now().to_rfc3339();
        let count = conn.execute(
            "DELETE FROM search_cache WHERE expires_at < ?1",
            params![now],
        )?;

        if count > 0 {
            debug!(count = count, "Cleaned up expired cache entries");
        }
        Ok(count)
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let conn = self.open_connection()?;

        let total: i64 = conn.query_row("SELECT COUNT(*) FROM search_cache", [], |row| {
            row.get(0)
        })?;

        let expired: i64 = conn.query_row(
            "SELECT COUNT(*) FROM search_cache WHERE expires_at < ?1",
            params![Utc::now().to_rfc3339()],
            |row| row.get(0),
        )?;

        Ok(CacheStats {
            total_entries: total as usize,
            expired_entries: expired as usize,
            valid_entries: (total - expired) as usize,
        })
    }

    /// Generate cache key from query and filters
    fn hash_query(&self, query: &str, filters: &SearchFilters) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hasher.update(serde_json::to_string(filters).unwrap_or_default().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Search filters for cache key generation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchFilters {
    pub type_filter: Option<Vec<String>>,
    pub source_type_filter: Option<Vec<String>>,
    pub organism: Option<String>,
    pub format: Option<String>,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_cache() -> (SearchCache, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_cache.db");
        let cache = SearchCache::new(db_path.clone()).unwrap();
        cache.init().unwrap();
        (cache, dir)
    }

    fn mock_search_response() -> SearchResponse {
        SearchResponse {
            results: vec![],
            total: 0,
            page: 1,
            page_size: 10,
        }
    }

    fn mock_filters() -> SearchFilters {
        SearchFilters {
            type_filter: Some(vec!["data_source".to_string()]),
            source_type_filter: None,
            organism: None,
            format: None,
        }
    }

    #[test]
    fn test_cache_init() {
        let (cache, _dir) = create_test_cache();
        // Check that the database file exists
        assert!(cache.db_path.exists(), "Database file should exist after init");
        // Check that we can get stats (which queries the table)
        let stats_result = cache.stats();
        if let Err(ref e) = stats_result {
            eprintln!("Stats error: {:?}", e);
            eprintln!("DB path: {:?}", cache.db_path);
        }
        assert!(stats_result.is_ok());
    }

    #[test]
    fn test_cache_set_and_get() {
        let (cache, _dir) = create_test_cache();
        let response = mock_search_response();
        let filters = mock_filters();

        // Verify database file exists
        assert!(cache.db_path.exists(), "DB file should exist after init");

        // Try to set a value
        let set_result = cache.set("insulin", &filters, &response);
        if let Err(ref e) = set_result {
            eprintln!("Set error: {:?}", e);
            eprintln!("DB path: {:?}", cache.db_path);
            eprintln!("DB exists: {}", cache.db_path.exists());
        }
        set_result.unwrap();

        let cached = cache.get("insulin", &filters).unwrap();
        assert!(cached.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let (cache, _dir) = create_test_cache();
        let filters = mock_filters();
        let cached = cache.get("nonexistent", &filters).unwrap();
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let (cache, _dir) = create_test_cache();
        let response = mock_search_response();
        let filters = mock_filters();

        cache.set("insulin", &filters, &response).unwrap();
        let count = cache.clear().unwrap();
        assert_eq!(count, 1);

        let cached = cache.get("insulin", &filters).unwrap();
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_hash_consistency() {
        let (cache, _dir) = create_test_cache();
        let filters = mock_filters();

        let hash1 = cache.hash_query("insulin", &filters);
        let hash2 = cache.hash_query("insulin", &filters);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_cache_hash_differs_with_different_filters() {
        let (cache, _dir) = create_test_cache();
        let filters1 = mock_filters();
        let mut filters2 = mock_filters();
        filters2.organism = Some("human".to_string());

        let hash1 = cache.hash_query("insulin", &filters1);
        let hash2 = cache.hash_query("insulin", &filters2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cache_stats() {
        let (cache, _dir) = create_test_cache();
        let response = mock_search_response();
        let filters = mock_filters();

        cache.set("query1", &filters, &response).unwrap();
        cache.set("query2", &filters, &response).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 2);
    }
}
