# Gene Ontology Version Discovery - Quick Start

## TL;DR

```bash
# 1. Test discovery (no database needed)
cargo run --example test_go_version_discovery

# 2. Check what's new (requires database)
cargo run --example go_historical_ingestion -- check

# 3. Backfill from a date
cargo run --example go_historical_ingestion -- backfill 2024-01-01

# 4. Ingest specific version
cargo run --example go_historical_ingestion -- ingest 2025-01-01
```

## What It Does

Discovers available Gene Ontology versions from `http://release.geneontology.org/` and allows:
- Listing all available versions
- Checking for new versions to ingest
- Backfilling historical data
- Ingesting specific versions

## Version Format

- **Format**: `YYYY-MM-DD` (e.g., `2025-01-01`)
- **Frequency**: Monthly releases
- **Source**: HTTP release archive

## Commands

### discover
Lists all available GO versions from the archive.

```bash
cargo run --example go_historical_ingestion -- discover
```

### check
Compares available versions against database to find new ones.

```bash
cargo run --example go_historical_ingestion -- check
```

### backfill
Ingests all versions from a start date onwards.

```bash
# From date onwards
cargo run --example go_historical_ingestion -- backfill 2024-01-01

# Specific date range
cargo run --example go_historical_ingestion -- backfill-range 2024-01-01 2024-12-31
```

### ingest
Ingests a specific GO version.

```bash
cargo run --example go_historical_ingestion -- ingest 2025-01-01
```

## Programmatic Usage

```rust
use bdp_server::ingest::gene_ontology::{GoHttpConfig, VersionDiscovery};

// Create discovery service
let config = GoHttpConfig::default();
let discovery = VersionDiscovery::new(config)?;

// Discover all versions
let versions = discovery.discover_all_versions().await?;

// Filter to new versions
let ingested = discovery.get_ingested_versions(&pool, entry_id).await?;
let new_versions = discovery.filter_new_versions(versions, ingested);

// Ingest a version
let pipeline = GoPipeline::new(config, db, s3, org_id);
let stats = pipeline.run_ontology_version("1.0", Some("2025-01-01")).await?;
```

## Performance

- **Discovery**: ~1-2 seconds for 50 versions
- **Per Version**: ~15-20 seconds (download + parse + store)
- **12 Months**: ~3-4 minutes total

## Files

### Implementation
- `crates/bdp-server/src/ingest/gene_ontology/version_discovery.rs`

### Tools
- `crates/bdp-server/examples/test_go_version_discovery.rs` (no database)
- `crates/bdp-server/examples/go_historical_ingestion.rs` (full tool)

### Documentation
- `docs/gene-ontology-version-discovery.md` (comprehensive)
- `GO_VERSION_DISCOVERY_COMPLETE.md` (summary)
- `GO_QUICK_START.md` (this file)

## Requirements

- Rust toolchain
- Database connection (for ingestion only)
- S3 storage (for ingestion only)
- Network access to `http://release.geneontology.org/`

## Testing

```bash
# Unit tests
cargo test --package bdp-server go_version_discovery_test

# Integration test (requires network)
cargo test --package bdp-server --lib gene_ontology::version_discovery -- --ignored

# Example tool
cargo run --example test_go_version_discovery
```

## Troubleshooting

**Can't connect to GO archive**
- Check network connectivity
- Verify `http://release.geneontology.org/` is accessible

**No versions discovered**
- Check HTML structure hasn't changed
- Enable debug logging: `RUST_LOG=debug`

**Version already exists**
- Use `check` command to see ingested versions
- Database prevents duplicate ingestion

## Next Steps

1. Run `test_go_version_discovery` to verify it works
2. Run `check` to see what needs ingestion
3. Run `backfill` with appropriate date range
4. Set up automated checks for new versions

For more details, see `docs/gene-ontology-version-discovery.md`
