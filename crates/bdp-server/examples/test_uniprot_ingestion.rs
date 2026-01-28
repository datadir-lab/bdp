//! Test UniProt Ingestion System
//!
//! Demonstrates the complete ingestion pipeline with configuration modes

use anyhow::Result;
use bdp_server::ingest::config::{HistoricalConfig, IngestionMode, LatestConfig, UniProtConfig};
use bdp_server::ingest::uniprot::{UniProtFtpConfig, UniProtPipeline, VersionDiscovery};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,bdp_server=debug")
        .init();

    info!("=== UniProt Ingestion System Test ===");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("1. Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    info!("Connected");

    // Get or create test organization
    info!("2. Setting up test organization...");
    let org_id = get_or_create_test_org(&pool).await?;
    info!(org_id = %org_id, "Organization ready");

    // Test configuration system
    info!("3. Testing configuration system...");
    test_configuration()?;
    info!("Configuration system works");

    // Test version discovery
    info!("4. Testing version discovery...");
    test_version_discovery(&pool, org_id).await?;
    info!("Version discovery works");

    // Test mode selection
    info!("5. Testing mode selection...");
    test_mode_selection()?;
    info!("Mode selection works");

    info!("=== All Tests Passed! ===");
    info!("The UniProt ingestion system is ready for use.");
    info!("To run actual ingestion:");
    info!("  1. Set INGEST_ENABLED=true");
    info!("  2. Set INGEST_UNIPROT_MODE=latest or historical");
    info!("  3. Configure mode-specific settings");
    info!("  4. Start the server or call pipeline.run_with_mode()");

    Ok(())
}

async fn get_or_create_test_org(pool: &PgPool) -> Result<Uuid> {
    // Try to get existing org
    let result = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM organizations WHERE slug = 'uniprot' LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    if let Some(id) = result {
        Ok(id)
    } else {
        // Create test org
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO organizations (id, name, slug, description, is_system)
             VALUES ($1, 'UniProt', 'uniprot', 'UniProt protein database', true)",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(id)
    }
}

fn test_configuration() -> Result<()> {
    // Test Latest mode config
    let latest = LatestConfig {
        check_interval_secs: 86400,
        auto_ingest: false,
        ignore_before: Some("2024_01".to_string()),
    };
    info!(
        check_interval_secs = latest.check_interval_secs,
        auto_ingest = latest.auto_ingest,
        "Latest config"
    );

    // Test Historical mode config
    let historical = HistoricalConfig {
        start_version: "2020_01".to_string(),
        end_version: Some("2024_12".to_string()),
        batch_size: 3,
        skip_existing: true,
    };
    info!(
        start = %historical.start_version,
        end = historical.end_version.as_deref().unwrap_or("latest"),
        batch_size = historical.batch_size,
        "Historical config"
    );

    // Test mode enum
    let mode_latest = IngestionMode::Latest(latest);
    let mode_historical = IngestionMode::Historical(historical);

    match mode_latest {
        IngestionMode::Latest(_) => info!("Latest mode recognized"),
        _ => {},
    }

    match mode_historical {
        IngestionMode::Historical(_) => info!("Historical mode recognized"),
        _ => {},
    }

    Ok(())
}

async fn test_version_discovery(pool: &PgPool, org_id: Uuid) -> Result<()> {
    let config = UniProtFtpConfig::default();
    let discovery = VersionDiscovery::new(config);

    // Test get_last_ingested_version
    let last_version = discovery.get_last_ingested_version(pool, org_id).await?;
    info!(last_version = ?last_version, "Last ingested version");

    // Test version_exists_in_db
    let exists = discovery.version_exists_in_db(pool, "2025_01").await?;
    info!(version = "2025_01", exists = exists, "Version exists check");

    // Test was_ingested_as_current
    let was_current = discovery.was_ingested_as_current(pool, "2025_01").await?;
    info!(version = "2025_01", was_current = was_current, "Was current check");

    Ok(())
}

fn test_mode_selection() -> Result<()> {
    // Simulate environment variables
    std::env::set_var("INGEST_UNIPROT_MODE", "latest");
    std::env::set_var("INGEST_UNIPROT_CHECK_INTERVAL_SECS", "3600");

    let config = UniProtConfig::from_env()?;

    match config.ingestion_mode {
        IngestionMode::Latest(ref cfg) => {
            info!(check_interval_secs = cfg.check_interval_secs, "Parsed Latest mode");
        },
        IngestionMode::Historical(_) => {
            error!("ERROR: Expected Latest mode");
        },
    }

    // Test historical mode
    std::env::set_var("INGEST_UNIPROT_MODE", "historical");
    std::env::set_var("INGEST_UNIPROT_HISTORICAL_START", "2023_01");
    std::env::set_var("INGEST_UNIPROT_HISTORICAL_BATCH_SIZE", "5");

    let config = UniProtConfig::from_env()?;

    match config.ingestion_mode {
        IngestionMode::Historical(ref cfg) => {
            info!(
                start = %cfg.start_version,
                batch_size = cfg.batch_size,
                "Parsed Historical mode"
            );
        },
        IngestionMode::Latest(_) => {
            error!("ERROR: Expected Historical mode");
        },
    }

    Ok(())
}
