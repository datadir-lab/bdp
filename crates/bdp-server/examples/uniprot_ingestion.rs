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
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting UniProt ingestion pipeline");

    // 1. Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/bdp".to_string());

    info!("Connecting to database...");
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Run migrations
    info!("Running migrations...");
    sqlx::migrate!("../../migrations").run(&db_pool).await?;

    // 2. Create/get organization
    let org_id = get_or_create_organization(&db_pool).await?;
    info!(org_id = %org_id, "Organization ready");

    // 3. Parse UniProt data
    info!("Parsing UniProt DAT file...");
    let dat_path = Path::new("tests/fixtures/uniprot_ci_sample.dat");

    if !dat_path.exists() {
        error!(path = %dat_path.display(), "Sample DAT file not found");
        error!("Run this from the bdp-server crate directory");
        return Ok(());
    }

    let parser = DatParser::new();
    let entries = parser.parse_file(dat_path)?;
    info!(count = entries.len(), "Parsed protein entries");

    // 4. Store in database
    info!("Storing proteins in database...");
    let storage = UniProtStorage::new(
        db_pool.clone(),
        org_id,
        "1.0".to_string(),
        "2024_01".to_string(),
    );

    let stored_count = storage.store_entries(&entries).await?;
    info!(stored = stored_count, "Stored proteins");

    // 5. Create aggregate source
    info!("Creating aggregate source...");
    let aggregate_id = storage.create_aggregate_source(stored_count).await?;
    info!(aggregate_id = %aggregate_id, "Created aggregate source");

    // 6. Query results
    info!("Summary:");

    let protein_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_metadata"
    )
    .fetch_one(&db_pool)
    .await?;
    info!(proteins = protein_count, "Proteins in database");

    let version_file_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM version_files"
    )
    .fetch_one(&db_pool)
    .await?;
    info!(version_files = version_file_count, "Version files created");

    let dependency_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dependencies"
    )
    .fetch_one(&db_pool)
    .await?;
    info!(dependencies = dependency_count, "Dependencies created");

    // List some proteins
    let proteins = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT accession, protein_name, sequence_length
         FROM protein_metadata
         LIMIT 5"
    )
    .fetch_all(&db_pool)
    .await?;

    info!("Sample proteins:");
    for (accession, name, length) in proteins {
        info!(
            accession = %accession,
            name = %name,
            length = length,
            "Protein"
        );
    }

    info!("Pipeline completed successfully!");
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
