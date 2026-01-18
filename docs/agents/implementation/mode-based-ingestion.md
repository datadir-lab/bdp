# Mode-Based Ingestion Implementation

**Date**: 2026-01-18
**Module**: `crates/bdp-server/src/ingest/uniprot/idempotent_pipeline.rs`

## Overview

Implemented mode-based execution methods for the UniProt ingestion pipeline, enabling two distinct ingestion strategies:

1. **Latest Mode**: Ingest only the newest available version (incremental updates)
2. **Historical Mode**: Backfill multiple versions within a specified range (batch processing)

## Implementation Details

### 1. Public API Methods

#### `run_with_mode(&self, config: &UniProtConfig) -> Result<IngestStats>`

Main dispatcher that routes to the appropriate mode handler based on configuration.

**Behavior**:
- Matches on `config.ingestion_mode`
- Calls `run_latest_mode()` for `IngestionMode::Latest`
- Calls `run_historical_mode()` for `IngestionMode::Historical`

**Usage**:
```rust
let pipeline = IdempotentUniProtPipeline::new(pool, org_id, config, batch_config, storage);
let stats = pipeline.run_with_mode(&uniprot_config).await?;
```

---

#### `run_latest_mode(&self, config: &LatestConfig) -> Result<IngestStats>`

Ingests only the newest available version if it's newer than the last ingested version.

**Algorithm**:
1. Use `VersionDiscovery.check_for_newer_version()` to find newer version
2. Apply `ignore_before` filter if configured
3. If newer version available:
   - Set `is_current = true` in version metadata
   - Ingest the version using existing pipeline
   - Return statistics with version info
4. If up-to-date:
   - Return empty stats (no-op)

**Configuration Parameters**:
- `check_interval_secs`: How often to check for updates (default: 86400 = daily)
- `auto_ingest`: Whether to auto-ingest when newer version detected (default: false)
- `ignore_before`: Skip versions older than this date (format: YYYY_MM)

**Example Config**:
```rust
LatestConfig {
    check_interval_secs: 86400,
    auto_ingest: true,
    ignore_before: Some("2024_01".to_string()),
}
```

**Metadata Storage**:
- Stores `is_current: true` in `ingestion_jobs.source_metadata`
- Enables distinguishing "current" vs "historical" versions

---

#### `run_historical_mode(&self, config: &HistoricalConfig) -> Result<IngestStats>`

Backfills multiple versions within a specified range, processing them sequentially in batches.

**Algorithm**:
1. Use `VersionDiscovery.discover_all_versions()` to get all available versions
2. Filter by `start_version..end_version` range
3. If `skip_existing = true`, exclude already-ingested versions
4. Process versions in chunks (size from `batch_size`)
5. For each version:
   - Set `is_current = false` in version metadata
   - Ingest using existing pipeline
   - Merge statistics
6. Return aggregated stats from all versions

**Configuration Parameters**:
- `start_version`: Beginning of version range (e.g., "2020_01")
- `end_version`: End of version range (None = all available)
- `batch_size`: Number of versions to process sequentially (default: 3)
- `skip_existing`: Skip versions already in database (default: true)

**Example Config**:
```rust
HistoricalConfig {
    start_version: "2024_01".to_string(),
    end_version: Some("2024_12".to_string()),
    batch_size: 5,
    skip_existing: true,
}
```

**Metadata Storage**:
- Stores `is_current: false` in `ingestion_jobs.source_metadata`
- Enables identifying historical backfill versions

---

### 2. Helper Methods

#### `get_job_stats(&self, job_id: Uuid) -> Result<IngestStats>`

Retrieves ingestion statistics from a completed job in the database.

**Query**:
```sql
SELECT records_processed, records_stored, records_failed
FROM ingestion_jobs
WHERE id = $1
```

**Mapping**:
- `total_entries` ← `records_processed`
- `entries_inserted` ← `records_stored`
- `entries_failed` ← `records_failed`
- `entries_updated` = 0 (UniProt uses UPSERT, not tracked separately)

---

### 3. Existing Method Integration

The existing `ingest_version(&self, version: &DiscoveredVersion) -> Result<Uuid>` method already:

1. ✅ Stores `is_current` in `source_metadata` JSONB field
2. ✅ Stores `release_date` in `source_metadata`
3. ✅ Stores `ftp_path` in `source_metadata`

**Code Reference** (lines 487-491):
```rust
source_metadata: Some(serde_json::json!({
    "is_current": version.is_current,
    "release_date": version.release_date.to_string(),
    "ftp_path": version.ftp_path,
})),
```

No changes were needed to the existing ingestion pipeline.

---

## Usage Examples

### Latest Mode (Incremental Updates)

```rust
use crate::ingest::config::{IngestionMode, LatestConfig, UniProtConfig};

let uniprot_config = UniProtConfig {
    ingestion_mode: IngestionMode::Latest(LatestConfig {
        check_interval_secs: 86400, // Check daily
        auto_ingest: true,
        ignore_before: Some("2024_01".to_string()),
    }),
    // ... other fields
};

let pipeline = IdempotentUniProtPipeline::new(
    pool,
    org_id,
    ftp_config,
    batch_config,
    storage
);

let stats = pipeline.run_with_mode(&uniprot_config).await?;

if stats.total_entries > 0 {
    println!("Ingested {} entries from version {:?}",
        stats.total_entries,
        stats.version_synced
    );
} else {
    println!("Already up-to-date");
}
```

