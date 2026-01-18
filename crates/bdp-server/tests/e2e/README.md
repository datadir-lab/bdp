# E2E Test Infrastructure

This directory contains the end-to-end test infrastructure for the BDP ingestion pipeline.

## Architecture

The E2E test framework consists of four main components:

### 1. Test Data Management (`fixtures.rs`)
Manages test data in two modes:
- **CI Mode**: Uses small committed sample data (`tests/fixtures/uniprot_ci_sample.dat`)
- **Real Mode**: Downloads and caches real UniProt data for realistic testing

### 2. Environment Orchestration (`harness.rs`)
Provides the main `E2EEnvironment` struct that:
- Starts Docker containers (PostgreSQL, MinIO, optionally BDP server)
- Runs database migrations
- Provides helper methods for triggering jobs and waiting for completion
- Manages cleanup of resources

### 3. Assertions (`assertions.rs`)
High-level assertion helpers for verifying system state:
- Count records in database tables
- Count files in S3 storage
- Verify specific proteins exist
- Verify S3 files exist with correct checksums
- Batch verification of expected counts

### 4. Observability (`observability.rs`)
Debugging and monitoring utilities:
- Print pipeline status
- Query job statuses
- List S3 contents
- Get database statistics
- Export database state to JSON
- Wait for conditions with polling

## Usage Example

```rust
use bdp_server::e2e::{E2EEnvironment, TestDataManager, TestDataMode, ExpectedCounts};
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_full_ingestion_pipeline() {
    // Setup environment
    let env = E2EEnvironment::new().await.unwrap();
    let data_mgr = TestDataManager::new(TestDataMode::CI);

    // Upload test data
    let dat_path = data_mgr.get_uniprot_dat_path().unwrap();
    env.upload_test_data(&dat_path).await.unwrap();

    // Trigger ingestion job
    let org_id = Uuid::new_v4(); // Or get from database
    let job_id = env.trigger_ingestion_job(org_id, "2024_01").await.unwrap();

    // Wait for completion (with timeout)
    env.wait_for_job_completion(job_id.clone(), Duration::from_secs(60))
        .await
        .unwrap();

    // Verify results
    let assertions = env.assertions();

    // Check counts
    let expected = ExpectedCounts::new()
        .proteins(3)
        .organisms(2)
        .version_files(3);
    assertions.verify_counts(expected).await.unwrap();

    // Check specific protein
    assertions
        .verify_protein_exists("P01308", "Insulin")
        .await
        .unwrap();

    // Check S3 files
    assertions
        .verify_s3_file_exists("test-data/uniprot_ci_sample.dat", None)
        .await
        .unwrap();

    // Get job statistics
    let stats = assertions.get_job_stats(&job_id).await.unwrap();
    assert_eq!(stats.total_entries, 3);

    // Print status for debugging (if test fails)
    let obs = env.observability();
    obs.print_pipeline_status().await.unwrap();

    // Cleanup
    env.cleanup().await;
}
```

## Environment Variables

- `BDP_E2E_MODE`: Set to `ci` (default) or `real` to choose test data mode
- `BDP_SERVER_URL`: URL of running BDP server (default: `http://localhost:3000`)

## Prerequisites

### Docker
The E2E tests require Docker to be running for testcontainers:
- PostgreSQL 16-alpine
- MinIO latest

### Database Setup
The tests will automatically:
1. Start a PostgreSQL container
2. Run all migrations from `migrations/` directory
3. Let apalis auto-create its own schema on first use

### BDP Server
Currently, the BDP server must be running externally. Future improvements will include:
- Building BDP server Docker image
- Starting server container automatically
- Health checks before running tests

## Running Tests

### CI Mode (Fast, Small Dataset)
```bash
cargo test --test e2e -- --test-threads=1
```

### Real Mode (Slower, Real Data)
```bash
export BDP_E2E_MODE=real
just e2e-download-data  # Download real UniProt data (first time only)
cargo test --test e2e -- --test-threads=1
```

### Run Specific Test
```bash
cargo test --test e2e test_ingestion_pipeline -- --nocapture
```

## Troubleshooting

### Compilation Errors with apalis-postgres
If you see errors like "column 'last_result' does not exist", this is due to sqlx compile-time checking against a database that doesn't have the apalis schema yet.

**Solutions:**
1. Set up a test database with apalis schema and point `DATABASE_URL` to it
2. Use `SQLX_OFFLINE=true` to skip compile-time checks
3. Wait for apalis to create its schema at runtime (tests will work fine)

The E2E test infrastructure code itself is valid - the compilation errors are in the apalis-postgres dependency.

### Container Cleanup
Containers are automatically cleaned up when `E2EEnvironment::cleanup()` is called or when the environment is dropped. If tests fail unexpectedly, you may need to manually clean up:

```bash
docker ps -a | grep postgres
docker ps -a | grep minio
docker rm -f <container_id>
```

### Port Conflicts
If you get port binding errors, ensure:
- No other PostgreSQL/MinIO instances are running
- Previous test containers were cleaned up
- Docker has enough resources allocated

## Future Improvements

1. **BDP Server Container**: Automatically build and start BDP server in Docker
2. **Parallel Test Execution**: Improve isolation to allow parallel test execution
3. **Performance Benchmarking**: Add timing and throughput measurements
4. **Snapshot Testing**: Capture and compare database/S3 state snapshots
5. **Log Collection**: Automatically collect container logs on test failure
6. **Cleanup Strategies**: More sophisticated cleanup and test data management

## File Structure

```
tests/e2e/
├── README.md           # This file
├── mod.rs             # Module exports
├── harness.rs         # Main E2E environment orchestration (~450 lines)
├── fixtures.rs        # Test data management (~200 lines)
├── assertions.rs      # Assertion helpers (~300 lines)
└── observability.rs   # Debugging utilities (~350 lines)

tests/fixtures/
├── uniprot_ci_sample.dat  # Small CI test data (3 proteins)
└── real/                   # Downloaded real data (gitignored)
```

## Contributing

When adding new E2E tests:
1. Always use `E2EEnvironment::new()` to set up the environment
2. Call `env.cleanup()` at the end (or use `defer` pattern)
3. Use the assertion helpers rather than raw database queries
4. Add observability calls to help debug failures
5. Run tests in single-threaded mode to avoid conflicts
6. Document expected data and counts clearly

## Related Documentation

- [TESTING.md](../../TESTING.md) - Overall testing strategy
- [SETUP.md](../../SETUP.md) - Local development setup
- [docs/agents/workflows/](../../docs/agents/workflows/) - Pipeline workflows
