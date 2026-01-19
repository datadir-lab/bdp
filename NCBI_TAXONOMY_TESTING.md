# NCBI Taxonomy Testing Guide

This guide covers how to run tests for the NCBI Taxonomy ingestion module.

## Quick Start

### Run Unit Tests (No Database Required)

```bash
# Linux/macOS
./scripts/test/test_ncbi_taxonomy.sh --unit-only

# Windows PowerShell
.\scripts\test\test_ncbi_taxonomy.ps1 -UnitOnly

# Or use cargo directly
cargo test --test ncbi_taxonomy_parser_test
cargo test --lib ncbi_taxonomy::pipeline::tests
cargo test --lib ncbi_taxonomy::version_discovery::tests
```

**Unit tests include:**
- ✅ 12 parser tests (rankedlineage, merged, delnodes parsing)
- ✅ 2 pipeline tests (PipelineResult functionality)
- ✅ Version discovery tests (smart version bumping)

All unit tests should pass without any setup.

### Run Integration Tests (Requires Database)

Integration tests require a running PostgreSQL database with migrations applied.

#### Step 1: Set up Test Database

```bash
# Using Docker (recommended)
docker run -d \
  --name bdp-test-db \
  -e POSTGRES_DB=bdp_test \
  -e POSTGRES_USER=bdp_test \
  -e POSTGRES_PASSWORD=test_password \
  -p 5433:5432 \
  postgres:15
```

Or use your existing database:
```bash
# Create test database
psql -c "CREATE DATABASE bdp_test;"
```

#### Step 2: Set DATABASE_URL

```bash
# For Docker setup above
export DATABASE_URL="postgresql://bdp_test:test_password@localhost:5433/bdp_test"

# For local PostgreSQL
export DATABASE_URL="postgresql://localhost/bdp_test"
```

#### Step 3: Run Migrations

```bash
cargo sqlx migrate run
```

#### Step 4: Run Integration Tests

```bash
# Using test script
./scripts/test/test_ncbi_taxonomy.sh --integration

# Or use cargo directly
cargo test --test ncbi_taxonomy_integration_test -- --ignored --nocapture
```

**Integration tests include:**
- ✅ Storage basic functionality
- ✅ Idempotency verification (re-running same data)
- ✅ Multiple versions handling
- ✅ Merged taxa deprecation
- ✅ Deleted taxa deprecation
- ✅ Version files creation (JSON + TSV)

## Test Structure

### Unit Tests

Located in:
- `tests/ncbi_taxonomy_parser_test.rs` - Parser tests with fixtures
- `src/ingest/ncbi_taxonomy/pipeline.rs` - Pipeline result tests
- `src/ingest/ncbi_taxonomy/version_discovery.rs` - Version bumping tests

Test fixtures:
- `tests/fixtures/ncbi/rankedlineage_sample.dmp`
- `tests/fixtures/ncbi/merged_sample.dmp`
- `tests/fixtures/ncbi/delnodes_sample.dmp`

### Integration Tests

Located in:
- `tests/ncbi_taxonomy_integration_test.rs`

Each test:
1. Creates a test organization
2. Runs operations against the database
3. Verifies results
4. Cleans up test data

## Test Scenarios

### 1. Basic Storage Test
Tests that taxonomy data is correctly stored in the database.

```bash
cargo test --test ncbi_taxonomy_integration_test test_storage_basic -- --ignored
```

Verifies:
- Registry entries created
- Data sources created with source_type='taxonomy'
- Taxonomy metadata stored
- Versions created
- Version files created (JSON + TSV)

### 2. Idempotency Test
Tests that re-running the same data doesn't create duplicates.

```bash
cargo test --test ncbi_taxonomy_integration_test test_storage_idempotency -- --ignored
```

Verifies:
- First run: 3 stored, 0 updated
- Second run: 0 stored, 3 updated
- Only 3 entries exist (not 6)

### 3. Multiple Versions Test
Tests that different versions can coexist.

```bash
cargo test --test ncbi_taxonomy_integration_test test_storage_multiple_versions -- --ignored
```

Verifies:
- Version 1.0 stored
- Version 1.1 stored (same data, different version)
- Both versions exist in database

### 4. Merged Taxa Test
Tests that merged taxa are marked with deprecation notes.

```bash
cargo test --test ncbi_taxonomy_integration_test test_merged_taxa_handling -- --ignored
```

Verifies:
- Merged taxon stored
- Lineage contains "[MERGED INTO {new_id}]"

### 5. Deleted Taxa Test
Tests that deleted taxa are marked with deprecation notes.

```bash
cargo test --test ncbi_taxonomy_integration_test test_deleted_taxa_handling -- --ignored
```

Verifies:
- Deleted taxon stored
- Lineage contains "[DELETED FROM NCBI]"

