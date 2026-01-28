# Migration Safety Tests

## Overview

The migration safety tests in `crates/bdp-server/tests/migration_tests.rs` verify that when UniProt moves a release from the current location to historical releases (which happens monthly), we don't re-ingest the same data.

## Problem Statement

UniProt releases data monthly:
- **Current release**: Available at `ftp://ftp.uniprot.org/pub/databases/uniprot/current_release/`
- **Historical releases**: Moved to `ftp://ftp.uniprot.org/pub/databases/uniprot/previous_releases/release-YYYY_MM/`

**The challenge**: When UniProt releases a new version (e.g., 2025_02), the previous version (2025_01) moves from `current_release/` to `previous_releases/release-2025_01/`. The version number stays the same, only the FTP location changes.

**Without migration safety**: We would re-ingest 2025_01 when we discover it in `previous_releases/`, wasting bandwidth and storage.

**With migration safety**: We track whether a version was ingested as "current" using the `source_metadata->>'is_current'` field in the `ingestion_jobs` table. If we later discover the same version in historical releases, we skip it.

## Test Coverage

### Test 1: `test_current_to_historical_no_reingest`

**Scenario**:
- Month 1: Ingest 2025_01 from `current_release/` (is_current=true)
- Month 2: UniProt moves 2025_01 to `previous_releases/release-2025_01/`
- Month 2: Discovery finds 2025_01 in historical location

**Expected Result**: 
- `was_ingested_as_current()` returns `true`
- Pipeline skips re-ingestion

**What it tests**: Core migration safety logic - same version, different location = skip

### Test 2: `test_new_version_in_historical_ingests`

**Scenario**:
- Discover 2024_12 in `previous_releases/` (never ingested before)

**Expected Result**:
- `was_ingested_as_current()` returns `false`
- Pipeline ingests it (it's genuinely new to us)

**What it tests**: Historical backfill - we should still ingest versions we never had

### Test 3: `test_pipeline_stores_is_current_metadata`

**Scenario**:
- Ingest 2025_02 from current (is_current=true)
- Ingest 2025_01 from historical (is_current=false)

**Expected Result**:
- 2025_02: `source_metadata->>'is_current'` = 'true'
- 2025_01: `source_metadata->>'is_current'` = 'false'

**What it tests**: Metadata tracking - we correctly store where data came from

### Test 4: `test_monthly_update_scenario`

**Scenario**:
- Month 1: Ingest 2025_01 as current (is_current=true)
- Month 2: Discover both:
  - 2025_01 in `previous_releases/` (was current)
  - 2025_02 in `current_release/` (new)

**Expected Result**:
- 2025_01: Skip (already have it)
- 2025_02: Ingest (new version)

**What it tests**: Complete monthly update flow - realistic scenario

## Implementation Details

### Database Schema

The `ingestion_jobs` table has a `source_metadata` JSONB column that stores:
```json
{
  "is_current": true/false,
  "release_date": "2025-01-15",
  "ftp_path": "current_release" or "previous_releases/release-2025_01"
}
```

### Key Method: `was_ingested_as_current()`

Located in `crates/bdp-server/src/ingest/uniprot/version_discovery.rs`:

```rust
pub async fn was_ingested_as_current(
    &self,
    pool: &PgPool,
    external_version: &str,
) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM ingestion_jobs
            WHERE external_version = $1
              AND source_metadata->>'is_current' = 'true'
        ) as "exists!"
        "#,
        external_version
    )
    .fetch_one(pool)
    .await?;

    Ok(result.exists)
}
```

### Pipeline Integration

In `crates/bdp-server/src/ingest/uniprot/pipeline.rs`:

```rust
// Check if it was ingested as current but is now in historical
let was_current = discovery
    .was_ingested_as_current(&self.db, &version.external_version)
    .await?;

if was_current && !version.is_current {
    // Same data, just moved location - skip
    info!(
        "Version {} was ingested as current and is now historical (same data), skipping",
        version.external_version
    );
    continue;
}
```

## Running the Tests

```bash
# Run all migration safety tests
cargo test --test migration_tests

# Run a specific test
cargo test --test migration_tests test_current_to_historical_no_reingest

# Run with output
cargo test --test migration_tests -- --nocapture
```

## Test Database Requirements

These tests use `#[sqlx::test]` which:
- Creates a fresh database for each test
- Runs migrations automatically
- Provides full isolation between tests
- Cleans up after each test

## Related Documentation

- [UniProt Ingestion Complete](../UNIPROT_INGESTION_COMPLETE.md) - Full ingestion pipeline overview
- [Version Discovery](../crates/bdp-server/src/ingest/uniprot/version_discovery.rs) - Implementation
- [Version Checking Tests](../crates/bdp-server/tests/version_checking_tests.rs) - Related tests
