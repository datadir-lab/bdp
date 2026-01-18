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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();

    println!("=== BDP Database Usage Example ===\n");

    // ========================================================================
    // 1. Database Connection Setup
    // ========================================================================

    println!("1. Setting up database connection...");

    // Load configuration from environment
    let config = DbConfig::from_env()?;
    println!("   Database URL: {}", mask_password(&config.url));
    println!("   Max connections: {}", config.max_connections);

    // Create connection pool
    let pool = create_pool(&config).await?;
    println!("   ✓ Connection pool created\n");

    // ========================================================================
    // 2. Health Check
    // ========================================================================

    println!("2. Performing health check...");
    health_check(&pool).await?;
    println!("   ✓ Database is healthy\n");

    // ========================================================================
    // 3. Create Organizations
    // ========================================================================

    println!("3. Creating sample organizations...");

    let org1 = organizations::create_organization(
        &pool,
        "acme-corp",
        "ACME Corporation",
        Some("Leading provider of innovative solutions"),
    )
    .await;

    match org1 {
        Ok(org) => {
            println!("   ✓ Created: {} ({})", org.name, org.slug);
            println!("     ID: {}", org.id);
            println!("     Created at: {}", org.created_at);
        },
        Err(DbError::Duplicate(_)) => {
            println!("   ⚠ Organization 'acme-corp' already exists (skipping)");
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
        Ok(org) => println!("   ✓ Created: {} ({})", org.name, org.slug),
        Err(DbError::Duplicate(_)) => {
            println!("   ⚠ Organization 'biotech-labs' already exists (skipping)");
        },
        Err(e) => return Err(e.into()),
    }

    let org3 =
        organizations::create_organization(&pool, "data-science-inc", "Data Science Inc", None)
            .await;

    match org3 {
        Ok(org) => println!("   ✓ Created: {} ({})", org.name, org.slug),
        Err(DbError::Duplicate(_)) => {
            println!("   ⚠ Organization 'data-science-inc' already exists (skipping)");
        },
        Err(e) => return Err(e.into()),
    }

    println!();

    // ========================================================================
    // 4. Retrieve Organization
    // ========================================================================

    println!("4. Retrieving organization by slug...");

    let org = organizations::get_organization_by_slug(&pool, "acme-corp").await?;
    println!("   ✓ Found: {}", org.name);
    println!("     Description: {:?}", org.description);
    println!("     Last updated: {}\n", org.updated_at);

    // ========================================================================
    // 5. List Organizations (with pagination)
    // ========================================================================

    println!("5. Listing organizations with pagination...");

    // First page
    let page1 = organizations::list_organizations(&pool, Pagination::new(2, 0)).await?;
    println!("   Page 1 ({} items):", page1.len());
    for (i, org) in page1.iter().enumerate() {
        println!("     {}. {} ({})", i + 1, org.name, org.slug);
    }

    // Second page
    let page2 = organizations::list_organizations(&pool, Pagination::new(2, 2)).await?;
    println!("   Page 2 ({} items):", page2.len());
    for (i, org) in page2.iter().enumerate() {
        println!("     {}. {} ({})", i + 1, org.name, org.slug);
    }

    // Count total
    let total = organizations::count_organizations(&pool).await?;
    println!("   Total organizations: {}\n", total);

    // ========================================================================
    // 6. Update Organization
    // ========================================================================

    println!("6. Updating organization...");

    let updated = organizations::update_organization(
        &pool,
        "acme-corp",
        Some("ACME Corp"),
        Some("Renamed and updated description"),
    )
    .await?;

    println!("   ✓ Updated: {}", updated.name);
    println!("     New description: {:?}", updated.description);
    println!("     Updated at: {}\n", updated.updated_at);

    // ========================================================================
    // 7. Search Organizations
    // ========================================================================

    println!("7. Searching organizations...");

    let search_results =
        organizations::search_organizations(&pool, "bio", Pagination::default()).await?;

    println!("   Found {} results for 'bio':", search_results.len());
    for org in search_results {
        println!("     - {} ({})", org.name, org.slug);
    }
    println!();

    // ========================================================================
    // 8. Error Handling
    // ========================================================================

    println!("8. Demonstrating error handling...");

    // Try to get a non-existent organization
    match organizations::get_organization_by_slug(&pool, "non-existent").await {
        Ok(_) => println!("   Unexpected: Found non-existent organization"),
        Err(DbError::NotFound(msg)) => println!("   ✓ Correctly handled NotFound: {}", msg),
        Err(e) => println!("   ✗ Unexpected error: {}", e),
    }

    // Try to create a duplicate organization
    match organizations::create_organization(&pool, "acme-corp", "Duplicate", None).await {
        Ok(_) => println!("   Unexpected: Created duplicate organization"),
        Err(DbError::Duplicate(msg)) => println!("   ✓ Correctly handled Duplicate: {}", msg),
        Err(e) => println!("   ✗ Unexpected error: {}", e),
    }
    println!();

    // ========================================================================
    // 9. Cleanup (Optional)
    // ========================================================================

    println!("9. Cleanup (deleting test organizations)...");
    println!("   Skipping cleanup - organizations left in database for inspection");
    println!("   To clean up manually, run:");
    println!("   DELETE FROM organizations WHERE slug IN ('acme-corp', 'biotech-labs', 'data-science-inc');\n");

    // Uncomment to actually delete:
    // for slug in &["acme-corp", "biotech-labs", "data-science-inc"] {
    //     match organizations::delete_organization(&pool, slug).await {
    //         Ok(_) => println!("   ✓ Deleted: {}", slug),
    //         Err(DbError::NotFound(_)) => println!("   ⚠ Not found: {}", slug),
    //         Err(e) => println!("   ✗ Error deleting {}: {}", slug, e),
    //     }
    // }

    println!("=== Example Complete ===");

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
