//! Database integration tests using SQLx
//!
//! These tests demonstrate the use of the #[sqlx::test] macro for
//! database testing with automatic setup/teardown and migration support.
//!
//! Coverage includes:
//! - Organization CRUD operations
//! - Registry entry creation and retrieval
//! - Version management
//! - Dependency handling
//! - Search functionality
//! - Edge cases and error handling

use sqlx::PgPool;
use uuid::Uuid;

mod helpers;

// ============================================================================
// Organization Tests
// ============================================================================

#[sqlx::test]
async fn test_create_organization(pool: PgPool) -> sqlx::Result<()> {
    // Create a new organization
    let org = sqlx::query!(
        r#"
        INSERT INTO organizations (slug, name, website, is_system)
        VALUES ($1, $2, $3, $4)
        RETURNING id, slug, name, website, is_system, created_at, updated_at
        "#,
        "test-org",
        "Test Organization",
        Some("https://example.com"),
        false
    )
    .fetch_one(&pool)
    .await?;

    // Verify the organization was created correctly
    assert_eq!(org.slug, "test-org");
    assert_eq!(org.name, "Test Organization");
    assert_eq!(org.website.as_deref(), Some("https://example.com"));
    assert!(!org.is_system);
    assert!(org.created_at <= org.updated_at);

    Ok(())
}

#[sqlx::test]
async fn test_organization_slug_uniqueness(pool: PgPool) -> sqlx::Result<()> {
    // Create first organization
    sqlx::query!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2)",
        "unique-slug",
        "First Organization"
    )
    .execute(&pool)
    .await?;

    // Attempt to create organization with duplicate slug should fail
    let result = sqlx::query!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2)",
        "unique-slug",
        "Second Organization"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected duplicate slug to fail");

    Ok(())
}

#[sqlx::test]
async fn test_update_organization(pool: PgPool) -> sqlx::Result<()> {
    // Create an organization
    let org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        "update-test",
        "Original Name"
    )
    .fetch_one(&pool)
    .await?;

    // Update the organization
    sqlx::query!(
        "UPDATE organizations SET name = $1, website = $2 WHERE id = $3",
        "Updated Name",
        Some("https://updated.com"),
        org_id
    )
    .execute(&pool)
    .await?;

    // Verify the update
    let updated = sqlx::query!("SELECT name, website FROM organizations WHERE id = $1", org_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.website.as_deref(), Some("https://updated.com"));

    Ok(())
}

#[sqlx::test]
async fn test_delete_organization(pool: PgPool) -> sqlx::Result<()> {
    // Create an organization
    let org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        "delete-test",
        "To Be Deleted"
    )
    .fetch_one(&pool)
    .await?;

    // Delete the organization
    let deleted = sqlx::query!("DELETE FROM organizations WHERE id = $1 RETURNING id", org_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(deleted.id, org_id);

    // Verify it's gone
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations WHERE id = $1", org_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(count, Some(0));

    Ok(())
}

// ============================================================================
// Registry Entry Tests
// ============================================================================

