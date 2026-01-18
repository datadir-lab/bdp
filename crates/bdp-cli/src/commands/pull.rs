//! `bdp pull` command implementation
//!
//! Downloads and caches sources from the manifest.

use crate::api::ApiClient;
use crate::cache::CacheManager;
use crate::checksum;
use crate::error::{CliError, Result};
use crate::lockfile::{Lockfile, SourceEntry};
use crate::manifest::{parse_source_spec, Manifest};
use crate::progress;
use colored::Colorize;

/// Pull sources from manifest
pub async fn run(server_url: String, force: bool) -> Result<()> {
    // Load manifest
    let manifest = Manifest::load("bdp.yml")
        .map_err(|_| CliError::NotInitialized("Run 'bdp init' first".to_string()))?;

    if manifest.sources.is_empty() {
        println!("No sources to pull. Add sources with 'bdp source add'");
        return Ok(());
    }

    println!("{} Resolving dependencies...", "→".cyan());

    // Initialize API client
    let api_client = ApiClient::new(server_url)?;

    // Check server health
    if !api_client.health_check().await? {
        return Err(CliError::api("Cannot connect to BDP server"));
    }

    // Resolve manifest
    let resolved = api_client.resolve_manifest(&manifest).await?;

    println!("{} Found {} source(s)", "✓".green(), resolved.sources.len());

    // Initialize cache
    let cache = CacheManager::new().await?;

    // Create/update lockfile
    let mut lockfile = Lockfile::new();

    // Download sources
    for (spec, resolved_source) in &resolved.sources {
        // Check if cached and not forcing
        if !force && cache.is_cached(spec).await? {
            println!("{} {} (cached)", "✓".green(), spec);

            // Add to lockfile
            let entry = SourceEntry::new(
                resolved_source.resolved.clone(),
                resolved_source.format.clone(),
                resolved_source.checksum.clone(),
                resolved_source.size,
                resolved_source.external_version.clone(),
            );
            lockfile.add_source(spec.clone(), entry);

            continue;
        }

        println!("{} Downloading {}...", "↓".cyan(), spec);

        // Parse spec to get components
        let (org, name, version, format) = parse_source_spec(spec)?;
        let format_str = format.as_deref().unwrap_or(&resolved_source.format);

        // Create progress bar
        let pb = progress::create_download_progress(resolved_source.size as u64, spec);

        // Download file
        let bytes = api_client
            .download_file(&org, &name, &version, format_str)
            .await?;

        pb.set_position(bytes.len() as u64);
        pb.finish();

        // Verify checksum
        checksum::verify_checksum(&bytes, &resolved_source.checksum)?;

        // Store in cache
        cache
            .store(
                spec,
                &resolved_source.resolved,
                format_str,
                bytes,
                &resolved_source.checksum,
            )
            .await?;

        println!("{} {} ({}) verified", "✓".green(), spec, progress::format_bytes(resolved_source.size as u64));

        // Add to lockfile
        let entry = SourceEntry::new(
            resolved_source.resolved.clone(),
            resolved_source.format.clone(),
            resolved_source.checksum.clone(),
            resolved_source.size,
            resolved_source.external_version.clone(),
        );
        lockfile.add_source(spec.clone(), entry);
    }

    // Save lockfile
    lockfile.save("bdl.lock")?;

    println!("\n{} All sources downloaded and verified", "✓".green().bold());
    println!("Lockfile saved: bdl.lock");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running server, so they're integration tests
    // and should be run with `cargo test --features integration`

    #[tokio::test]
    #[ignore] // Requires server
    async fn test_pull_command() {
        // This would test against a live server
    }
}
