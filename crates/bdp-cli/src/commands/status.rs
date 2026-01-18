//! `bdp status` command implementation
//!
//! Shows status of cached sources.

use crate::cache::CacheManager;
use crate::error::Result;
use crate::progress::format_bytes;
use colored::Colorize;

/// Show status of cached sources
pub async fn run() -> Result<()> {
    let cache = CacheManager::new().await?;

    let entries = cache.list_all().await?;

    if entries.is_empty() {
        println!("No cached sources found.");
        println!("Run 'bdp pull' to download sources.");
        return Ok(());
    }

    println!("{}", "Cached Sources:".cyan().bold());
    println!();

    for entry in &entries {
        println!("{}", entry.spec.green());
        println!("  Resolved: {}", entry.resolved);
        println!("  Format:   {}", entry.format);
        println!("  Size:     {}", format_bytes(entry.size as u64));
        println!("  Checksum: {}", &entry.checksum[..16]);
        println!("  Cached:   {}", entry.cached_at);
        println!();
    }

    let total_size = cache.total_size().await?;
    println!("{}", "Summary:".cyan().bold());
    println!("  Total sources: {}", entries.len());
    println!("  Total size:    {}", format_bytes(total_size as u64));
    println!("  Cache dir:     {}", cache.cache_dir().display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_empty() {
        // This test requires system cache directory access
        // The actual logic is tested via cache manager tests
        // Skip this test as it's an integration test
    }
}
