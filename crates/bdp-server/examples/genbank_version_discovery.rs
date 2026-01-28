//! GenBank/RefSeq version discovery example
//!
//! Demonstrates how to discover available GenBank/RefSeq versions from FTP
//!
//! Usage:
//! ```bash
//! cargo run --example genbank_version_discovery -- --database genbank
//! cargo run --example genbank_version_discovery -- --database refseq
//! ```

use anyhow::Result;
use bdp_server::ingest::genbank::{GenbankFtpConfig, VersionDiscovery};
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Source database (genbank or refseq)
    #[clap(long, default_value = "genbank")]
    database: String,

    /// Filter to versions from this release number onwards
    #[clap(long)]
    from_release: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    // Create configuration based on database type
    let config = match args.database.to_lowercase().as_str() {
        "genbank" => {
            info!("Discovering GenBank versions");
            GenbankFtpConfig::new().with_genbank()
        }
        "refseq" => {
            info!("Discovering RefSeq versions");
            GenbankFtpConfig::new().with_refseq()
        }
        _ => {
            eprintln!("Invalid database type. Use 'genbank' or 'refseq'");
            std::process::exit(1);
        }
    };

    // Create version discovery service
    let discovery = VersionDiscovery::new(config);

    // Discover all versions
    info!("Connecting to NCBI FTP server...");
    let mut versions = discovery.discover_all_versions().await?;

    // Apply release filter if specified
    if let Some(from_release) = args.from_release {
        info!(
            "Filtering versions from release {} onwards",
            from_release
        );
        versions = discovery.filter_from_release(versions, from_release);
    }

    // Display results
    info!("Found {} versions:", versions.len());
    println!("\n{:<20} {:<15} {:<12}", "Version", "Release #", "Est. Date");
    println!("{}", "-".repeat(50));

    for version in &versions {
        println!(
            "{:<20} {:<15} {:<12}",
            version.external_version,
            version.release_number,
            version.release_date.format("%Y-%m-%d")
        );
    }

    // Show statistics
    if !versions.is_empty() {
        let oldest = versions.first().unwrap();
        let newest = versions.last().unwrap();

        println!("\nStatistics:");
        println!("  Oldest: {} (Release {})", oldest.external_version, oldest.release_number);
        println!("  Newest: {} (Release {})", newest.external_version, newest.release_number);
        println!("  Total: {} versions", versions.len());
    }

    Ok(())
}
