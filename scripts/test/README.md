# Test Scripts

This directory contains scripts for running and managing BDP tests.

## Scripts

### test_ncbi_taxonomy.sh / test_ncbi_taxonomy.ps1

Test runner specifically for NCBI Taxonomy ingestion module.

**Usage:**
```bash
# Linux/macOS
./scripts/test/test_ncbi_taxonomy.sh [OPTIONS]

# Windows PowerShell
.\scripts\test\test_ncbi_taxonomy.ps1 [OPTIONS]
```

**Options:**
- `--unit-only`: Run only unit tests (parser, pipeline, version discovery)
- `--integration`: Run integration tests (requires database)
- `--all`: Run all tests (default)
- `--nocapture`: Show test output (useful for debugging)

**Examples:**
```bash
# Run only unit tests (no database required)
./scripts/test/test_ncbi_taxonomy.sh --unit-only

# Run integration tests (requires database)
export DATABASE_URL=postgresql://localhost/bdp_test
./scripts/test/test_ncbi_taxonomy.sh --integration

# Run all tests with output
./scripts/test/test_ncbi_taxonomy.sh --all --nocapture
```

**Test Coverage:**
- **Unit Tests (14 tests)**:
  - Parser tests (12 tests) - rankedlineage, merged, delnodes parsing
  - Pipeline tests (2 tests) - PipelineResult functionality
  - Version discovery tests - Smart version bumping logic

- **Integration Tests (8 tests)**:
  - Storage basic functionality
  - Idempotency verification
  - Multiple versions handling
  - Merged taxa deprecation
  - Deleted taxa deprecation
  - Version files creation

### run-integration-tests.sh

Automated test runner that handles database setup, test execution, and cleanup.

**Usage:**
```bash
./scripts/test/run-integration-tests.sh [OPTIONS]
```

**Options:**
- `--verbose, -v`: Show detailed output
- `--no-cleanup`: Keep database running after tests
- `--keep-on-failure`: Keep database only if tests fail
- `--help, -h`: Show help message

**Examples:**
```bash
# Normal test run
./scripts/test/run-integration-tests.sh

# Verbose mode with debugging
./scripts/test/run-integration-tests.sh --verbose

# Keep database for manual inspection
./scripts/test/run-integration-tests.sh --no-cleanup

# Debug test failures
./scripts/test/run-integration-tests.sh --keep-on-failure
```

**What it does:**
1. Checks prerequisites (Docker, cargo, etc.)
2. Starts test database in Docker
3. Waits for database to be ready
4. Runs database migrations
5. Executes integration tests
6. Cleans up (unless --no-cleanup or --keep-on-failure)

### wait-for-postgres.sh

Utility script that waits for PostgreSQL to become available.

**Usage:**
```bash
./scripts/test/wait-for-postgres.sh [OPTIONS]
```

**Options:**
- `--host, -h HOST`: PostgreSQL host (default: localhost)
- `--port, -p PORT`: PostgreSQL port (default: 5433)
- `--user, -U USER`: PostgreSQL user (default: bdp_test)
- `--database, -d DB`: Database name (default: bdp_test)
- `--timeout, -t SECS`: Timeout in seconds (default: 30)
- `--container, -c NAME`: Docker container name
- `--quiet, -q`: Suppress output
- `--help`: Show help message

**Examples:**
```bash
# Wait for test database
./scripts/test/wait-for-postgres.sh

# Wait for database in container
./scripts/test/wait-for-postgres.sh --container bdp-postgres-test

# Custom timeout
./scripts/test/wait-for-postgres.sh --timeout 60

# Different port
./scripts/test/wait-for-postgres.sh --port 5432
```

## Environment Variables

Both scripts respect these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection URL | `postgresql://bdp_test:test_password@localhost:5433/bdp_test` |
| `RUST_LOG` | Logging level | `info` |
| `TEST_THREADS` | Number of test threads | Auto (CPU cores) |
| `SQLX_LOG` | SQLx logging level | `warn` |

## Configuration

Create a `.env.test` file in the project root to override defaults:

```bash
cp .env.test.example .env.test
# Edit .env.test with your preferences
```

## Workflow

### Quick Test Run

```bash
./scripts/test/run-integration-tests.sh
```

### Development Workflow

```bash
# Start database once
docker-compose -f docker/docker-compose.test.yml up -d

# Wait for it to be ready
./scripts/test/wait-for-postgres.sh --container bdp-postgres-test

# Run migrations
export DATABASE_URL=postgresql://bdp_test:test_password@localhost:5433/bdp_test
cargo sqlx migrate run

# Run tests repeatedly during development
cargo test --test db_tests

# When done, clean up
docker-compose -f docker/docker-compose.test.yml down -v
```

### Debugging Failed Tests

```bash
# Keep database running if tests fail
./scripts/test/run-integration-tests.sh --keep-on-failure

# If tests failed, the database will still be running
# Connect to investigate
psql postgresql://bdp_test:test_password@localhost:5433/bdp_test

# When done debugging
docker-compose -f docker/docker-compose.test.yml down -v
```

## CI/CD Integration

These scripts are used in GitHub Actions workflows:

```yaml
- name: Run Integration Tests
  run: ./scripts/test/run-integration-tests.sh --verbose
```

See `.github/workflows/ci.yml` for the complete CI configuration.

## Troubleshooting

### Script Permission Issues

```bash
# Make scripts executable
chmod +x scripts/test/*.sh
```

### Docker Not Found

Ensure Docker is installed and running:
```bash
docker --version
docker ps
```

### Port Already in Use

If port 5433 is already in use:

```bash
# Find what's using the port
lsof -i :5433  # macOS/Linux
netstat -ano | findstr :5433  # Windows

# Either stop that service or change the port in docker-compose.test.yml
```

### Database Won't Start

```bash
# Check Docker logs
docker logs bdp-postgres-test

# Remove old containers
docker-compose -f docker/docker-compose.test.yml down -v

# Try again
docker-compose -f docker/docker-compose.test.yml up -d
```

### Tests Hang

```bash
# Run with timeout
timeout 300 ./scripts/test/run-integration-tests.sh

# Or reduce test parallelism
TEST_THREADS=1 ./scripts/test/run-integration-tests.sh
```

## Best Practices

1. **Use the automated script** for consistent test execution
2. **Keep database running during development** to speed up iteration
3. **Use --keep-on-failure** when debugging test failures
4. **Check script exit codes** in CI/CD pipelines
5. **Review logs** when tests fail unexpectedly

## Additional Resources

- [Testing Guide](../../docs/development/testing.md)
- [Quick Reference](../../docs/development/testing-quick-reference.md)
- [Test README](../../crates/bdp-server/tests/README.md)
