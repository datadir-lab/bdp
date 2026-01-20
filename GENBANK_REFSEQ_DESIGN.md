# GenBank/RefSeq Ingestion Design

**Date**: 2026-01-19
**Status**: Planning Phase

## Overview

Design for ingesting NCBI GenBank and RefSeq nucleotide sequences following the established patterns from UniProt (proteins) and NCBI Taxonomy (organisms).

## Data Relationships

```
NCBI Taxonomy (organisms)
    ↓
GenBank/RefSeq (DNA/RNA sequences)
    ↓
UniProt (proteins from CDS)
```

This completes the **central dogma**: DNA → RNA → Protein

---

## 1. Source Type Strategy

### In `data_sources` table:

**Current source types:**
- `protein` - UniProt proteins
- (taxonomy data uses `taxonomy_metadata` table directly)

**New source types:**
```sql
-- Two separate source types for clarity
'genbank'  -- For GenBank sequences
'refseq'   -- For RefSeq sequences
```

**Why separate?**
- RefSeq is curated subset (higher quality)
- GenBank is all submissions (comprehensive but may have duplicates)
- Different update schedules (RefSeq: bi-monthly, GenBank: daily)
- Users may want only RefSeq for production pipelines

**Slug pattern:**
```
genbank-{accession}-{molecule_type}@{version}
refseq-{accession}-{molecule_type}@{version}

Examples:
genbank-NC_000913-dna@1.0
refseq-NM_001301717-mrna@2.0
```

---

## 2. Metadata Table Structure

### `sequence_metadata` Table

```sql
-- Migration: 20260120000001_create_sequence_metadata.sql

CREATE TABLE sequence_metadata (
    -- Primary key links to data_sources
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Core identifiers
    accession VARCHAR(50) NOT NULL,           -- e.g., "NC_000913"
    accession_version VARCHAR(50) NOT NULL,   -- e.g., "NC_000913.3"
    gi_number VARCHAR(50),                    -- Legacy GI (deprecated but in old records)

    -- Sequence properties
    sequence_length INTEGER NOT NULL,
    molecule_type VARCHAR(50) NOT NULL,       -- DNA, RNA, mRNA, tRNA, rRNA, etc.
    topology VARCHAR(20),                     -- circular, linear

    -- Descriptive metadata
    definition TEXT NOT NULL,                 -- Sequence description
    organism VARCHAR(255),                    -- Organism name
    taxonomy_id INTEGER,                      -- FK to taxonomy_metadata

    -- Gene/product information
    gene_name VARCHAR(255),                   -- Primary gene name
    locus_tag VARCHAR(255),                   -- Systematic locus identifier
    product TEXT,                             -- Gene product description

    -- Structured annotations (JSONB for flexibility)
    features JSONB,                           -- CDS, gene, exon features
    qualifiers JSONB,                         -- Additional metadata
    keywords TEXT[],                          -- Keywords from record

    -- Source tracking
    source_database VARCHAR(20) NOT NULL,     -- 'genbank' or 'refseq'
    division VARCHAR(20),                     -- BCT, VRL, PLN, etc.

    -- Dates
    sequence_date DATE,                       -- Date in GenBank record
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_source_database CHECK (source_database IN ('genbank', 'refseq'))
);

-- Indexes for common queries
CREATE INDEX sequence_metadata_accession_idx ON sequence_metadata(accession);
CREATE INDEX sequence_metadata_accession_version_idx ON sequence_metadata(accession_version);
CREATE INDEX sequence_metadata_taxonomy_idx ON sequence_metadata(taxonomy_id);
CREATE INDEX sequence_metadata_gene_name_idx ON sequence_metadata(gene_name);
CREATE INDEX sequence_metadata_molecule_type_idx ON sequence_metadata(molecule_type);
CREATE INDEX sequence_metadata_source_database_idx ON sequence_metadata(source_database);
CREATE INDEX sequence_metadata_organism_idx ON sequence_metadata(organism);

-- Full-text search on definition and product
CREATE INDEX sequence_metadata_search_idx ON sequence_metadata
    USING gin(to_tsvector('english', definition || ' ' || COALESCE(product, '') || ' ' || COALESCE(gene_name, '')));

-- Add foreign key to taxonomy (if record has taxonomy_id)
ALTER TABLE sequence_metadata
    ADD CONSTRAINT sequence_metadata_taxonomy_fk
    FOREIGN KEY (taxonomy_id) REFERENCES taxonomy_metadata(taxonomy_id);
```

