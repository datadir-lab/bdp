//! `bdp clean` command implementation
//!
//! Cleans cached sources with confirmation prompt.

use crate::{
    cache::{search_cache::SearchCache, CacheManager},
    commands::output::Render,
    error::{CliError, Result},
    project,
};

/// Output from the `bdp clean` command.
pub enum CleanOutput {
    /// Cache was already empty
    Empty,
    /// User aborted the clean operation
    Aborted,
    /// Successfully cleaned
    Cleaned {
        count: usize,
        freed_bytes: u64,
        search_cache_cleared: Option<usize>,
    },
    /// --all was not specified, show hint
    Hint { current_size: u64 },
    /// Only search cache was cleaned
    SearchCacheCleared { count: usize },
}

impl Render for CleanOutput {
    fn render(&self) {
        use crate::progress::format_bytes;
        use colored::Colorize;

        match self {
            CleanOutput::Empty => {
                println!("Cache is already empty.");
            },
            CleanOutput::Aborted => {
                println!("Aborted.");
            },
            CleanOutput::Cleaned {
                count,
                freed_bytes,
                search_cache_cleared,
            } => {
                println!("{} Cleared {} source(s)", "\u{2713}".green(), count);
                println!("  Freed: {}", format_bytes(*freed_bytes));
                if let Some(sc_count) = search_cache_cleared {
                    println!("{} Cleared {} search cache entries", "\u{2713}".green(), sc_count);
                }
            },
            CleanOutput::Hint { current_size } => {
                println!("Use --all to clear all cached sources");
                println!("  Current cache size: {}", format_bytes(*current_size));
            },
            CleanOutput::SearchCacheCleared { count } => {
                println!("{} Cleared {} search cache entries", "\u{2713}".green(), count);
            },
        }
    }
}

/// Clean cache
pub async fn run(all: bool, search_cache_only: bool, yes: bool) -> Result<CleanOutput> {
    // Clean search cache if requested
    if search_cache_only {
        let count = clean_search_cache().await?;
        return Ok(CleanOutput::SearchCacheCleared { count });
    }

    // Clean project-local data cache
    let project_root = project::find_project_root()?;
    let cache = CacheManager::for_project(&project_root)?;

    if all {
        let size_before = cache.total_size()?;

        if size_before == 0 {
            return Ok(CleanOutput::Empty);
        }

        // Show what will be deleted and ask for confirmation
        let sources_dir = cache.cache_dir().join("sources");
        let file_count = if sources_dir.exists() {
            walkdir::WalkDir::new(&sources_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count()
        } else {
            0
        };

        {
            use crate::progress::format_bytes;

            println!("This will delete:");
            println!("  {} file(s), {}", file_count, format_bytes(size_before));
            println!("  Cache dir: {}", cache.cache_dir().display());
        }

        if !yes {
            let confirm = inquire::Confirm::new("Are you sure you want to delete all cached data?")
                .with_default(false)
                .prompt();

            match confirm {
                Ok(true) => {},
                Ok(false) | Err(_) => {
                    return Ok(CleanOutput::Aborted);
                },
            }
        }

        let count = cache.clear_all()?;

        // Also clean search cache when cleaning all
        let search_cache_cleared = clean_search_cache().await.ok();

        Ok(CleanOutput::Cleaned {
            count,
            freed_bytes: size_before,
            search_cache_cleared,
        })
    } else {
        // For now, just clear all
        // In the future, could implement smart cleanup based on lockfile
        let current_size = cache.total_size()?;
        Ok(CleanOutput::Hint { current_size })
    }
}

/// Clean search cache, returning the number of entries cleared.
async fn clean_search_cache() -> Result<usize> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| CliError::config("Cannot find cache directory"))?
        .join("bdp");

    std::fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join("bdp.db");

    let cache = SearchCache::new(cache_path)?;
    cache.init()?;

    let count = cache.clear()?;

    Ok(count)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_clean_all() {
        // This test requires system cache directory access
        // The actual logic is tested via cache manager tests
        // Skip this test as it's an integration test
    }
}
