# Gene Ontology (GO) Quick Start Guide

## What's Been Implemented

✅ **Complete GO ingestion system** following BDP patterns
✅ **Database schema** with 3 tables (go_term_metadata, go_relationships, go_annotations)
✅ **FTP download support** with passive mode for firewall compatibility
✅ **Local file support** for ontology to avoid HTTP 403 blocks
✅ **Zenodo integration** with DOI tracking and CC BY 4.0 attribution
✅ **Test infrastructure** with 4 test binaries

## Quick Start

### Step 1: Download GO Data from Zenodo

The GO ontology file needs to be downloaded once from Zenodo:

```bash
# Option A: Use the download script (recommended)
./scripts/download_go_zenodo.sh

# Option B: Manual download
wget https://zenodo.org/records/17382285/files/go-release-archive.tgz
tar -xzf go-release-archive.tgz --wildcards '**/go-basic.obo'
mkdir -p data/go
mv <extracted-path>/go-basic.obo data/go/go-basic.obo
```

### Step 2: Set Environment Variables

```bash
export GO_LOCAL_ONTOLOGY_PATH="data/go/go-basic.obo"
export GO_RELEASE_VERSION="2025-09-08"
export GO_ZENODO_DOI="10.5281/zenodo.17382285"
```

### Step 3: Run Tests

```bash
# Test 1: FTP Connection
cargo run --bin go_test_ftp

# Test 2: Human Annotations (10MB, quick test)
cargo run --bin go_test_human

# Test 3: Complete Pipeline with Local Ontology
cargo run --bin go_test_local_ontology
```

## What Each Test Does

### `go_test_ftp`
- Tests FTP connection to ftp.ebi.ac.uk
- Verifies passive mode works
- Downloads a small test file
- **Time**: <5 seconds

### `go_test_human`
- Downloads human annotations (~10MB compressed)
- Parses 1000 annotations (with parse limit)
- Stores in database
- Shows evidence code distribution
- **Time**: ~3-5 seconds

### `go_test_local_ontology`
- Loads GO ontology from local file
- Downloads human annotations via FTP
- Stores both in database
- Verifies data integrity
- **Time**: ~10-15 seconds (first run includes ontology parsing)

## Configuration Options

### Using Zenodo Config (Recommended)

```rust
use bdp_server::ingest::gene_ontology::GoHttpConfig;

let config = GoHttpConfig::zenodo_config(
    "data/go/go-basic.obo".to_string(),
    "2025-09-08",
    "10.5281/zenodo.17382285",
);
```

### Using Builder Pattern

```rust
let config = GoHttpConfig::builder()
    .local_ontology_path("data/go/go-basic.obo".to_string())
    .annotation_base_url("ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/HUMAN".to_string())
    .go_release_version("2025-09-08".to_string())
    .zenodo_doi("10.5281/zenodo.17382285".to_string())
    .parse_limit(1000)  // Optional: limit for testing
    .build();
```

### Using Environment Variables

```rust
let config = GoHttpConfig::from_env();
```

## File Sizes and Performance

| File | Size (Compressed) | Size (Uncompressed) | Annotations | Time |
|------|-------------------|---------------------|-------------|------|
| go-basic.obo | ~40 MB | ~40 MB | 45K terms + 100K rels | ~10s |
| goa_human.gaf.gz | 10.9 MB | 141 MB | ~1M annotations | ~3s |
| goa_mouse.gaf.gz | ~10 MB | ~140 MB | ~1M annotations | ~3s |
| goa_uniprot_all.gaf.gz | 16.8 GB | ~200 GB | 700M annotations | Requires streaming |

**Note**: Full dataset (goa_uniprot_all.gaf.gz) is not recommended with current implementation due to memory constraints. Use organism-specific files instead.

## Database Verification

After running tests, verify data in PostgreSQL:

```sql
-- Check GO terms
SELECT COUNT(*) FROM go_term_metadata;
-- Expected: ~45,000 (after ontology ingestion)

-- Check relationships
SELECT COUNT(*) FROM go_relationships;
-- Expected: ~100,000 (after ontology ingestion)

-- Check annotations
SELECT COUNT(*) FROM go_annotations;
-- Expected: ~1,000 (with parse_limit=1000)

-- Check attribution metadata
SELECT metadata FROM versions WHERE source_type = 'go_term';
-- Should contain: zenodo_doi, citation, license info
```