### `nucleotide_sequences` Table

For actual sequence data (ACGT strings):

```sql
-- Store sequences separately from metadata for performance
-- Uses hash-based deduplication (many sequences are identical)

CREATE TABLE nucleotide_sequences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Sequence data
    sequence TEXT NOT NULL,                   -- ACGT string
    sequence_hash VARCHAR(64) NOT NULL,       -- SHA256 for deduplication
    gc_content DECIMAL(5,2),                  -- GC% (calculated)

    -- Alternative: Store in S3 for very large sequences
    s3_key VARCHAR(500),                      -- If stored in S3
    compression VARCHAR(20),                  -- gzip, none

    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Unique constraint on hash for deduplication
    CONSTRAINT unique_sequence_hash UNIQUE (sequence_hash)
);

CREATE INDEX nucleotide_sequences_data_source_idx ON nucleotide_sequences(data_source_id);
CREATE INDEX nucleotide_sequences_hash_idx ON nucleotide_sequences(sequence_hash);
```

**Storage Strategy:**
- **Small sequences (<10KB)**: Store in `sequence` column
- **Large sequences (>10KB)**: Store in S3, reference in `s3_key`
- **Deduplication**: Use `sequence_hash` to avoid storing duplicates

---

## 3. Linking Tables

### `sequence_protein_mappings` Table

Links GenBank CDS to UniProt proteins:

```sql
-- Maps nucleotide sequences to their protein products
-- GenBank CDS features often have protein_id that maps to UniProt

CREATE TABLE sequence_protein_mappings (
    sequence_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Mapping details
    mapping_type VARCHAR(50) NOT NULL,        -- 'cds', 'translation', 'cross_ref'
    mapping_source VARCHAR(50) NOT NULL,      -- 'genbank_protein_id', 'refseq_protein_id'
    confidence VARCHAR(20) NOT NULL,          -- 'exact', 'inferred', 'predicted'

    -- CDS location (if from CDS feature)
    cds_start INTEGER,
    cds_end INTEGER,
    cds_strand VARCHAR(1),                    -- '+' or '-'

    created_at TIMESTAMPTZ DEFAULT NOW(),

    PRIMARY KEY (sequence_data_source_id, protein_data_source_id),

    CONSTRAINT valid_confidence CHECK (confidence IN ('exact', 'inferred', 'predicted'))
);

CREATE INDEX sequence_protein_mappings_sequence_idx ON sequence_protein_mappings(sequence_data_source_id);
CREATE INDEX sequence_protein_mappings_protein_idx ON sequence_protein_mappings(protein_data_source_id);
```

---

## 4. Versioning Strategy

### RefSeq Releases

RefSeq uses numbered releases (e.g., Release 226, 227):

```
External Version: "226"
Internal Version: "1.0", "1.1", etc. (if we re-ingest same release)
```

### GenBank Releases

GenBank uses bi-monthly releases with dates:

```
External Version: "259.0" (release number)
or: "2026-01"
Internal Version: "1.0", "1.1", etc.
```

### Individual Sequence Versions

Each sequence has its own version:

```
Accession: NC_000913
Version: NC_000913.3 (the ".3" is the version)
```

**Versioning in `versions` table:**

```
data_source_id: UUID for this specific sequence
external_version: "3" (from accession.version)
internal_version: "1.0" (our ingestion version)
created_at: When we ingested this version
```

### Version Tracking Example

```
User request: "genbank-NC_000913-dna@1.0"

Lookup:
1. Find data_source with slug "genbank-NC_000913-dna"
2. Find version where internal_version = "1.0"
3. Get sequence_metadata for that data_source_id
4. accession_version = "NC_000913.3"
```

---

## 5. Module Structure

Following UniProt/NCBI Taxonomy patterns:

