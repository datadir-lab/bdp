//! Complete UniProt pipeline with version deduplication
//!
//! This example demonstrates:
//! 1. Downloading release notes to extract actual version
//! 2. Checking if version already exists (deduplication)
//! 3. Only downloading and storing if it's a new version
//! 4. Works with both current and previous releases
//!
//! Run with:
//! ```bash
//! cargo run --example uniprot_pipeline_with_dedup
//! ```

use anyhow::Result;
use bdp_server::ingest::uniprot::{ReleaseType, UniProtFtpConfig, UniProtPipeline};
use sqlx::postgres::PgPoolOptions;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting UniProt pipeline with version deduplication");

    // 1. Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/bdp".to_string());

    info!("📊 Connecting to database...");
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Run migrations
    info!("🔧 Running migrations...");
    sqlx::migrate!("../../migrations").run(&db_pool).await?;

    // 2. Create/get organization
    let org_id = get_or_create_organization(&db_pool).await?;
    info!("✅ Organization ID: {}", org_id);

    // 3. Configure pipeline for CURRENT release
    info!("📡 Configuring pipeline for current release...");
    let config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Current)
        .with_parse_limit(10); // Limit for demo

    let pipeline = UniProtPipeline::new(db_pool.clone(), org_id, config.clone());

    // 4. Check what version is available first (without downloading full dataset)
    info!("🔍 Checking current release version...");
    let release_info = pipeline.get_release_info(None).await?;
    info!("📋 Available version: {}", release_info.external_version);
    info!("   Release date: {}", release_info.release_date);
    info!("   Protein count: {}", release_info.swissprot_count);

    if let Some(license) = &release_info.license {
        info!("📜 License: {}", license.name);
        info!("   {}", license.citation_text());
    }

    // 5. Run pipeline - it will automatically:
    //    - Extract actual version from release notes
    //    - Check if it exists in DB
    //    - Skip download if already present
    //    - Download and store if it's new
    info!("▶️  Running ingestion pipeline...");
    let stats = pipeline.run(None).await?;

    info!("✅ Pipeline completed!");
    info!("   Total entries: {}", stats.total_entries);
    info!("   Entries inserted: {}", stats.entries_inserted);
    info!("   Entries failed: {}", stats.entries_failed);

    // 6. Run again - should skip because version exists
    info!("\n🔄 Running pipeline again (should skip due to deduplication)...");
    let stats2 = pipeline.run(None).await?;

    info!("✅ Second run completed!");
    info!("   Total entries: {}", stats2.total_entries);
    info!("   Entries inserted: {}", stats2.entries_inserted);

    if stats2.total_entries == 0 {
        info!("✨ Version already exists - skipped download as expected!");
    }

    // 7. Example: Run for specific previous version
    info!("\n📅 Attempting to download previous version 2024_01...");
    let prev_config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Previous)
        .with_parse_limit(5);

    let prev_pipeline = UniProtPipeline::new(db_pool.clone(), org_id, prev_config);

    match prev_pipeline.run(Some("2024_01")).await {
        Ok(stats) => {
            info!("✅ Previous version pipeline completed!");
            info!("   Total entries: {}", stats.total_entries);
            info!("   Entries inserted: {}", stats.entries_inserted);
        }
        Err(e) => {
            info!("ℹ️  Previous version download skipped or failed: {}", e);
        }
    }

    // 8. Verify data in database
    info!("\n📋 Database summary:");

    let protein_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_metadata"
    )
    .fetch_one(&db_pool)
    .await?;
    info!("   Total proteins: {}", protein_count);

    let version_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT external_version) FROM versions"
    )
    .fetch_one(&db_pool)
    .await?;
    info!("   Unique versions: {}", version_count);

    // List versions we have
    let versions = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT external_version FROM versions ORDER BY external_version"
    )
    .fetch_all(&db_pool)
    .await?;

    info!("\n🗂️  Versions in database:");
    for version in versions {
        info!("   • {}", version);
    }

    info!("\n✨ Example completed successfully!");
    Ok(())
}

async fn get_or_create_organization(db_pool: &sqlx::PgPool) -> Result<Uuid> {
    // Check if organization exists
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM organizations WHERE slug = 'uniprot'"
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    // Create organization with full metadata
    let org_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (
            id, name, slug, description, website, is_system,
            license, license_url, citation, citation_url,
            version_strategy, version_description,
            data_source_url, documentation_url, contact_email
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"
    )
    .bind(org_id)
    .bind("Universal Protein Resource")
    .bind("uniprot")
    .bind("UniProt Knowledgebase - Protein sequences and functional information")
    .bind("https://www.uniprot.org")
    .bind(true)
    .bind("CC-BY-4.0")
    .bind("https://creativecommons.org/licenses/by/4.0/")
    .bind("UniProt Consortium (2023). UniProt: the Universal Protein Knowledgebase in 2023. Nucleic Acids Research.")
    .bind("https://www.uniprot.org/help/publications")
    .bind("date-based")
    .bind("UniProt releases follow YYYY_MM format (e.g., 2025_01). Each release is a complete snapshot of the database.")
    .bind("https://ftp.uniprot.org/pub/databases/uniprot/")
    .bind("https://www.uniprot.org/help")
    .bind("help@uniprot.org")
    .execute(db_pool)
    .await?;

    Ok(org_id)
}
