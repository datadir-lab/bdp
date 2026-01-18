# BDP Testing Infrastructure

This document provides an overview of the testing infrastructure for the BDP (Biological Data Platform) project.

## Overview

BDP uses a comprehensive testing strategy that includes:

- **Unit tests**: Testing individual functions and modules
- **Integration tests**: Testing database operations with SQLx
- **End-to-end tests**: Testing complete workflows
- **CI/CD automation**: Automated testing on every commit

## Quick Start

### Run All Tests

```bash
# Run all tests (recommended)
just test

# Run tests with verbose output
just test-verbose
```

### Run Specific Tests

```bash
# Unit tests only
just test-unit

# Integration tests only
just test-integration

# Specific test function
just test-one test_create_organization

# Fresh test run (resets database)
just test-fresh
```

## Project Structure

```
bdp/
├── .env.test.example              # Test environment configuration template
├── .github/workflows/ci.yml       # CI/CD pipeline with test automation
├── crates/bdp-server/
│   ├── Cargo.toml                 # Test dependencies
│   └── tests/
│       ├── README.md              # Test documentation
│       ├── db_tests.rs            # Database integration tests
│       ├── fixtures/              # SQL test data
│       │   ├── organizations.sql
│       │   └── registry_entries.sql
│       └── helpers/               # Test utilities
│           └── mod.rs             # Builders, assertions, setup
├── docker/
│   └── docker-compose.test.yml    # Test database configuration
├── docs/development/
│   ├── testing.md                 # Comprehensive testing guide
│   └── testing-quick-reference.md # Quick reference
└── scripts/test/
    ├── README.md                  # Script documentation
    ├── run-integration-tests.sh   # Automated test runner
    └── wait-for-postgres.sh       # Database readiness checker
```

## Key Features

### 1. SQLx Testing Framework

- Uses `#[sqlx::test]` macro for automatic database setup/teardown
- Each test gets its own isolated database
- Automatic migration application
- Support for test fixtures

### 2. Test Helpers

**Builders**:
- `OrganizationBuilder`: Create test organizations
- `RegistryEntryBuilder`: Create test registry entries

**Assertions**:
- `assert_table_count()`: Verify row counts
- `assert_exists_by_id()`: Verify record existence
- `assert_not_exists_by_id()`: Verify record absence

**Utilities**:
- `TestDb`: Manual test database management
- `FixtureLoader`: Load SQL fixtures programmatically

### 3. Docker-Based Test Database

- Isolated from development database
- Runs on port 5433 (vs. 5432 for dev)
- Uses tmpfs for fast execution
- Optimized PostgreSQL configuration for testing

### 4. Automated Test Scripts

- Just commands handle database setup automatically
- Test database runs on separate port (5433)
- Configurable via environment variables in `.env.test`
- Full CI/CD integration

### 5. CI/CD Integration

- Automated testing on GitHub Actions
- PostgreSQL service container
- Migration verification
- SQLx prepare checking
- Separate unit, integration, and doc test runs

## Testing Workflow

### Development Workflow

1. Write test using `#[sqlx::test]` macro
2. Run tests: `just test`
3. If test fails, run with verbose output: `just test-verbose`
4. Iterate until tests pass

### Example Test

```rust
#[sqlx::test]
async fn test_create_organization(pool: PgPool) -> sqlx::Result<()> {
    let org = sqlx::query!(
        "INSERT INTO organizations (slug, name) VALUES ($1, $2) RETURNING id, slug, name",
        "test-org",
        "Test Organization"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(org.slug, "test-org");
    assert_eq!(org.name, "Test Organization");

    Ok(())
}
```

### Using Fixtures

```rust
#[sqlx::test(fixtures("organizations", "registry_entries"))]
async fn test_with_data(pool: PgPool) -> sqlx::Result<()> {
    // Organizations and entries are already loaded
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations")
        .fetch_one(&pool)
        .await?;

    assert!(count > 0);
    Ok(())
}
```

### Using Test Helpers

