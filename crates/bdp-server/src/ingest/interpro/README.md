# InterPro Integration Module

Complete InterPro protein family and domain database ingestion with version discovery and historical data support.

## Features

- ✅ **Version Discovery**: Automatically discover all available InterPro versions from FTP
- ✅ **Historical Ingestion**: Backfill data from any version onwards (e.g., from 96.0)
- ✅ **Incremental Updates**: Detect and ingest only new versions
- ✅ **FTP Download**: Efficient download of protein2ipr.dat.gz and entry.list files
- ✅ **Streaming Parsing**: Memory-efficient parsing of large data files
- ✅ **Batch Storage**: Optimized database insertion with proper foreign key handling
- ✅ **GO Term Mapping**: Integration with Gene Ontology annotations
- ✅ **Signature Databases**: Support for Pfam, SMART, PROSITE, and other member databases

## Quick Start

### Discover Available Versions

```bash
cargo run --example interpro_version_discovery
```

### Ingest Current Version

```rust
use bdp_server::ingest::interpro::{
    config::InterProConfig,
    pipeline::InterProPipeline,
};

let config = InterProConfig::from_env();
let pipeline = InterProPipeline::new(pool, config, download_dir);

// Ingest latest version
if let Some((version, stats)) = pipeline.ingest_latest().await? {
    println!("Ingested version {}: {} entries", version, stats.entries_stored);
}
```

### Historical Ingestion

```bash
# Ingest all versions from 96.0 onwards (skip existing)
cargo run --example interpro_historical_ingestion -- 96.0
```

## Architecture

```
interpro/
├── mod.rs                  # Module exports
├── config.rs               # Configuration (FTP, batch sizes)
├── ftp.rs                  # FTP download operations
├── version_discovery.rs    # Version discovery from FTP (NEW)
├── parser.rs               # Entry and protein match parsing
├── models.rs               # Data structures
├── storage.rs              # Database storage operations
├── helpers.rs              # Lookup helpers for batch operations
└── pipeline.rs             # End-to-end ingestion pipeline (ENHANCED)
```

## Version Format

InterPro uses **MAJOR.MINOR** versioning:

- Format: `XX.Y` (e.g., 96.0, 97.0, 98.0)
- Sequential releases
- Approximately quarterly

## FTP Structure

```
ftp.ebi.ac.uk/pub/databases/interpro/
├── current/              # Current release
├── 96.0/                 # Historical releases
│   ├── protein2ipr.dat.gz
│   └── entry.list
├── 97.0/
├── 98.0/
└── ...
```

## Data Flow

```
FTP Download → Parse Entries → Store Entries
    ↓              ↓               ↓
protein2ipr    entry.list     data_sources
   .dat.gz                        ↓
    ↓                          versions
Parse Matches                     ↓
    ↓                       interpro_entries
Extract Signatures               ↓
    ↓                       interpro_signatures
Store Signatures                 ↓
    ↓                       interpro_protein_matches
Store Matches
```

## API Reference

### Version Discovery

```rust
use bdp_server::ingest::interpro::version_discovery::VersionDiscovery;

let discovery = VersionDiscovery::new(config);

// Discover all versions
let versions = discovery.discover_all_versions().await?;

// Filter new versions
let ingested = discovery.get_ingested_versions(&pool).await?;
let new = discovery.filter_new_versions(versions, ingested);

// Filter from specific version
let filtered = discovery.filter_from_version(versions, "96.0")?;
```

### Pipeline Operations

```rust
use bdp_server::ingest::interpro::pipeline::InterProPipeline;

let pipeline = InterProPipeline::new(pool, config, download_dir);

// Discover available versions
let versions = pipeline.discover_versions().await?;

// Get only new versions
let new_versions = pipeline.discover_new_versions().await?;

// Ingest from specific version
let results = pipeline.ingest_from_version("96.0", true).await?;

// Ingest latest if available
let result = pipeline.ingest_latest().await?;
```

## Configuration

### Environment Variables

```bash
# FTP Configuration
export INGEST_INTERPRO_FTP_HOST="ftp.ebi.ac.uk"
export INGEST_INTERPRO_FTP_PATH="/pub/databases/interpro/"
export INGEST_INTERPRO_FTP_TIMEOUT_SECS="300"

# Processing Configuration
export INGEST_INTERPRO_BATCH_SIZE="500"

# Scheduling
export INGEST_INTERPRO_AUTO_ENABLED="false"
export INGEST_INTERPRO_SCHEDULE="0 2 * * *"  # Daily at 2 AM
```

## Testing

```bash
# Unit tests
cargo test --package bdp-server interpro

# Version discovery tests
cargo test --package bdp-server version_discovery

# Integration tests
cargo test --package bdp-server interpro_version_discovery_test
```

## Performance

### Version Discovery
- Time: ~5-10 seconds
- Network: 2-3 FTP commands

### Single Version Ingestion
- Time: ~10-30 minutes
- Network: ~1-2 GB download
- Memory: ~500 MB peak
- Disk: ~2-3 GB (compressed files)

### Historical Ingestion (10 versions)
- Time: ~2-5 hours
- Network: ~10-20 GB
- Disk: ~20-30 GB

## Documentation

- [Version Discovery Guide](../../../../docs/interpro-version-discovery.md) - Complete documentation
- [Implementation Summary](../../../../docs/agents/implementation/interpro-version-discovery-implementation.md) - Technical details

## Examples

### 1. Version Discovery
```bash
cargo run --example interpro_version_discovery
```

### 2. Historical Ingestion
```bash
# From specific version
cargo run --example interpro_historical_ingestion -- 96.0

# Single version
cargo run --example interpro_historical_ingestion -- 98.0 --single
```

## Troubleshooting

### FTP Connection Issues

```bash
# Test FTP manually
ftp ftp.ebi.ac.uk
> cd /pub/databases/interpro/
> ls
```

### Firewall Issues

Ensure FTP passive mode is allowed through your firewall.

### Timeout Issues

```bash
# Increase timeout
export INGEST_INTERPRO_FTP_TIMEOUT_SECS="600"
```

## Future Enhancements

- Parallel version downloads
- Delta/incremental updates
- Checksum verification
- Resume failed ingestion
- Exact release date parsing

## Related Modules

- [UniProt](../uniprot/) - Similar version discovery pattern
- [Gene Ontology](../gene_ontology/) - GO term integration
- [NCBI Taxonomy](../ncbi_taxonomy/) - Organism taxonomy

## License

Part of the BDP project.
