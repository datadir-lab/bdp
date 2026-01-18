# UniProt Version Tracking & Deduplication

## Overview

The UniProt ingestion pipeline now automatically tracks actual release versions and prevents re-downloading existing data, even when using "current" release paths.

## The Problem

Without version tracking:
```
Month 1: Download from "current" (actually 2025_06)
Month 2: Download from "current" (actually 2025_07)
        ❌ Re-downloads ALL data from Month 1 too!
```

## The Solution

With version tracking:
```
Month 1: Download from "current"
         → Extracts version "2025_06" from release notes
         → Downloads and stores data
         → Stores external_version = "2025_06" in database

Month 2: Download from "current"
         → Extracts version "2025_07" from release notes
         → Checks database: "2025_07" not found
         → Downloads and stores NEW data only

Month 2 (retry):
         → Extracts version "2025_07"
         → Checks database: "2025_07" EXISTS
         → ✅ Skips download (idempotent!)
```

## Implementation

### 1. Version Extraction (pipeline.rs:65-81)

```rust
// Download release notes first (lightweight)
let notes = ftp.download_release_notes(version).await?;
let release_info = ftp.parse_release_notes(&notes)?;

let actual_version = &release_info.external_version; // e.g., "2025_06"
info!("Detected UniProt version: {}", actual_version);
```

**Key Point**: Even when downloading "current", we extract the ACTUAL version number from the release notes.

### 2. Deduplication Check (pipeline.rs:79-99)

```rust
// Check if this version already exists
if self.version_exists(actual_version).await? {
    info!("Version {} already exists in database, skipping download", actual_version);
    return Ok(IngestStats {
        total_entries: 0,
        entries_inserted: 0,
        // ... skipped stats
    });
}
```

**Database Query**:
```sql
SELECT COUNT(*) FROM versions WHERE external_version = $1
```

If count > 0, version exists → skip download.

### 3. Version Storage

When storing proteins, the actual version is used:
```rust
let storage = UniProtStorage::new(
    db_pool,
    org_id,
    "1.0",              // internal version
    actual_version      // "2025_06" (NOT "current")
);
```

This ensures database records track actual versions:
```sql
INSERT INTO versions (entry_id, version, external_version)
VALUES ($1, '1.0', '2025_06')
```

## Usage

### Automatic Version Tracking (Recommended)

```rust
use bdp_server::ingest::uniprot::{UniProtPipeline, UniProtFtpConfig, ReleaseType};

// Configure for current release
let config = UniProtFtpConfig::default()
    .with_release_type(ReleaseType::Current);

let pipeline = UniProtPipeline::new(db_pool, org_id, config);

// Run monthly - handles versioning automatically
let stats = pipeline.run(None).await?;

println!("Version synced: {:?}", stats.version_synced); // Some("2025_06")
println!("Total entries: {}", stats.total_entries);      // 570000 (first run)
println!("Entries inserted: {}", stats.entries_inserted); // 570000 (first run)

// Run again same month
let stats2 = pipeline.run(None).await?;
println!("Total entries: {}", stats2.total_entries);      // 0 (skipped)
println!("Entries inserted: {}", stats2.entries_inserted); // 0 (skipped)
```

### Manual Version Check

```rust
// Just check what version is available without downloading
let release_info = pipeline.get_release_info(None).await?;
println!("Available version: {}", release_info.external_version);
println!("Release date: {}", release_info.release_date);
println!("Protein count: {}", release_info.swissprot_count);
```

## Benefits

1. **No Redundant Downloads**: Won't re-download existing versions
2. **Automatic Detection**: Works with both current and previous releases
3. **Idempotent**: Safe to run pipeline multiple times
4. **Version History**: Database tracks which versions were ingested
5. **Efficient**: Only downloads ~30MB release notes to check version

## Database Schema

