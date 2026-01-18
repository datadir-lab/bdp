//! Test organization creation idempotency by slug

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn test_organization_creation_idempotency() -> Result<()> {
    // Connect to test database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;

    const UNIPROT_SLUG: &str = "uniprot";

    // First call - should find existing or create new
    let id1 = get_or_create_organization(&pool, UNIPROT_SLUG).await?;

    // Second call - should find the same organization
    let id2 = get_or_create_organization(&pool, UNIPROT_SLUG).await?;

    // Third call - should still find the same organization
    let id3 = get_or_create_organization(&pool, UNIPROT_SLUG).await?;

    // All three IDs should be identical (idempotent)
    assert_eq!(id1, id2, "Second call should return same ID");
    assert_eq!(id2, id3, "Third call should return same ID");

    // Verify only one record exists with this slug
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM organizations WHERE slug = $1"
    )
    .bind(UNIPROT_SLUG)
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, 1, "Should only have one organization with slug 'uniprot'");

    println!("✓ Idempotency test passed - ID: {}", id1);
    println!("✓ Only one organization exists with slug '{}'", UNIPROT_SLUG);

    Ok(())
}

async fn get_or_create_organization(pool: &sqlx::PgPool, slug: &str) -> Result<Uuid> {
    // Check for existing organization by slug (unique identifier)
    let result = sqlx::query!(
        r#"SELECT id FROM organizations WHERE slug = $1"#,
        slug
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        // Create organization
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, name, slug, description, is_system)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Universal Protein Resource",
            slug,
            "UniProt Knowledgebase - Protein sequences and functional information",
            true
        )
        .execute(pool)
        .await?;

        // Fetch the ID in case another process created it concurrently
        let record = sqlx::query!(
            r#"SELECT id FROM organizations WHERE slug = $1"#,
            slug
        )
        .fetch_one(pool)
        .await?;

        Ok(record.id)
    }
}
