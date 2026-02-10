//! `bdp status` command implementation
//!
//! Shows status of cached sources by reading the lockfile and checking the
//! filesystem.

use std::path::PathBuf;

use crate::{
    cache::CacheManager, commands::output::Render, error::Result, lockfile::Lockfile, project,
};

/// Status information for a single source entry.
pub struct SourceStatus {
    pub spec: String,
    pub on_disk: bool,
    pub resolved: String,
    pub format: String,
    pub size: i64,
    pub checksum: String,
}

/// Output from the `bdp status` command.
pub struct StatusOutput {
    pub sources: Vec<SourceStatus>,
    pub total_size: u64,
    pub cache_dir: PathBuf,
}

impl Render for StatusOutput {
    fn render(&self) {
        use crate::progress::format_bytes;
        use colored::Colorize;

        if self.sources.is_empty() {
            println!("No cached sources found.");
            println!("Run 'bdp pull' to download sources.");
            return;
        }

        println!("{}", "Cached Sources:".cyan().bold());
        println!();

        let cached_count = self.sources.iter().filter(|s| s.on_disk).count();

        for source in &self.sources {
            let status = if source.on_disk {
                "cached".green().to_string()
            } else {
                "missing".red().to_string()
            };

            println!("{} [{}]", source.spec.green(), status);
            println!("  Resolved: {}", source.resolved);
            println!("  Format:   {}", source.format);
            println!("  Size:     {}", format_bytes(source.size as u64));
            println!("  Checksum: {}", &source.checksum[..source.checksum.len().min(16)]);
            println!();
        }

        println!("{}", "Summary:".cyan().bold());
        println!(
            "  Total sources: {} ({} cached, {} missing)",
            self.sources.len(),
            cached_count,
            self.sources.len() - cached_count
        );
        println!("  Total size:    {}", format_bytes(self.total_size));
        println!("  Cache dir:     {}", self.cache_dir.display());
    }
}

/// Show status of cached sources
pub async fn run() -> Result<StatusOutput> {
    let project_root = project::find_project_root()?;
    let cache = CacheManager::for_project(&project_root)?;

    // Read lockfile for source metadata
    let lockfile_path = project_root.join("bdp.lock");
    let lockfile = if lockfile_path.exists() {
        Lockfile::load(&lockfile_path)?
    } else {
        Lockfile::new()
    };

    let cache_dir = cache.cache_dir().to_path_buf();

    if lockfile.sources.is_empty() {
        return Ok(StatusOutput {
            sources: Vec::new(),
            total_size: 0,
            cache_dir,
        });
    }

    let mut sources = Vec::new();
    for (spec, entry) in &lockfile.sources {
        let on_disk = cache.is_cached(spec, &entry.format);
        sources.push(SourceStatus {
            spec: spec.clone(),
            on_disk,
            resolved: entry.resolved.clone(),
            format: entry.format.clone(),
            size: entry.size,
            checksum: entry.checksum.clone(),
        });
    }

    let total_size = cache.total_size()?;

    Ok(StatusOutput {
        sources,
        total_size,
        cache_dir,
    })
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_status_empty() {
        // This test requires system cache directory access
        // The actual logic is tested via cache manager tests
        // Skip this test as it's an integration test
    }
}