### Historical Mode (Backfill)

```rust
use crate::ingest::config::{IngestionMode, HistoricalConfig, UniProtConfig};

let uniprot_config = UniProtConfig {
    ingestion_mode: IngestionMode::Historical(HistoricalConfig {
        start_version: "2023_01".to_string(),
        end_version: Some("2024_12".to_string()),
        batch_size: 3,
        skip_existing: true,
    }),
    // ... other fields
};

let pipeline = IdempotentUniProtPipeline::new(
    pool,
    org_id,
    ftp_config,
    batch_config,
    storage
);

let stats = pipeline.run_with_mode(&uniprot_config).await?;

println!("Historical backfill complete:");
println!("  Versions: {}", stats.version_synced.unwrap_or_default());
println!("  Total entries: {}", stats.total_entries);
println!("  Duration: {:.2}s", stats.duration_secs);
```

---

## Return Type: `IngestStats`

The `IngestStats` struct provides detailed metrics about the ingestion:

```rust
pub struct IngestStats {
    pub total_entries: i64,        // Total entries processed
    pub entries_inserted: i64,     // Entries inserted
    pub entries_updated: i64,      // Entries updated
    pub entries_skipped: i64,      // Entries skipped
    pub entries_failed: i64,       // Entries that failed
    pub bytes_processed: i64,      // Total bytes processed
    pub duration_secs: f64,        // Duration in seconds
    pub version_synced: Option<String>,  // Version identifier
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

**Latest Mode**: `version_synced` = single version (e.g., "2025_01")
**Historical Mode**: `version_synced` = summary (e.g., "Historical: 3 versions (2023_01, 2023_02, 2023_03)")

---

## Database Schema

### `ingestion_jobs.source_metadata` JSONB Field

Stores mode-specific metadata:

```json
{
  "is_current": true,              // Latest mode: true, Historical mode: false
  "release_date": "2025-01-15",   // Release date from FTP
  "ftp_path": "current_release"   // FTP directory path
}
```

**Query Examples**:

```sql
-- Find all "current" versions
SELECT * FROM ingestion_jobs
WHERE source_metadata->>'is_current' = 'true';

-- Find historical backfills
SELECT * FROM ingestion_jobs
WHERE source_metadata->>'is_current' = 'false';

-- Get release dates
SELECT
    external_version,
    source_metadata->>'release_date' as release_date,
    source_metadata->>'is_current' as is_current
FROM ingestion_jobs
WHERE job_type LIKE 'uniprot_%'
ORDER BY external_version;
```

---

## Integration with Existing Pipeline

The new mode-based methods integrate seamlessly with the existing pipeline:

1. **Discovery**: Uses existing `VersionDiscovery` service
2. **Ingestion**: Uses existing `ingest_version()` method
3. **Storage**: Uses existing `IngestionCoordinator` framework
4. **Metadata**: Uses existing `source_metadata` JSONB field

**No breaking changes** to existing code.

---

## Testing

### Unit Tests

Existing tests for `IdempotentStats` still pass:
- `test_idempotent_stats()`
- `test_idempotent_stats_partial_failure()`

### Integration Testing

To test the new methods:

```bash
# Set environment variables for Latest mode
export INGEST_UNIPROT_MODE=latest
export INGEST_UNIPROT_AUTO_INGEST=true
export INGEST_UNIPROT_IGNORE_BEFORE=2024_01

# Or for Historical mode
export INGEST_UNIPROT_MODE=historical
export INGEST_UNIPROT_HISTORICAL_START=2023_01
export INGEST_UNIPROT_HISTORICAL_END=2024_12
export INGEST_UNIPROT_HISTORICAL_BATCH_SIZE=3
export INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING=true

# Run integration test
cargo test --package bdp-server --test uniprot_ingestion
```

---

## Configuration Reference

### Environment Variables

**Latest Mode**:
```bash
INGEST_UNIPROT_MODE=latest
INGEST_UNIPROT_CHECK_INTERVAL_SECS=86400
INGEST_UNIPROT_AUTO_INGEST=true
INGEST_UNIPROT_IGNORE_BEFORE=2024_01
```

**Historical Mode**:
```bash
INGEST_UNIPROT_MODE=historical
INGEST_UNIPROT_HISTORICAL_START=2020_01
INGEST_UNIPROT_HISTORICAL_END=2024_12  # Optional
INGEST_UNIPROT_HISTORICAL_BATCH_SIZE=3
INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING=true
```

### Config Files

See `crates/bdp-server/src/ingest/config.rs` for:
- `IngestionMode` enum
- `LatestConfig` struct
- `HistoricalConfig` struct
- `UniProtConfig` struct

---

## Future Improvements

1. **Parallel Historical Processing**: Process multiple versions concurrently
2. **Resume Support**: Resume interrupted historical backfills
3. **Progress Tracking**: Real-time progress updates for historical mode
4. **Smart Batching**: Dynamically adjust batch size based on version size
5. **Version Validation**: Verify version integrity before ingestion

---

## References

- **Config Module**: `crates/bdp-server/src/ingest/config.rs`
- **Pipeline Module**: `crates/bdp-server/src/ingest/uniprot/idempotent_pipeline.rs`
- **Version Discovery**: `crates/bdp-server/src/ingest/uniprot/version_discovery.rs`
- **Stats Type**: `crates/bdp-server/src/ingest/jobs.rs`
