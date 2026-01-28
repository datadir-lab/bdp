# Testing Strategy

This document describes the testing strategy for the BDP project, including how to use testcontainers for integration testing without manual database/service setup.

## Testing Levels

### 1. Unit Tests

Unit tests are fast, isolated tests that test individual functions and modules.

**Location**: Inline in source files (`#[cfg(test)]` modules)

**Running**:
```bash
# Run all unit tests
cargo test --lib

# Run tests for a specific crate
cargo test -p bdp-server --lib
```

**Guidelines**:
- Mock external dependencies
- Test edge cases and error conditions
- Keep tests fast (< 100ms each)

### 2. Integration Tests

Integration tests verify that multiple components work together correctly.

**Location**: `crates/bdp-server/tests/`

**Types**:
- **Database integration tests**: Test database operations with real PostgreSQL
- **S3 integration tests**: Test storage operations with real MinIO
- **API integration tests**: Test HTTP endpoints

### 3. End-to-End (E2E) Tests

E2E tests verify complete workflows from end to end.

**Location**: `crates/bdp-server/tests/e2e/`

**Running**:
```bash
# Run E2E tests (CI mode - uses sample data)
cargo test --test e2e -- --ignored --nocapture

# Run E2E tests (real data mode)
BDP_E2E_MODE=real cargo test --test e2e -- --ignored --nocapture
```

## Testcontainers Infrastructure

