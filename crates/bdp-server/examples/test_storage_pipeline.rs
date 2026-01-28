//! Test storage pipeline with local fixture data
//!
//! Usage: cargo run --example test_storage_pipeline

use anyhow::Result;
use bdp_server::ingest::uniprot::{parser::DatParser, storage::UniProtStorage};
use bdp_server::storage::{config::StorageConfig, Storage};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("=== Testing Storage Pipeline ===");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    info!("Connected");

    // Get or create organization
    let org_id = get_or_create_organization(&pool).await?;
    info!(org_id = %org_id, "Organization ready");

    // Skip S3 storage for this test - just test database
    info!("Skipping S3 storage (testing database only)");

    // Read test fixture
    let fixture_path = "tests/fixtures/uniprot_sample_10.dat";
    info!(path = %fixture_path, "Reading fixture");
    let data = std::fs::read(fixture_path)?;
    info!(bytes = data.len(), "Read fixture file");

    // Parse entries
    info!("Parsing entries...");
    let parser = DatParser::new();
    let entries = parser.parse_bytes(&data)?;
    info!(count = entries.len(), "Parsed entries");

    // Display first entry
    if let Some(first) = entries.first() {
        info!(
            accession = %first.accession,
            name = %first.protein_name,
            organism = %first.organism_name,
            taxonomy_id = first.taxonomy_id,
            sequence_length = first.sequence_length,
            "First entry"
        );
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
    info!(count = entries.len(), "Storing entries to database...");
    let stored_count = uniprot_storage.store_entries(&entries).await?;
    info!(stored = stored_count, "Stored entries successfully");

    // Verify in database
    info!("Verifying storage...");
    let protein_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM protein_metadata")
        .fetch_one(&pool)
        .await?;

    let sequence_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM protein_sequences")
        .fetch_one(&pool)
        .await?;

    let organism_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organism_metadata")
        .fetch_one(&pool)
        .await?;

    info!(
        proteins = protein_count,
        sequences = sequence_count,
        organisms = organism_count,
        "Database verification"
    );

    // Check sequence deduplication
    info!(
        unique_sequences = sequence_count,
        total_proteins = protein_count,
        "Sequence deduplication"
    );

    info!("=== Test Complete ===");
    Ok(())
}

async fn get_or_create_organization(pool: &sqlx::PgPool) -> Result<Uuid> {
    const UNIPROT_SLUG: &str = "uniprot";

    let result = sqlx::query!(r#"SELECT id FROM organizations WHERE slug = $1"#, UNIPROT_SLUG)
        .fetch_optional(pool)
        .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (
                id, name, slug, description, website, is_system,
                license, license_url, citation, citation_url,
                version_strategy, version_description,
                data_source_url, documentation_url, contact_email
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Universal Protein Resource",
            UNIPROT_SLUG,
            "UniProt Knowledgebase - Protein sequences and functional information",
            Some("https://www.uniprot.org"),
            true,
            Some("CC-BY-4.0"),
            Some("https://creativecommons.org/licenses/by/4.0/"),
            Some("UniProt Consortium (2023). UniProt: the Universal Protein Knowledgebase in 2023. Nucleic Acids Research."),
            Some("https://www.uniprot.org/help/publications"),
            Some("date-based"),
            Some("UniProt releases follow YYYY_MM format (e.g., 2025_01). Each release is a complete snapshot of the database."),
            Some("https://ftp.uniprot.org/pub/databases/uniprot/"),
            Some("https://www.uniprot.org/help"),
            Some("help@uniprot.org")
        )
        .execute(pool)
        .await?;

        let record = sqlx::query!(r#"SELECT id FROM organizations WHERE slug = $1"#, UNIPROT_SLUG)
            .fetch_one(pool)
            .await?;

        Ok(record.id)
    }
}
