# GenBank/RefSeq Version Discovery - Quick Start

Quick reference for using the GenBank/RefSeq version discovery system.

## Basic Usage

### 1. Discover Available Versions

```rust
use bdp_server::ingest::genbank::{GenbankFtpConfig, VersionDiscovery};

// GenBank
let config = GenbankFtpConfig::new().with_genbank();
let discovery = VersionDiscovery::new(config);
let versions = discovery.discover_all_versions().await?;

// RefSeq
let config = GenbankFtpConfig::new().with_refseq();
let discovery = VersionDiscovery::new(config);
let versions = discovery.discover_all_versions().await?;
```

### 2. Filter Versions

```rust
// Filter by release number
let from_255 = discovery.filter_from_release(versions, 255);

// Filter already-ingested
let ingested = vec!["GB_Release_255.0".to_string()];
let new_only = discovery.filter_new_versions(versions, ingested);
```

### 3. Ingest Historical Versions

```rust
use bdp_server::ingest::genbank::GenbankOrchestrator;

let orchestrator = GenbankOrchestrator::new(config, db, s3);

// Ingest from release 255 onwards
let results = orchestrator.run_historical_ingestion(
    organization_id,
    None,        // default divisions
    Some(255),   // from release 255
).await?;
```

## Command-Line Examples

### Discover Versions

```bash
# GenBank
cargo run --example genbank_version_discovery -- --database genbank

# RefSeq from release 100+
cargo run --example genbank_version_discovery -- \
  --database refseq \
  --from-release 100
```

### Historical Ingestion

```bash
# Dry run
cargo run --example genbank_historical_ingestion -- \
  --database genbank \
  --division phage \
  --dry-run

# Actual ingestion with limits
cargo run --example genbank_historical_ingestion -- \
  --database genbank \
  --division phage \
  --from-release 256 \
  --parse-limit 100
```

## API Reference

### VersionDiscovery

```rust
// Create
let discovery = VersionDiscovery::new(config);

// Discover versions
let versions = discovery.discover_all_versions().await?;

// Filter methods
discovery.filter_new_versions(versions, ingested);
discovery.filter_from_release(versions, start_release);

// Database integration
discovery.check_for_newer_version(&pool, org_id).await?;
discovery.get_last_ingested_version(&pool, org_id).await?;
discovery.version_exists_in_db(&pool, ext_version).await?;
```

### DiscoveredVersion

```rust
pub struct DiscoveredVersion {
    pub external_version: String,    // "GB_Release_257.0"
    pub release_date: NaiveDate,     // Estimated date
    pub release_number: i32,         // 257
    pub source_database: SourceDatabase,  // Genbank or Refseq
}
```

### GenbankOrchestrator

```rust
// Create
let orchestrator = GenbankOrchestrator::new(config, db, s3);

// Single version
orchestrator.run_release(organization_id).await?;
orchestrator.run_divisions(org_id, &divisions, Some(release)).await?;

// Historical ingestion
orchestrator.run_historical_ingestion(
    organization_id,
    Some(divisions),     // divisions to process
    Some(start_release), // starting release number
).await?;
```

## Configuration

```rust
let config = GenbankFtpConfig::new()
    .with_genbank()           // or .with_refseq()
    .with_parse_limit(1000)   // limit records for testing
    .with_batch_size(500)     // database batch size
    .with_concurrency(4)      // parallel processing
    .with_timeout(600);       // FTP timeout in seconds
```

## Version Formats

| Database | Format              | Example         | Release # |
|----------|---------------------|-----------------|-----------|
| GenBank  | GB_Release_XXX.0    | GB_Release_257.0| 257       |
| RefSeq   | RefSeq-XXX          | RefSeq-117      | 117       |

## Release Date Estimation

| Database | Base Year | Releases/Year | Formula                    |
|----------|-----------|---------------|----------------------------|
| GenBank  | 1982      | 6             | 1982 + (release / 6)       |
| RefSeq   | 2000      | 6             | 2000 + (release / 6)       |

## Error Handling

```rust
// Handle version discovery errors
match discovery.discover_all_versions().await {
    Ok(versions) => {
        info!("Found {} versions", versions.len());
    }
    Err(e) => {
        warn!("Version discovery failed: {}", e);
        // Fall back to manual version specification
    }
}

// Handle ingestion failures
for version in versions {
    match pipeline.run_division(org_id, div, &version.external_version).await {
        Ok(result) => info!("Success: {}", version.external_version),
        Err(e) => {
            warn!("Failed {}: {}", version.external_version, e);
            continue;  // Process remaining versions
        }
    }
}
```

## Testing

```bash
# Unit tests
cargo test --package bdp-server --lib genbank::version_discovery

# Integration tests
cargo run --example genbank_version_discovery

# End-to-end test
cargo run --example genbank_historical_ingestion -- \
  --dry-run \
  --parse-limit 10
```

## Common Patterns

### Check for New Releases

```rust
let discovery = VersionDiscovery::new(config);

if let Some(newer) = discovery.check_for_newer_version(&pool, org_id).await? {
    info!("New version available: {}", newer.external_version);

    // Trigger ingestion
    orchestrator.run_release(organization_id).await?;
}
```

### Backfill Historical Data

```rust
// Get all versions from release 250 onwards
let versions = discovery.discover_all_versions().await?;
let from_250 = discovery.filter_from_release(versions, 250);

// Get already ingested
let ingested = discovery.get_ingested_versions(&pool, entry_id).await?;

// Filter to new only
let to_ingest = discovery.filter_new_versions(from_250, ingested);

// Ingest sequentially
for version in to_ingest {
    orchestrator.run_divisions(org_id, &divisions, Some(version.external_version)).await?;
}
```

### Production Deployment

```rust
// Configuration for production
let config = GenbankFtpConfig::new()
    .with_genbank()
    .with_batch_size(500)      // Optimize batch size
    .with_concurrency(4)       // Parallel divisions
    .with_timeout(600);        // 10 min timeout

// Use all primary divisions
let divisions = GenbankFtpConfig::get_primary_divisions();

// Run with error handling
let orchestrator = GenbankOrchestrator::new(config, db, s3);

match orchestrator.run_historical_ingestion(org_id, Some(divisions), None).await {
    Ok(results) => {
        info!("Successfully ingested {} versions", results.len());
    }
    Err(e) => {
        error!("Historical ingestion failed: {}", e);
        // Alert monitoring system
    }
}
```

## Performance Tips

1. **Start with test division**: Use `Division::Phage` (smallest) for testing
2. **Use parse limits**: Set `parse_limit` during development
3. **Adjust concurrency**: Balance between speed and resource usage
4. **Monitor memory**: Watch memory usage with large divisions
5. **Use batch operations**: Default batch size (500) is optimized

## Troubleshooting

| Problem | Solution |
|---------|----------|
| No versions found | Expected for GenBank (only current available) |
| FTP timeout | Increase timeout: `.with_timeout(900)` |
| Out of memory | Reduce concurrency: `.with_concurrency(1)` |
| Already ingested | Version exists in DB, will be filtered out |

## Next Steps

1. Review full documentation: `docs/genbank-version-discovery.md`
2. Understand versioning system: `docs/agents/implementation/versioning-design.md`
3. Check orchestrator code: `crates/bdp-server/src/ingest/genbank/orchestrator.rs`
4. Run examples to test functionality
