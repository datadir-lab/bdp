# Gene Ontology Version Discovery and Historical Ingestion

## Overview

This document describes the implementation of version discovery and historical ingestion for Gene Ontology (GO) data in the BDP system.

## Architecture

### Version Discovery Service

The `VersionDiscovery` service discovers available GO versions from the HTTP release archive at `http://release.geneontology.org/`.

**Key Features:**
- HTTP directory listing parsing
- Date-based version extraction (YYYY-MM-DD format)
- Chronological sorting
- Database integration for tracking ingested versions
- Filtering support for backfill operations

### Components

#### 1. Version Discovery (`version_discovery.rs`)

```rust
pub struct VersionDiscovery {
    config: GoHttpConfig,
    client: Client,
}

pub struct DiscoveredVersion {
    pub external_version: String,    // "2025-01-01"
    pub release_date: NaiveDate,     // Parsed date
    pub release_url: String,         // Full URL to release
}
```

**Main Methods:**
- `discover_all_versions()` - Discovers all available versions from HTTP archive
- `discover_versions_since(date)` - Filters to versions after a cutoff date
- `filter_new_versions()` - Filters out already-ingested versions
- `get_ingested_versions(pool, entry_id)` - Queries database for ingested versions
- `check_for_newer_version(pool, entry_id)` - Checks if updates are available

#### 2. Pipeline Integration (`pipeline.rs`)

Updated `GoPipeline` to support version parameters:

```rust
// New method with version parameter
pub async fn run_ontology_version(
    &self,
    internal_version: &str,
    external_version: Option<&str>,
) -> Result<PipelineStats>

// Original method (backward compatible)
pub async fn run_ontology(&self, internal_version: &str) -> Result<PipelineStats>
```

#### 3. Configuration Updates (`config.rs`)

Added version-aware URL building:

```rust
impl GoHttpConfig {
    pub fn ontology_url_for_version(&self, version: Option<&str>) -> String {
        let ver = version.unwrap_or(&self.go_release_version);
        format!(
            "http://release.geneontology.org/{}/ontology/go-basic.obo",
            ver
        )
    }
}
```

#### 4. Downloader Updates (`downloader.rs`)

Added version listing and versioned downloads:

```rust
impl GoDownloader {
    pub async fn download_ontology_version(&self, version: Option<&str>) -> Result<String>
    pub async fn list_available_versions(&self) -> Result<Vec<String>>
}
```

## Version Format

Gene Ontology uses date-based versioning:

- **Format**: `YYYY-MM-DD` (e.g., `2025-01-01`)
- **Release Frequency**: Monthly (typically first of month)
- **Archive Location**: `http://release.geneontology.org/{version}/`

Example URLs:
```
http://release.geneontology.org/2025-01-01/ontology/go-basic.obo
http://release.geneontology.org/2024-12-01/ontology/go-basic.obo
http://release.geneontology.org/2024-11-01/ontology/go-basic.obo
```

## HTTP Directory Parsing

The version discovery uses HTML parsing to extract dated directories:

1. **Fetch HTML**: Download directory listing from release archive
2. **Parse Links**: Extract all `<a>` tags using the `scraper` crate
3. **Filter Dates**: Match hrefs against regex pattern `^\d{4}-\d{2}-\d{2}/?$`
4. **Validate**: Parse dates using `chrono::NaiveDate::from_ymd_opt()`
5. **Sort**: Order chronologically (oldest to newest)

**Regex Pattern:**
```rust
let date_pattern = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})/?$")?;
```

This matches:
- ✅ `2025-01-01`
- ✅ `2025-01-01/`
- ❌ `2025-1-1` (missing leading zeros)
- ❌ `2025_01_01` (wrong separator)

## Database Integration

### Querying Ingested Versions

```rust
// Get all ingested versions for a GO data source
let ingested = discovery.get_ingested_versions(pool, entry_id).await?;

// Get the most recent ingested version
let last = discovery.get_last_ingested_version(pool, entry_id).await?;

// Check if a specific version exists
let exists = discovery.version_exists_in_db(pool, entry_id, "2025-01-01").await?;
```

### Version Storage

Versions are stored in the `versions` table:

```sql
SELECT
    version,           -- Internal version (e.g., "1.0")
    external_version,  -- GO version (e.g., "2025-01-01")
    release_date,      -- Date parsed from external_version
    entry_id           -- Links to registry_entries
FROM versions
WHERE entry_id = $1
ORDER BY release_date DESC;
```

