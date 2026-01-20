# GenBank/RefSeq Testing Guide

Complete guide for testing the GenBank/RefSeq ingestion pipeline both locally and in Docker.

## Quick Test Summary

| Test Type | Duration | Purpose | Command |
|-----------|----------|---------|---------|
| Parser Unit Tests | 1 min | Verify parser logic | `cargo test genbank` |
| Integration Tests | 2 min | Full parsing validation | `cargo test --test genbank_integration_test` |
| Phage Division (Local) | 2-5 min | Real FTP + DB + S3 | `cargo run --bin genbank_test_phage` |
| Phage Division (Docker) | 3-7 min | Full stack test | `docker-compose exec bdp-server cargo run --bin genbank_test_phage` |

## Prerequisites

### Local Testing
```bash
# Rust and Cargo
rustc --version  # Should be 1.70+

# PostgreSQL
psql --version

# Environment variables
export DATABASE_URL="postgresql://user:pass@localhost:5432/bdp"
export S3_BUCKET="bdp-sequences"
export AWS_REGION="us-east-1"
# For local MinIO:
export S3_ENDPOINT="http://localhost:9000"
export AWS_ACCESS_KEY_ID="minioadmin"
export AWS_SECRET_ACCESS_KEY="minioadmin"
```

### Docker Testing
```bash
# Docker and Docker Compose
docker --version
docker-compose --version

# Start services
docker-compose up -d
```

## Test Phases

### Phase 1: Parser Unit Tests (No Database Required)

**Purpose**: Verify GenBank flat file parsing logic

```bash
cd crates/bdp-server

# Run all GenBank parser tests
cargo test genbank_parser --lib

# Run with output
cargo test genbank_parser --lib -- --nocapture

# Run specific test
cargo test test_parse_sample_genbank_file --lib
```

**Tests Included** (5 total):
1. `test_parse_location` - Location string parsing (190..255, complement, join)
2. `test_calculate_gc_content` - GC% calculation accuracy
3. `test_calculate_hash` - SHA256 hash generation
4. `test_infer_division` - Division code to enum mapping
5. `test_parse_sample_genbank_file` - Complete file parsing

**Expected Output**:
```
running 5 tests
test genbank::parser::tests::test_parse_location ... ok
test genbank::parser::tests::test_calculate_gc_content ... ok
test genbank::parser::tests::test_calculate_hash ... ok
test genbank::parser::tests::test_infer_division ... ok
test genbank_parser_test::test_parse_sample_genbank_file ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Phase 2: Integration Tests (No Database Required)

**Purpose**: Verify complete parsing with fixtures

```bash
cd crates/bdp-server

# Run all integration tests
cargo test --test genbank_integration_test

# Run with details
cargo test --test genbank_integration_test -- --nocapture --test-threads=1
```

**Tests Included** (15 total):
1. Complete file parsing
2. Parse with limit
3. Extraction methods
4. S3 key generation
5. FASTA format validation
6. Config builder pattern
7. Division file patterns
8. GenBank vs RefSeq paths
9. All divisions available
10. Test division is phage
11. Parser performance
12. GC content accuracy
13. Hash determinism
14. Different hashes for different sequences
15. Model serialization

**Expected Output**:
```
running 15 tests
test test_parse_sample_file_complete ... ok
test test_parse_with_limit ... ok
test test_extraction_methods ... ok
test test_s3_key_generation ... ok
test test_fasta_format ... ok
... (all tests pass)

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured
```

### Phase 3: Database Migration

**Purpose**: Create sequence tables

**Local**:
```bash
cd crates/bdp-server

# Check current migration status
sqlx migrate info

# Run migration
sqlx migrate run

# Verify tables created
psql $DATABASE_URL -c "\dt" | grep sequence
```

**Docker**:
```bash
# Migration runs automatically on container start
# Or run manually:
docker-compose exec bdp-server sqlx migrate run

# Verify tables
docker-compose exec postgres psql -U bdp -d bdp -c "\dt" | grep sequence
```

**Expected Tables**:
- `sequence_metadata` - 19 columns, 7 indexes
- `sequence_protein_mappings` - 9 columns, 3 indexes

### Phase 4: Phage Division Test (Local)

**Purpose**: Real-world ingestion test with NCBI FTP, PostgreSQL, and S3

**Setup**:
```bash
cd crates/bdp-server

# Ensure environment variables are set
cat .env

# Verify database connection
psql $DATABASE_URL -c "SELECT 1"

# Verify S3 access
aws --endpoint-url=$S3_ENDPOINT s3 ls s3://$S3_BUCKET
```

**Run Test**:
```bash
# Run phage test (1,000 records limit)
cargo run --bin genbank_test_phage

