//! BDP Ingest - Data ingestion tool

use anyhow::Result;
use bdp_common::logging::{init_logging, LogConfig, LogLevel};
use bdp_ingest::{uniprot, version_mapping};
use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "bdp-ingest")]
#[command(author, version, about = "BDP data ingestion tool")]
struct Cli {
    /// Data source to ingest
    #[command(subcommand)]
    source: Source,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Parser, Debug)]
enum Source {
    /// Ingest UniProt data
    Uniprot {
        /// Output directory
        #[arg(short, long, default_value = "./data/uniprot")]
        output: String,

        /// UniProt release version
        #[arg(short, long)]
        version: Option<String>,
    },

    /// Generate version mapping
    VersionMapping {
        /// Input directory
        #[arg(short, long)]
        input: String,

        /// Output file
        #[arg(short, long)]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbose flag
    let log_level = if cli.verbose {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    let log_config = LogConfig::builder()
        .level(log_level)
        .log_file_prefix("bdp-ingest".to_string())
        .build();

    // Merge with environment variables (they take precedence)
    let log_config = LogConfig::from_env().unwrap_or(log_config);

    init_logging(&log_config)?;

    match cli.source {
        Source::Uniprot { output, version } => {
            info!("Ingesting UniProt data");
            uniprot::ingest(&output, version.as_deref()).await?;
        },
        Source::VersionMapping { input, output } => {
            info!("Generating version mapping");
            version_mapping::generate(&input, &output).await?;
        },
    }

    info!("Ingestion complete");
    Ok(())
}