### 6. Version Files Test
Tests that JSON and TSV files are created for each taxonomy.

```bash
cargo test --test ncbi_taxonomy_integration_test test_version_files_creation -- --ignored
```

Verifies:
- 6 version files created (3 taxa × 2 formats)
- JSON format exists
- TSV format exists

## Running All Tests

```bash
# Run all tests (unit + integration)
./scripts/test/test_ncbi_taxonomy.sh --all --nocapture

# PowerShell
.\scripts\test\test_ncbi_taxonomy.ps1 -All -NoCapture
```

## Debugging Tests

### Show Test Output

```bash
cargo test --test ncbi_taxonomy_integration_test -- --ignored --nocapture
```

### Run Single Test

```bash
cargo test --test ncbi_taxonomy_integration_test test_storage_basic -- --ignored --nocapture
```

### Check Database State

```bash
# Connect to test database
psql $DATABASE_URL

# Query test data
SELECT * FROM organizations WHERE slug LIKE 'test-org-%';
SELECT * FROM registry_entries WHERE organization_id IN (SELECT id FROM organizations WHERE slug LIKE 'test-org-%');
SELECT * FROM taxonomy_metadata WHERE data_source_id IN (...);
```

## Cleanup

### After Tests

Tests clean up their own data, but if tests are interrupted:

```bash
# Connect to database
psql $DATABASE_URL

# Clean up test data
DELETE FROM organizations WHERE slug LIKE 'test-org-%';
```

### Stop Test Database

```bash
# If using Docker
docker stop bdp-test-db
docker rm bdp-test-db
```

## CI/CD Integration

For GitHub Actions or other CI systems:

```yaml
- name: Setup Test Database
  run: |
    docker run -d \
      --name bdp-test-db \
      -e POSTGRES_DB=bdp_test \
      -e POSTGRES_USER=bdp_test \
      -e POSTGRES_PASSWORD=test_password \
      -p 5433:5432 \
      postgres:15
    sleep 5

- name: Run Migrations
  env:
    DATABASE_URL: postgresql://bdp_test:test_password@localhost:5433/bdp_test
  run: cargo sqlx migrate run

- name: Run NCBI Taxonomy Tests
  env:
    DATABASE_URL: postgresql://bdp_test:test_password@localhost:5433/bdp_test
  run: ./scripts/test/test_ncbi_taxonomy.sh --all --nocapture

- name: Cleanup
  if: always()
  run: |
    docker stop bdp-test-db
    docker rm bdp-test-db
```

## Test Data

### Sample Taxa Used

The integration tests use sample data for these organisms:
- **Homo sapiens** (9606) - Human
- **Mus musculus** (10090) - Mouse
- **Drosophila melanogaster** (7227) - Fruit fly

Plus merged/deleted taxa for deprecation testing:
- Merged: 12345 → 9606
- Deleted: 99999

### Real Data Testing

To test with real NCBI data:

```rust
use bdp_server::ingest::ncbi_taxonomy::{
    NcbiTaxonomyFtpConfig,
    NcbiTaxonomyPipeline,
};

let config = NcbiTaxonomyFtpConfig::new()
    .with_parse_limit(100);  // Only process 100 entries for testing

let pipeline = NcbiTaxonomyPipeline::new(config, db_pool);
let result = pipeline.run(organization_id).await?;

println!("{}", result.summary());
```

## Troubleshooting

### Database Connection Failed

```bash
# Check database is running
docker ps

# Check DATABASE_URL is correct
echo $DATABASE_URL

# Test connection
psql $DATABASE_URL -c "SELECT 1;"
```

### Migrations Not Applied

```bash
# Run migrations
cargo sqlx migrate run

# Check migration status
cargo sqlx migrate info
```

### Tests Failing After Code Changes

```bash
# Rebuild and run tests
cargo clean
cargo build
cargo test --test ncbi_taxonomy_integration_test -- --ignored
```

### Permission Denied on Scripts

```bash
# Make scripts executable
chmod +x scripts/test/test_ncbi_taxonomy.sh
```

## Next Steps

After all tests pass:

1. ✅ Update UniProt code to use `taxonomy_metadata`
2. ✅ Run full ingestion with real NCBI data (parse limit for testing)
3. ✅ Test UniProt → Taxonomy FK relationships
4. ✅ Performance benchmarking
5. ✅ Production deployment

## Additional Resources

- [NCBI Taxonomy Status](NCBI_TAXONOMY_STATUS.md) - Implementation status and roadmap
- [NCBI Taxonomy Implementation](NCBI_TAXONOMY_IMPLEMENTATION.md) - Implementation plan
- [Test Scripts README](scripts/test/README.md) - All test scripts documentation