```rust
#[sqlx::test]
async fn test_with_builder(pool: PgPool) -> sqlx::Result<()> {
    let org_id = helpers::builders::OrganizationBuilder::new("test", "Test Org")
        .website("https://example.com")
        .description("A test organization")
        .create(&pool)
        .await?;

    helpers::assertions::assert_exists_by_id(&pool, "organizations", org_id).await?;

    Ok(())
}
```

## Test Categories

The test suite includes:

1. **Organization Tests**: CRUD operations, constraints, validation
2. **Registry Entry Tests**: Data sources, tools, foreign keys, search
3. **Version Tests**: Version management, constraints, relationships
4. **Transaction Tests**: Commit, rollback, error handling
5. **Complex Query Tests**: Joins, aggregations, full-text search

## Configuration

### Environment Variables

Create a `.env.test` file (copy from `.env.test.example`):

```bash
DATABASE_URL=postgresql://bdp_test:test_password@localhost:5433/bdp_test
RUST_LOG=info
TEST_THREADS=0
```

### Test Database

Configured in `docker/docker-compose.test.yml`:

- **Image**: postgres:16-alpine
- **Port**: 5433
- **Database**: bdp_test
- **User**: bdp_test
- **Password**: test_password
- **Storage**: tmpfs (ephemeral, fast)

## CI/CD Pipeline

GitHub Actions workflow includes:

1. **Linting**: Clippy and formatting checks
2. **Migration**: Database schema verification
3. **SQLx Prepare**: Query metadata verification
4. **Unit Tests**: Library tests
5. **Integration Tests**: Database tests
6. **Doc Tests**: Documentation examples
7. **Build**: Multi-target verification

## Best Practices

1. **Use `#[sqlx::test]`**: Automatic setup/teardown
2. **Test isolation**: Each test is independent
3. **Fixtures for common data**: Share test data via SQL files
4. **Builders for complex data**: Use builder pattern
5. **Meaningful assertions**: Assert specific values
6. **Test edge cases**: Include error conditions
7. **Descriptive names**: Clear test function names
8. **Clean up automatically**: Let SQLx handle it

## Troubleshooting

### Common Issues

**Database connection fails**:
```bash
# Start test database
just db-test-up
```

**Migrations fail**:
```bash
# Run migrations
just db-migrate
```

**Tests hang**:
```bash
# Reset test database
just db-test-down
just db-test-up
just test
```

**Too many connections**:
```bash
# Restart test database
just db-test-down
just db-test-up
```

### Debugging Failed Tests

```bash
# Run with full logging
RUST_LOG=debug just test-verbose

# Run specific test with output
just test-one test_name

# Connect to test database manually
just db-shell
```

## Documentation

- **[Comprehensive Testing Guide](docs/development/testing.md)**: Detailed guide with examples
- **[Quick Reference](docs/development/testing-quick-reference.md)**: Common commands and patterns
- **[Test README](crates/bdp-server/tests/README.md)**: Test structure and usage
- **[Script README](scripts/test/README.md)**: Test script documentation

## Contributing

When adding new tests:

1. Follow existing patterns
2. Use `#[sqlx::test]` for database tests
3. Add fixtures for reusable test data
4. Document complex test scenarios
5. Ensure tests are independent
6. Run `just test` before committing
7. Verify with `just ci` before pushing

## Performance

Test execution is optimized:

- **Parallel execution**: Tests run concurrently
- **tmpfs storage**: Fast ephemeral database storage
- **Optimized PostgreSQL**: Tuned for test performance
- **Connection pooling**: Efficient database connections

Typical test suite execution: < 30 seconds

## Future Enhancements

Planned improvements:

- [ ] Property-based testing with QuickCheck
- [ ] Mutation testing
- [ ] Performance benchmarks
- [ ] Load testing scripts
- [ ] Test coverage reporting
- [ ] Snapshot testing for complex queries

## Support

For questions or issues:

1. Check the documentation in `docs/development/`
2. Review test examples in `crates/bdp-server/tests/`
3. Consult the troubleshooting section
4. Open an issue on GitHub

## License

Same as the BDP project - see LICENSE file.