#[sqlx::test(fixtures("organizations"))]
async fn test_create_registry_entry(pool: PgPool) -> sqlx::Result<()> {
    // Get an organization ID from fixtures
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    // Create a registry entry
    let entry = sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type, description)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, slug, name, entry_type, description
        "#,
        org_id,
        "test-protein",
        "Test Protein Database",
        "data_source",
        Some("A test protein database")
    )
    .fetch_one(&pool)
    .await?;

    // Verify the entry
    assert_eq!(entry.slug, "test-protein");
    assert_eq!(entry.name, "Test Protein Database");
    assert_eq!(entry.entry_type, "data_source");
    assert_eq!(entry.description.as_deref(), Some("A test protein database"));

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_registry_entry_types(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'ncbi'")
        .fetch_one(&pool)
        .await?;

    // Create a data_source entry
    sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "test-data-source",
        "Test Data Source",
        "data_source"
    )
    .execute(&pool)
    .await?;

    // Create a tool entry
    sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "test-tool",
        "Test Tool",
        "tool"
    )
    .execute(&pool)
    .await?;

    // Verify both types exist
    let data_source_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM registry_entries WHERE entry_type = 'data_source'"
    )
    .fetch_one(&pool)
    .await?;

    let tool_count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM registry_entries WHERE entry_type = 'tool'")
            .fetch_one(&pool)
            .await?;

    assert!(data_source_count.unwrap_or(0) >= 1);
    assert!(tool_count.unwrap_or(0) >= 1);

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_registry_entry_foreign_key_constraint(pool: PgPool) -> sqlx::Result<()> {
    // Try to create entry with non-existent organization
    let invalid_org_id = Uuid::new_v4();
    let result = sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        invalid_org_id,
        "invalid-entry",
        "Invalid Entry",
        "data_source"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected foreign key constraint violation");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_full_text_search(pool: PgPool) -> sqlx::Result<()> {
    // Search for entries containing "human"
    let results = sqlx::query!(
        r#"
        SELECT slug, name, description
        FROM registry_entries
        WHERE to_tsvector('english', name || ' ' || COALESCE(description, ''))
              @@ to_tsquery('english', $1)
        "#,
        "human"
    )
    .fetch_all(&pool)
    .await?;

    // Should find entries with "human" in name or description
    assert!(!results.is_empty(), "Expected to find entries with 'human'");

    // Verify at least one result contains "human"
    let has_human = results.iter().any(|r| {
        r.name.to_lowercase().contains("human")
            || r.description
                .as_ref()
                .map(|d| d.to_lowercase().contains("human"))
                .unwrap_or(false)
    });

    assert!(has_human, "Expected at least one result to contain 'human'");

    Ok(())
}

