//! InterPro Version Discovery Example
//!
//! Demonstrates discovering available InterPro versions from FTP
//!
//! Usage:
//!   cargo run --example interpro_version_discovery

use anyhow::Result;
use bdp_server::ingest::interpro::{config::InterProConfig, version_discovery::VersionDiscovery};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("InterPro Version Discovery Example");
    tracing::info!("===================================");

    // Create configuration
    let config = InterProConfig::from_env();
    tracing::info!(
        "FTP: {}{}",
        config.ftp_host,
        config.ftp_path
    );

    // Create version discovery service
    let discovery = VersionDiscovery::new(config);

    // Discover all versions
    tracing::info!("Discovering all available InterPro versions...");
    match discovery.discover_all_versions().await {
        Ok(versions) => {
            tracing::info!("Found {} versions:", versions.len());
            tracing::info!("");

            for (i, version) in versions.iter().enumerate() {
                let current_marker = if version.is_current { " (CURRENT)" } else { "" };
                tracing::info!(
                    "{:3}. Version {} - Released: {} - Dir: {}{}",
                    i + 1,
                    version.external_version,
                    version.release_date,
                    version.ftp_directory,
                    current_marker
                );
            }

            tracing::info!("");
            tracing::info!("Summary:");
            tracing::info!("  Total versions: {}", versions.len());
            tracing::info!("  Earliest: {}", versions.first().unwrap().external_version);
            tracing::info!("  Latest: {}", versions.last().unwrap().external_version);

            // Demonstrate filtering
            tracing::info!("");
            tracing::info!("Example: Versions from 96.0 onwards:");
            let filtered = discovery.filter_from_version(versions.clone(), "96.0")?;
            for version in filtered.iter().take(5) {
                tracing::info!("  - {}", version.external_version);
            }
            if filtered.len() > 5 {
                tracing::info!("  ... and {} more", filtered.len() - 5);
            }
        }
        Err(e) => {
            tracing::error!("Version discovery failed: {}", e);
            tracing::error!("Possible causes:");
            tracing::error!("  1. FTP server is unreachable");
            tracing::error!("  2. Network/firewall blocking FTP passive mode");
            tracing::error!("  3. InterPro FTP directory structure changed");
            return Err(e);
        }
    }

    tracing::info!("");
    tracing::info!("Version discovery complete!");

    Ok(())
}