## Usage Examples

### 1. Discover Available Versions

```bash
# Simple discovery test
cargo run --example test_go_version_discovery

# Discover all versions
cargo run --example go_historical_ingestion -- discover
```

Output:
```
Found 48 available GO versions:

Version         Date         URL
--------------------------------------------------------------------------------
2025-01-01      2025-01-01   http://release.geneontology.org/2025-01-01/
2024-12-01      2024-12-01   http://release.geneontology.org/2024-12-01/
2024-11-01      2024-11-01   http://release.geneontology.org/2024-11-01/
...
```

### 2. Check for New Versions

```bash
cargo run --example go_historical_ingestion -- check
```

Output:
```
Discovered 48 total versions
Already ingested 40 versions

📦 Found 8 new versions to ingest:

Version         Date
------------------------------
2024-09-01      2024-09-01
2024-10-01      2024-10-01
2024-11-01      2024-11-01
...

💡 Run with 'backfill 2024-09-01' to ingest these versions
```

### 3. Backfill Historical Versions

```bash
# Ingest all versions from 2024-01-01 onwards
cargo run --example go_historical_ingestion -- backfill 2024-01-01

# Ingest versions within a specific range
cargo run --example go_historical_ingestion -- backfill-range 2024-01-01 2024-12-31
```

Output:
```
📦 Will ingest 12 GO versions:
  - 2024-01-01 (2024-01-01)
  - 2024-02-01 (2024-02-01)
  - 2024-03-01 (2024-03-01)
  ...

Processing version 1/12: 2024-01-01
✅ Ingested 2024-01-01: 45123 terms, 89234 relationships

Processing version 2/12: 2024-02-01
✅ Ingested 2024-02-01: 45234 terms, 89456 relationships

...

✅ Historical backfill complete!
```

### 4. Ingest a Specific Version

```bash
cargo run --example go_historical_ingestion -- ingest 2025-01-01
```

Output:
```
Ingesting GO version: 2025-01-01

Step 1/4: Downloading GO ontology...
Downloaded GO ontology: 42351616 bytes (41359 KB)

Step 2/4: Uploading ontology to S3...
Uploaded ontology to S3: go/ontology/2025-01-01/go-basic.obo

Step 3/4: Parsing GO ontology...
Parsed 45678 terms and 92345 relationships

Step 4/4: Storing GO ontology...

✅ Ingestion complete!
   Terms stored: 45678
   Relationships stored: 92345
```

## Programmatic Usage

### Basic Version Discovery

```rust
use bdp_server::ingest::gene_ontology::{GoHttpConfig, VersionDiscovery};

let config = GoHttpConfig::default();
let discovery = VersionDiscovery::new(config)?;

// Discover all versions
let versions = discovery.discover_all_versions().await?;

for version in versions {
    println!("{}: {}", version.external_version, version.release_date);
}
```

### Filter to Unprocessed Versions

```rust
// Get versions since 2024-01-01
let cutoff = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
let recent = discovery.discover_versions_since(cutoff).await?;

// Get already-ingested versions from database
let ingested = discovery.get_ingested_versions(&pool, entry_id).await?;

// Filter to new versions only
let to_ingest = discovery.filter_new_versions(recent, ingested);

println!("Found {} versions to ingest", to_ingest.len());
```

### Ingest a Specific Version

```rust
use bdp_server::ingest::gene_ontology::{GoHttpConfig, GoPipeline};

// Create config for specific version
let config = GoHttpConfig::builder()
    .go_release_version("2025-01-01".to_string())
    .build();

// Create pipeline
let pipeline = GoPipeline::new(config, db, s3, organization_id);

// Run ingestion
let stats = pipeline
    .run_ontology_version("1.0", Some("2025-01-01"))
    .await?;

println!("Stored {} terms and {} relationships",
    stats.terms_stored, stats.relationships_stored);
```

## Backfill Strategy

### Recommended Approach

1. **Start Date**: Choose a reasonable starting point (e.g., 2024-01-01)
2. **Chronological Order**: Ingest oldest to newest
3. **Error Handling**: Continue on failure (log and skip)
4. **Batch Size**: Process all at once or in date ranges
5. **Monitoring**: Track progress and failures

### Performance Considerations

- **Download Size**: ~40MB per OBO file
- **Parse Time**: ~2-5 seconds per file
- **Storage Time**: ~5-10 seconds per version
- **Total Time**: ~15-20 seconds per version