```
crates/bdp-server/src/ingest/
  genbank/
    config.rs           - GenBankFtpConfig (FTP host, paths, settings)
    ftp.rs              - FTP operations (download, list releases)
    models.rs           - GenBankRecord, SequenceData, Feature structures
    parser.rs           - Parse GenBank flat file format
    storage.rs          - Batch operations for DB inserts
    pipeline.rs         - Single file/release processing
    orchestrator.rs     - Multi-release parallel processing
    mod.rs              - Module exports

  refseq/
    (same structure as genbank)

  # Or shared if lots of common code:
  ncbi_sequences/
    shared/
      models.rs
      parser.rs
    genbank/
    refseq/
```

**Start with separate modules** (genbank + refseq) for clarity, refactor to shared if lots of duplication.

---

## 6. File Formats & Parsing

### GenBank Flat File Format

```
LOCUS       NC_000913          4641652 bp    DNA     circular BCT 15-DEC-2024
DEFINITION  Escherichia coli str. K-12 substr. MG1655, complete genome.
ACCESSION   NC_000913
VERSION     NC_000913.3
KEYWORDS    RefSeq.
SOURCE      Escherichia coli str. K-12 substr. MG1655
  ORGANISM  Escherichia coli str. K-12 substr. MG1655
            Bacteria; Pseudomonadota; Gammaproteobacteria; Enterobacterales;
            Enterobacteriaceae; Escherichia.
FEATURES             Location/Qualifiers
     source          1..4641652
                     /organism="Escherichia coli str. K-12 substr. MG1655"
                     /mol_type="genomic DNA"
                     /strain="K-12"
                     /db_xref="taxon:511145"
     gene            190..255
                     /gene="thrL"
                     /locus_tag="b0001"
     CDS             190..255
                     /gene="thrL"
                     /product="thr operon leader peptide"
                     /protein_id="NP_414542.1"
                     /translation="MKRISTTITTTITITTGNGAG"
ORIGIN
        1 agcttttcat tctgactgca acgggcaata tgtctctgtg tggattaaaa aaagagtgtc
       61 tgatagcagc ttctgaactg gttacctgcc gtgagtaaat taaaatttta ttgacttagg
// END
```

**Key sections to parse:**
- LOCUS line: Length, molecule type, topology, division, date
- DEFINITION: Description
- ACCESSION: Primary ID
- VERSION: Accession.version
- ORGANISM: Taxonomy lineage
- FEATURES: Genes, CDS, etc.
- ORIGIN: Actual sequence

### Rust Parsing Options

**Option 1**: Use `gb-io` crate
```rust
use gb_io::reader::SeqReader;
```

**Option 2**: Custom parser (more control)
```rust
struct GenBankParser {
    // Parse line by line
}
```

**Option 3**: `bio` crate
```rust
use bio::io::genbank;
```

**Recommendation**: Start with `gb-io` if available, fallback to custom parser for flexibility.

---

## 7. FTP Structure & Download Strategy

### GenBank FTP

```
ftp://ftp.ncbi.nlm.nih.gov/genbank/
  gbbct*.seq.gz       - Bacterial sequences
  gbvrl*.seq.gz       - Viral sequences
  gbpln*.seq.gz       - Plant sequences
  gbpri*.seq.gz       - Primate sequences
  gbrod*.seq.gz       - Rodent sequences
  gbmam*.seq.gz       - Other mammals
  gbvrt*.seq.gz       - Other vertebrates
  gbinv*.seq.gz       - Invertebrates
  release.notes/      - Release information
```

### RefSeq FTP

```
ftp://ftp.ncbi.nlm.nih.gov/refseq/
  release/
    release-catalog/
      release226.files.installed  - File listing
    bacteria/                     - Bacterial sequences
    viral/                        - Viral sequences
    complete/                     - Complete genomes
  release-notes/                  - Version info
```

### Download Strategy

**Phase 1: Start Small (Viral)**
```
Download: gbvrl*.seq.gz (~5GB)
Parse: ~100K sequences
Time: ~30 min download + 1-2 hours parse/store
```

