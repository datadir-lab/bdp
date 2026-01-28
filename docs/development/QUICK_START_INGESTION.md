# Quick Start: UniProt Protein Ingestion

## 🚀 Quick Test (Verified Working)

```bash
# Run the comprehensive test
cd /path/to/bdp
cargo run --package bdp-server --example test_uniprot_ingestion

# Expected output:
# === UniProt Ingestion System Test ===
# 1. Connecting to database... ✓
# 2. Setting up test organization... ✓
# 3. Testing configuration system... ✓
# 4. Testing version discovery... ✓
# 5. Testing mode selection... ✓
# === All Tests Passed! ===
```

## 🎯 Production Usage

### Option 1: Latest Mode (Recommended for Production)

**Purpose**: Keep database up-to-date with newest UniProt releases

```bash
# In docker-compose.yml or .env file:
INGEST_ENABLED=true
INGEST_UNIPROT_MODE=latest
INGEST_UNIPROT_CHECK_INTERVAL_SECS=86400  # Check daily
INGEST_UNIPROT_AUTO_INGEST=false          # Manual trigger
INGEST_UNIPROT_IGNORE_BEFORE=2024_01      # Skip versions before 2024_01

# Start server
docker compose up -d bdp-server
```

### Option 2: Historical Mode (For Initial Backfill)

**Purpose**: Backfill multiple historical versions

```bash
# In docker-compose.yml or .env file:
INGEST_ENABLED=true
INGEST_UNIPROT_MODE=historical
INGEST_UNIPROT_HISTORICAL_START=2020_01        # Start version
INGEST_UNIPROT_HISTORICAL_END=2024_12          # End version (optional)
INGEST_UNIPROT_HISTORICAL_BATCH_SIZE=3         # Process 3 at a time
INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING=true   # Skip if already ingested

# Start server
docker compose up -d bdp-server
```

## 📊 Verify Ingestion

### Check Database Tables

```sql
-- Check ingestion jobs
SELECT id, job_type, external_version, status, records_processed
FROM ingestion_jobs
ORDER BY created_at DESC
LIMIT 10;

-- Check organization sync status
SELECT organization_id, last_external_version, last_sync_at, status
FROM organization_sync_status;

-- Check versions
SELECT external_version, release_date, size_bytes
FROM versions
WHERE external_version LIKE '202%'
ORDER BY release_date DESC;

-- Check proteins ingested
SELECT COUNT(*) as protein_count FROM protein_metadata;
```

### Check Logs

```bash
# Docker logs
docker compose logs -f bdp-server | grep -i uniprot

# Look for:
# "Running UniProt ingestion in LATEST mode"
# "Running UniProt ingestion in HISTORICAL mode"
# "Successfully ingested version 2025_01"
```

## 🔧 Configuration Reference

### Environment Variables

| Variable | Mode | Default | Description |
|----------|------|---------|-------------|
| `INGEST_ENABLED` | Both | `false` | Enable ingestion system |
| `INGEST_UNIPROT_MODE` | Both | `latest` | Mode: `latest` or `historical` |
| `INGEST_UNIPROT_CHECK_INTERVAL_SECS` | Latest | `86400` | Check interval (seconds) |
| `INGEST_UNIPROT_AUTO_INGEST` | Latest | `false` | Auto-ingest when new version found |
| `INGEST_UNIPROT_IGNORE_BEFORE` | Latest | - | Ignore versions before (YYYY_MM) |
| `INGEST_UNIPROT_HISTORICAL_START` | Historical | `2020_01` | Start version (YYYY_MM) |
| `INGEST_UNIPROT_HISTORICAL_END` | Historical | - | End version (YYYY_MM, optional) |
| `INGEST_UNIPROT_HISTORICAL_BATCH_SIZE` | Historical | `3` | Versions per batch |
| `INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING` | Historical | `true` | Skip existing versions |

### FTP Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `INGEST_UNIPROT_FTP_HOST` | `ftp.uniprot.org` | FTP server |
| `INGEST_UNIPROT_FTP_PATH` | `/pub/databases/uniprot/current_release/knowledgebase/complete` | FTP path |
| `INGEST_UNIPROT_FTP_TIMEOUT_SECS` | `300` | Connection timeout |
| `INGEST_UNIPROT_BATCH_SIZE` | `1000` | Entries per batch |

## 📝 Example Scenarios

### Scenario 1: Production Deployment (Keep Up-to-Date)

```bash
INGEST_ENABLED=true
INGEST_UNIPROT_MODE=latest
INGEST_UNIPROT_CHECK_INTERVAL_SECS=86400  # Check daily
INGEST_UNIPROT_AUTO_INGEST=false          # Manual approval
```

**Workflow**:
1. System checks for new versions daily
2. Logs when newer version detected
3. Admin manually triggers ingestion
4. Only latest version ingested

### Scenario 2: Initial Database Population

```bash
INGEST_ENABLED=true
INGEST_UNIPROT_MODE=historical
INGEST_UNIPROT_HISTORICAL_START=2023_01
INGEST_UNIPROT_HISTORICAL_END=2025_01
INGEST_UNIPROT_HISTORICAL_BATCH_SIZE=2    # Start small
```

**Workflow**:
1. Discovers all versions from 2023_01 to 2025_01
2. Filters out any already ingested (if skip_existing=true)
3. Processes 2 versions, then pauses
4. Continues until all versions ingested

### Scenario 3: Testing with Small Sample

```bash
INGEST_ENABLED=true
INGEST_UNIPROT_MODE=latest
INGEST_UNIPROT_PARSE_LIMIT=100           # Only parse 100 entries
```

**Result**: Downloads full file but only parses/stores 100 proteins for testing

## 🛠️ Troubleshooting

### Problem: No Ingestion Happening

**Check**:
```bash
# 1. Is ingestion enabled?
echo $INGEST_ENABLED  # Should be 'true'

# 2. Check logs
docker compose logs bdp-server | grep -i "ingest"

# 3. Verify database connectivity
docker compose exec postgres psql -U bdp -d bdp -c "SELECT COUNT(*) FROM organizations;"
```

### Problem: "Version already exists"

**Solution**: This is expected behavior! The system prevents duplicate ingestion.

To re-ingest:
```sql
-- Delete existing version
DELETE FROM versions WHERE external_version = '2025_01';

-- Or set skip_existing=false in historical mode
INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING=false
```

### Problem: Migration Issues in Docker

**Solution**: Update the .sqlx offline cache or run locally:
```bash
# Option 1: Update cache
cargo sqlx prepare --database-url postgresql://bdp:bdp_dev_password@localhost:5432/bdp

# Option 2: Run locally instead of Docker
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"
cargo run --package bdp-server
```

## ✅ Success Indicators

**Ingestion Working If You See**:
- ✅ Logs: "Running UniProt ingestion in [MODE] mode"
- ✅ Logs: "Successfully ingested version X"
- ✅ Database: Rows in `ingestion_jobs` table
- ✅ Database: Rows in `protein_metadata` table
- ✅ Database: `organization_sync_status` updated

## 🎯 Next Steps

1. **Run the test**: `cargo run --package bdp-server --example test_uniprot_ingestion`
2. **Choose mode**: Latest (production) or Historical (backfill)
3. **Set environment**: Update docker-compose.yml
4. **Start server**: `docker compose up -d bdp-server`
5. **Monitor**: Watch logs and database tables

For detailed implementation info, see `UNIPROT_INGESTION_COMPLETE.md`