The BDP project uses [testcontainers](https://docs.rs/testcontainers) to spin up real PostgreSQL and MinIO containers for integration testing. This eliminates the need for manual database/service setup.

### Quick Start

```rust
use testcontainers::runners::AsyncRunner;

mod common;
use common::{TestPostgres, TestMinio, TestEnvironment};

// PostgreSQL only
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_with_postgres() {
    let pg = TestPostgres::start().await.unwrap();
    let pool = pg.pool();

    // Your test code here
    sqlx::query("SELECT 1").execute(pool).await.unwrap();
}

// MinIO/S3 only
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_with_s3() {
    let minio = TestMinio::start().await.unwrap();

    minio.upload("test.txt", b"Hello".to_vec()).await.unwrap();
}

// Full environment (PostgreSQL + MinIO)
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_full() {
    let env = TestEnvironment::start().await.unwrap();

    let pool = env.db_pool();
    let s3 = env.s3_client();

    // Use both services
}
```

### Running Testcontainers Tests

Tests that require Docker are marked with `#[ignore = "requires Docker"]`. To run them:

```bash
# Run all testcontainers tests
cargo test -- --ignored

# Run with output visible
cargo test -- --ignored --nocapture

# Run specific test file
cargo test --test testcontainers_example_test -- --ignored --nocapture

# Run specific test
cargo test test_database_operations -- --ignored --nocapture
```

### Available Test Utilities

#### TestPostgres

Starts a PostgreSQL container with migrations automatically applied.

```rust
use common::{TestPostgres, PostgresOptions};

// Default configuration (PostgreSQL 16, migrations enabled)
let pg = TestPostgres::start().await?;

// Custom configuration
let pg = TestPostgres::start_with_options(
    PostgresOptions::default()
        .with_version("15-alpine")
        .with_max_connections(10)
).await?;

// Without migrations
let pg = TestPostgres::start_with_options(
    PostgresOptions::without_migrations()
).await?;

// Access the pool
let pool = pg.pool();
let pool_clone = pg.pool_clone();
let conn_string = pg.connection_string();
```

#### TestMinio

Starts a MinIO (S3-compatible) container with a default bucket.

```rust
use common::{TestMinio, MinioOptions};

// Default configuration
let minio = TestMinio::start().await?;

// Custom bucket
let minio = TestMinio::start_with_options(
    MinioOptions::default().with_bucket("my-bucket")
).await?;

// Convenience methods
minio.upload("key", data).await?;
let data = minio.download("key").await?;
let objects = minio.list_objects(Some("prefix/")).await?;

// Direct S3 client access
let client = minio.client();
```

#### TestEnvironment

Starts both PostgreSQL and MinIO in parallel for faster startup.

```rust
use common::TestEnvironment;

let env = TestEnvironment::start().await?;

// Access services
let pool = env.db_pool();
let s3 = env.s3_client();
let bucket = env.s3_bucket();

// Individual container access
let postgres = env.postgres();
let minio = env.minio();
```

#### TestDataHelper

Helper for creating test data quickly.

```rust
use common::TestDataHelper;

let pg = TestPostgres::start().await?;
let helper = TestDataHelper::new(pg.pool());

// Create individual entities
let org_id = helper.create_organization("test-org", "Test Org").await?;
let entry_id = helper.create_registry_entry(org_id, "entry", "Entry", "data_source").await?;
let version_id = helper.create_version(entry_id, "1.0").await?;

// Or create a complete dataset at once
let (org_id, entry_id, version_id) = helper
    .create_test_dataset("org-slug", "entry-slug", "1.0")
    .await?;
```

### Test Isolation

Each test gets its own container instances, ensuring complete isolation:

```rust
// These tests can run in parallel without conflict
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_a() {
    let pg = TestPostgres::start().await?;
    // This is a completely separate database
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_b() {
    let pg = TestPostgres::start().await?;
    // This is a different database from test_a
}
```

### Converting Ignored Tests

To convert a test that was previously ignored due to database requirements:

**Before**:
```rust
#[tokio::test]
#[ignore] // Requires database
async fn test_search() {
    let pool = get_pool_from_env().await; // Manual setup required
    // test code...
}
```

**After**:
```rust
mod common;
use common::TestPostgres;

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_search() {
    let pg = TestPostgres::start().await.unwrap();
    let pool = pg.pool();
    // test code...
}
```

## CI/CD Considerations

### GitHub Actions

Testcontainers works in GitHub Actions out of the box. The default runners have Docker installed.

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests (with Docker)
        run: cargo test -- --ignored
```

### Docker-in-Docker (DinD)

If running in a container environment, ensure Docker-in-Docker is enabled:

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      docker:
        image: docker:dind
        options: --privileged
```

### Performance Tips

1. **Parallel container startup**: Use `TestEnvironment` which starts PostgreSQL and MinIO in parallel
2. **Image caching**: Container images are cached after first pull
3. **Test parallelism**: Each test gets isolated containers, so tests can run in parallel
4. **Skip when no Docker**: Use the `skip_if_no_docker!()` macro for graceful degradation

## When to Use Testcontainers vs Mocks

### Use Testcontainers When:

- Testing database queries (SQL correctness, migrations)
- Testing S3 operations (upload, download, list)
- Integration testing multiple components
- Testing real service behavior (connection pooling, transactions)
- E2E testing complete workflows

### Use Mocks When:

- Unit testing business logic
- Testing error handling for external services
- Performance-critical test suites
- Testing retry/timeout behavior
- Testing without Docker availability

### Hybrid Approach

```rust
// Use mocks for the external service, real database for data layer
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_ingestion_with_mocked_ftp() {
    let pg = TestPostgres::start().await?;
    let mock_ftp = MockFtpServer::new(); // Use wiremock or similar

    // Test ingestion logic with real DB, mocked FTP
}
```

## Existing Test Infrastructure

### E2E Test Module

Located in `crates/bdp-server/tests/e2e/`, this provides comprehensive E2E testing:

- **harness.rs**: Full environment orchestration (PostgreSQL, MinIO, BDP server)
- **fixtures.rs**: Test data management (CI fixtures, real data download)
- **assertions.rs**: High-level assertion helpers
- **observability.rs**: Debugging and monitoring utilities

### Helpers Module

Located in `crates/bdp-server/tests/helpers/`, provides:

- **mod.rs**: Test database setup, fixture loading, builders
- **fixtures.rs**: Fluent builders for test data (OrganizationFixture, VersionFixture, etc.)

## File Structure

```
crates/bdp-server/tests/
├── common/
│   └── mod.rs              # Testcontainers utilities (TestPostgres, TestMinio, etc.)
├── e2e/
│   ├── mod.rs              # E2E module exports
│   ├── harness.rs          # Full E2E environment
│   ├── fixtures.rs         # Test data management
│   ├── assertions.rs       # Assertion helpers
│   └── observability.rs    # Debugging utilities
├── helpers/
│   ├── mod.rs              # Test database helpers
│   └── fixtures.rs         # Data builders
├── testcontainers_example_test.rs  # Example testcontainers usage
├── e2e.rs                  # E2E test runner
└── *.rs                    # Various integration test files
```

## Best Practices

1. **Always use `#[ignore = "requires Docker"]`** for testcontainers tests
2. **Initialize tracing** with `init_test_tracing()` for debugging
3. **Use TestDataHelper** for quick test data setup
4. **Prefer TestEnvironment** when you need both DB and S3
5. **Keep containers running** - don't manually stop them, let Rust's Drop handle cleanup
6. **Use specific PostgreSQL versions** for reproducibility
7. **Document test requirements** in test file headers
