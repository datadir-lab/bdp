//! Example demonstrating database usage with SQLx in BDP server.
//!
//! This example shows how to:
//! - Set up a database connection pool
//! - Perform CRUD operations on organizations
//! - Handle errors properly
//! - Use pagination
//! - Search for records
//!
//! # Running the Example
//!
//! 1. Set up a PostgreSQL database:
//!    ```bash
//!    createdb bdp
//!    ```
//!
//! 2. Run migrations (from the bdp-server directory):
//!    ```bash
//!    sqlx migrate run
//!    ```
//!
//! 3. Set environment variables:
//!    ```bash
//!    export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/bdp"
//!    ```
//!
//! 4. Run the example:
//!    ```bash
//!    cargo run --example database_usage
//!    ```

use bdp_common::types::Pagination;
use bdp_server::db::{create_pool, health_check, organizations, DbConfig, DbError};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();

    info!("=== BDP Database Usage Example ===");

    // ========================================================================
    // 1. Database Connection Setup
    // ========================================================================

    info!("1. Setting up database connection...");

    // Load configuration from environment
    let config = DbConfig::from_env()?;
    info!(
        url = %mask_password(&config.url),
        max_connections = config.max_connections,
        "Database configuration"
    );

    // Create connection pool
    let pool = create_pool(&config).await?;
    info!("Connection pool created");

    // ========================================================================
    // 2. Health Check
    // ========================================================================

    info!("2. Performing health check...");
    health_check(&pool).await?;
    info!("Database is healthy");

    // ========================================================================
    // 3. Create Organizations
    // ========================================================================

    info!("3. Creating sample organizations...");

    let org1 = organizations::create_organization(
        &pool,
        "acme-corp",
        "ACME Corporation",
        Some("Leading provider of innovative solutions"),
    )
    .await;

    match org1 {
        Ok(org) => {
            info!(
                name = %org.name,
                slug = %org.slug,
                id = %org.id,
                created_at = %org.created_at,
                "Created organization"
            );
        },
        Err(DbError::Duplicate(_)) => {
            info!(slug = "acme-corp", "Organization already exists (skipping)");
        },
        Err(e) => return Err(e.into()),
    }

    let org2 = organizations::create_organization(
        &pool,
        "biotech-labs",
        "BioTech Labs",
        Some("Advanced biological research and development"),
    )
    .await;

    match org2 {
        Ok(org) => info!(name = %org.name, slug = %org.slug, "Created organization"),
        Err(DbError::Duplicate(_)) => {
            info!(slug = "biotech-labs", "Organization already exists (skipping)");
        },
        Err(e) => return Err(e.into()),
    }

    let org3 =
        organizations::create_organization(&pool, "data-science-inc", "Data Science Inc", None)
            .await;

    match org3 {
        Ok(org) => info!(name = %org.name, slug = %org.slug, "Created organization"),
        Err(DbError::Duplicate(_)) => {
            info!(slug = "data-science-inc", "Organization already exists (skipping)");
        },
        Err(e) => return Err(e.into()),
    }

    // ========================================================================
    // 4. Retrieve Organization
    // ========================================================================

    info!("4. Retrieving organization by slug...");

    let org = organizations::get_organization_by_slug(&pool, "acme-corp").await?;
    info!(
        name = %org.name,
        description = ?org.description,
        updated_at = %org.updated_at,
        "Found organization"
    );

    // ========================================================================
    // 5. List Organizations (with pagination)
    // ========================================================================

    info!("5. Listing organizations with pagination...");

    // First page
    let page1 = organizations::list_organizations(&pool, Pagination::new(2, 0)).await?;
    info!(page = 1, items = page1.len(), "Page results");
    for (i, org) in page1.iter().enumerate() {
        info!(index = i + 1, name = %org.name, slug = %org.slug, "Organization");
    }

    // Second page
    let page2 = organizations::list_organizations(&pool, Pagination::new(2, 2)).await?;
    info!(page = 2, items = page2.len(), "Page results");
    for (i, org) in page2.iter().enumerate() {
        info!(index = i + 1, name = %org.name, slug = %org.slug, "Organization");
    }

    // Count total
    let total = organizations::count_organizations(&pool).await?;
    info!(total = total, "Total organizations");

    // ========================================================================
    // 6. Update Organization
    // ========================================================================

    info!("6. Updating organization...");

    let updated = organizations::update_organization(
        &pool,
        "acme-corp",
        Some("ACME Corp"),
        Some("Renamed and updated description"),
    )
    .await?;

    info!(
        name = %updated.name,
        description = ?updated.description,
        updated_at = %updated.updated_at,
        "Updated organization"
    );

    // ========================================================================
    // 7. Search Organizations
    // ========================================================================

    info!("7. Searching organizations...");

    let search_results =
        organizations::search_organizations(&pool, "bio", Pagination::default()).await?;

    info!(query = "bio", count = search_results.len(), "Search results");
    for org in search_results {
        info!(name = %org.name, slug = %org.slug, "Match");
    }

    // ========================================================================
    // 8. Error Handling
    // ========================================================================

    info!("8. Demonstrating error handling...");

    // Try to get a non-existent organization
    match organizations::get_organization_by_slug(&pool, "non-existent").await {
        Ok(_) => info!("Unexpected: Found non-existent organization"),
        Err(DbError::NotFound(msg)) => info!(message = %msg, "Correctly handled NotFound"),
        Err(e) => info!(error = %e, "Unexpected error"),
    }

    // Try to create a duplicate organization
    match organizations::create_organization(&pool, "acme-corp", "Duplicate", None).await {
        Ok(_) => info!("Unexpected: Created duplicate organization"),
        Err(DbError::Duplicate(msg)) => info!(message = %msg, "Correctly handled Duplicate"),
        Err(e) => info!(error = %e, "Unexpected error"),
    }

    // ========================================================================
    // 9. Cleanup (Optional)
    // ========================================================================

    info!("9. Cleanup (deleting test organizations)...");
    info!("Skipping cleanup - organizations left in database for inspection");
    info!("To clean up manually, run:");
    info!("DELETE FROM organizations WHERE slug IN ('acme-corp', 'biotech-labs', 'data-science-inc');");

    // Uncomment to actually delete:
    // for slug in &["acme-corp", "biotech-labs", "data-science-inc"] {
    //     match organizations::delete_organization(&pool, slug).await {
    //         Ok(_) => info!(slug = %slug, "Deleted organization"),
    //         Err(DbError::NotFound(_)) => info!(slug = %slug, "Organization not found"),
    //         Err(e) => info!(slug = %slug, error = %e, "Error deleting"),
    //     }
    // }

    info!("=== Example Complete ===");

    Ok(())
}

/// Masks the password in a database URL for safe logging.
///
/// # Examples
///
/// ```
/// let url = "postgresql://user:password@localhost:5432/db";
/// let masked = mask_password(url);
/// assert_eq!(masked, "postgresql://user:****@localhost:5432/db");
/// ```
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        assert_eq!(
            mask_password("postgresql://user:pass@localhost/db"),
            "postgresql://user:****@localhost/db"
        );
        assert_eq!(mask_password("postgresql://localhost/db"), "postgresql://localhost/db");
    }
}
