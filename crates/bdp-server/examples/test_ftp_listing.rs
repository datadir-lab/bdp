//! Test program to verify FTP directory listing implementation
//!
//! This program connects to the UniProt FTP server and lists the previous releases.
//! Run with: cargo run --example test_ftp_listing

use bdp_server::ingest::uniprot::{config::UniProtFtpConfig, version_discovery::VersionDiscovery};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Testing FTP directory listing for UniProt previous releases...");

    // Create config
    let config = UniProtFtpConfig::default();
    info!(
        ftp_server = %config.ftp_host,
        ftp_port = config.ftp_port,
        base_path = %config.ftp_base_path,
        username = %config.ftp_username,
        "FTP configuration"
    );

    // Create version discovery service
    let discovery = VersionDiscovery::new(config);

    // Test listing all available versions
    info!("Discovering all available versions from FTP...");
    match discovery.discover_all_versions().await {
        Ok(versions) => {
            info!(count = versions.len(), "Successfully discovered versions");
            info!("Version          Date         Current    FTP Path");
            info!("{}", "-".repeat(70));

            for version in &versions {
                info!(
                    version = %version.external_version,
                    date = %version.release_date.format("%Y-%m-%d"),
                    is_current = version.is_current,
                    ftp_path = %version.ftp_path,
                    "Version details"
                );
            }

            info!("Test completed successfully!");
            Ok(())
        },
        Err(e) => {
            error!(error = %e, "Error discovering versions");
            error!("This could be due to:");
            error!("  - Network connectivity issues");
            error!("  - FTP server being unavailable");
            error!("  - Firewall blocking FTP connections");
            error!("  - Changes to the FTP server structure");
            Err(e)
        },
    }
}