For 12 months of backfill:
- **Estimated Time**: 3-4 minutes
- **Storage Impact**: ~500MB (compressed)
- **Database Growth**: ~50k terms × 12 versions = 600k rows

### Error Recovery

The backfill process is designed to be resilient:

```rust
for version in versions {
    match ingest_version(&pipeline, &version.external_version).await {
        Ok(stats) => {
            info!("✅ Ingested {}", version.external_version);
        }
        Err(e) => {
            warn!("❌ Failed to ingest {}: {}", version.external_version, e);
            // Continue with next version
        }
    }
}
```

## Dependencies

### New Crate Dependencies

Added to `Cargo.toml`:

```toml
scraper = "0.22"  # HTML parsing for GO version discovery
```

### Existing Dependencies Used

- `regex` - Pattern matching for date extraction
- `chrono` - Date parsing and manipulation
- `reqwest` - HTTP client for directory listing
- `sqlx` - Database queries for version tracking

## Testing

### Unit Tests

```bash
# Run all GO tests
cargo test --package bdp-server --lib gene_ontology

# Run version discovery tests specifically
cargo test --package bdp-server --lib gene_ontology::version_discovery
```

### Integration Tests

```bash
# Test version discovery (requires network)
cargo test --package bdp-server --lib gene_ontology::version_discovery -- --ignored

# Run test example
cargo run --example test_go_version_discovery
```

### Manual Testing

```bash
# 1. Test discovery
cargo run --example test_go_version_discovery

# 2. Check for new versions (requires database)
cargo run --example go_historical_ingestion -- check

# 3. Dry run (discover only, no ingestion)
cargo run --example go_historical_ingestion -- discover
```

## Comparison with Other Data Sources

### UniProt Version Discovery

- **Source**: FTP directory listing
- **Format**: `YYYY_MM` (e.g., `2025_01`)
- **Path**: `/pub/databases/uniprot/previous_releases/release-YYYY_MM/`
- **Method**: FTP LIST command

### NCBI Taxonomy Version Discovery

- **Source**: FTP `new_taxdump/` directory
- **Format**: `taxdump_YYYY-MM-DD.tar.gz`
- **Method**: FTP LIST command, regex extraction

### Gene Ontology Version Discovery

- **Source**: HTTP release archive
- **Format**: `YYYY-MM-DD` (e.g., `2025-01-01`)
- **Path**: `http://release.geneontology.org/YYYY-MM-DD/`
- **Method**: HTTP HTML parsing, link extraction

## Future Enhancements

### Potential Improvements

1. **Caching**: Cache discovered versions for 24 hours
2. **Parallel Downloads**: Download multiple versions concurrently
3. **Checksums**: Verify file integrity using MD5/SHA checksums
4. **Differential Updates**: Track what changed between versions
5. **Version Metadata**: Store additional metadata (file sizes, dates, etc.)
6. **Notification**: Alert when new versions are available
7. **Scheduling**: Automatic daily/weekly checks for updates

### API Integration

Future work could add REST endpoints:

```
GET  /api/go/versions              # List all available versions
GET  /api/go/versions/latest       # Get latest version
GET  /api/go/versions/ingested     # Get ingested versions
POST /api/go/versions/ingest       # Trigger ingestion
GET  /api/go/versions/check        # Check for updates
```

## Troubleshooting

### Common Issues

**Issue**: "Failed to fetch release archive page"
- **Cause**: Network connectivity or GO website down
- **Solution**: Retry with exponential backoff, check network

**Issue**: "No versions discovered"
- **Cause**: HTML structure changed or parsing error
- **Solution**: Check HTML format, update selectors

**Issue**: "Version already exists"
- **Cause**: Attempting to re-ingest a version
- **Solution**: Use `check` command to see what's already ingested

**Issue**: "Invalid date format"
- **Cause**: Non-standard directory name in listing
- **Solution**: Regex validation prevents invalid dates

### Debugging

Enable detailed logging:

```bash
RUST_LOG=debug cargo run --example test_go_version_discovery
```

This shows:
- HTTP requests and responses
- HTML parsing details
- Date validation results
- Filtering logic

## References

- [Gene Ontology Release Archive](http://release.geneontology.org/)
- [GO Data Archive Documentation](http://geneontology.org/docs/download-ontology/)
- [OBO Format Specification](http://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html)
- [BDP UniProt Version Discovery](../crates/bdp-server/src/ingest/uniprot/version_discovery.rs)
- [scraper crate](https://docs.rs/scraper/)
