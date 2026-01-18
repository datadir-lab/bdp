//! Complete UniProt ingestion pipeline example
//!
//! This demonstrates the full workflow:
//! 1. Parse UniProt DAT file
//! 2. Store proteins in database (with multi-format support)
//! 3. Upload files to S3 (optional)
//! 4. Create aggregate source with dependencies
//!
//! Run with:
//! ```bash
//! cargo run --example uniprot_ingestion
//! ```

use anyhow::Result;
use bdp_server::ingest::uniprot::{DatParser, UniProtStorage};
use sqlx::postgres::PgPoolOptions;
use std::path::Path;
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

    info!("🚀 Starting UniProt ingestion pipeline");

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

    // 3. Parse UniProt data
    info!("📖 Parsing UniProt DAT file...");
    let dat_path = Path::new("tests/fixtures/uniprot_ci_sample.dat");

    if !dat_path.exists() {
        eprintln!("❌ Sample DAT file not found: {}", dat_path.display());
        eprintln!("   Run this from the bdp-server crate directory");
        return Ok(());
    }

    let parser = DatParser::new();
    let entries = parser.parse_file(dat_path)?;
    info!("✅ Parsed {} protein entries", entries.len());

    // 4. Store in database
    info!("💾 Storing proteins in database...");
    let storage = UniProtStorage::new(
        db_pool.clone(),
        org_id,
        "1.0".to_string(),
        "2024_01".to_string(),
    );

    let stored_count = storage.store_entries(&entries).await?;
    info!("✅ Stored {} proteins", stored_count);

    // 5. Create aggregate source
    info!("🔗 Creating aggregate source...");
    let aggregate_id = storage.create_aggregate_source(stored_count).await?;
    info!("✅ Created aggregate source: {}", aggregate_id);

    // 6. Query results
    info!("\n📋 Summary:");

    let protein_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_metadata"
    )
    .fetch_one(&db_pool)
    .await?;
    info!("  • Proteins in database: {}", protein_count);

    let version_file_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM version_files"
    )
    .fetch_one(&db_pool)
    .await?;
    info!("  • Version files created: {}", version_file_count);

    let dependency_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dependencies"
    )
    .fetch_one(&db_pool)
    .await?;
    info!("  • Dependencies created: {}", dependency_count);

    // List some proteins
    let proteins = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT accession, protein_name, sequence_length
         FROM protein_metadata
         LIMIT 5"
    )
    .fetch_all(&db_pool)
    .await?;

    info!("\n🧬 Sample proteins:");
    for (accession, name, length) in proteins {
        info!("  • {} - {} ({} aa)", accession, name, length);
    }

    info!("\n✨ Pipeline completed successfully!");
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

    // Create organization
    let org_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, is_system)
         VALUES ($1, 'uniprot', 'Universal Protein Resource', true)"
    )
    .bind(org_id)
    .execute(db_pool)
    .await?;

    Ok(org_id)
}
