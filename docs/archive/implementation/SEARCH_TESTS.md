# Search Optimization Tests

Comprehensive test suite for validating the search performance optimizations.

## Test Categories

### 1. Unit Tests (`src/features/search/queries/*_tests.rs`)

Basic functionality tests for search queries:
- Query validation
- Parameter parsing
- Filter logic
- Error handling

**Run:**
```bash
cargo test --package bdp-server --lib features::search::queries
```

### 2. Integration Tests (`tests/search_integration_tests.rs`)

End-to-end tests verifying search functionality with real database:

**Test Coverage:**
- ✅ Basic search queries
- ✅ Type filters (data_source, tool, organization)
- ✅ Source type filters (protein, genome, etc.)
- ✅ Organism filtering (scientific and common names)
- ✅ Format filtering (fasta, json, xml)
- ✅ Pagination
- ✅ Result ranking
- ✅ Pre-computed fields (latest_version, formats, downloads)
- ✅ Autocomplete/suggestions
- ✅ Materialized view refresh
- ✅ Combined filters

**Run:**
```bash
cargo test --package bdp-server --test search_integration_tests -- --nocapture
```

**Expected Output:**
```
running 13 tests
test test_search_basic_query ... ok
test test_search_with_type_filter ... ok
test test_search_with_source_type_filter ... ok
test test_search_with_organism_filter ... ok
test test_search_with_format_filter ... ok
test test_search_pagination ... ok
test test_search_ranking ... ok
test test_search_precomputed_fields ... ok
test test_suggestions_basic ... ok
test test_suggestions_with_filters ... ok
test test_suggestions_limit ... ok
test test_materialized_view_refresh ... ok
test test_combined_filters ... ok

test result: ok. 13 passed; 0 failed
```

### 3. Load Tests (`tests/search_load_tests.rs`)

Concurrent user simulation tests measuring scalability:

**Test Scenarios:**
- **Concurrent Searches**: 100 users × 10 queries each (1000 total queries)
- **Concurrent Suggestions**: 50 users × 20 queries each (1000 autocomplete queries)
- **Search During MV Refresh**: Verify no blocking during concurrent refresh
- **Sustained Load**: 50 users for 60 seconds continuous load

**Metrics Collected:**
- Successful queries
- Failed queries
- Response times (min, p50, p95, p99, max)
- Throughput (queries/sec)

**Run:**
```bash
# Run all load tests
cargo test --package bdp-server --test search_load_tests -- --ignored --nocapture --test-threads=1

# Run specific load test
cargo test --package bdp-server --test search_load_tests test_concurrent_searches -- --ignored --nocapture
```

**Expected Output:**
```
🚀 Starting load test: 100 concurrent users, 10 queries each
Creating 10000 test entries for load testing...
Refreshing materialized view...
MV refresh took 2.34s

Total test duration: 15.3s
Throughput: 65.36 queries/sec

=== Concurrent Search Load Test Results ===
Successful: 1000
Failed: 0
Total time: 15.3s
Avg: 152ms
Min: 23ms
p50: 145ms
p95: 287ms
p99: 341ms
Max: 456ms
```

**Performance Assertions:**
- ✅ Less than 1% failure rate
- ✅ p95 latency < 500ms
- ✅ p95 suggestions latency < 100ms
- ✅ No failures during concurrent MV refresh

### 4. Performance Benchmarks (`benches/search_performance.rs`)

Detailed performance measurements using Criterion:

**Benchmark Groups:**
1. **Simple Queries**: Search with different dataset sizes (100, 1K, 10K)
2. **Filtered Searches**: Type, source type, organism, format, combined filters
3. **Suggestions**: Short queries, longer queries, with filters
4. **Pagination**: Different page numbers (1, 10, 50, 100)
5. **MV Refresh**: Concurrent vs non-concurrent refresh

**Run:**
```bash
# Run all benchmarks
cargo bench --bench search_performance

# Run specific benchmark
cargo bench --bench search_performance search_simple_query
```

**Output:**
- Statistical analysis (mean, std dev, median)
- Performance graphs (HTML reports)
- Comparison with previous runs
- Regression detection

**Reports Location:**
```
target/criterion/report/index.html
```

## Test Data

### Integration Tests
- 3 organizations (test-org, uniprot, ncbi)
- 1 taxonomy entry (Homo sapiens)
- 5 protein data sources
- 1 genome data source
- 1 tool (BLAST)
- Multiple versions and file formats per source

### Load Tests
- Configurable entry count (default: 10,000)
- Batch insertion for performance
- Realistic data distribution
- Pre-populated materialized view