**Phase 2: Add Bacterial**
```
Download: gbbct*.seq.gz (~30GB)
Parse: ~1M sequences
Time: ~3 hours download + 10-15 hours parse/store
```

**Phase 3: Full GenBank**
```
Download: All divisions (~250GB)
Parse: ~5-10M sequences
Time: ~2 days full ingestion
```

---

## 8. Batch Operations (Following NCBI Taxonomy Pattern)

### Storage Pattern

```rust
// Similar to NcbiTaxonomyStorage
pub struct GenBankStorage {
    organization_id: Uuid,
    pool: PgPool,
    s3_client: Option<S3Client>,
}

impl GenBankStorage {
    pub async fn store(&self, records: &[GenBankRecord]) -> Result<StorageStats> {
        const CHUNK_SIZE: usize = 500;

        for chunk in records.chunks(CHUNK_SIZE) {
            self.store_chunk_batch(chunk).await?;
        }
    }

    async fn store_chunk_batch(&self, chunk: &[GenBankRecord]) -> Result<()> {
        // 1. Batch upsert registry_entries
        // 2. Batch insert data_sources
        // 3. Batch insert sequence_metadata
        // 4. Batch insert/deduplicate nucleotide_sequences
        // 5. Batch insert versions
        // 6. Batch insert version_files (S3)
        // 7. Batch insert sequence_protein_mappings (if CDS present)
    }
}
```

**Expected Performance:**
- Similar to NCBI Taxonomy: 500-700x query reduction
- Chunk size: 500 (safe for PostgreSQL parameter limit)
- Processing speed: ~5000-10000 records/minute

---

## 9. Parallel Processing

### Division-Level Parallelism

```rust
// Process different organism divisions in parallel
let divisions = vec!["viral", "bacterial", "plant", "mammalian"];

let results = stream::iter(divisions)
    .map(|division| process_division(division))
    .buffer_unordered(4)  // 4 divisions at once
    .collect()
    .await;
```

### Release-Level Parallelism

```rust
// For RefSeq: process multiple release files in parallel
let release_files = list_release_files("226").await?;

let results = stream::iter(release_files)
    .map(|file| process_release_file(file))
    .buffer_unordered(4)
    .collect()
    .await;
```

---

## 10. Data Size Estimates

### GenBank
- **Compressed**: ~250GB
- **Uncompressed**: ~1TB
- **Records**: ~5-10M nucleotide sequences
- **Database size**: ~100-200GB (with metadata + sequences)

### RefSeq
- **Compressed**: ~200GB
- **Uncompressed**: ~800GB
- **Records**: ~3-5M curated sequences
- **Database size**: ~80-150GB

### With Deduplication
- Many sequences are duplicates (different accessions, same sequence)
- Hash-based deduplication can reduce by 20-30%

---

## 11. Integration with Existing Data

### Taxonomy Links

```sql
-- Sequence to organism
SELECT
    sm.accession,
    sm.definition,
    tm.scientific_name,
    tm.rank
FROM sequence_metadata sm
JOIN taxonomy_metadata tm ON sm.taxonomy_id = tm.taxonomy_id
WHERE sm.gene_name = 'thrA';
```

### Protein Links

```sql
-- Sequence to protein (CDS)
SELECT
    sm.accession AS sequence_id,
    sm.gene_name,
    pm.uniprot_accession AS protein_id,
    spm.cds_start,
    spm.cds_end
FROM sequence_metadata sm
JOIN sequence_protein_mappings spm ON sm.data_source_id = spm.sequence_data_source_id
JOIN protein_metadata pm ON spm.protein_data_source_id = pm.data_source_id
WHERE sm.organism = 'Escherichia coli';
```

### Complete Central Dogma Query

```sql
-- DNA -> RNA -> Protein -> Organism
SELECT
    sm.accession AS dna_accession,
    sm.gene_name,
    pm.uniprot_accession AS protein_accession,
    tm.scientific_name AS organism
FROM sequence_metadata sm
LEFT JOIN sequence_protein_mappings spm ON sm.data_source_id = spm.sequence_data_source_id
LEFT JOIN protein_metadata pm ON spm.protein_data_source_id = pm.data_source_id
LEFT JOIN taxonomy_metadata tm ON sm.taxonomy_id = tm.taxonomy_id
WHERE sm.gene_name = 'dnaA';
```

