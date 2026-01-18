# BDP Server Integration Tests

This directory contains integration tests for the BDP server, including database operations and storage functionality.

## Structure

```
tests/
├── README.md           # This file
├── db_tests.rs         # Database integration tests
├── storage_tests.rs    # S3/MinIO storage integration tests
├── fixtures/           # SQL fixtures for test data
│   ├── organizations.sql
│   └── registry_entries.sql
└── helpers/            # Test helper utilities
    └── mod.rs          # Test database setup, builders, assertions
```

## Running Tests

### Quick Start

```bash
# Run all integration tests
cargo test --test '*'

# Run specific test file
cargo test --test db_tests
cargo test --test storage_tests

# Run with output
cargo test --test db_tests -- --nocapture

# Run specific test
cargo test test_create_organization
cargo test test_storage_upload_download
```

### Using the Automated Script

The project includes a comprehensive test runner script:

```bash
# Run tests with automatic database setup/teardown
./scripts/test/run-integration-tests.sh

# Run with verbose output
./scripts/test/run-integration-tests.sh --verbose

# Keep database running for debugging
./scripts/test/run-integration-tests.sh --no-cleanup

# Keep database only if tests fail
./scripts/test/run-integration-tests.sh --keep-on-failure
```

### Manual Database Setup

If you prefer to manage the test database manually:

```bash
# Start test database
docker-compose -f docker/docker-compose.test.yml up -d

# Wait for database to be ready
until docker exec bdp-postgres-test pg_isready -U bdp_test -d bdp_test; do
  sleep 1
done

# Run migrations
export DATABASE_URL=postgresql://bdp_test:test_password@localhost:5433/bdp_test
cargo sqlx migrate run

# Run tests
cargo test --test '*'

# Clean up
docker-compose -f docker/docker-compose.test.yml down -v
```

## Writing Tests

### Using `#[sqlx::test]` Macro

The recommended way to write database tests is using the `#[sqlx::test]` macro:

```rust
#[sqlx::test]
async fn test_example(pool: PgPool) -> sqlx::Result<()> {
    // Test code here
    Ok(())
}
```

This macro:
- Creates a new test database
- Runs all migrations
- Provides a clean database pool
- Cleans up automatically

### Using Fixtures

Load predefined test data from SQL files:

```rust
#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_with_data(pool: PgPool) -> sqlx::Result<()> {
    // Organizations and registry entries are already loaded
    Ok(())
}
```

### Using Test Helpers

The `helpers` module provides utilities for common test operations:

```rust
use crate::helpers;

#[sqlx::test]
async fn test_with_helpers(pool: PgPool) -> sqlx::Result<()> {
    // Create test data using builders
    let org_id = helpers::builders::OrganizationBuilder::new("test", "Test Org")
        .website("https://example.com")
        .create(&pool)
        .await?;

    // Use assertions
    helpers::assertions::assert_exists_by_id(&pool, "organizations", org_id).await?;

    Ok(())
}
```

## Test Categories

### Database Tests (`db_tests.rs`)

#### Organization Tests
- Creating, reading, updating, deleting organizations
- Unique constraint testing
- System vs. user organizations

#### Registry Entry Tests
- Creating data sources and tools
- Foreign key constraints
- Full-text search
- Entry type validation

#### Version Tests
- Creating versions for registry entries
- Status constraints
- Multiple versions per entry

#### Transaction Tests
- Commit behavior
- Rollback behavior
- Constraint violation handling

#### Complex Query Tests
- Joins between tables
- Aggregations
- Full-text search

### Storage Tests (`storage_tests.rs`)

#### Upload/Download Tests
- File upload and download
- Binary file handling
- Large file support
- Content verification

#### Checksum Tests
- SHA256 checksum generation
- Checksum verification
- Empty file checksums

#### Presigned URL Tests
- URL generation
- Different expiration times
- URL validation

#### Existence Tests
- File existence checks
- Non-existent file handling

#### Metadata Tests
- Size and content-type retrieval
- Last modified timestamps
- Various content types

