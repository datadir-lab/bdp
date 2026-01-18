# UniProt Ingestion Pipeline

This module provides functionality to parse and ingest UniProt protein data into the BDP database.

## Architecture

The ingestion pipeline consists of several components:

### 1. Parser (`parser.rs`)
- Parses UniProt DAT (flat file) format
- Extracts protein metadata, sequences, taxonomy information
- Handles compressed `.dat.gz` files

### 2. Storage (`storage.rs`)
- Creates database records following BDP schema
- Each protein becomes its own data source with:
  - `registry_entry` (with unique slug = accession)
  - `data_source` (linked to organism)
  - `protein_metadata` (extends data_source)
  - `version` (e.g., "1.0" internal, "2024_01" external)
  - `version_files` (DAT format record)

### 3. FTP Client (`ftp.rs`)
- Downloads data from UniProt FTP server
- Handles release notes and DAT files
- Supports gzip decompression

### 4. Configuration (`config.rs`)
- FTP paths and connection settings
- Parse limits for testing

## Usage

### Basic Example

```rust
use bdp_server::ingest::uniprot::{DatParser, UniProtStorage};
use sqlx::PgPool;
use std::path::Path;

// Parse UniProt DAT file
let parser = DatParser::new();
let entries = parser.parse_file(Path::new("uniprot_sprot.dat"))?;

// Store in database
let storage = UniProtStorage::new(
    db_pool.clone(),
    organization_id,      // UUID of UniProt organization
    "1.0".to_string(),    // Internal version
    "2024_01".to_string() // External version (from UniProt)
);

let stored_count = storage.store_entries(&entries).await?;
println!("Stored {} proteins", stored_count);
```

### With FTP Download

```rust
use bdp_server::ingest::uniprot::{UniProtFtp, UniProtFtpConfig, DatParser};

// Configure FTP
let config = UniProtFtpConfig::new()
    .with_parse_limit(100); // Limit for testing

// Download release notes
let ftp = UniProtFtp::new(config);
let notes = ftp.download_release_notes("2024_01").await?;
let release_info = ftp.parse_release_notes(&notes)?;

// Download DAT file
let dat_data = ftp.download_dat_file("2024_01").await?;

// Parse and store
let parser = DatParser::new();
let entries = parser.parse_bytes(&dat_data)?;
```

### Database Schema

Each protein ingestion creates the following records:

1. **Organizations** (created once)
   ```sql
   INSERT INTO organizations (id, slug, name, is_system)
   VALUES (uuid, 'uniprot', 'Universal Protein Resource', true);
   ```

2. **Organisms** (created/reused per taxonomy ID)
   ```sql
   INSERT INTO organisms (id, ncbi_taxonomy_id, scientific_name)
   VALUES (uuid, 9606, 'Homo sapiens');
   ```

3. **Registry Entry** (per protein)
   ```sql
   INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
   VALUES (uuid, org_id, 'Q6GZX4', 'Protein name [Organism]', 'data_source');
   ```

4. **Data Source** (per protein)
   ```sql
   INSERT INTO data_sources (id, source_type, external_id, organism_id)
   VALUES (entry_id, 'protein', 'Q6GZX4', organism_id);
   ```

5. **Protein Metadata** (per protein)
   ```sql
   INSERT INTO protein_metadata (
       data_source_id, accession, entry_name, protein_name, gene_name,
       sequence_length, mass_da, sequence_checksum
   )
   VALUES (entry_id, 'Q6GZX4', 'ENTRY_NAME', 'Protein Name', 'GENE',
           100, 11937, 'sha256hash');
   ```

6. **Version** (per protein, per release)
   ```sql
   INSERT INTO versions (entry_id, version, external_version)
   VALUES (entry_id, '1.0', '2024_01');
   ```

7. **Version Files** (per version, per format)
   ```sql
   INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
   VALUES (version_id, 'dat', 'proteins/uniprot/Q6GZX4/1.0/Q6GZX4.dat',
           'sha256hash', 1024);
   ```

## Testing

Run E2E tests:

```bash
cd crates/bdp-server
cargo test --test e2e_parser_tests -- --nocapture
```

Tests include:
- `test_parse_ci_sample` - Validates parser can read DAT files
- `test_parse_and_store` - Full pipeline from parsing to database
- `test_parse_invalid_data` - Error handling for malformed data

## Configuration

Environment variables:

```bash
# Database (required)
DATABASE_URL=postgresql://user:pass@localhost/bdp

# Ingestion (optional)
INGEST_ENABLED=true
INGEST_WORKER_THREADS=4

# UniProt FTP (optional)
UNIPROT_AUTO_INGEST_ENABLED=false
UNIPROT_FTP_HOST=ftp.uniprot.org
UNIPROT_FTP_BASE_PATH=/pub/databases/uniprot/current_release
```

## Error Handling

The storage layer continues on individual entry failures:

```rust
for entry in entries {
    if let Err(e) = self.store_entry(entry).await {
        debug!("Failed to store entry {}: {}", entry.accession, e);
        continue; // Don't fail entire batch
    }
    stored_count += 1;
}
```

This ensures partial success when some entries have data issues.

## Performance

For large datasets (e.g., full SwissProt with 570k proteins):

1. **Batch Processing**: Process in chunks of 1000
2. **Connection Pooling**: Use adequate pool size (20-50 connections)
3. **Parallel Processing**: Use tokio tasks with semaphore
4. **Progress Tracking**: Log every 1000 entries
5. **Idempotency**: ON CONFLICT clauses allow re-running

## Future Enhancements

- [ ] FASTA and XML format support in version_files
- [ ] Parallel FTP downloads
- [ ] Incremental updates (only new/modified proteins)
- [ ] Aggregate data source (uniprot:all@version)
- [ ] Citation parsing and storage
- [ ] Full-text search index updates
- [ ] S3 upload integration
- [ ] Progress webhooks/notifications

## Related Documentation

- [Database Schema](../../../../../docs/agents/design/database-schema.md)
- [UniProt Ingestion Strategy](../../../../../docs/agents/design/uniprot-ingestion.md)
- [SQLX Implementation Guide](../../../../../docs/agents/implementation/sqlx-guide.md)
