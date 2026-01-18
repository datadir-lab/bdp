# Testing Guide

This document provides comprehensive guidance on testing the BDP project, with a focus on SQLx-based database testing.

## Table of Contents

- [Overview](#overview)
- [SQLx Testing Setup](#sqlx-testing-setup)
- [Test Database Isolation](#test-database-isolation)
- [Docker-Based Testing](#docker-based-testing)
- [Running Tests](#running-tests)
- [Integration Test Patterns](#integration-test-patterns)
- [Fixtures and Test Data](#fixtures-and-test-data)
- [Parallel Test Execution](#parallel-test-execution)
- [CI/CD Testing Workflow](#cicd-testing-workflow)
- [Best Practices](#best-practices)

## Overview

BDP uses a comprehensive testing strategy that includes:

- **Unit tests**: Testing individual functions and modules
- **Integration tests**: Testing database interactions using SQLx
- **End-to-end tests**: Testing complete workflows
- **Contract tests**: Testing API contracts

This guide focuses primarily on integration testing with SQLx and PostgreSQL.

## SQLx Testing Setup

### The `#[sqlx::test]` Macro

SQLx provides a powerful `#[sqlx::test]` macro that automatically handles test database setup and teardown. This macro:

- Creates a new database for each test
- Runs all migrations before the test
- Provides a clean database pool to the test
- Cleans up the database after the test

### Basic Example

```rust
#[sqlx::test]
async fn test_create_organization(pool: PgPool) -> sqlx::Result<()> {
    let org = sqlx::query!(
        r#"
        INSERT INTO organizations (slug, name, is_system)
        VALUES ($1, $2, $3)
        RETURNING id, slug, name, is_system
        "#,
        "test-org",
        "Test Organization",
        false
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(org.slug, "test-org");
    assert_eq!(org.name, "Test Organization");

    Ok(())
}
```

### Using Custom Fixtures

You can provide custom SQL fixtures to run before each test:

```rust
#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_with_fixtures(pool: PgPool) -> sqlx::Result<()> {
    // Organizations and registry entries are already loaded
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM organizations"
    )
    .fetch_one(&pool)
    .await?;

    assert!(count > 0);
    Ok(())
}
```

Fixtures are SQL files located in `tests/fixtures/`:
- `tests/fixtures/organizations.sql`
- `tests/fixtures/registry_entries.sql`

## Test Database Isolation

### Per-Test Database Strategy

SQLx's `#[sqlx::test]` macro ensures complete isolation by:

1. Creating a new test database with a unique name
2. Running all migrations to set up the schema
3. Optionally loading fixture data
4. Providing a clean database pool to the test
5. Dropping the database after the test completes

This ensures:
- **No test pollution**: Each test starts with a clean slate
- **Parallel execution**: Tests can run concurrently without conflicts
- **Predictable state**: Test outcomes are reproducible

### Manual Test Database Management

For custom test setups, you can manually manage test databases:

```rust
use crate::helpers::TestDb;

#[tokio::test]
async fn test_custom_setup() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Your test code here

    // Cleanup happens automatically when test_db is dropped
}
```

## Docker-Based Testing

### Test Database Container

The project includes a dedicated Docker Compose configuration for testing:

```bash
# Start test database
just db-test-up

# Run tests
just test

# Clean up
just db-test-down
```

### Configuration

Test database configuration (in `docker/docker-compose.test.yml`):

- **Port**: 5433 (different from development database on 5432)
- **Database**: bdp_test
- **User**: bdp_test
- **Password**: test_password
- **Volumes**: Temporary, cleaned up after tests

### Environment Variables

Set these environment variables for test database connection:

```bash
export DATABASE_URL=postgresql://bdp_test:test_password@localhost:5433/bdp_test
```

Or use a `.env.test` file:

```env
DATABASE_URL=postgresql://bdp_test:test_password@localhost:5433/bdp_test
```

## Running Tests

### All Tests

```bash
# Run all tests (including integration tests)
just test

# Run tests with output
just test-verbose

# Run tests with logging
RUST_LOG=debug just test
```

### Specific Test Categories

```bash
# Run only unit tests
just test-unit

# Run only integration tests
just test-integration

# Run specific test function
just test-one test_create_organization
```

### Using Just for Testing

```bash
# Run all tests with automated setup
just test

# Run tests with output
just test-verbose

# Run fresh tests (reset database first)
just test-fresh
```

### Testing with SQLx Prepared Queries

SQLx can verify queries at compile time. To enable this:

```bash
# Prepare query metadata
just sqlx-prepare

# Verify prepared queries are up to date
just sqlx-check

# Run CI checks locally (includes offline mode)
just ci
```

## Integration Test Patterns

### Testing CRUD Operations

```rust
#[sqlx::test]
async fn test_organization_crud(pool: PgPool) -> sqlx::Result<()> {
    // Create
    let org = sqlx::query!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        "test-org",
        "Test Org"
    )
    .fetch_one(&pool)
    .await?;

    // Read
    let fetched = sqlx::query!(
        "SELECT slug, name FROM organizations WHERE id = $1",
        org.id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(fetched.slug, "test-org");

    // Update
    sqlx::query!(
        "UPDATE organizations SET name = $1 WHERE id = $2",
        "Updated Org",
        org.id
    )
    .execute(&pool)
    .await?;

    // Delete
    let deleted = sqlx::query!(
        "DELETE FROM organizations WHERE id = $1 RETURNING id",
        org.id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(deleted.id, org.id);

    Ok(())
}
```

### Testing Complex Queries

```rust
#[sqlx::test(fixtures("organizations"))]
async fn test_registry_search(pool: PgPool) -> sqlx::Result<()> {
    // Insert test data
    let org_id = sqlx::query_scalar!(
        "SELECT id FROM organizations WHERE slug = 'uniprot'"
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, $4)
        "#,
        org_id,
        "test-entry",
        "Test Entry for Insulin",
        "data_source"
    )
    .execute(&pool)
    .await?;

    // Test full-text search
    let results = sqlx::query!(
        r#"
        SELECT slug, name
        FROM registry_entries
        WHERE to_tsvector('english', name || ' ' || COALESCE(description, ''))
              @@ to_tsquery('english', $1)
        "#,
        "insulin"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!results.is_empty());
    Ok(())
}
```

### Testing Transactions

```rust
#[sqlx::test]
async fn test_transaction_rollback(pool: PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    // Insert data in transaction
    sqlx::query!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2)",
        "temp-org",
        "Temporary Org"
    )
    .execute(&mut *tx)
    .await?;

    // Rollback
    tx.rollback().await?;

    // Verify data was not persisted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM organizations WHERE slug = 'temp-org'"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(0));
    Ok(())
}
```

### Testing Foreign Key Constraints

```rust
#[sqlx::test(fixtures("organizations"))]
async fn test_foreign_key_constraint(pool: PgPool) -> sqlx::Result<()> {
    let org_id = sqlx::query_scalar!(
        "SELECT id FROM organizations WHERE slug = 'uniprot'"
    )
    .fetch_one(&pool)
    .await?;

    // This should succeed
    sqlx::query!(
        "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
         VALUES ($1, $2, $3, $4)",
        org_id,
        "test-entry",
        "Test Entry",
        "data_source"
    )
    .execute(&pool)
    .await?;

    // This should fail (invalid organization_id)
    let invalid_uuid = uuid::Uuid::new_v4();
    let result = sqlx::query!(
        "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
         VALUES ($1, $2, $3, $4)",
        invalid_uuid,
        "invalid-entry",
        "Invalid Entry",
        "data_source"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err());
    Ok(())
}
```

## Fixtures and Test Data

### Creating Fixtures

Fixtures are SQL files that set up common test data. Create them in `crates/bdp-server/tests/fixtures/`:

**`tests/fixtures/organizations.sql`**:
```sql
-- System organizations
INSERT INTO organizations (slug, name, is_system, website) VALUES
    ('uniprot', 'UniProt', true, 'https://www.uniprot.org'),
    ('ncbi', 'NCBI', true, 'https://www.ncbi.nlm.nih.gov'),
    ('ensembl', 'Ensembl', true, 'https://www.ensembl.org');
```

**`tests/fixtures/registry_entries.sql`**:
```sql
-- Assumes organizations fixture is loaded
INSERT INTO registry_entries (organization_id, slug, name, entry_type, description)
SELECT
    o.id,
    'swissprot-human',
    'Swiss-Prot Human Proteins',
    'data_source',
    'Manually annotated human proteins from UniProt'
FROM organizations o
WHERE o.slug = 'uniprot';
```

### Using Fixtures in Tests

```rust
// Load single fixture
#[sqlx::test(fixtures("organizations"))]
async fn test_with_orgs(pool: PgPool) -> sqlx::Result<()> {
    // ...
}

// Load multiple fixtures (order matters!)
#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_with_multiple_fixtures(pool: PgPool) -> sqlx::Result<()> {
    // ...
}
```

### Helper Functions for Test Data

Create reusable helper functions in `tests/helpers/mod.rs`:

```rust
pub async fn create_test_organization(
    pool: &PgPool,
    slug: &str,
    name: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id",
        slug,
        name
    )
    .fetch_one(pool)
    .await?;

    Ok(id)
}
```

## Parallel Test Execution

SQLx tests can run in parallel safely because each test gets its own database.

### Controlling Parallelism

```bash
# Run tests sequentially
cargo test -- --test-threads=1

# Run with 4 parallel test threads
cargo test -- --test-threads=4

# Default: Uses number of CPU cores
cargo test
```

### Performance Considerations

- **Database creation overhead**: Each test creates a new database
- **Migration time**: Migrations run for every test
- **Connection pool**: Each test has its own connection pool

To speed up tests:

1. Use fixtures instead of inserting data in each test
2. Group related tests when possible
3. Use connection pooling efficiently
4. Consider snapshot testing for complex setups

## CI/CD Testing Workflow

### GitHub Actions Configuration

The CI workflow (`.github/workflows/ci.yml`) includes:

1. **PostgreSQL Service**: Runs test database
2. **Migration Check**: Verifies migrations are up to date
3. **SQLx Prepare Check**: Ensures prepared queries match current schema
4. **Test Execution**: Runs all tests with proper database connection
5. **Coverage Report**: Generates test coverage metrics

### Required Environment Variables

```yaml
env:
  DATABASE_URL: postgresql://bdp_test:test_password@localhost:5432/bdp_test
  RUST_LOG: debug
  SQLX_OFFLINE: true  # For SQLx offline mode in CI
```

### CI Test Steps

```yaml
- name: Start PostgreSQL
  uses: ...

- name: Run Migrations
  run: sqlx migrate run

- name: Verify SQLx Prepared Queries
  run: cargo sqlx prepare --check

- name: Run Tests
  run: cargo test --all-features

- name: Generate Coverage
  run: cargo tarpaulin --out Xml
```

## Best Practices

### Do's

1. **Use `#[sqlx::test]`**: Prefer the macro for automatic database management
2. **Test isolation**: Each test should be independent
3. **Use fixtures**: Share common test data via fixture files
4. **Test edge cases**: Include tests for error conditions
5. **Meaningful assertions**: Assert specific values, not just "no error"
6. **Transaction testing**: Test both commit and rollback scenarios
7. **Clean test names**: Use descriptive test function names
8. **Document complex tests**: Add comments explaining non-obvious test logic

### Don'ts

1. **Don't share state**: Never rely on test execution order
2. **Don't use production database**: Always use dedicated test database
3. **Don't skip cleanup**: Let SQLx handle cleanup automatically
4. **Don't test implementation**: Test behavior, not internal details
5. **Don't hardcode IDs**: Use fixtures or helpers to create test data
6. **Don't ignore errors**: Use `?` or proper error handling
7. **Don't test everything**: Focus on critical paths and edge cases

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(fixtures("organizations"))]
    async fn test_feature_name_success_case(pool: PgPool) -> sqlx::Result<()> {
        // Arrange: Set up test data
        let test_data = setup_test_data(&pool).await?;

        // Act: Execute the operation being tested
        let result = perform_operation(&pool, test_data).await?;

        // Assert: Verify the outcome
        assert_eq!(result.expected_field, expected_value);
        assert!(result.condition_is_true);

        Ok(())
    }

    #[sqlx::test]
    async fn test_feature_name_error_case(pool: PgPool) -> sqlx::Result<()> {
        // Arrange: Set up conditions for error

        // Act: Execute operation that should fail
        let result = perform_operation(&pool, invalid_data).await;

        // Assert: Verify error is as expected
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExpectedError));

        Ok(())
    }
}
```

### Testing Checklist

- [ ] Test creates its own test data or uses fixtures
- [ ] Test is independent and can run in any order
- [ ] Test has meaningful assertions
- [ ] Test handles errors appropriately
- [ ] Test cleans up automatically (via `#[sqlx::test]`)
- [ ] Test name describes what is being tested
- [ ] Edge cases are covered
- [ ] Foreign key constraints are tested where applicable
- [ ] Unique constraints are tested where applicable

## Troubleshooting

### Common Issues

**Issue**: Tests fail with "database does not exist"
**Solution**: Ensure migrations have run: `just db-migrate`

**Issue**: SQLx prepare fails with "query ... not found"
**Solution**: Run `just sqlx-prepare` to regenerate query metadata

**Issue**: Tests hang indefinitely
**Solution**: Check for unclosed database connections or transactions

**Issue**: "too many open connections"
**Solution**: Restart test database: `just db-test-down && just db-test-up`

**Issue**: Fixture not found
**Solution**: Ensure fixture files are in `tests/fixtures/` directory

### Debug Mode

Run tests with full logging:

```bash
RUST_LOG=sqlx=debug,bdp_server=debug cargo test -- --nocapture
```

### Inspecting Test Database

To keep test database for inspection:

```rust
#[sqlx::test]
async fn test_inspect_database(pool: PgPool) -> sqlx::Result<()> {
    // Your test code

    // Print database URL for manual inspection
    println!("Database: {:?}", pool);

    // Pause to allow manual inspection
    // std::thread::sleep(std::time::Duration::from_secs(300));

    Ok(())
}
```

## Additional Resources

- [SQLx Documentation](https://docs.rs/sqlx)
- [SQLx Testing Guide](https://github.com/launchbadge/sqlx/blob/main/sqlx-macros/README.md)
- [PostgreSQL Testing Best Practices](https://www.postgresql.org/docs/current/regress.html)
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)

## Summary

This testing infrastructure provides:

- Automated test database creation and cleanup
- Isolated test execution for reliability
- Fixtures for shared test data
- Docker-based test environments
- CI/CD integration
- Comprehensive test patterns and examples

Follow these patterns to ensure robust, maintainable tests for the BDP project.
