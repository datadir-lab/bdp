# Mode-Based Ingestion Implementation Summary

**Date**: 2026-01-18
**Status**: ✅ Complete
**Module**: `crates/bdp-server/src/ingest/uniprot/idempotent_pipeline.rs`

## What Was Implemented

Added three new public methods to `IdempotentUniProtPipeline` for mode-based ingestion:

### 1. `run_with_mode(&self, config: &UniProtConfig) -> Result<IngestStats>`
**Purpose**: Main entry point that dispatches to the appropriate mode handler

**Behavior**:
- Matches on `config.ingestion_mode` enum
- Routes to `run_latest_mode()` or `run_historical_mode()`

---

### 2. `run_latest_mode(&self, config: &LatestConfig) -> Result<IngestStats>`
**Purpose**: Ingest only the newest available version (incremental updates)

**Features**:
- ✅ Uses `VersionDiscovery.check_for_newer_version()` to find newer versions
- ✅ Applies `ignore_before` filter if configured
- ✅ Sets `is_current=true` in source_metadata
- ✅ Returns empty stats if already up-to-date (no-op)

**Configuration**:
```rust
LatestConfig {
    check_interval_secs: 86400,  // How often to check (default: daily)
    auto_ingest: true,           // Auto-ingest when newer version found
    ignore_before: Some("2024_01".to_string()),  // Skip older versions
}
```

---

### 3. `run_historical_mode(&self, config: &HistoricalConfig) -> Result<IngestStats>`
**Purpose**: Backfill multiple versions within a specified range

**Features**:
- ✅ Uses `VersionDiscovery.discover_all_versions()` to get all versions
- ✅ Filters by `start_version..end_version` range
- ✅ Skips already-ingested versions if `skip_existing=true`
- ✅ Processes versions in sequential batches (size from `batch_size`)
- ✅ Sets `is_current=false` in source_metadata
- ✅ Merges statistics from all ingested versions

**Configuration**:
```rust
HistoricalConfig {
    start_version: "2023_01".to_string(),
    end_version: Some("2024_12".to_string()),  // Optional, None = all
    batch_size: 3,              // Process 3 versions at a time
    skip_existing: true,        // Skip versions already in DB
}
```

---

## Metadata Storage

The pipeline stores `is_current` in the `ingestion_jobs.source_metadata` JSONB field:

```json
{
  "is_current": true,              // Latest: true, Historical: false
  "release_date": "2025-01-15",
  "ftp_path": "current_release"
}
```

**No changes were needed** to the existing `ingest_version()` method - it already stores this metadata correctly.

---

## Integration

### Existing Pipeline Integration
- ✅ Uses existing `VersionDiscovery` service
- ✅ Uses existing `ingest_version()` method
- ✅ Uses existing `IngestionCoordinator` framework
- ✅ No breaking changes to existing code

### Database Schema
- ✅ Uses existing `ingestion_jobs.source_metadata` JSONB column
- ✅ No new database migrations required

---

## Usage Example

```rust
use crate::ingest::config::{IngestionMode, LatestConfig, UniProtConfig};
use crate::ingest::uniprot::idempotent_pipeline::IdempotentUniProtPipeline;

// Create pipeline
let pipeline = IdempotentUniProtPipeline::new(
    pool,
    org_id,
    ftp_config,
    batch_config,
    storage
);

// Configure for Latest mode
let uniprot_config = UniProtConfig {
    ingestion_mode: IngestionMode::Latest(LatestConfig {
        check_interval_secs: 86400,
        auto_ingest: true,
        ignore_before: Some("2024_01".to_string()),
    }),
    // ... other fields
};

// Run ingestion
let stats = pipeline.run_with_mode(&uniprot_config).await?;

println!("Ingested {} entries from {:?}",
    stats.total_entries,
    stats.version_synced
);
```

---

## Configuration Types

All configuration types are defined in `crates/bdp-server/src/ingest/config.rs`:

- `IngestionMode` enum (Latest | Historical)
- `LatestConfig` struct
- `HistoricalConfig` struct
- `UniProtConfig` struct

---

## Environment Variables

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
INGEST_UNIPROT_HISTORICAL_END=2024_12
INGEST_UNIPROT_HISTORICAL_BATCH_SIZE=3
INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING=true
```

---

## Return Type: `IngestStats`

```rust
pub struct IngestStats {
    pub total_entries: i64,
    pub entries_inserted: i64,
    pub entries_updated: i64,
    pub entries_skipped: i64,
    pub entries_failed: i64,
    pub bytes_processed: i64,
    pub duration_secs: f64,
    pub version_synced: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

**Latest Mode**: `version_synced` = "2025_01"
**Historical Mode**: `version_synced` = "Historical: 3 versions (2023_01, 2023_02, 2023_03)"

---

## Testing Status

- ✅ Code compiles successfully
- ✅ No breaking changes to existing tests
- ✅ Existing `IdempotentStats` tests pass
- ⏳ Integration tests pending (requires database setup)

---

## Files Modified

1. **`crates/bdp-server/src/ingest/uniprot/idempotent_pipeline.rs`**
   - Added imports: `IngestionMode`, `LatestConfig`, `HistoricalConfig`, `UniProtConfig`, `IngestStats`, `chrono::Utc`
   - Added 3 public methods: `run_with_mode()`, `run_latest_mode()`, `run_historical_mode()`
   - Added 1 helper method: `get_job_stats()`
   - Organized code into logical sections with comments

---

## Documentation Created

1. **`docs/agents/implementation/mode-based-ingestion.md`**
   - Comprehensive implementation guide
   - Usage examples
   - Configuration reference
   - Database schema details
   - Integration notes

2. **`MODE_BASED_INGESTION_SUMMARY.md`** (this file)
   - Quick reference summary
   - High-level overview
   - Configuration examples

---

## Next Steps (Optional)

1. Add integration tests for mode-based ingestion
2. Add CLI commands to trigger mode-based ingestion
3. Add scheduled job support for Latest mode
4. Add progress tracking for Historical mode
5. Add metrics/telemetry for ingestion operations

---

## References

- **Config Module**: `crates/bdp-server/src/ingest/config.rs`
- **Pipeline Module**: `crates/bdp-server/src/ingest/uniprot/idempotent_pipeline.rs`
- **Version Discovery**: `crates/bdp-server/src/ingest/uniprot/version_discovery.rs`
- **Stats Type**: `crates/bdp-server/src/ingest/jobs.rs`
- **Detailed Documentation**: `docs/agents/implementation/mode-based-ingestion.md`
