# InterPro Version Discovery and Historical Ingestion

## Overview

This document describes the InterPro version discovery system that enables:

1. **Version Discovery**: Automatically discover all available InterPro versions from FTP
2. **Historical Ingestion**: Ingest multiple versions starting from a specific version
3. **Update Detection**: Check if newer versions are available
4. **Incremental Ingestion**: Skip already-ingested versions

## Architecture

### Components

```
interpro/
├── version_discovery.rs    # Version discovery service
├── ftp.rs                   # FTP operations (enhanced with list_versions)
├── pipeline.rs              # Pipeline with version support (enhanced)
└── config.rs                # Configuration (unchanged)
```

### Key Types

#### `DiscoveredVersion`

Represents a discovered InterPro version from FTP:

```rust
pub struct DiscoveredVersion {
    pub external_version: String,  // e.g., "96.0", "97.0"
    pub major: u32,                 // e.g., 96
    pub minor: u32,                 // e.g., 0
    pub release_date: NaiveDate,   // Estimated from version number
    pub is_current: bool,           // Whether this is in /current/
    pub ftp_directory: String,      // e.g., "96.0" or "current"
}
```

#### `VersionDiscovery`

Service for discovering and filtering versions:

```rust
pub struct VersionDiscovery {
    config: InterProConfig,
}
```

## Version Format

InterPro uses **MAJOR.MINOR** versioning:

- Format: `XX.Y` (e.g., 96.0, 97.0, 98.0, 100.0)
- No patch version
- Sequential releases: 96.0 → 97.0 → 98.0 → ...

## FTP Directory Structure

```
ftp.ebi.ac.uk/pub/databases/interpro/
├── current/              # Current release (e.g., 98.0)
├── 96.0/                 # Historical release
│   ├── protein2ipr.dat.gz
│   └── entry.list
├── 97.0/                 # Historical release
│   ├── protein2ipr.dat.gz
│   └── entry.list
├── 98.0/                 # Historical release (same as current)
│   ├── protein2ipr.dat.gz
│   └── entry.list
└── ...
```

## Usage Examples

### 1. Discover All Versions

```rust
use bdp_server::ingest::interpro::{
    config::InterProConfig,
    version_discovery::VersionDiscovery,
};

let config = InterProConfig::from_env();
let discovery = VersionDiscovery::new(config);

// Discover all versions
let versions = discovery.discover_all_versions().await?;

for version in versions {
    println!("Version {}: released {}",
        version.external_version,
        version.release_date
    );
}
```

### 2. Discover New Versions

```rust
// Get only versions not yet ingested
let new_versions = pipeline.discover_new_versions().await?;

println!("Found {} new versions to ingest", new_versions.len());
```

### 3. Historical Ingestion

```rust
use bdp_server::ingest::interpro::{
    config::InterProConfig,
    pipeline::InterProPipeline,
};

let pipeline = InterProPipeline::new(pool, config, download_dir);

// Ingest all versions from 96.0 onwards (skip existing)
let results = pipeline
    .ingest_from_version("96.0", true)
    .await?;

for (version, stats) in results {
    println!("Ingested {}: {} entries", version, stats.entries_stored);
}
```

### 4. Ingest Latest Version Only

```rust
// Check for newer version and ingest if available
match pipeline.ingest_latest().await? {
    Some((version, stats)) => {
        println!("Ingested new version {}: {} entries",
            version, stats.entries_stored);
    }
    None => {
        println!("Already up-to-date");
    }
}
```

## Command-Line Examples

### Version Discovery

```bash
# Discover all available versions
cargo run --example interpro_version_discovery

# Output:
# Found 25 versions:
#   1. Version 74.0 - Released: 2019-01-01 - Dir: 74.0
#   2. Version 75.0 - Released: 2019-04-01 - Dir: 75.0
#   ...
#  25. Version 98.0 - Released: 2025-01-01 - Dir: current (CURRENT)
```

### Historical Ingestion

```bash
# Ingest all versions from 96.0 onwards (skip existing)
cargo run --example interpro_historical_ingestion -- 96.0

# Ingest a single specific version
cargo run --example interpro_historical_ingestion -- 98.0 --single
```

## Database Integration

### Tracking Ingested Versions

The system uses two tables to track ingested versions:

1. **`versions` table**: Stores version metadata for each data source
2. **`organization_sync_status` table**: Tracks last ingested version per organization

### Querying Ingested Versions

```rust
// Get all ingested InterPro versions
let ingested = discovery.get_ingested_versions(&pool).await?;

// Get last ingested version
let org_id = discovery.get_organization_id(&pool).await?;
let last = discovery.get_last_ingested_version(&pool, org_id).await?;

// Check if version already ingested
let exists = discovery.version_exists_in_db(&pool, "96.0").await?;
```

## Version Discovery Algorithm

### Step 1: Discover Current Release

1. Connect to FTP: `ftp.ebi.ac.uk`
2. Navigate to: `/pub/databases/interpro/current/`
3. Read version from directory listing or metadata
4. Create `DiscoveredVersion` with `is_current: true`

### Step 2: Discover Historical Releases

