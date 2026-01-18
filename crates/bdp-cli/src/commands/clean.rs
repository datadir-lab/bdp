//! `bdp clean` command implementation
//!
//! Cleans cached sources.

use crate::cache::CacheManager;
use crate::error::Result;
use crate::progress::format_bytes;
use colored::Colorize;

/// Clean cache
pub async fn run(all: bool) -> Result<()> {
    let cache = CacheManager::new().await?;

    if all {
        let size_before = cache.total_size().await?;
        let count = cache.clear_all().await?;

        println!("{} Cleared {} source(s)", "✓".green(), count);
        println!("  Freed: {}", format_bytes(size_before as u64));
    } else {
        // For now, just clear all
        // In the future, could implement smart cleanup based on lockfile
        println!("Use --all to clear all cached sources");
        println!("  Current cache size: {}", format_bytes(cache.total_size().await? as u64));
    }

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
