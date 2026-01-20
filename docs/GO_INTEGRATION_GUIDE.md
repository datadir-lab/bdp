# Gene Ontology (GO) Integration Guide

This guide explains how to integrate Gene Ontology data into BDP with proper attribution and licensing compliance.

## Table of Contents

- [Quick Start](#quick-start)
- [Data Sources](#data-sources)
- [Attribution Requirements](#attribution-requirements)
- [Configuration Options](#configuration-options)
- [Download Options](#download-options)
- [Testing](#testing)
- [Production Deployment](#production-deployment)

---

## Quick Start

### 1. Download GO Data from Zenodo

```bash
# Download latest GO release from Zenodo (21.4 GB compressed)
wget https://zenodo.org/records/17382285/files/go-release-archive.tgz

# Extract go-basic.obo
tar -xzf go-release-archive.tgz --wildcards '**/go-basic.obo'

# Move to data directory
mkdir -p data/go
mv <extracted-path>/go-basic.obo data/go/go-basic.obo
```

### 2. Configure BDP

```rust
use bdp_server::ingest::gene_ontology::GoHttpConfig;

// Configure with local ontology file + FTP annotations
let config = GoHttpConfig::zenodo_config(
    "data/go/go-basic.obo".to_string(),
    "2025-09-08",  // Release date
    "10.5281/zenodo.17382285",  // Zenodo DOI
);
```

### 3. Run Ingestion

```bash
# Test with human annotations (recommended for initial testing)
cargo run --bin go_test_human

# Or ingest multiple organisms
cargo run --bin go_ingest_multi_organism
```

---

## Data Sources

### GO Ontology (Terms + Relationships)

**Source**: Zenodo DOI Archives
**URL**: https://doi.org/10.5281/zenodo.1205166
**Format**: OBO (go-basic.obo, ~40 MB uncompressed)
**License**: CC BY 4.0

**Why Zenodo?**
- Official versioned releases with DOI
- Not behind Cloudflare (HTTP sources return 403)
- Required for reproducible research
- Complete release bundles

### GO Annotations (Protein → GO Mappings)

**Source**: EBI FTP Server
**URL**: ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/
**Format**: GAF 2.2 (gzipped)
**License**: CC BY 4.0

**Available Files:**
- `goa_uniprot_all.gaf.gz` - All UniProt annotations (16.8 GB, 700M annotations)
- `goa_human.gaf.gz` - Human only (10 MB, ~1M annotations)
- `goa_mouse.gaf.gz` - Mouse only
- `goa_<organism>.gaf.gz` - Other organisms

---

## Attribution Requirements

### License

Gene Ontology data is licensed under **Creative Commons Attribution 4.0 International (CC BY 4.0)**.

### Required Attribution

When using GO data in BDP, you must:

1. **Cite the 2026 GO paper:**
   ```
   The Gene Ontology Consortium (2025)
   "The Gene Ontology knowledgebase in 2026"
   Nucleic Acids Research, 54(D1):D1779-D1792
   doi: 10.1093/nar/gkaf1292
   ```

2. **Cite the original 2000 paper:**
   ```
   Ashburner M, Ball CA, Blake JA, et al. (2000)
   "Gene ontology: tool for the unification of biology"
   Nat Genet. 25(1):25-9.
   doi: 10.1038/75556
   ```

3. **Include release information:**
   ```
   GO Release: 2025-09-08
   Zenodo DOI: 10.5281/zenodo.17382285
   ```

4. **Display attribution notice:**
   ```
   "Gene Ontology data from the 2025-09-08 release (DOI: 10.5281/zenodo.17382285)
   is made available under the terms of the Creative Commons Attribution 4.0
   International license (CC BY 4.0)."
   ```

### BDP Implementation

Attribution is automatically included via:

- **Database**: `versions` table stores DOI and citation in metadata
- **API**: Response includes `attribution` field with citation info
- **UI**: Attribution displayed when showing GO data
- **Documentation**: [THIRD_PARTY_ATTRIBUTIONS.md](../THIRD_PARTY_ATTRIBUTIONS.md)

---

## Configuration Options

### Option 1: Zenodo Archive (Recommended)

```rust
let config = GoHttpConfig::zenodo_config(
    "data/go/go-basic.obo".to_string(),
    "2025-09-08",
    "10.5281/zenodo.17382285",
);
```

**Pros:**
- ✅ Official versioned release
- ✅ Proper attribution included
- ✅ Reproducible
- ✅ Not blocked by Cloudflare

**Cons:**
- ❌ Requires manual download (one-time)
- ❌ Large archive size (21.4 GB)

### Option 2: Environment Variables

```bash
export GO_LOCAL_ONTOLOGY_PATH="data/go/go-basic.obo"
export GO_RELEASE_VERSION="2025-09-08"
export GO_ZENODO_DOI="10.5281/zenodo.17382285"
export GOA_BASE_URL="ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT"
```

```rust
let config = GoHttpConfig::from_env();
```

### Option 3: Builder Pattern

```rust
let config = GoHttpConfig::builder()
    .local_ontology_path("data/go/go-basic.obo".to_string())
    .annotation_base_url("ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/HUMAN".to_string())
    .go_release_version("2025-09-08".to_string())
    .zenodo_doi("10.5281/zenodo.17382285".to_string())
    .citation("Gene Ontology data from the 2025-09-08 release...".to_string())
    .parse_limit(1000)  // For testing
    .build();
```

---

## Download Options

### Recommended: Organism-Specific Files

**Best for:**
- Testing and development
- Most production use cases
- Memory-constrained environments

**Coverage:**
- Human: ~1M annotations
- Mouse: ~1M annotations
- Top 5 organisms: ~80% of common use cases

**Performance:**
```
Human (10 MB):
  Download:    <1 second (FTP)
  Parse:       <1 second
  Store:       ~2 seconds
  Total:       ~3 seconds
```

**Example:**

```rust
// Ingest multiple organisms
let organisms = vec!["human", "mouse", "rat", "zebrafish", "fly"];
for organism in organisms {
    pipeline.run_organism_annotations(organism).await?;
}
```

### Full Dataset (Advanced)

**Best for:**
- Complete production deployment
- Comprehensive coverage needed

**Characteristics:**
- Size: 16.8 GB compressed
- Annotations: 700M entries
- Time: 2-3 hours (estimated)
- **Memory**: Requires streaming implementation (current implementation will OOM)

**Status:** ⚠️ Not recommended until streaming download is implemented

---

## Testing

### Test 1: FTP Connection

```bash
cargo run --bin go_test_ftp
```

**Expected Output:**
```
✓ Connected to ftp.ebi.ac.uk
✓ Downloaded current_release_numbers.txt
✓ Found goa_uniprot_all.gaf.gz
```

### Test 2: Human Annotations (Recommended)

```bash
cargo run --bin go_test_human
```

**Expected Output:**
```
✓ Downloaded: 10.9MB compressed → 141MB decompressed
✓ Parsed: 1000 annotations
✓ Stored: 998 annotations (2 duplicates skipped)
✓ Evidence codes: IEA (610), IBA (202), TAS (152), IDA (13)
```

### Test 3: Complete Pipeline with Local Ontology

```bash
# First, download and extract go-basic.obo as shown in Quick Start

# Then run test
cargo run --bin go_test_local_ontology
```

**Expected Output:**
```
✓ Loaded GO ontology from local file: 40MB
✓ Parsed: 45,000 terms + 100,000 relationships
✓ Stored: 45K terms, 100K relationships
✓ Annotation ingestion: 1000 annotations
```

---

## Production Deployment

### Step 1: Download Zenodo Archive

```bash
# One-time download
cd /path/to/bdp/data
wget https://zenodo.org/records/17382285/files/go-release-archive.tgz

# Extract ontology file
tar -xzf go-release-archive.tgz --wildcards '**/go-basic.obo'
mv <extracted-path>/go-basic.obo go/go-basic.obo

# Optional: Extract annotations if needed
tar -xzf go-release-archive.tgz --wildcards '**/annotations/*'
```

### Step 2: Configure Environment

```bash
# In your .env file or environment
export GO_LOCAL_ONTOLOGY_PATH="/path/to/bdp/data/go/go-basic.obo"
export GO_RELEASE_VERSION="2025-09-08"
export GO_ZENODO_DOI="10.5281/zenodo.17382285"
export GOA_BASE_URL="ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/HUMAN"
```

### Step 3: Run Ingestion Service

```rust
// In your ingestion service
let config = GoHttpConfig::from_env();
let pipeline = GoPipeline::new(db, org_id, config)?;

// Ingest ontology (from local file)
pipeline.run_ontology("1.0").await?;

// Ingest annotations (from FTP)
// Option A: Single organism
pipeline.run_organism_annotations("human").await?;

// Option B: Multiple organisms
for organism in ["human", "mouse", "rat"] {
    pipeline.run_organism_annotations(organism).await?;
}
```

### Step 4: Verify Data

```sql
-- Check GO terms
SELECT COUNT(*) FROM go_term_metadata;
-- Expected: ~45,000

-- Check relationships
SELECT COUNT(*) FROM go_relationships;
-- Expected: ~100,000

-- Check annotations
SELECT COUNT(*) FROM go_annotations;
-- Expected: Depends on organisms ingested

-- Verify attribution
SELECT metadata FROM versions WHERE source_type = 'go_term';
-- Should contain: zenodo_doi, citation, license info
```

### Step 5: Update Schedule

GO releases monthly. Set up automated updates:

```bash
# Cron job to check for new releases (monthly)
0 0 1 * * /path/to/bdp/scripts/check_go_updates.sh

# Update process:
# 1. Download new Zenodo archive
# 2. Extract go-basic.obo
# 3. Run ingestion with new version
# 4. Update version metadata
```

---

## Performance Benchmarks

### Ontology Ingestion (from local file)

```
File size: 40 MB (go-basic.obo)
Parse: 45,000 terms + 100,000 relationships
Time: 5-10 seconds
Memory: <100 MB
Storage: 90 chunks @ 500/chunk
```

### Annotations Ingestion (organism-specific)

```
Human (10 MB compressed):
  Download: <1 sec (FTP)
  Decompress: ~1 sec
  Parse: 1000 annotations in <0.1 sec
  Store: 1000 annotations in ~1 sec
  Total: ~3 seconds

Mouse (similar size):
  Total: ~3 seconds

Top 5 organisms:
  Total: ~15-20 seconds
```

---

## Troubleshooting

### Issue: 403 Forbidden when downloading ontology

**Cause:** HTTP URLs are behind Cloudflare
**Solution:** Use Zenodo archive with local file configuration

### Issue: Out of Memory when downloading full GOA dataset

**Cause:** 16.8 GB file loaded into memory at once
**Solution:** Use organism-specific files instead

### Issue: Duplicate annotations error

**Cause:** Same annotation exists multiple times in batch
**Solution:** Already fixed - using `ON CONFLICT DO NOTHING`

### Issue: Missing GO term definitions

**Cause:** Only ingested annotations, not ontology
**Solution:** Ingest ontology first before annotations

---

## References

- [GO Citation Policy](https://geneontology.org/docs/go-citation-policy/)
- [GO Downloads](https://geneontology.org/docs/downloads/)
- [Zenodo GO Archive](https://doi.org/10.5281/zenodo.1205166)
- [GAF Format Specification](https://geneontology.org/docs/go-annotation-file-gaf-format-2.1/)
- [THIRD_PARTY_ATTRIBUTIONS.md](../THIRD_PARTY_ATTRIBUTIONS.md)

---

*Last Updated: 2026-01-20*
