/// Example: Refresh the search materialized view
///
/// This example demonstrates how to manually refresh the search index.
/// Run this periodically (e.g., via cron) to keep search results up-to-date.
///
/// # Usage
///
/// ```bash
/// # Refresh concurrently (non-blocking, safe for production)
/// cargo run --example refresh_search_index
///
/// # Refresh non-concurrently (faster, but blocks searches briefly)
/// cargo run --example refresh_search_index -- --no-concurrent
/// ```
///
/// # Scheduling with Cron
///
/// Add to crontab to refresh every 5 minutes:
/// ```cron
/// */5 * * * * /path/to/bdp/target/release/examples/refresh_search_index
/// ```
use bdp_server::{
    db::{create_pool, DbConfig},
    features::search::queries::{self, RefreshSearchIndexCommand},
};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Use non-concurrent refresh (faster but blocks reads)
    #[arg(long)]
    no_concurrent: bool,

    /// Database URL (defaults to DATABASE_URL environment variable)
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let cli = Cli::parse();

    // Load database configuration
    let db_config = if let Some(url) = cli.database_url {
        DbConfig {
            url,
            ..Default::default()
        }
    } else {
        DbConfig::from_env()?
    };

    // Create database pool
    let pool = create_pool(&db_config).await?;

    // Create refresh command
    let command = RefreshSearchIndexCommand {
        concurrent: !cli.no_concurrent,
    };

    tracing::info!(
        "Refreshing search index ({} mode)...",
        if command.concurrent {
            "concurrent"
        } else {
            "non-concurrent"
        }
    );

    // Execute refresh
    let response = queries::refresh_search_index::handle(pool, command).await?;

    tracing::info!("✓ {}", response.message);

    Ok(())
}