# Monitor progress
cargo run --bin genbank_test_phage 2>&1 | tee genbank_test.log
```

**Expected Output**:
```
=== GenBank Phage Division Test ===
Connected to database
Initialized S3 storage
Using organization: <uuid>

GenBank configuration:
  Source: GenBank
  Division: Phage (test)
  Parse limit: 1000 records
  Batch size: 500

Starting phage division ingestion...
Downloading gbphg1.seq.gz
Downloaded gbphg1.seq.gz (18,234,567 bytes)
Parsing file: gbphg1.seq.gz (18,234,567 bytes)
Parsed 1,000 records from gbphg1.seq.gz

Processing chunk 1 / 2 (500 records)
Created 500 data_sources entries
Inserted 500 sequence_metadata entries
Uploaded 500 FASTA files (2.3 MB bytes)
Found 234 protein data sources for 500 protein IDs
Created 234 protein mappings
Chunk 1 complete: 500 records, 2345678 bytes uploaded, 234 mappings

Processing chunk 2 / 2 (500 records)
... (similar output)

=== GenBank Ingestion Complete ===
Release: 259
Division: phage
Records processed: 1000
Sequences inserted: 950
Protein mappings: 456
Bytes uploaded: 4.5 MB
Duration: 45.23 seconds
Throughput: 22 records/second

Verifying stored data...
Database verification:
  Expected sequences: 950
  Actual sequences: 950
  ✓ Data verified

Sample records:
  NC_001416.1 - Enterobacteria phage lambda, complete genome (5386bp, 49.5% GC, div: phage)
  ... (4 more samples)

=== Test Successful ===
```

**Duration**: 2-5 minutes (depending on network speed)

**Verify Results**:
```bash
# Count sequences
psql $DATABASE_URL -c "SELECT COUNT(*) FROM sequence_metadata;"

# Check divisions
psql $DATABASE_URL -c "SELECT division, COUNT(*) FROM sequence_metadata GROUP BY division;"

# Sample sequences
psql $DATABASE_URL -c "SELECT accession_version, sequence_length, gc_content FROM sequence_metadata LIMIT 5;"

# Check protein mappings
psql $DATABASE_URL -c "SELECT COUNT(*) FROM sequence_protein_mappings;"

# Check S3 files
aws --endpoint-url=$S3_ENDPOINT s3 ls s3://$S3_BUCKET/genbank/release-259/phage/ | head -10
```

### Phase 5: Phage Division Test (Docker)

**Purpose**: Full stack integration test in containerized environment

**Setup**:
```bash
# Ensure Docker services are running
docker-compose ps

# Should show: postgres, minio, bdp-server running
```

**Run Test**:
```bash
# Execute test inside container
docker-compose exec bdp-server cargo run --bin genbank_test_phage

# Or with logs
docker-compose exec bdp-server cargo run --bin genbank_test_phage 2>&1 | tee genbank_docker_test.log
```

**Expected Output**: Same as local test

**Verify Results**:
```bash
# Check database
docker-compose exec postgres psql -U bdp -d bdp -c "SELECT COUNT(*) FROM sequence_metadata;"

# Check MinIO
# Open browser: http://localhost:9001
# Login: minioadmin / minioadmin
# Navigate to: bdp-sequences/genbank/release-259/phage/
```

**Duration**: 3-7 minutes (includes container overhead)

### Phase 6: Larger Division Test (Optional)

**Purpose**: Test with larger dataset

**Remove Parse Limit**:
```rust
// Edit src/bin/genbank_test_phage.rs
// Change line 55:
.with_parse_limit(1000)  // Remove or set to None
// To:
.with_parse_limit(None)  // Process all records
```

**Rebuild and Run**:
```bash
cargo build --bin genbank_test_phage
cargo run --bin genbank_test_phage
```

**Expected**:
- Records: ~50,000 (full phage division)
- Duration: 10-15 minutes
- Memory: ~500MB
- Storage: ~100MB in S3

### Phase 7: Parallel Processing Test (Advanced)

**Purpose**: Test orchestrator with multiple divisions

**Create Test Script**:
```bash
# scripts/test-genbank-parallel.sh
#!/bin/bash
cd crates/bdp-server