// ============================================================================
// Version Tests
// ============================================================================

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_create_version(pool: PgPool) -> sqlx::Result<()> {
    // Get a registry entry
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    // Create a version
    let version = sqlx::query!(
        r#"
        INSERT INTO versions (registry_entry_id, version, status, release_date)
        VALUES ($1, $2, $3, $4)
        RETURNING id, version, status, release_date
        "#,
        entry_id,
        "2024.01",
        "published",
        Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(version.version, "2024.01");
    assert_eq!(version.status, "published");
    assert!(version.release_date.is_some());

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_version_status_constraint(pool: PgPool) -> sqlx::Result<()> {
    let entry_id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'blast'")
        .fetch_one(&pool)
        .await?;

    // Valid status should work
    sqlx::query!(
        r#"
        INSERT INTO versions (registry_entry_id, version, status)
        VALUES ($1, $2, $3)
        "#,
        entry_id,
        "1.0.0",
        "published"
    )
    .execute(&pool)
    .await?;

    // Invalid status should fail
    let result = sqlx::query!(
        r#"
        INSERT INTO versions (registry_entry_id, version, status)
        VALUES ($1, $2, $3)
        "#,
        entry_id,
        "2.0.0",
        "invalid-status"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected invalid status to fail");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_multiple_versions_same_entry(pool: PgPool) -> sqlx::Result<()> {
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    // Create multiple versions
    let versions = vec!["2023.01", "2023.02", "2023.03", "2024.01"];

    for version_str in &versions {
        sqlx::query!(
            "INSERT INTO versions (registry_entry_id, version, status) VALUES ($1, $2, $3)",
            entry_id,
            version_str,
            "published"
        )
        .execute(&pool)
        .await?;
    }

    // Count versions
    let count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM versions WHERE registry_entry_id = $1", entry_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(count, Some(versions.len() as i64));

    Ok(())
}

// ============================================================================
// Transaction Tests
// ============================================================================

#[sqlx::test]
async fn test_transaction_commit(pool: PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    // Insert data in transaction
    let org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        "tx-test",
        "Transaction Test"
    )
    .fetch_one(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Verify data was persisted
    let exists =
        sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)", org_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(exists, Some(true));

    Ok(())
}

#[sqlx::test]
async fn test_transaction_rollback(pool: PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    // Insert data in transaction
    let org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        "rollback-test",
        "Rollback Test"
    )
    .fetch_one(&mut *tx)
    .await?;

    // Rollback transaction
    tx.rollback().await?;

    // Verify data was not persisted
    let exists =
        sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)", org_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(exists, Some(false));

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_transaction_with_constraint_violation(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    let mut tx = pool.begin().await?;

    // Create valid entry
    sqlx::query!(
        "INSERT INTO registry_entries (organization_id, slug, name, entry_type) VALUES ($1, $2, $3, $4)",
        org_id,
        "tx-entry-1",
        "Transaction Entry 1",
        "data_source"
    )
    .execute(&mut *tx)
    .await?;

    // Attempt to create duplicate slug (should fail)
    let result = sqlx::query!(
        "INSERT INTO registry_entries (organization_id, slug, name, entry_type) VALUES ($1, $2, $3, $4)",
        org_id,
        "tx-entry-1",  // Duplicate slug
        "Transaction Entry Duplicate",
        "data_source"
    )
    .execute(&mut *tx)
    .await;

    assert!(result.is_err());

    // Transaction should be aborted, rollback
    tx.rollback().await?;

    // Verify no entries were created
    let count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM registry_entries WHERE slug LIKE 'tx-entry-%'")
            .fetch_one(&pool)
            .await?;

    assert_eq!(count, Some(0));

    Ok(())
}

// ============================================================================
// Complex Query Tests
// ============================================================================

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_join_organizations_with_entries(pool: PgPool) -> sqlx::Result<()> {
    // Query that joins organizations with their registry entries
    let results = sqlx::query!(
        r#"
        SELECT
            o.slug as org_slug,
            o.name as org_name,
            COUNT(re.id) as entry_count
        FROM organizations o
        LEFT JOIN registry_entries re ON o.id = re.organization_id
        GROUP BY o.id, o.slug, o.name
        ORDER BY o.slug
        "#
    )
    .fetch_all(&pool)
    .await?;

    assert!(!results.is_empty());

    // Find UniProt (should have entries from fixtures)
    let uniprot = results
        .iter()
        .find(|r| r.org_slug == "uniprot")
        .expect("UniProt not found");

    assert!(uniprot.entry_count.unwrap_or(0) > 0, "UniProt should have at least one entry");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_entries_by_type(pool: PgPool) -> sqlx::Result<()> {
    // Count entries by type
    let stats = sqlx::query!(
        r#"
        SELECT
            entry_type,
            COUNT(*) as count
        FROM registry_entries
        GROUP BY entry_type
        ORDER BY entry_type
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Should have both data_source and tool entries
    let has_data_source = stats.iter().any(|s| s.entry_type == "data_source");
    let has_tool = stats.iter().any(|s| s.entry_type == "tool");

    assert!(has_data_source, "Should have data_source entries");
    assert!(has_tool, "Should have tool entries");

    Ok(())
}

// ============================================================================
// Helper Function Tests
// ============================================================================

#[sqlx::test]
async fn test_helper_create_organization(pool: PgPool) -> sqlx::Result<()> {
    let org_id = helpers::create_test_organization(&pool, "helper-test", "Helper Test Org").await?;

    // Verify organization exists
    let org = sqlx::query!("SELECT slug, name FROM organizations WHERE id = $1", org_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(org.slug, "helper-test");
    assert_eq!(org.name, "Helper Test Org");

    Ok(())
}

#[sqlx::test]
async fn test_builder_organization(pool: PgPool) -> sqlx::Result<()> {
    let org_id = helpers::builders::OrganizationBuilder::new("builder-test", "Builder Test")
        .website("https://builder.test")
        .description("Created with builder pattern")
        .system()
        .create(&pool)
        .await?;

    let org = sqlx::query!(
        "SELECT slug, name, website, description, is_system FROM organizations WHERE id = $1",
        org_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(org.slug, "builder-test");
    assert_eq!(org.name, "Builder Test");
    assert_eq!(org.website.as_deref(), Some("https://builder.test"));
    assert_eq!(org.description.as_deref(), Some("Created with builder pattern"));
    assert!(org.is_system);

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_builder_registry_entry(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'ncbi'")
        .fetch_one(&pool)
        .await?;

    let entry_id =
        helpers::builders::RegistryEntryBuilder::new(org_id, "builder-entry", "Builder Entry Test")
            .description("Created with builder")
            .as_tool()
            .create(&pool)
            .await?;

    let entry = sqlx::query!(
        "SELECT slug, name, description, entry_type FROM registry_entries WHERE id = $1",
        entry_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(entry.slug, "builder-entry");
    assert_eq!(entry.name, "Builder Entry Test");
    assert_eq!(entry.description.as_deref(), Some("Created with builder"));
    assert_eq!(entry.entry_type, "tool");

    Ok(())
}

#[sqlx::test]
async fn test_assertion_table_count(pool: PgPool) -> sqlx::Result<()> {
    // Initially should have 0 organizations
    helpers::assertions::assert_table_count(&pool, "organizations", 0).await?;

    // Create some organizations
    helpers::create_test_organization(&pool, "count-1", "Count 1").await?;
    helpers::create_test_organization(&pool, "count-2", "Count 2").await?;

    // Should now have 2
    helpers::assertions::assert_table_count(&pool, "organizations", 2).await?;

    Ok(())
}

#[sqlx::test]
async fn test_assertion_exists(pool: PgPool) -> sqlx::Result<()> {
    let org_id = helpers::create_test_organization(&pool, "exists-test", "Exists Test").await?;

    // Should exist
    helpers::assertions::assert_exists_by_id(&pool, "organizations", org_id).await?;

    // Delete it
    sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
        .execute(&pool)
        .await?;

    // Should not exist
    helpers::assertions::assert_not_exists_by_id(&pool, "organizations", org_id).await?;

    Ok(())
}

// ============================================================================
// Additional Organization CRUD Tests
// ============================================================================

#[sqlx::test]
async fn test_organization_read_by_id(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        "read-test",
        "Read Test"
    )
    .fetch_one(&pool)
    .await?;

    let org = sqlx::query!("SELECT id, slug, name FROM organizations WHERE id = $1", org_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(org.id, org_id);
    assert_eq!(org.slug, "read-test");

    Ok(())
}

#[sqlx::test]
async fn test_organization_not_found(pool: PgPool) -> sqlx::Result<()> {
    let non_existent_id = Uuid::new_v4();
    let result = sqlx::query!("SELECT id FROM organizations WHERE id = $1", non_existent_id)
        .fetch_optional(&pool)
        .await?;

    assert!(result.is_none(), "Expected None for non-existent organization");

    Ok(())
}

#[sqlx::test]
async fn test_organization_list_pagination(pool: PgPool) -> sqlx::Result<()> {
    for i in 0..10 {
        sqlx::query!(
            "INSERT INTO organizations (slug, name) VALUES ($1, $2)",
            format!("org-{}", i),
            format!("Organization {}", i)
        )
        .execute(&pool)
        .await?;
    }

    let first_page = sqlx::query!(
        "SELECT id FROM organizations ORDER BY created_at LIMIT $1 OFFSET $2",
        5i64,
        0i64
    )
    .fetch_all(&pool)
    .await?;

    let second_page = sqlx::query!(
        "SELECT id FROM organizations ORDER BY created_at LIMIT $1 OFFSET $2",
        5i64,
        5i64
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(first_page.len(), 5);
    assert_eq!(second_page.len(), 5);

    Ok(())
}

#[sqlx::test]
async fn test_organization_validation_empty_slug(pool: PgPool) -> sqlx::Result<()> {
    let result = sqlx::query!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2)",
        "",
        "Empty Slug Org"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected error for empty slug");

    Ok(())
}

#[sqlx::test]
async fn test_organization_website_field(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name, website) VALUES ($1, $2, $3) RETURNING id",
        "web-org",
        "Web Org",
        Some("https://example.com")
    )
    .fetch_one(&pool)
    .await?;

    let org = sqlx::query!("SELECT website FROM organizations WHERE id = $1", org_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(org.website.as_deref(), Some("https://example.com"));

    Ok(())
}

#[sqlx::test]
async fn test_organization_system_flag(pool: PgPool) -> sqlx::Result<()> {
    let system_org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name, is_system) VALUES ($1, $2, $3) RETURNING id",
        "system-org",
        "System Organization",
        true
    )
    .fetch_one(&pool)
    .await?;

    let user_org_id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name, is_system) VALUES ($1, $2, $3) RETURNING id",
        "user-org",
        "User Organization",
        false
    )
    .fetch_one(&pool)
    .await?;

    let system_orgs = sqlx::query!("SELECT id FROM organizations WHERE is_system = true")
        .fetch_all(&pool)
        .await?;

    let user_orgs = sqlx::query!("SELECT id FROM organizations WHERE is_system = false")
        .fetch_all(&pool)
        .await?;

    assert!(system_orgs.iter().any(|o| o.id == system_org_id));
    assert!(user_orgs.iter().any(|o| o.id == user_org_id));

    Ok(())
}

// ============================================================================
// Registry Entry Extended Tests
// ============================================================================

#[sqlx::test(fixtures("organizations"))]
async fn test_registry_entry_read_by_id(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    let entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        org_id,
        "test-entry",
        "Test Entry",
        "data_source"
    )
    .fetch_one(&pool)
    .await?;

    let entry = sqlx::query!("SELECT id, slug, name FROM registry_entries WHERE id = $1", entry_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(entry.id, entry_id);
    assert_eq!(entry.slug, "test-entry");

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_registry_entry_list_by_organization(pool: PgPool) -> sqlx::Result<()> {
    let uniprot_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    let ncbi_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'ncbi'")
        .fetch_one(&pool)
        .await?;

    for i in 0..3 {
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4)
            "#,
            uniprot_id,
            format!("uniprot-entry-{}", i),
            format!("UniProt Entry {}", i),
            "data_source"
        )
        .execute(&pool)
        .await?;
    }

    for i in 0..2 {
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4)
            "#,
            ncbi_id,
            format!("ncbi-entry-{}", i),
            format!("NCBI Entry {}", i),
            "data_source"
        )
        .execute(&pool)
        .await?;
    }

    let uniprot_entries =
        sqlx::query!("SELECT id FROM registry_entries WHERE organization_id = $1", uniprot_id)
            .fetch_all(&pool)
            .await?;

    let ncbi_entries =
        sqlx::query!("SELECT id FROM registry_entries WHERE organization_id = $1", ncbi_id)
            .fetch_all(&pool)
            .await?;

    assert_eq!(uniprot_entries.len(), 3);
    assert_eq!(ncbi_entries.len(), 2);

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_registry_entry_slug_uniqueness(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "unique-entry",
        "Unique Entry",
        "data_source"
    )
    .execute(&pool)
    .await?;

    let result = sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "unique-entry",
        "Duplicate Entry",
        "data_source"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected duplicate slug to fail");

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_registry_entry_invalid_type(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    let result = sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "invalid-type-entry",
        "Invalid Type Entry",
        "invalid_type"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected invalid entry_type to fail");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_search_registry_entries_by_name(pool: PgPool) -> sqlx::Result<()> {
    let results = sqlx::query!(
        r#"
        SELECT slug, name
        FROM registry_entries
        WHERE name ILIKE $1
        "#,
        "%BLAST%"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!results.is_empty(), "Expected to find BLAST entries");
    assert!(results.iter().any(|r| r.name.contains("BLAST")));

    Ok(())
}