---

## 12. Implementation Phases

### Phase 1: Schema & Infrastructure (1-2 days)
- [ ] Create migrations for new tables
- [ ] Add source types to data_sources
- [ ] Set up module structure
- [ ] Write basic models

### Phase 2: GenBank Viral (3-5 days)
- [ ] FTP configuration
- [ ] FTP download logic
- [ ] GenBank flat file parser
- [ ] Storage with batch operations
- [ ] Test with viral division (~100K sequences)

### Phase 3: GenBank Full (3-5 days)
- [ ] Extend to all divisions
- [ ] Parallel processing by division
- [ ] Performance optimization
- [ ] Test with full GenBank (~5M sequences)

### Phase 4: RefSeq (2-3 days)
- [ ] RefSeq FTP configuration
- [ ] Adapt parser for RefSeq format (mostly same)
- [ ] Release-based versioning
- [ ] Test with RefSeq release

### Phase 5: Protein Mappings (2-3 days)
- [ ] Parse CDS features
- [ ] Extract protein_id from CDS
- [ ] Map to UniProt accessions
- [ ] Populate sequence_protein_mappings table

### Phase 6: Polish & Documentation (1-2 days)
- [ ] Documentation
- [ ] CLI commands for GenBank/RefSeq
- [ ] Testing guide
- [ ] Performance benchmarks

**Total Estimate: 12-20 days**

---

## 13. Testing Strategy

### Unit Tests
```rust
#[test]
fn test_parse_genbank_locus_line() {
    let line = "LOCUS       NC_000913  4641652 bp    DNA     circular BCT 15-DEC-2024";
    let parsed = parse_locus(line)?;
    assert_eq!(parsed.accession, "NC_000913");
    assert_eq!(parsed.length, 4641652);
    assert_eq!(parsed.molecule_type, "DNA");
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_genbank_viral_ingestion() {
    // Download small viral file
    // Parse records
    // Store in test database
    // Verify counts and data integrity
}
```

### Performance Tests
```rust
#[tokio::test]
async fn test_batch_operations_performance() {
    // Process 10K sequences
    // Measure time
    // Verify < 10 minutes (target: 5K-10K records/min)
}
```

---

## 14. Comparison with UniProt & NCBI Taxonomy

| Feature | UniProt | NCBI Taxonomy | GenBank/RefSeq |
|---------|---------|---------------|----------------|
| **Source Type** | `protein` | (taxonomy table) | `genbank`, `refseq` |
| **Metadata Table** | `protein_metadata` | `taxonomy_metadata` | `sequence_metadata` |
| **Sequences Table** | `protein_sequences` | N/A | `nucleotide_sequences` |
| **Versioning** | UniProt releases | Monthly archives | RefSeq releases / GenBank releases |
| **Batch Operations** | ✅ 500 chunks | ✅ 500 chunks | ✅ 500 chunks (planned) |
| **Parallel Processing** | ❌ | ✅ 4x concurrency | ✅ 4x concurrency (planned) |
| **FTP Source** | UniProt FTP | NCBI FTP | NCBI FTP |
| **File Format** | DAT | Custom taxdump | GenBank flat file |
| **Size** | ~100GB | ~100MB per version | ~250GB (GenBank) |
| **Records** | ~200M proteins | ~2.5M taxa | ~5-10M sequences |

**Key Similarities:**
- Same batch operation pattern (500 chunks)
- Same parallel processing approach
- Same version tracking in `versions` table
- Same S3 storage for files

**Key Differences:**
- GenBank has linking table to proteins (sequence_protein_mappings)
- GenBank needs sequence deduplication (hash-based)
- GenBank has organism divisions (viral, bacterial, etc.)

---

## Next Steps

1. **Review this design** - Does this match your vision?
2. **Schema approval** - Are the tables structured correctly?
3. **Start Phase 1** - Create migrations and basic structure
4. **Decide**: GenBank first or RefSeq first? (Recommend: GenBank viral for quick win)

Ready to implement?
