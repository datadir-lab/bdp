//! `bdp pull` command implementation
//!
//! Downloads and caches sources from the manifest.

use colored::Colorize;

use crate::{
    api::ApiClient,
    cache::CacheManager,
    checksum,
    commands::output::Render,
    error::{CliError, Result},
    lockfile::{Lockfile, SourceEntry},
    manifest::{parse_source_spec, Manifest},
    progress, project,
};

/// Result of a `bdp pull` operation.
pub enum PullOutput {
    /// Manifest has no sources to pull.
    NoSources,
    /// Pull completed successfully.
    Complete {
        total_sources: usize,
        cached_count: usize,
        downloaded_count: usize,
        /// When true, hooks ran and the summary was already printed inline
        /// (before hook output). Render skips the summary to avoid duplication.
        hooks_ran: bool,
    },
}

impl Render for PullOutput {
    fn render(&self) {
        use colored::Colorize;

        match self {
            PullOutput::NoSources => {
                println!("No sources to pull. Add sources with 'bdp source add'");
            },
            PullOutput::Complete { hooks_ran, .. } => {
                // When hooks ran, the summary was already printed inline before
                // hook output so the ordering stays correct. Skip it here.
                if !hooks_ran {
                    println!("\n{} All sources downloaded and verified", "✓".green().bold());
                    println!("Lockfile saved: bdp.lock");
                }
            },
        }
    }
}

/// Pull sources from manifest
pub async fn run(server_url: String, force: bool) -> Result<PullOutput> {
    // Find project root
    let project_root = project::find_project_root()?;

    // Load manifest
    let manifest = Manifest::load(project_root.join("bdp.yml")).map_err(|_| {
        CliError::NotInitialized(
            "No bdp.yml found in current directory. Initialize a project with 'bdp init' first."
                .to_string(),
        )
    })?;

    if manifest.sources.is_empty() {
        return Ok(PullOutput::NoSources);
    }

    println!("{} Resolving dependencies...", "→".cyan());

    // Initialize API client
    let api_client = ApiClient::new(server_url.clone())?;

    // Check server health
    if !api_client.health_check().await? {
        return Err(CliError::api(format!(
            "Cannot connect to BDP server at '{}'. Ensure the server is running or set \
             BDP_SERVER_URL to the correct address.",
            server_url
        )));
    }

    // Resolve manifest
    let resolved = api_client.resolve_manifest(&manifest).await?;

    println!("{} Found {} source(s)", "✓".green(), resolved.sources.len());

    // Initialize project-local cache
    let cache = CacheManager::for_project(&project_root)?;

    // Create/update lockfile
    let mut lockfile = Lockfile::new();

    let total_sources = resolved.sources.len();
    let mut cached_count: usize = 0;
    let mut downloaded_count: usize = 0;

    // Download sources
    for (spec, resolved_source) in &resolved.sources {
        // Parse spec to get components (needed for format and download)
        let (org, name, version, format) = parse_source_spec(spec)?;
        let format_str = format.as_deref().unwrap_or(&resolved_source.format);

        // Check if cached and not forcing
        if !force && cache.is_cached(spec, format_str) {
            println!("{} {} (cached)", "✓".green(), spec);
            cached_count += 1;

            // Add to lockfile
            let entry = SourceEntry::new(
                resolved_source.resolved.clone(),
                resolved_source.format.clone(),
                resolved_source.checksum.clone(),
                resolved_source.size,
                resolved_source.external_version.clone().unwrap_or_default(),
            );
            lockfile.add_source(spec.clone(), entry);

            continue;
        }

        println!("{} Downloading {}...", "↓".cyan(), spec);

        // Create progress bar
        let pb = progress::create_download_progress(resolved_source.size as u64, spec);

        // Download file: prefer presigned URL, fall back to legacy endpoint
        let bytes = if let Some(ref download_url) = resolved_source.download_url {
            tracing::debug!("Downloading from presigned URL for {}", spec);
            api_client.download_from_url(download_url).await?
        } else {
            tracing::debug!("Falling back to legacy download endpoint for {}", spec);
            api_client
                .download_file(&org, &name, &version, format_str)
                .await?
        };

        pb.set_position(bytes.len() as u64);
        pb.finish();

        // Verify checksum
        checksum::verify_checksum(&bytes, &resolved_source.checksum)?;

        // Store in cache
        cache.store(spec, format_str, &bytes)?;

        println!(
            "{} {} ({}) verified",
            "✓".green(),
            spec,
            progress::format_bytes(resolved_source.size as u64)
        );
        downloaded_count += 1;

        // Record download metric (best-effort, don't block pull)
        if let Err(e) = api_client
            .record_download(&org, &name, &version, format_str)
            .await
        {
            tracing::warn!("Failed to record download metric for {}: {}", spec, e);
        }

        // Add to lockfile
        let entry = SourceEntry::new(
            resolved_source.resolved.clone(),
            resolved_source.format.clone(),
            resolved_source.checksum.clone(),
            resolved_source.size,
            resolved_source.external_version.clone().unwrap_or_default(),
        );
        lockfile.add_source(spec.clone(), entry);
    }

    // Save lockfile
    lockfile.save(project_root.join("bdp.lock"))?;

    // Execute post-pull hooks (progressive output, stays inline).
    // The summary must appear before hook output, so we print it here
    // and set hooks_ran = true so render() skips it.
    let hooks_ran = if let Some(ref hooks) = manifest.hooks {
        if !hooks.post_pull.is_empty() {
            println!("\n{} All sources downloaded and verified", "✓".green().bold());
            println!("Lockfile saved: bdp.lock");
            println!("\n{} Running post-pull hooks...", "→".cyan());
            run_hooks(&hooks.post_pull, &project_root);
            true
        } else {
            false
        }
    } else {
        false
    };

    Ok(PullOutput::Complete {
        total_sources,
        cached_count,
        downloaded_count,
        hooks_ran,
    })
}

/// Execute hook commands sequentially. Hooks are best-effort: warn on failure.
fn run_hooks(commands: &[String], project_root: &std::path::Path) {
    for cmd in commands {
        tracing::info!("Running hook: {}", cmd);
        println!("  {} {}", "→".cyan(), cmd);

        let result = if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", cmd])
                .current_dir(project_root)
                .status()
        } else {
            std::process::Command::new("sh")
                .args(["-c", cmd])
                .current_dir(project_root)
                .status()
        };

        match result {
            Ok(status) if status.success() => {
                println!("  {} {}", "✓".green(), cmd);
            },
            Ok(status) => {
                tracing::warn!("Hook '{}' exited with status: {}", cmd, status);
                println!("  {} Hook '{}' exited with {}", "⚠".yellow(), cmd, status);
            },
            Err(e) => {
                tracing::warn!("Hook '{}' failed: {}", cmd, e);
                println!("  {} Hook '{}' failed: {}", "⚠".yellow(), cmd, e);
            },
        }
    }
}

#[cfg(test)]
mod tests {

    // Note: These tests require a running server, so they're integration tests
    // and should be run with `cargo test --features integration`

    #[tokio::test]
    #[ignore] // Requires server
    async fn test_pull_command() {
        // This would test against a live server
    }
}
