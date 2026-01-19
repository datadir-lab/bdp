//! Test storage pipeline with local fixture data
//!
//! Usage: cargo run --example test_storage_pipeline

use anyhow::Result;
use bdp_server::ingest::uniprot::{parser::DatParser, storage::UniProtStorage};
use bdp_server::storage::{config::StorageConfig, Storage};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("=== Testing Storage Pipeline ===\n");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    println!("✓ Connected\n");

    // Get or create organization
    let org_id = get_or_create_organization(&pool).await?;
    println!("✓ Organization ID: {}\n", org_id);

    // Skip S3 storage for this test - just test database
    println!("✓ Skipping S3 storage (testing database only)\n");

    // Read test fixture
    let fixture_path = "tests/fixtures/uniprot_sample_10.dat";
    println!("Reading fixture: {}", fixture_path);
    let data = std::fs::read(fixture_path)?;
    println!("✓ Read {} bytes\n", data.len());

    // Parse entries
    println!("Parsing entries...");
    let parser = DatParser::new();
    let entries = parser.parse_bytes(&data)?;
    println!("✓ Parsed {} entries\n", entries.len());

    // Display first entry
    if let Some(first) = entries.first() {
        println!("First entry:");
        println!("  Accession: {}", first.accession);
        println!("  Name: {}", first.protein_name);
        println!("  Organism: {}", first.organism_name);
        println!("  Taxonomy ID: {}", first.taxonomy_id);
        println!("  Sequence length: {} AA\n", first.sequence_length);
    }

    // Create storage handler (without S3 for this test)
    let internal_version = "1.0.0";
    let external_version = "test_fixture";

    let uniprot_storage = UniProtStorage::new(
        pool.clone(),
        org_id,
        internal_version.to_string(),
        external_version.to_string(),
    );

    // Store entries
    println!("Storing {} entries to database...", entries.len());
    let stored_count = uniprot_storage.store_entries(&entries).await?;
    println!("✓ Stored {} entries successfully\n", stored_count);

    // Verify in database
    println!("Verifying storage...");
    let protein_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM protein_metadata"
    )
    .fetch_one(&pool)
    .await?;

    let sequence_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM protein_sequences")
        .fetch_one(&pool)
        .await?;

    let organism_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organism_metadata")
        .fetch_one(&pool)
        .await?;

    println!("✓ Database verification:");
    println!("  Proteins: {}", protein_count);
    println!("  Sequences: {}", sequence_count);
    println!("  Organisms: {}", organism_count);

    // Check sequence deduplication
    println!("\n✓ Sequence deduplication: {} unique sequences from {} proteins",
        sequence_count, protein_count);

    println!("\n=== Test Complete ===");
    Ok(())
}

async fn get_or_create_organization(pool: &sqlx::PgPool) -> Result<Uuid> {
    const UNIPROT_SLUG: &str = "uniprot";

    let result = sqlx::query!(
        r#"SELECT id FROM organizations WHERE slug = $1"#,
        UNIPROT_SLUG
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, name, slug, description, is_system)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Universal Protein Resource",
            UNIPROT_SLUG,
            "UniProt Knowledgebase - Protein sequences and functional information",
            true
        )
        .execute(pool)
        .await?;

        let record = sqlx::query!(
            r#"SELECT id FROM organizations WHERE slug = $1"#,
            UNIPROT_SLUG
        )
        .fetch_one(pool)
        .await?;

        Ok(record.id)
    }
}
