//! `bdp audit` command implementation
//!
//! Audits integrity of cached sources.

use crate::cache::CacheManager;
use crate::checksum;
use crate::error::{CliError, Result};
use crate::lockfile::Lockfile;
use colored::Colorize;
use std::fs;

/// Audit cached sources
pub async fn run() -> Result<()> {
    // Load lockfile
    let lockfile = Lockfile::load("bdl.lock")
        .map_err(|_| CliError::NotInitialized("No lockfile found. Run 'bdp pull' first".to_string()))?;

    if lockfile.is_empty() {
        println!("Lockfile is empty. Nothing to audit.");
        return Ok(());
    }

    println!("{} Auditing {} source(s)...", "→".cyan(), lockfile.sources.len());

    let cache = CacheManager::new().await?;

    let mut errors = Vec::new();
    let mut verified = 0;

    for (spec, entry) in &lockfile.sources {
        // Check if cached
        match cache.get_path(spec).await? {
            Some(path) => {
                if !path.exists() {
                    errors.push(format!("{}: file not found at {}", spec, path.display()));
                    continue;
                }

                // Read file and verify checksum
                match fs::read(&path) {
                    Ok(bytes) => {
                        match checksum::verify_checksum(&bytes, &entry.checksum) {
                            Ok(_) => {
                                println!("{} {}", "✓".green(), spec);
                                verified += 1;
                            }
                            Err(_) => {
                                errors.push(format!("{}: checksum mismatch", spec));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("{}: failed to read file: {}", spec, e));
                    }
                }
            }
            None => {
                errors.push(format!("{}: not cached", spec));
            }
        }
    }

    println!();

    if errors.is_empty() {
        println!("{} All {} source(s) verified successfully", "✓".green().bold(), verified);
        Ok(())
    } else {
        println!("{} Verification failed for {} source(s):", "✗".red().bold(), errors.len());
        for error in &errors {
            println!("  {} {}", "✗".red(), error);
        }
        Err(CliError::cache(format!("{} verification failures", errors.len())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires lockfile
    async fn test_audit_no_lockfile() {
        // Test with missing lockfile
    }
}