#### List Tests
- Listing files by prefix
- Max keys limitation
- Nested path handling
- Empty prefix handling

#### Delete Tests
- File deletion
- Multiple file deletion
- Non-existent file deletion

#### Copy Tests
- File copying
- Metadata preservation
- Overwrite behavior

#### Error Handling Tests
- Non-existent file operations
- Invalid operations
- Edge cases

**Requirements**: Storage tests require MinIO or S3 to be running with `S3_ENDPOINT` environment variable set. Tests will be skipped if not configured.

## Fixtures

Fixtures are SQL files in the `fixtures/` directory that provide common test data:

- **organizations.sql**: System organizations (UniProt, NCBI, Ensembl)
- **registry_entries.sql**: Sample registry entries for testing

To create a new fixture:

1. Create a `.sql` file in `tests/fixtures/`
2. Add INSERT statements for your test data
3. Use the fixture in tests: `#[sqlx::test(fixtures("your_fixture"))]`

## Test Helpers

The `helpers` module provides:

### Builders
- `OrganizationBuilder`: Fluent API for creating test organizations
- `RegistryEntryBuilder`: Fluent API for creating test registry entries

### Assertions
- `assert_table_count()`: Verify row count in a table
- `assert_exists_by_id()`: Verify a record exists
- `assert_not_exists_by_id()`: Verify a record doesn't exist

### Utilities
- `create_test_organization()`: Quick organization creation
- `create_test_registry_entry()`: Quick registry entry creation
- `create_test_version()`: Quick version creation

## Best Practices

1. **Use `#[sqlx::test]`**: Prefer this over manual setup
2. **Test isolation**: Each test should be independent
3. **Use fixtures**: Share common test data via fixtures
4. **Builder pattern**: Use builders for complex test data
5. **Meaningful assertions**: Assert specific values, not just "no error"
6. **Test edge cases**: Include error conditions
7. **Clean test names**: Use descriptive function names

## Troubleshooting

### Tests fail with "database does not exist"
- Ensure migrations have run: `cargo sqlx migrate run`
- Check DATABASE_URL is correct
- Verify test database is running

### Tests hang
- Check for unclosed connections or transactions
- Reduce parallelism: `cargo test -- --test-threads=1`

### "too many open connections"
- Reduce test parallelism
- Check for connection leaks in tests

### Fixture not found
- Ensure fixture files are in `tests/fixtures/`
- Check file name matches fixture name in test

## Documentation

For more detailed information, see:
- [Testing Guide](../../../docs/development/testing.md)
- [SQLx Documentation](https://docs.rs/sqlx)
- [SQLx Testing Guide](https://github.com/launchbadge/sqlx/blob/main/sqlx-macros/README.md)

## Environment Variables

### Database Tests
- `DATABASE_URL`: PostgreSQL connection string for tests
  - Default: `postgresql://bdp_test:test_password@localhost:5433/bdp_test`

### Storage Tests
- `S3_ENDPOINT`: S3/MinIO endpoint URL (required for storage tests)
  - Example: `http://localhost:9000`
- `S3_REGION`: AWS region (default: `us-east-1`)
- `S3_BUCKET`: Bucket name (default: `bdp-data`)
- `S3_ACCESS_KEY`: Access key (default: `minioadmin`)
- `S3_SECRET_KEY`: Secret key (default: `minioadmin`)
- `S3_PATH_STYLE`: Use path-style URLs (default: `false`)

### General
- `RUST_LOG`: Logging level for tests
  - Example: `RUST_LOG=debug cargo test`
- `TEST_THREADS`: Number of parallel test threads
  - Example: `TEST_THREADS=1 cargo test`

### Example: Running Storage Tests with MinIO

```bash
# Start MinIO via docker-compose
docker-compose up -d minio

# Set environment variables
export S3_ENDPOINT=http://localhost:9000
export S3_ACCESS_KEY=minioadmin
export S3_SECRET_KEY=minioadmin
export S3_BUCKET=bdp-data
export S3_PATH_STYLE=true

# Run storage tests
cargo test --test storage_tests
```
