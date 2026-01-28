//! `bdp clean` command implementation
//!
//! Cleans cached sources.

use crate::cache::search_cache::SearchCache;
use crate::cache::CacheManager;
use crate::error::{CliError, Result};
use crate::progress::format_bytes;
use colored::Colorize;

/// Clean cache
pub async fn run(all: bool, search_cache_only: bool) -> Result<()> {
    // Clean search cache if requested
    if search_cache_only {
        return clean_search_cache().await;
    }

    // Clean data cache
    let cache = CacheManager::new().await?;

    if all {
        let size_before = cache.total_size().await?;
        let count = cache.clear_all().await?;

        println!("{} Cleared {} source(s)", "✓".green(), count);
        println!("  Freed: {}", format_bytes(size_before as u64));

        // Also clean search cache when cleaning all
        let _ = clean_search_cache().await;
    } else {
        // For now, just clear all
        // In the future, could implement smart cleanup based on lockfile
        println!("Use --all to clear all cached sources");
        println!("  Current cache size: {}", format_bytes(cache.total_size().await? as u64));
    }

    Ok(())
}

/// Clean search cache
async fn clean_search_cache() -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| CliError::config("Cannot find cache directory"))?
        .join("bdp");

    std::fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join("bdp.db");

    let cache = SearchCache::new(cache_path)?;
    cache.init()?;

    let count = cache.clear()?;

    println!("{} Cleared {} search cache entries", "✓".green(), count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clean_all() {
        // This test requires system cache directory access
        // The actual logic is tested via cache manager tests
        // Skip this test as it's an integration test
    }
}