cargo run --release <<EOF
use bdp_server::ingest::genbank::{GenbankFtpConfig, GenbankOrchestrator};
use bdp_server::storage::config::StorageConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = GenbankFtpConfig::new()
        .with_genbank()
        .with_parse_limit(5000)  // 5k per division
        .with_concurrency(4);     // 4 divisions parallel

    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    let storage = bdp_server::storage::Storage::new(StorageConfig::from_env()?).await?;

    let orchestrator = GenbankOrchestrator::new(config, db, storage);

    // Test with phage, viral, mammalian, primate
    let divisions = vec![
        Division::Phage,
        Division::Viral,
        Division::Mammalian,
        Division::Primate,
    ];

    let result = orchestrator.run_divisions(org_id, &divisions, None).await?;

    println!("Parallel test complete:");
    println!("  Divisions: {}", result.divisions_processed);
    println!("  Total records: {}", result.total_records);
    println!("  Duration: {:.2}s", result.duration_seconds);

    Ok(())
}
EOF
```

## Troubleshooting

### Test Failures

#### 1. Parser Tests Fail
```bash
# Error: Cannot read fixture file
# Solution: Run from correct directory
cd crates/bdp-server
cargo test genbank_parser
```

#### 2. Database Connection Error
```bash
# Error: "connection to server failed"
# Check: Is PostgreSQL running?
pg_isready -h localhost -p 5432

# Check: Are credentials correct?
psql $DATABASE_URL -c "SELECT 1"

# Docker: Check container
docker-compose ps postgres
docker-compose logs postgres
```

#### 3. S3 Connection Error
```bash
# Error: "bucket not found"
# Create bucket:
aws --endpoint-url=$S3_ENDPOINT s3 mb s3://bdp-sequences

# Check MinIO running (Docker):
docker-compose ps minio
curl http://localhost:9000/minio/health/live
```

#### 4. FTP Timeout
```bash
# Error: "connection timeout" or "read timeout"
# Solution: Increase timeout in config
.with_timeout(600)  // 10 minutes

# Check: Can reach NCBI FTP?
curl ftp://ftp.ncbi.nlm.nih.gov/genbank/GB_Release_Number
```

#### 5. Migration Already Run
```bash
# Error: "relation already exists"
# This is OK - tables already created
# To reset:
sqlx migrate revert  # Revert last migration
sqlx migrate run     # Re-run
```

## Test Data

### Fixture Files

**Location**: `tests/fixtures/genbank/sample.gbk`

**Contents**:
- 1 complete GenBank record
- Enterobacteria phage lambda
- 5,386 bp
- 2 CDS features with protein_ids
- Complete FEATURES and ORIGIN sections

**Add More Fixtures**:
```bash
# Download real GenBank file
curl "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=nuccore&id=NC_001416&rettype=gb&retmode=text" \
  > tests/fixtures/genbank/lambda_phage.gbk

# Or create custom fixture
cat > tests/fixtures/genbank/minimal.gbk <<EOF
LOCUS       TEST001                 100 bp    DNA     linear   PHG 01-JAN-2026
DEFINITION  Test sequence.
ACCESSION   TEST001
VERSION     TEST001.1
FEATURES             Location/Qualifiers
     source          1..100
                     /organism="Test organism"
                     /db_xref="taxon:12345"
ORIGIN
        1 atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc
       61 atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc
//
EOF
```

## Performance Benchmarks

### Expected Performance (Local)

| Metric | Phage (1K) | Phage (Full) | Viral (100K) |
|--------|------------|--------------|--------------|
| Parse time | <5s | 30-60s | 5-10min |
| DB insert | <2s | 10-20s | 2-4min |
| S3 upload | <3s | 20-40s | 5-10min |
| Total time | 2-5min | 10-15min | 30-45min |
| Memory peak | ~200MB | ~500MB | ~2GB |

### Expected Performance (Docker)

Add ~20-30% overhead for Docker virtualization.

## CI/CD Integration

### GitHub Actions Example

```yaml
name: GenBank Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
      minio:
        image: minio/minio
        env:
          MINIO_ROOT_USER: minioadmin
          MINIO_ROOT_PASSWORD: minioadmin
        options: >-
          --health-cmd "curl -f http://localhost:9000/minio/health/live"

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run parser tests
        run: cargo test genbank_parser

      - name: Run integration tests
        run: cargo test --test genbank_integration_test

      # Skip full ingestion test in CI (too slow)
```

## Next Steps After Testing

1. ✅ Parser tests pass → Parser is correct
2. ✅ Integration tests pass → Complete parsing works
3. ✅ Phage test succeeds → Full pipeline works
4. 📊 Monitor performance → Identify bottlenecks
5. 🚀 Test larger divisions → Scale validation
6. 🔄 Enable continuous testing → CI/CD integration

## Resources

- **Implementation**: `GENBANK_IMPLEMENTATION_SUMMARY.md`
- **Optimization**: `GENBANK_OPTIMIZATION_ANALYSIS.md`
- **Quick Start**: `GENBANK_QUICK_START.md`
- **Design**: `GENBANK_REFSEQ_DESIGN.md`
