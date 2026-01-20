# Gene Ontology Pipeline Restructure

## Summary

The GO ingestion pipeline has been restructured to follow the exact same pattern as UniProt, GenBank, and NCBI Taxonomy pipelines in BDP.

## Key Changes

### 1. Pipeline Constructor

**Before:**
```rust
let pipeline = GoPipeline::new(db, organization_id, config)?;
```

**After:**
```rust
use crate::storage::{Storage, config::StorageConfig};

// Create S3 storage
let storage_config = StorageConfig::from_env()?;
let s3 = Storage::new(storage_config).await?;

// Create pipeline with S3
let pipeline = GoPipeline::new(config, db, s3, organization_id);
```

### 2. Automatic S3 Upload

All downloaded files are now automatically uploaded to S3:

- **Ontology files**: `s3://bucket/go/ontology/{version}/go-basic.obo`
- **Annotation files**: `s3://bucket/go/annotations/{version}/goa_{organism}.gaf`

### 3. Pipeline Steps Updated

**Ontology Ingestion** (`run_ontology`):
1. Download OBO from Zenodo/HTTP
2. **Upload to S3** ← NEW
3. Parse OBO content
4. Store to PostgreSQL

**Annotation Ingestion** (`run_organism_annotations`):
1. Build protein lookup map
2. Download GAF from FTP
3. **Upload to S3** ← NEW
4. Parse annotations
5. Store to PostgreSQL

### 4. Storage Pattern

Follows the same pattern as other BDP pipelines:

| Pipeline | S3 Required | Pattern |
|----------|-------------|---------|
| GenBank | ✅ Yes | `new(config, db, s3)` |
| UniProt | ✅ Yes | `new(pool, org_id, config, batch_config, storage, cache_dir)` |
| NCBI Taxonomy | ⚠️ Optional | `with_s3(config, db, s3)` |
| **GO** | ✅ **Yes** | **`new(config, db, s3, organization_id)`** |

## Updated API

### GoPipeline::new()

```rust
pub fn new(
    config: GoHttpConfig,
    db: PgPool,
    s3: Storage,
    organization_id: Uuid,
) -> Self
```

**Parameters:**
- `config`: GoHttpConfig - Download configuration (Zenodo, FTP)
- `db`: PgPool - PostgreSQL connection pool
- `s3`: Storage - S3 storage for file archival
- `organization_id`: Uuid - Organization ID

### Helper Methods

```rust
// Get S3 storage reference
pipeline.s3_storage() -> &Storage

// Create new database storage instance
pipeline.create_storage() -> GoStorage

// Get configuration
pipeline.config() -> &GoHttpConfig
```

## Migration Guide

### For Test Binaries

**Old code:**
```rust
let config = GoHttpConfig::zenodo_config(...);
let pipeline = GoPipeline::new(db, org_id, config)?;
pipeline.run_ontology("1.0").await?;
```

**New code:**
```rust
use bdp_server::storage::{Storage, config::StorageConfig};

let config = GoHttpConfig::zenodo_config(...);

// Create S3 storage
let storage_config = StorageConfig::from_env()?;
let s3 = Storage::new(storage_config).await?;

// Create pipeline
let pipeline = GoPipeline::new(config, db, s3, org_id);
pipeline.run_ontology("1.0").await?;
```

### For Production Code

```rust
use bdp_server::ingest::gene_ontology::{GoHttpConfig, GoPipeline};
use bdp_server::storage::{Storage, config::StorageConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load config
    let app_config = Config::load()?;

    // Connect to database
    let db = PgPool::connect(&app_config.database.url).await?;

    // Create S3 storage
    let storage_config = StorageConfig::from_env()?;
    let s3 = Storage::new(storage_config).await?;

    // Configure GO ingestion
    let go_config = GoHttpConfig::zenodo_config(
        "data/go/go-basic.obo".to_string(),
        "2025-09-08",
        "10.5281/zenodo.17382285",
    );

    // Create pipeline
    let pipeline = GoPipeline::new(
        go_config,
        db,
        s3,
        organization_id,
    );

    // Run ingestion
    pipeline.run_ontology("1.0").await?;
    pipeline.run_organism_annotations("human").await?;

    Ok(())
}
```

## S3 Storage Structure

```
s3://bucket/
├── go/
│   ├── ontology/
│   │   ├── 2025-09-08/
│   │   │   └── go-basic.obo
│   │   └── 2026-01-01/
│   │       └── go-basic.obo
│   └── annotations/
│       ├── current/
│       │   ├── goa_human.gaf
│       │   ├── goa_mouse.gaf
│       │   └── goa_uniprot_all.gaf
│       └── 2025-09-15/
│           ├── goa_human.gaf
│           └── goa_mouse.gaf
```

## Environment Variables

Required for S3 storage:

```bash
# S3/MinIO Configuration
S3_ENDPOINT=http://localhost:9000  # Optional, for MinIO
S3_REGION=us-east-1
S3_BUCKET=bdp-data
S3_ACCESS_KEY=minioadmin
S3_SECRET_KEY=minioadmin
S3_PATH_STYLE=true  # true for MinIO, false for AWS S3

# GO Configuration
GO_LOCAL_ONTOLOGY_PATH=data/go/go-basic.obo  # Optional
GO_RELEASE_VERSION=2025-09-08
GO_ZENODO_DOI=10.5281/zenodo.17382285
```

## Benefits

### 1. Consistency with BDP Patterns
- Follows same structure as UniProt, GenBank, NCBI Taxonomy
- Easier to understand and maintain
- Familiar API for developers

### 2. Automatic Archival
- All downloaded files automatically backed up to S3
- Version-specific paths for easy retrieval
- Supports disaster recovery

### 3. Separation of Concerns
- Download logic: `GoDownloader`
- Storage logic: `GoStorage` (PostgreSQL) + `Storage` (S3)
- Pipeline orchestration: `GoPipeline`

### 4. Testability
- Can mock S3 storage for tests
- Explicit dependencies
- Clear data flow

## Breaking Changes

❗ **All test binaries need to be updated** to use the new API:

- `go_test_ftp.rs` - FTP connection test
- `go_test_human.rs` - Human annotations test
- `go_test_sample.rs` - Sample pipeline test
- `go_test_local_ontology.rs` - Local ontology test

## Next Steps

1. ✅ Pipeline restructured
2. ✅ S3 upload integrated
3. ✅ Library compiles successfully
4. ⏳ Update test binaries to use new API
5. ⏳ Test with MinIO/S3
6. ⏳ Update documentation

## Status

- **Library**: ✅ Compiles successfully
- **Test Binaries**: ⚠️ Need updates
- **Documentation**: ⚠️ Needs review

---

**Last Updated**: 2026-01-20
**Breaking Change**: Yes - requires S3 storage parameter