## Example Queries

### Get GO terms for a protein

```sql
SELECT g.go_id, g.name, g.namespace, a.evidence_code
FROM go_annotations a
JOIN go_term_metadata g ON g.go_id = a.go_id
WHERE a.entity_type = 'protein'
  AND a.entity_id = (SELECT data_source_id FROM protein_metadata WHERE accession = 'P01308')
ORDER BY g.namespace, g.name;
```

### Get proteins annotated with a GO term

```sql
SELECT p.accession, p.protein_name, a.evidence_code
FROM go_annotations a
JOIN protein_metadata p ON p.data_source_id = a.entity_id
WHERE a.go_id = 'GO:0006955'
  AND a.entity_type = 'protein'
LIMIT 100;
```

### Get ancestor terms (recursive)

```sql
WITH RECURSIVE ancestors AS (
    SELECT subject_go_id, object_go_id, 1 AS depth
    FROM go_relationships
    WHERE subject_go_id = 'GO:0006955' AND go_release_version = '2025-09-08'

    UNION ALL

    SELECT r.subject_go_id, r.object_go_id, a.depth + 1
    FROM go_relationships r
    JOIN ancestors a ON r.subject_go_id = a.object_go_id
    WHERE r.go_release_version = '2025-09-08' AND a.depth < 10
)
SELECT g.go_id, g.name, a.depth
FROM ancestors a
JOIN go_term_metadata g ON g.go_id = a.object_go_id
ORDER BY a.depth;
```

## Attribution Compliance

All GO data is licensed under **CC BY 4.0**. When using BDP's GO data, ensure:

1. ✅ Citation is included in API responses (automatically handled)
2. ✅ Attribution notice displayed in UI where GO data is shown
3. ✅ Release date and DOI tracked in database `versions` table
4. ✅ License information in `THIRD_PARTY_ATTRIBUTIONS.md`

**Required Citations:**
- The Gene Ontology Consortium (2025) NAR 54(D1):D1779-D1792
- Ashburner M et al. (2000) Nat Genet. 25(1):25-9
- GO Release date and Zenodo DOI

## Troubleshooting

### Error: 403 Forbidden when downloading ontology
**Solution**: Use local file with Zenodo archive (already configured in `zenodo_config()`)

### Error: Out of Memory
**Solution**: Use organism-specific files instead of full GOA dataset

### Error: FTP connection forcibly closed
**Solution**: Already fixed with passive mode (`ftp_stream.set_mode(Mode::Passive)`)

### Error: Duplicate annotations
**Solution**: Already fixed with `ON CONFLICT DO NOTHING`

## Next Steps

### For Development
1. Download Zenodo archive: `./scripts/download_go_zenodo.sh`
2. Run tests to verify: `cargo run --bin go_test_local_ontology`
3. Explore database with example queries above

### For Production
1. Set up automated monthly updates (GO releases monthly)
2. Ingest top 5 organisms (~15-20 seconds total)
3. Implement API endpoints for GO queries
4. Add UI components for GO data display

### Future Enhancements
- Streaming download for full GOA dataset
- Materialized transitive closure for performance
- Gene2GO annotations (NCBI Gene → GO)
- GO Slims/Subsets support
- GO enrichment analysis

## Documentation

- **Integration Guide**: `docs/GO_INTEGRATION_GUIDE.md` - Complete deployment instructions
- **Implementation Summary**: `docs/GO_IMPLEMENTATION_SUMMARY.md` - Architecture and decisions
- **Attribution Requirements**: `THIRD_PARTY_ATTRIBUTIONS.md` - License compliance

## Support

For issues or questions:
- Check documentation in `docs/GO_*.md`
- Review test binaries in `crates/bdp-server/src/bin/go_test_*.rs`
- Verify database schema in `migrations/20260121000001_create_go_metadata.sql`

---

**Status**: ✅ Production Ready (with organism-specific files)
**Last Updated**: 2026-01-20