### versions table
```sql
CREATE TABLE versions (
    id UUID PRIMARY KEY,
    entry_id UUID NOT NULL,
    version VARCHAR(255) NOT NULL,              -- internal version (e.g., "1.0")
    external_version VARCHAR(255),              -- UniProt version (e.g., "2025_06")
    status VARCHAR(50) DEFAULT 'published',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Query to check existing versions**:
```sql
SELECT DISTINCT external_version
FROM versions
WHERE external_version IS NOT NULL
ORDER BY external_version DESC;
```

## Testing

### Unit Test (pipeline_dedup_tests.rs)

```rust
#[tokio::test]
async fn test_pipeline_version_deduplication() -> Result<()> {
    // Create test version in database
    sqlx::query(
        "INSERT INTO versions (id, entry_id, version, external_version)
         VALUES ($1, $2, '1.0', '2024_01')"
    ).execute(&db_pool).await?;

    // Run pipeline for same version
    let stats = pipeline.run(Some("2024_01")).await?;

    // Should have skipped
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.entries_inserted, 0);
}
```

### Example (uniprot_pipeline_with_dedup.rs)

```bash
cargo run --example uniprot_pipeline_with_dedup
```

Demonstrates:
1. First run downloads data
2. Second run skips (version exists)
3. Lists all versions in database

## Performance

**Version Check**: ~5-10 seconds
- Downloads release notes (~30MB compressed)
- Parses version number
- Single database query

**Full Download** (if new version): ~5-10 minutes
- Downloads DAT file (~655MB compressed)
- Parses 570k proteins
- Stores in database + S3

## Migration from Manual Versioning

If you were manually specifying versions:

**Before**:
```rust
// Had to manually track versions
let storage = UniProtStorage::new(db_pool, org_id, "1.0", "2024_01");
```

**After**:
```rust
// Pipeline handles it automatically
let pipeline = UniProtPipeline::new(db_pool, org_id, config);
let stats = pipeline.run(None).await?; // Auto-detects version
```

## Monitoring

Check what versions are in your database:

```sql
-- List all versions with counts
SELECT
    v.external_version,
    COUNT(DISTINCT v.entry_id) as protein_count,
    MIN(v.created_at) as first_ingested,
    MAX(v.created_at) as last_ingested
FROM versions v
WHERE v.external_version IS NOT NULL
GROUP BY v.external_version
ORDER BY v.external_version DESC;
```

Example output:
```
external_version | protein_count | first_ingested      | last_ingested
-----------------+---------------+---------------------+-------------------
2025_07          | 571000        | 2025-07-15 10:00:00 | 2025-07-15 11:30:00
2025_06          | 570500        | 2025-06-15 10:00:00 | 2025-06-15 11:25:00
2025_05          | 570000        | 2025-05-15 10:00:00 | 2025-05-15 11:20:00
```

## Error Handling

If version check fails:
```rust
match pipeline.run(None).await {
    Ok(stats) if stats.total_entries == 0 => {
        println!("Version already exists, skipped download");
    }
    Ok(stats) => {
        println!("Downloaded new version: {:?}", stats.version_synced);
    }
    Err(e) => {
        eprintln!("Pipeline failed: {}", e);
        // Version check failed, FTP download failed, etc.
    }
}
```

## FAQ

**Q: What if I want to force re-download?**
A: Delete the version records from database first:
```sql
DELETE FROM versions WHERE external_version = '2025_06';
```

**Q: Can I download multiple versions?**
A: Yes! Use `ReleaseType::Previous` with different versions:
```rust
pipeline.run(Some("2024_01")).await?;
pipeline.run(Some("2024_02")).await?;
// Each version stored separately
```

**Q: How do I know if a download was skipped?**
A: Check `stats.total_entries`:
```rust
if stats.total_entries == 0 {
    println!("Skipped - version already exists");
} else {
    println!("Downloaded {} proteins", stats.entries_inserted);
}
```

**Q: Does this work with S3?**
A: Yes! Version tracking works with both local and S3 storage:
```rust
let pipeline = UniProtPipeline::with_s3(db_pool, s3, org_id, config);
```

## See Also

- [PIPELINE_COMPLETE.md](./PIPELINE_COMPLETE.md) - Full feature documentation
- [GETTING_STARTED.md](./GETTING_STARTED.md) - Quick start guide
- [examples/uniprot_pipeline_with_dedup.rs](../../examples/uniprot_pipeline_with_dedup.rs) - Example code