1. List all directories in `/pub/databases/interpro/`
2. Filter directories matching pattern `^\d+\.\d+$`
3. Parse version numbers: "96.0" → (major=96, minor=0)
4. Estimate release dates from version numbers
5. Create `DiscoveredVersion` for each with `is_current: false`

### Step 3: Sort and Deduplicate

1. Sort all versions by major.minor (oldest first)
2. Remove duplicates (if current also appears as versioned)

## Filtering Strategies

### Filter New Versions

```rust
// Get only versions not in database
let new_versions = discovery.filter_new_versions(
    all_versions,
    ingested_versions
);
```

### Filter From Version

```rust
// Get versions >= 96.0
let filtered = discovery.filter_from_version(
    all_versions,
    "96.0"
)?;
```

## Error Handling

### FTP Connection Errors

```rust
match discovery.discover_all_versions().await {
    Ok(versions) => { /* ... */ },
    Err(e) => {
        // Possible causes:
        // - FTP server unreachable
        // - Network/firewall blocking passive mode
        // - Directory structure changed
        tracing::error!("Version discovery failed: {}", e);
    }
}
```

### Partial Failures

The historical ingestion pipeline continues even if individual versions fail:

```rust
let results = pipeline.ingest_from_version("96.0", true).await?;

// Some versions may have failed, but others succeeded
// Check results.len() vs total expected versions
```

## Performance Considerations

### Release Date Estimation

For speed, release dates are **estimated** from version numbers rather than parsing release notes:

```rust
// Fast: O(1) estimation
let date = DiscoveredVersion::estimate_release_date(96, 0);

// Slow: Would require downloading/parsing release notes for each version
// let date = parse_release_notes(&ftp, "96.0")?;
```

This is acceptable because:
- InterPro releases are sequential and predictable
- Exact dates aren't critical for ordering
- Saves significant FTP round-trips

### Batch Processing

Historical ingestion processes versions sequentially to avoid:
- Overwhelming FTP server with parallel connections
- Memory issues from loading multiple large files
- Database lock contention

## Testing

### Unit Tests

```bash
# Run version discovery tests
cargo test --package bdp-server version_discovery

# Test version parsing
cargo test --package bdp-server test_parse_version

# Test version ordering
cargo test --package bdp-server test_version_ordering

# Test filtering
cargo test --package bdp-server test_filter_from_version
```

### Integration Tests

```bash
# Test FTP connection (requires network)
cargo test --package bdp-server test_list_versions -- --ignored

# Test full discovery (requires network)
cargo test --package bdp-server test_discover_all_versions -- --ignored
```

## Configuration

### Environment Variables

```bash
# FTP configuration
export INGEST_INTERPRO_FTP_HOST="ftp.ebi.ac.uk"
export INGEST_INTERPRO_FTP_PATH="/pub/databases/interpro/"
export INGEST_INTERPRO_FTP_TIMEOUT_SECS="300"

# Processing configuration
export INGEST_INTERPRO_BATCH_SIZE="500"
```

## Monitoring

### Logging

The system uses structured logging:

```rust
// Version discovery
tracing::info!(
    version = %version.external_version,
    date = %version.release_date,
    "Discovered historical version"
);

// Ingestion progress
tracing::info!(
    "Ingesting InterPro version {} ({}/{})",
    version.external_version,
    current_index,
    total_versions
);
```

### Metrics to Track

- Total versions discovered
- New versions found
- Ingestion success rate
- Time per version
- Database size growth

## Future Enhancements

### Potential Improvements

1. **Parallel Version Downloads**: Download multiple versions in parallel (respecting FTP limits)
2. **Resume Failed Ingestion**: Track partial ingestion state and resume
3. **Accurate Release Dates**: Parse release notes for exact dates
4. **Version Validation**: Verify data integrity after ingestion
5. **Incremental Updates**: Detect and apply delta updates for versions

### Known Limitations

1. **Release date estimation**: Not exact, but sufficient for ordering
2. **Current version detection**: May need adjustment if FTP structure changes
3. **No retry for failed versions**: Manual re-run required

## Troubleshooting

### Problem: No versions found

**Cause**: FTP connection or listing failed

**Solution**:
```bash
# Test FTP connection manually
ftp ftp.ebi.ac.uk
> cd /pub/databases/interpro/
> ls

# Check firewall allows FTP passive mode
# Verify network connectivity
```

### Problem: Version already exists error

**Cause**: Attempting to re-ingest existing version

**Solution**:
```rust
// Use skip_existing flag
pipeline.ingest_from_version("96.0", true).await?;
//                                      ^^^^ skip existing
```

### Problem: Download timeout

**Cause**: Slow FTP connection or large files

**Solution**:
```bash
# Increase timeout
export INGEST_INTERPRO_FTP_TIMEOUT_SECS="600"
```

## Summary

The InterPro version discovery system provides:

✅ **Automatic discovery** of all available versions from FTP
✅ **Historical backfill** starting from any version
✅ **Incremental ingestion** of only new versions
✅ **Update detection** for latest releases
✅ **Robust error handling** with partial failure recovery
✅ **Database integration** for tracking ingested versions
✅ **Production-ready** with proper logging and error handling

This enables complete historical data ingestion and continuous updates for the InterPro database.