### Benchmarks
- Configurable dataset sizes
- Batch creation optimized
- Includes MV refresh in setup

## Quick Start

### Run All Tests (Quick Mode)
```bash
# Linux/Mac
./scripts/run_search_tests.sh --quick

# Windows
.\scripts\run_search_tests.ps1 -Quick
```

### Run Specific Test Category
```bash
# Unit tests only
./scripts/run_search_tests.sh --unit

# Integration tests only
./scripts/run_search_tests.sh --integration

# Load tests only
./scripts/run_search_tests.sh --load

# Benchmarks only
./scripts/run_search_tests.sh --bench
```

### Run Full Test Suite
```bash
# This will take 30-60 minutes
./scripts/run_search_tests.sh --full

# Or on Windows
.\scripts\run_search_tests.ps1 -Full
```

## Prerequisites

### Required
1. **PostgreSQL running** with configured DATABASE_URL
2. **Migrations applied**: `sqlx migrate run`
3. **Test database** (separate from production)

### Optional for Benchmarks
4. **Criterion installed** (automatically installed as dev-dependency)
5. **Sufficient disk space** for benchmark reports (~100MB)

### Environment Setup
```bash
# Set database URL
export DATABASE_URL="postgresql://user:pass@localhost:5432/bdp_test"

# Run migrations
sqlx migrate run

# Verify connection
psql $DATABASE_URL -c "SELECT COUNT(*) FROM search_registry_entries_mv"
```

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Search Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run migrations
        run: |
          cargo install sqlx-cli --features postgres
          sqlx migrate run
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/bdp

      - name: Run unit tests
        run: cargo test --lib features::search::queries
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/bdp

      - name: Run integration tests
        run: cargo test --test search_integration_tests
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/bdp

      - name: Run load tests (quick)
        run: cargo test --test search_load_tests test_concurrent_searches -- --ignored
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/bdp
```

## Troubleshooting

### Tests Fail with "relation does not exist"
```bash
# Run migrations
sqlx migrate run

# Verify materialized view exists
psql $DATABASE_URL -c "\d search_registry_entries_mv"
```

### Load Tests Timeout
```bash
# Increase PostgreSQL connection limits
psql $DATABASE_URL -c "ALTER SYSTEM SET max_connections = 200"
psql $DATABASE_URL -c "SELECT pg_reload_conf()"

# Or run with fewer concurrent users (edit test file)
```

### Benchmarks Take Too Long
```bash
# Run quick benchmarks
cargo bench --bench search_performance -- --sample-size 10

# Or benchmark specific functions
cargo bench --bench search_performance search_simple_query
```

### Out of Memory During Tests
```bash
# Increase PostgreSQL memory settings
psql $DATABASE_URL -c "ALTER SYSTEM SET work_mem = '256MB'"
psql $DATABASE_URL -c "ALTER SYSTEM SET shared_buffers = '2GB'"
psql $DATABASE_URL -c "SELECT pg_reload_conf()"
```

## Performance Targets

### Unit & Integration Tests
- ✅ All tests pass
- ✅ No flaky tests
- ✅ Complete within 5 minutes

### Load Tests
- ✅ 0% failure rate
- ✅ p95 < 500ms for searches
- ✅ p95 < 100ms for suggestions
- ✅ Throughput > 50 queries/sec

### Benchmarks
- ✅ Search < 100ms for 1M entries
- ✅ Suggestions < 50ms
- ✅ Pagination constant time
- ✅ MV refresh < 5min for 100K entries

## Continuous Monitoring

### Daily Benchmark Runs
```bash
# Run benchmarks nightly and save results
cargo bench --bench search_performance > benchmark_results_$(date +%Y%m%d).txt

# Compare with baseline
cargo bench --bench search_performance --baseline main
```

### Production Metrics
Monitor these in production:
- Search query latency (p50, p95, p99)
- Autocomplete latency
- MV refresh duration
- Database connection pool usage
- Cache hit rates

## Contributing

When adding new search features:

1. **Add unit tests** for query logic
2. **Add integration test** for end-to-end verification
3. **Update load tests** if concurrency behavior changes
4. **Add benchmark** if performance-critical
5. **Run full test suite** before submitting PR

```bash
# Before submitting PR
./scripts/run_search_tests.sh --full
```

## References

- [Search Performance Optimization Guide](./search-performance-optimization.md)
- [Search Optimization Summary](./SEARCH_OPTIMIZATION_SUMMARY.md)
- [Criterion Documentation](https://bheisler.github.io/criterion.rs/book/)
- [sqlx Testing Guide](https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-do-database-tests)
