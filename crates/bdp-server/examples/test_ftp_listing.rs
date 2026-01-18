//! Test program to verify FTP directory listing implementation
//!
//! This program connects to the UniProt FTP server and lists the previous releases.
//! Run with: cargo run --example test_ftp_listing

use bdp_server::ingest::uniprot::{config::UniProtFtpConfig, version_discovery::VersionDiscovery};
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

    println!("Testing FTP directory listing for UniProt previous releases...\n");

    // Create config
    let config = UniProtFtpConfig::default();
    println!("FTP Server: {}:{}", config.ftp_host, config.ftp_port);
    println!("Base Path: {}", config.ftp_base_path);
    println!("Username: {}\n", config.ftp_username);

    // Create version discovery service
    let discovery = VersionDiscovery::new(config);

    // Test listing all available versions
    println!("Discovering all available versions from FTP...\n");
    match discovery.discover_all_versions().await {
        Ok(versions) => {
            println!("Successfully discovered {} versions:", versions.len());
            println!("{:<15} {:<12} {:<10} {}", "Version", "Date", "Current", "FTP Path");
            println!("{}", "-".repeat(70));

            for version in &versions {
                println!(
                    "{:<15} {:<12} {:<10} {}",
                    version.external_version,
                    version.release_date.format("%Y-%m-%d"),
                    version.is_current,
                    version.ftp_path
                );
            }

            println!("\nTest completed successfully!");
            Ok(())
        }
        Err(e) => {
            eprintln!("\nError discovering versions: {}", e);
            eprintln!("\nThis could be due to:");
            eprintln!("  - Network connectivity issues");
            eprintln!("  - FTP server being unavailable");
            eprintln!("  - Firewall blocking FTP connections");
            eprintln!("  - Changes to the FTP server structure");
            Err(e)
        }
    }
}