// ============================================================================
// Version Extended Tests
// ============================================================================

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_version_read_by_id(pool: PgPool) -> sqlx::Result<()> {
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let version_id = sqlx::query_scalar!(
        r#"
        INSERT INTO versions (entry_id, version, external_version)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        entry_id,
        "1.0",
        Some("2024_01")
    )
    .fetch_one(&pool)
    .await?;

    let version = sqlx::query!(
        "SELECT id, version, external_version FROM versions WHERE id = $1",
        version_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(version.id, version_id);
    assert_eq!(version.version, "1.0");
    assert_eq!(version.external_version.as_deref(), Some("2024_01"));

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_version_list_by_entry(pool: PgPool) -> sqlx::Result<()> {
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let versions = vec!["1.0", "1.1", "2.0"];
    for version in &versions {
        sqlx::query!("INSERT INTO versions (entry_id, version) VALUES ($1, $2)", entry_id, version)
            .execute(&pool)
            .await?;
    }

    let results =
        sqlx::query!("SELECT version FROM versions WHERE entry_id = $1 ORDER BY version", entry_id)
            .fetch_all(&pool)
            .await?;

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].version, "1.0");
    assert_eq!(results[1].version, "1.1");
    assert_eq!(results[2].version, "2.0");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_version_uniqueness_constraint(pool: PgPool) -> sqlx::Result<()> {
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    sqlx::query!("INSERT INTO versions (entry_id, version) VALUES ($1, $2)", entry_id, "1.0")
        .execute(&pool)
        .await?;

    let result =
        sqlx::query!("INSERT INTO versions (entry_id, version) VALUES ($1, $2)", entry_id, "1.0")
            .execute(&pool)
            .await;

    assert!(result.is_err(), "Expected duplicate version to fail");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_version_cascade_delete(pool: PgPool) -> sqlx::Result<()> {
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let version_id = sqlx::query_scalar!(
        "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
        entry_id,
        "1.0"
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!("DELETE FROM registry_entries WHERE id = $1", entry_id)
        .execute(&pool)
        .await?;

    let version_exists =
        sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM versions WHERE id = $1)", version_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(version_exists, Some(false), "Version should be cascade deleted");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_version_metadata_jsonb(pool: PgPool) -> sqlx::Result<()> {
    let entry_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let metadata = serde_json::json!({
        "format": "fasta",
        "compression": "gzip",
        "checksum": "sha256:abc123"
    });

    let version_id = sqlx::query_scalar!(
        r#"
        INSERT INTO versions (entry_id, version, additional_metadata)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        entry_id,
        "1.0",
        metadata
    )
    .fetch_one(&pool)
    .await?;

    let result = sqlx::query!("SELECT additional_metadata FROM versions WHERE id = $1", version_id)
        .fetch_one(&pool)
        .await?;

    assert!(result.additional_metadata.is_some());
    let retrieved_metadata = result.additional_metadata.unwrap();
    assert_eq!(retrieved_metadata["format"], "fasta");
    assert_eq!(retrieved_metadata["compression"], "gzip");

    Ok(())
}

// ============================================================================
// Dependency Tests
// ============================================================================

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_dependency_creation(pool: PgPool) -> sqlx::Result<()> {
    let swissprot_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let blast_id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'blast'")
        .fetch_one(&pool)
        .await?;

    let version_id = sqlx::query_scalar!(
        "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
        swissprot_id,
        "1.0"
    )
    .fetch_one(&pool)
    .await?;

    let dep_id = sqlx::query_scalar!(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version, dependency_type)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        version_id,
        blast_id,
        "2.14.0",
        "required"
    )
    .fetch_one(&pool)
    .await?;

    let dep = sqlx::query!(
        r#"
        SELECT id, depends_on_version, dependency_type
        FROM dependencies
        WHERE id = $1
        "#,
        dep_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(dep.depends_on_version, "2.14.0");
    assert_eq!(dep.dependency_type.as_deref(), Some("required"));

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_dependency_uniqueness(pool: PgPool) -> sqlx::Result<()> {
    let swissprot_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let blast_id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'blast'")
        .fetch_one(&pool)
        .await?;

    let version_id = sqlx::query_scalar!(
        "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
        swissprot_id,
        "1.0"
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
        VALUES ($1, $2, $3)
        "#,
        version_id,
        blast_id,
        "2.14.0"
    )
    .execute(&pool)
    .await?;

    let result = sqlx::query!(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
        VALUES ($1, $2, $3)
        "#,
        version_id,
        blast_id,
        "2.15.0"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Expected duplicate dependency to fail");

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_dependency_list_for_version(pool: PgPool) -> sqlx::Result<()> {
    let swissprot_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let blast_id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'blast'")
        .fetch_one(&pool)
        .await?;

    let refseq_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'refseq-human'")
            .fetch_one(&pool)
            .await?;

    let version_id = sqlx::query_scalar!(
        "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
        swissprot_id,
        "1.0"
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
        VALUES ($1, $2, $3)
        "#,
        version_id,
        blast_id,
        "2.14.0"
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
        VALUES ($1, $2, $3)
        "#,
        version_id,
        refseq_id,
        "1.0"
    )
    .execute(&pool)
    .await?;

    let dependencies =
        sqlx::query!("SELECT id FROM dependencies WHERE version_id = $1", version_id)
            .fetch_all(&pool)
            .await?;

    assert_eq!(dependencies.len(), 2);

    Ok(())
}

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_dependency_cascade_delete(pool: PgPool) -> sqlx::Result<()> {
    let swissprot_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(&pool)
            .await?;

    let blast_id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'blast'")
        .fetch_one(&pool)
        .await?;

    let version_id = sqlx::query_scalar!(
        "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
        swissprot_id,
        "1.0"
    )
    .fetch_one(&pool)
    .await?;

    let dep_id = sqlx::query_scalar!(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        version_id,
        blast_id,
        "2.14.0"
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!("DELETE FROM versions WHERE id = $1", version_id)
        .execute(&pool)
        .await?;

    let dep_exists =
        sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM dependencies WHERE id = $1)", dep_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(dep_exists, Some(false), "Dependency should be cascade deleted");

    Ok(())
}

// ============================================================================
// Search and Query Tests
// ============================================================================

#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_full_text_search_with_multiple_terms(pool: PgPool) -> sqlx::Result<()> {
    let results = sqlx::query!(
        r#"
        SELECT slug, name
        FROM registry_entries
        WHERE to_tsvector('english', name || ' ' || COALESCE(description, ''))
              @@ to_tsquery('english', $1)
        "#,
        "protein & human"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!results.is_empty());

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_search_organizations_partial_match(pool: PgPool) -> sqlx::Result<()> {
    let results =
        sqlx::query!("SELECT slug, name FROM organizations WHERE name ILIKE $1", "%Prot%")
            .fetch_all(&pool)
            .await?;

    assert!(results.iter().any(|r| r.slug == "uniprot"));

    Ok(())
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[sqlx::test]
async fn test_empty_table_queries(pool: PgPool) -> sqlx::Result<()> {
    let orgs = sqlx::query!("SELECT id FROM organizations")
        .fetch_all(&pool)
        .await?;

    assert_eq!(orgs.len(), 0);

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_organization_delete_with_entries_restriction(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(&pool)
        .await?;

    sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "test-entry",
        "Test Entry",
        "data_source"
    )
    .execute(&pool)
    .await?;

    let result = sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
        .execute(&pool)
        .await;

    assert!(result.is_err(), "Expected delete to fail due to foreign key constraint");

    Ok(())
}

#[sqlx::test]
async fn test_concurrent_inserts(pool: PgPool) -> sqlx::Result<()> {
    let mut handles = vec![];

    for i in 0..5 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            sqlx::query!(
                "INSERT INTO organizations (slug, name) VALUES ($1, $2)",
                format!("concurrent-org-{}", i),
                format!("Concurrent Org {}", i)
            )
            .execute(&pool_clone)
            .await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap()?;
    }

    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations")
        .fetch_one(&pool)
        .await?;

    assert_eq!(count, Some(5));

    Ok(())
}
