# UniProt Ingestion - Getting Started

Quick guide to get the UniProt ingestion pipeline running.

## Prerequisites

1. **PostgreSQL 14+**
2. **Rust 1.75+**
3. **MinIO or S3** (optional, for file storage)

## Quick Start (5 minutes)

### 1. Setup Database

```bash
# Start PostgreSQL (using Docker)
docker run -d \
  --name bdp-postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=bdp \
  -p 5432:5432 \
  postgres:16-alpine

# Set environment variable
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/bdp
```

### 2. Run Migrations

```bash
cd crates/bdp-server
sqlx migrate run
```

### 3. Run Example

```bash
# Run the complete pipeline example
cargo run --example uniprot_ingestion

# Expected output:
# 🚀 Starting UniProt ingestion pipeline
# ✅ Organization ID: xxx
# ✅ Parsed 3 protein entries
# ✅ Stored 3 proteins
# ✅ Created aggregate source
# ✨ Pipeline completed successfully!
```

### 4. Verify Data

```sql
-- Check proteins
SELECT accession, protein_name, sequence_length
FROM protein_metadata;

-- Check version files (should have 3 formats per protein)
SELECT format, COUNT(*)
FROM version_files
GROUP BY format;

-- Check aggregate
SELECT re.slug, v.dependency_count
FROM registry_entries re
JOIN versions v ON v.entry_id = re.id
WHERE re.slug = 'uniprot-all';
```

## Test the Full Pipeline

```bash
# Run all E2E tests
cargo test --test e2e_parser_tests -- --nocapture

# Expected:
# test test_parse_ci_sample ... ok
# test test_parse_and_store ... ok
# test test_parse_invalid_data ... ok
# test result: ok. 3 passed
```

## Download Real Data (Optional)

### Current Release (Latest)

```rust
use bdp_server::ingest::uniprot::{UniProtFtp, UniProtFtpConfig, ReleaseType};

// Configure for current release
let config = UniProtFtpConfig::default()
    .with_release_type(ReleaseType::Current)
    .with_parse_limit(100);

let ftp = UniProtFtp::new(config);

// Download current release (no version needed)
let notes = ftp.download_release_notes(None).await?;
let dat_data = ftp.download_dat_file(None, None).await?;
```

### Previous Release (Historical)

```rust
use bdp_server::ingest::uniprot::{UniProtFtp, UniProtFtpConfig, ReleaseType};

// Configure for previous release
let config = UniProtFtpConfig::default()
    .with_release_type(ReleaseType::Previous);

let ftp = UniProtFtp::new(config);

// Download specific version
let notes = ftp.download_release_notes(Some("2024_01")).await?;
let dat_data = ftp.download_dat_file(Some("2024_01"), None).await?;
```

### TrEMBL Dataset (Unreviewed)

```rust
// Download TrEMBL instead of Swiss-Prot
let dat_data = ftp.download_dat_file(Some("2024_01"), Some("trembl")).await?;
```

### FTP Integration Tests

```bash
# Run FTP integration tests (requires network)
cargo test --test ftp_integration_tests -- --ignored --nocapture

# This will:
# - Connect to ftp.uniprot.org
# - Download release notes
# - Download and parse real DAT file (limited entries)
# - Test current and previous releases
# - Test Swiss-Prot and TrEMBL datasets
# - Store in test database
```

## With S3/MinIO

### Setup MinIO (Local S3-compatible storage)

```bash
# Start MinIO
docker run -d \
  --name bdp-minio \
  -p 9000:9000 \
  -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# Set environment variables
export S3_ENDPOINT=http://localhost:9000
export S3_BUCKET=bdp-data
export S3_ACCESS_KEY=minioadmin
export S3_SECRET_KEY=minioadmin
export S3_REGION=us-east-1
export S3_PATH_STYLE=true

# Create bucket (first time only)
# Visit http://localhost:9001 and create bucket "bdp-data"
```

### Use with Pipeline

```rust
use bdp_server::storage::{Storage, StorageConfig};
use bdp_server::ingest::uniprot::UniProtStorage;

// Initialize S3
let storage_config = StorageConfig::from_env()?;
let s3 = Storage::new(storage_config).await?;

// Use with UniProt storage
let storage = UniProtStorage::with_s3(
    db_pool,
    s3,
    org_id,
    "1.0".to_string(),
    "2024_01".to_string()
);

// Files will be uploaded to S3 automatically
storage.store_entries(&entries).await?;
```

## Common Issues

### Issue: "Failed to connect to database"

**Solution**: Check DATABASE_URL and ensure PostgreSQL is running

```bash
# Test connection
psql $DATABASE_URL -c "SELECT 1;"
```

### Issue: "Failed to run migrations"

**Solution**: Check migrations directory path

```bash
# From crates/bdp-server directory
sqlx migrate run --source ../../migrations
```

### Issue: "FTP download timeout"

**Solution**: Increase timeout in configuration

```rust
let config = UniProtFtpConfig::default()
    .with_connection_timeout(60)  // 60 seconds
    .with_read_timeout(600);      // 10 minutes
```

### Issue: "S3 upload failed"

**Solution**: Verify S3/MinIO credentials and endpoint

```bash
# Test S3 connection with AWS CLI
aws --endpoint-url $S3_ENDPOINT \
    s3 ls s3://$S3_BUCKET/

# Or use MinIO client
mc alias set local $S3_ENDPOINT $S3_ACCESS_KEY $S3_SECRET_KEY
mc ls local/$S3_BUCKET
```

## Performance Tuning

### For Large Datasets (100k+ proteins)

```rust
// Increase database connection pool
let db_pool = PgPoolOptions::new()
    .max_connections(20)  // Default: 10
    .connect(&database_url)
    .await?;

// Process in batches
for chunk in entries.chunks(1000) {
    storage.store_entries(chunk).await?;
}
```

### Parallel Processing

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

for chunk in entries.chunks(100) {
    let storage = storage.clone();
    let chunk = chunk.to_vec();

    set.spawn(async move {
        storage.store_entries(&chunk).await
    });
}

// Wait for all tasks
while let Some(result) = set.join_next().await {
    result??;
}
```

## Development Workflow

```bash
# 1. Make changes to code
# 2. Run tests
cargo test --test e2e_parser_tests

# 3. Run example
cargo run --example uniprot_ingestion

# 4. Check database
psql $DATABASE_URL -c "SELECT COUNT(*) FROM protein_metadata;"
```

## Next Steps

- Read [PIPELINE_COMPLETE.md](./PIPELINE_COMPLETE.md) for full feature documentation
- Check [README.md](./README.md) for API documentation
- See `examples/uniprot_ingestion.rs` for complete code example
- Run FTP integration tests for real data download

## Production Deployment

For production deployment guidance, see:
- [../../../docs/development/](../../../docs/development/) - Development guides
- [../../../SETUP.md](../../../SETUP.md) - Setup instructions
- [../../../docker-compose.yml](../../../docker-compose.yml) - Docker setup

## Support

- **Issues**: https://github.com/yourusername/bdp/issues
- **Docs**: See `docs/` directory
- **Tests**: Run with `--nocapture` for detailed output
