# Testing Quick Reference

Quick reference guide for common testing tasks in BDP.

## Common Commands

### Running Tests

```bash
# All tests
cargo test

# Only integration tests
cargo test --test '*'

# Specific test file
cargo test --test db_tests

# Specific test function
cargo test test_create_organization

# With output
cargo test -- --nocapture

# With logging
RUST_LOG=debug cargo test

# Sequential execution (no parallelism)
cargo test -- --test-threads=1
```

### Using the Test Script

```bash
# Normal run
./scripts/test/run-integration-tests.sh

# Verbose output
./scripts/test/run-integration-tests.sh --verbose

# Keep database for debugging
./scripts/test/run-integration-tests.sh --no-cleanup

# Keep database only on failure
./scripts/test/run-integration-tests.sh --keep-on-failure
```

### Database Management

```bash
# Start test database
docker-compose -f docker/docker-compose.test.yml up -d

# Wait for database
./scripts/test/wait-for-postgres.sh --container bdp-postgres-test

# Run migrations
export DATABASE_URL=postgresql://bdp_test:test_password@localhost:5433/bdp_test
cargo sqlx migrate run

# Connect to test database
psql postgresql://bdp_test:test_password@localhost:5433/bdp_test

# Stop and clean up
docker-compose -f docker/docker-compose.test.yml down -v
```

### SQLx Commands

```bash
# Prepare query metadata
cargo sqlx prepare

# Check prepared queries are up to date
cargo sqlx prepare --check

# Create a new migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Test Patterns

### Basic Test

```rust
#[sqlx::test]
async fn test_example(pool: PgPool) -> sqlx::Result<()> {
    // Your test code
    Ok(())
}
```

### Test with Fixtures

```rust
#[sqlx::test(fixtures("organizations"))]
async fn test_with_data(pool: PgPool) -> sqlx::Result<()> {
    // Data from fixtures is already loaded
    Ok(())
}
```

### Test with Builder

```rust
#[sqlx::test]
async fn test_builder(pool: PgPool) -> sqlx::Result<()> {
    let org_id = helpers::builders::OrganizationBuilder::new("test", "Test")
        .website("https://example.com")
        .create(&pool)
        .await?;
    Ok(())
}
```

### Test with Assertions

```rust
#[sqlx::test]
async fn test_assertions(pool: PgPool) -> sqlx::Result<()> {
    helpers::assertions::assert_table_count(&pool, "organizations", 0).await?;
    Ok(())
}
```

### Transaction Test

```rust
#[sqlx::test]
async fn test_transaction(pool: PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    // Do work
    tx.commit().await?;
    Ok(())
}
```

## Creating Fixtures

1. Create file in `crates/bdp-server/tests/fixtures/`
2. Name it `<fixture_name>.sql`
3. Add INSERT statements
4. Use in tests: `#[sqlx::test(fixtures("<fixture_name>"))]`

Example fixture:

```sql
-- tests/fixtures/example.sql
INSERT INTO organizations (slug, name) VALUES
    ('example-1', 'Example 1'),
    ('example-2', 'Example 2');
```

## Helper Functions

### Creating Test Data

```rust
// Create organization
let org_id = helpers::create_test_organization(&pool, "slug", "Name").await?;

// Create organization with details
let org_id = helpers::create_test_organization_full(
    &pool,
    "slug",
    "Name",
    Some("https://example.com"),
    Some("Description"),
    false
).await?;

// Create registry entry
let entry_id = helpers::create_test_registry_entry(
    &pool,
    org_id,
    "entry-slug",
    "Entry Name",
    "data_source"
).await?;

// Create version
let version_id = helpers::create_test_version(
    &pool,
    entry_id,
    "1.0.0"
).await?;
```

### Assertions

```rust
// Assert row count
helpers::assertions::assert_table_count(&pool, "organizations", 5).await?;

// Assert record exists
helpers::assertions::assert_exists_by_id(&pool, "organizations", org_id).await?;

// Assert record doesn't exist
helpers::assertions::assert_not_exists_by_id(&pool, "organizations", org_id).await?;
```

## Troubleshooting

### Database Connection Issues

```bash
# Check if container is running
docker ps | grep bdp-postgres-test

# Check container logs
docker logs bdp-postgres-test

# Verify database is ready
docker exec bdp-postgres-test pg_isready -U bdp_test
```

### Migration Issues

```bash
# Check migration status
sqlx migrate info

# Force reset (DANGER: deletes all data)
docker-compose -f docker/docker-compose.test.yml down -v
docker-compose -f docker/docker-compose.test.yml up -d
cargo sqlx migrate run
```

### Test Failures

```bash
# Run with full logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run single test
cargo test test_name -- --exact

# Keep database on failure
./scripts/test/run-integration-tests.sh --keep-on-failure
```

### SQLx Prepare Issues

```bash
# Regenerate prepared queries
cargo sqlx prepare

# Run in offline mode
cargo test --features sqlx/offline
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | Test database connection | `postgresql://bdp_test:test_password@localhost:5433/bdp_test` |
| `RUST_LOG` | Logging level | `info` |
| `TEST_THREADS` | Parallel test threads | Auto |
| `SQLX_OFFLINE` | Use offline mode | `false` |

## File Locations

| File | Purpose |
|------|---------|
| `crates/bdp-server/tests/` | Integration tests |
| `crates/bdp-server/tests/fixtures/` | Test data fixtures |
| `crates/bdp-server/tests/helpers/` | Test utilities |
| `docker/docker-compose.test.yml` | Test database config |
| `scripts/test/` | Test automation scripts |
| `.env.test` | Test environment config |

## Best Practices

1. Use `#[sqlx::test]` for database tests
2. Keep tests independent and isolated
3. Use fixtures for common test data
4. Use builders for complex test data
5. Test both success and error cases
6. Use descriptive test names
7. Clean up resources (handled by sqlx::test)
8. Don't test implementation details

## CI/CD

Tests run automatically on:
- Push to `main` or `develop`
- Pull requests

CI runs:
1. Database migrations
2. SQLx prepare verification
3. Unit tests
4. Integration tests
5. Doc tests

View results: `.github/workflows/ci.yml`

## Additional Resources

- [Full Testing Guide](./testing.md)
- [Test README](../../crates/bdp-server/tests/README.md)
- [SQLx Documentation](https://docs.rs/sqlx)
