# GenBank/RefSeq Implementation Plan

## Overview
Implement GenBank and RefSeq nucleotide sequence ingestion with S3 storage, PostgreSQL metadata, and protein mappings following the proven patterns from UniProt and NCBI Taxonomy implementations.

## Design Decisions (Confirmed)
- **Storage**: S3 for sequences (FASTA files), PostgreSQL for metadata
- **Scope**: Full GenBank (all divisions: viral, bacterial, plant, mammalian, etc.)
- **Source Types**: Separate `genbank` and `refseq` types
- **Mappings**: Implement sequence-to-protein mappings in Phase 1
- **Parser**: Custom GenBank flat file parser
- **Performance**: Batch operations (500 chunks) + parallel processing

## Database Schema

### New Tables

```sql
-- migrations/20260120000002_create_sequence_tables.sql

-- Sequence metadata (queryable)
CREATE TABLE sequence_metadata (
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    accession VARCHAR(50) NOT NULL,
    accession_version VARCHAR(50) NOT NULL UNIQUE,
    sequence_length INTEGER NOT NULL,
    molecule_type VARCHAR(50) NOT NULL,
    topology VARCHAR(20), -- linear, circular
    definition TEXT NOT NULL,
    organism VARCHAR(255),
    taxonomy_id INTEGER REFERENCES taxonomy_metadata(taxonomy_id),
    gene_name VARCHAR(255),
    locus_tag VARCHAR(100),
    protein_id VARCHAR(50), -- For CDS features
    product TEXT, -- Protein product description
    features JSONB, -- All features: CDS, gene, regulatory, etc.
    gc_content DECIMAL(5,2),
    sequence_hash VARCHAR(64) NOT NULL,
    s3_key VARCHAR(500) NOT NULL, -- Path to FASTA file in S3
    source_database VARCHAR(20) NOT NULL CHECK (source_database IN ('genbank', 'refseq')),
    division VARCHAR(20), -- viral, bacterial, plant, mammalian, etc.
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_sequence_metadata_accession ON sequence_metadata(accession);
CREATE INDEX idx_sequence_metadata_taxonomy_id ON sequence_metadata(taxonomy_id);
CREATE INDEX idx_sequence_metadata_gene_name ON sequence_metadata(gene_name);
CREATE INDEX idx_sequence_metadata_source_database ON sequence_metadata(source_database);
CREATE INDEX idx_sequence_metadata_division ON sequence_metadata(division);
CREATE INDEX idx_sequence_metadata_hash ON sequence_metadata(sequence_hash);

-- Sequence to protein mappings (central dogma linking)
CREATE TABLE sequence_protein_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sequence_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    mapping_type VARCHAR(50) NOT NULL, -- 'cds', 'translation', 'db_xref'
    cds_start INTEGER,
    cds_end INTEGER,
    strand VARCHAR(1), -- '+' or '-'
    codon_start INTEGER,
    transl_table INTEGER,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(sequence_data_source_id, protein_data_source_id, mapping_type)
);

CREATE INDEX idx_sequence_protein_seq ON sequence_protein_mappings(sequence_data_source_id);
CREATE INDEX idx_sequence_protein_prot ON sequence_protein_mappings(protein_data_source_id);
CREATE INDEX idx_sequence_protein_type ON sequence_protein_mappings(mapping_type);
```

## Module Structure

```
crates/bdp-server/src/ingest/genbank/
├── mod.rs              # Module exports
├── config.rs           # FTP configuration, paths
├── ftp.rs              # FTP client, download, list releases
├── models.rs           # GenBankRecord, Feature, CDS structures
├── parser.rs           # Custom GenBank flat file parser
├── storage.rs          # Batch operations for PostgreSQL + S3
├── pipeline.rs         # Single file/release processing
└── orchestrator.rs     # Parallel processing across divisions
```

## Implementation Phases

### Phase 1: Database Foundation (Day 1)
**Goal**: Database schema and basic module structure

Tasks:
- [x] Create migration `20260120000002_create_sequence_tables.sql`
- [ ] Create `src/ingest/genbank/mod.rs`
- [ ] Create `src/ingest/genbank/config.rs` with FTP paths
- [ ] Create `src/ingest/genbank/models.rs` with data structures
- [ ] Update `src/ingest/mod.rs` to export genbank module

Files:
- `migrations/20260120000002_create_sequence_tables.sql`
- `crates/bdp-server/src/ingest/genbank/mod.rs`
- `crates/bdp-server/src/ingest/genbank/config.rs`
- `crates/bdp-server/src/ingest/genbank/models.rs`

### Phase 2: GenBank Parser (Day 1-2)
**Goal**: Parse GenBank flat file format

GenBank Format:
```
LOCUS       NC_000913            4641652 bp    DNA     circular BCT 01-JAN-2026
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
                     /sub_strain="MG1655"
                     /db_xref="taxon:511145"
     gene            190..255
                     /locus_tag="b0001"
                     /gene="thrL"
     CDS             190..255
                     /locus_tag="b0001"
                     /gene="thrL"
                     /product="thr operon leader peptide"
                     /protein_id="NP_414542.1"
                     /translation="MKRISTTITTTITITTGNGAG"
ORIGIN
        1 agcttttcat tctgactgca acgggcaata tgtctctgtg tggattaaaa aaagagtgtc
       61 tgatagcagc ttctgaactg gttacctgcc gtgagtaaat taaaatttta ttgacttagg
//
```

Parser Tasks:
- [ ] Implement `parse_locus()` - Extract accession, length, molecule type, topology, division
- [ ] Implement `parse_definition()` - Extract sequence definition
- [ ] Implement `parse_accession()` - Primary accession
- [ ] Implement `parse_version()` - Accession.version
- [ ] Implement `parse_organism()` - Organism name and taxonomy lineage
- [ ] Implement `parse_features()` - Parse all feature annotations
- [ ] Implement `parse_cds_feature()` - Extract CDS with protein_id, product, translation
- [ ] Implement `parse_origin()` - Extract nucleotide sequence
- [ ] Calculate GC content
- [ ] Calculate SHA256 hash for deduplication

Files:
- `crates/bdp-server/src/ingest/genbank/parser.rs`

### Phase 3: FTP Client (Day 2)
**Goal**: Download GenBank releases from NCBI FTP

FTP Paths:
```
ftp://ftp.ncbi.nlm.nih.gov/genbank/
├── gbvrt*.seq.gz     # Vertebrate
├── gbpri*.seq.gz     # Primate
├── gbrod*.seq.gz     # Rodent
├── gbmam*.seq.gz     # Other mammals
├── gbbct*.seq.gz     # Bacterial
├── gbvrl*.seq.gz     # Viral
├── gbpln*.seq.gz     # Plant
├── gbinv*.seq.gz     # Invertebrate
├── gbphg*.seq.gz     # Phage
├── GB_Release_Number # Current release number
```

RefSeq Path:
```
ftp://ftp.ncbi.nlm.nih.gov/refseq/release/
├── complete/         # Complete genomes
├── viral/            # Viral sequences
├── RELEASE_NUMBER    # Current release
```

Tasks:
- [ ] Implement `connect()` - Connect to NCBI FTP
- [ ] Implement `get_current_release()` - Read release number
- [ ] Implement `list_division_files()` - List files for a division
- [ ] Implement `download_file()` - Download and decompress .gz
- [ ] Implement `download_release()` - Download all files for a release
- [ ] Handle FTP timeouts and retries

Files:
- `crates/bdp-server/src/ingest/genbank/ftp.rs`

### Phase 4: Storage Layer (Day 2-3)
**Goal**: Batch operations for PostgreSQL + S3

Following NCBI Taxonomy pattern (666x improvement):

Tasks:
- [ ] Implement `insert_sequence_metadata_batch()` - Batch insert 500 records
- [ ] Implement `insert_protein_mappings_batch()` - Batch insert mappings
- [ ] Implement `upload_to_s3()` - Upload FASTA to S3
- [ ] Implement `generate_s3_key()` - Format: `genbank/release-259/NC_000913.3.fasta`
- [ ] Implement `check_existing_sequences()` - Deduplication by hash
- [ ] Implement `update_data_source()` - Mark ingestion complete

S3 Structure:
```
bdp-sequences/
├── genbank/
│   ├── release-259/
│   │   ├── viral_division/
│   │   │   ├── NC_001416.1.fasta
│   │   │   └── ...
│   │   ├── bacterial_division/
│   │   └── ...
│   └── release-260/
└── refseq/
    ├── release-226/
    └── ...
```

Files:
- `crates/bdp-server/src/ingest/genbank/storage.rs`

### Phase 5: Pipeline (Day 3-4)
**Goal**: Process single GenBank file

Tasks:
- [ ] Implement `GenbankPipeline::new()`
- [ ] Implement `run_file()` - Process single .seq.gz file
- [ ] Parse all records in file
- [ ] Extract protein_id from CDS features
- [ ] Query UniProt data_sources by accession
- [ ] Build protein mappings
- [ ] Batch insert metadata (500 chunks)
- [ ] Batch insert mappings (500 chunks)
- [ ] Upload sequences to S3
- [ ] Update data_source with counts
- [ ] Transaction per file

Processing Flow:
```rust
1. Download gbvrl1.seq.gz
2. Decompress and stream parse
3. For each GenBank record:
   - Parse metadata
   - Parse features
   - Extract CDS with protein_id
   - Calculate GC content and hash
   - Generate FASTA content
4. Batch: Collect 500 records
5. Insert metadata batch to PostgreSQL
6. Upload FASTA files to S3
7. Query UniProt for protein_ids
8. Insert mapping batch
9. Repeat until file complete
10. Commit transaction
```

Files:
- `crates/bdp-server/src/ingest/genbank/pipeline.rs`

### Phase 6: Orchestrator (Day 4-5)
**Goal**: Parallel processing across divisions

Following NCBI Taxonomy parallel pattern (4x improvement):

Tasks:
- [ ] Implement `GenbankOrchestrator::new()`
- [ ] Implement `run_release()` - Process entire release
- [ ] Implement `run_division()` - Process single division
- [ ] Parallel processing with `buffer_unordered(concurrency)`
- [ ] Progress tracking
- [ ] Error handling and retry logic
- [ ] Create data_source per division per release

Parallelization:
```rust
// Process divisions in parallel (concurrency = 4)
let divisions = vec!["viral", "bacterial", "plant", "mammalian"];
let results = stream::iter(divisions)
    .map(|division| {
        let pipeline = GenbankPipeline::new(config, db);
        pipeline.run_division(division, release)
    })
    .buffer_unordered(4)
    .collect()
    .await;
```

Files:
- `crates/bdp-server/src/ingest/genbank/orchestrator.rs`

### Phase 7: Integration (Day 5)
**Goal**: Integrate with existing system

Tasks:
- [ ] Update `src/ingest/mod.rs` with exports
- [ ] Add GenBank config to main config
- [ ] Create API endpoints for GenBank search
- [ ] Update search to include sequences
- [ ] Add sequence retrieval endpoint (by accession)
- [ ] Create FASTA download endpoint
- [ ] Update web frontend to display sequences

Files:
- `crates/bdp-server/src/ingest/mod.rs`
- `crates/bdp-server/src/features/search/queries/unified_search.rs`
- `crates/bdp-server/src/features/sequences/` (new module)

### Phase 8: Testing (Day 5-6)
**Goal**: Comprehensive testing

Unit Tests:
- [ ] Test GenBank parser with sample files
- [ ] Test batch operations logic
- [ ] Test protein mapping extraction
- [ ] Test S3 key generation
- [ ] Test parallel orchestration logic

Integration Tests:
- [ ] Test small file ingestion (gbphg1.seq.gz ~20MB)
- [ ] Test viral division (gbvrl*.seq.gz ~500MB)
- [ ] Test metadata queries
- [ ] Test sequence retrieval
- [ ] Test protein mappings

Test Files:
- `crates/bdp-server/tests/genbank_parser_test.rs`
- `crates/bdp-server/tests/genbank_batch_test.rs`
- `crates/bdp-server/tests/genbank_small_ingestion_test.rs`
- `crates/bdp-server/src/bin/genbank_test_phage.rs` (test binary)

Test Data:
- Create `tests/fixtures/genbank/sample.gbk` with test records
- Use real phage division for integration test (~20MB, thousands of records)

### Phase 9: Documentation (Day 6)
**Goal**: User-facing documentation

Tasks:
- [ ] Create `GENBANK_QUICK_REFERENCE.md`
- [ ] Create `GENBANK_TESTING_GUIDE.md`
- [ ] Update main README.md
- [ ] Add API documentation
- [ ] Create ingestion guide

Files:
- `GENBANK_QUICK_REFERENCE.md`
- `GENBANK_TESTING_GUIDE.md`
- `README.md`

## Performance Targets

Based on NCBI Taxonomy achievements:

### GenBank Scale:
- ~5-10M sequences per release
- ~250GB compressed data
- 18 divisions
- Monthly releases since 1982 (500+ releases available)

### Expected Performance:
**Without Optimizations** (naive approach):
- 5M records × 10 queries each = 50M queries
- At 10ms per query = 139 hours per release

**With Batch Operations** (500 chunks):
- 5M records / 500 = 10K batches
- 10K batches × 2 tables = 20K queries
- At 10ms per query = 3.3 minutes per release
- **Improvement**: 2,520x faster

**With Parallel Processing** (concurrency=4):
- 18 divisions / 4 = 4.5 concurrent batches
- 3.3 minutes / 4 = ~50 minutes per release
- **Total Improvement**: ~168x faster than naive

### Initial Release Ingestion:
- Target: Complete GenBank release 259 in under 1 hour
- Stretch: Under 30 minutes with higher concurrency

### Historical Catchup:
- 12 most recent releases (1 year) in under 12 hours
- 50 releases (4 years) in under 2 days

## S3 Storage Estimates

### Per Sequence:
- Metadata in DB: ~2KB per record
- FASTA in S3: ~1-10KB per sequence (varies widely)

### Per Release:
- 5M sequences × 2KB = 10GB metadata in PostgreSQL
- 5M sequences × 5KB = 25GB FASTA in S3
- Total: ~35GB per release

### Multi-Release:
- 12 releases × 35GB = 420GB (1 year)
- 50 releases × 35GB = 1.75TB (4 years)

S3 is perfect for this scale.

## Dependencies

### Existing (from UniProt/NCBI Taxonomy):
- `suppaftp` - FTP client
- `flate2` - gzip decompression
- `sha2` - hash computation
- `aws-sdk-s3` - S3 storage
- `sqlx` - PostgreSQL batch operations
- `tokio` - async runtime
- `futures` - parallel processing

### New Dependencies: None needed

## Risk Mitigation

### Risk 1: GenBank Format Complexity
**Mitigation**:
- Start with simple records (phage division)
- Incremental parser development
- Extensive test fixtures

### Risk 2: Large File Sizes
**Mitigation**:
- Stream parsing (don't load entire file)
- Process records in batches
- S3 multipart uploads for large files

### Risk 3: FTP Connection Issues
**Mitigation**:
- Implement retry logic
- Resume interrupted downloads
- Graceful timeout handling

### Risk 4: Protein Mapping Mismatches
**Mitigation**:
- Fuzzy matching on protein_id
- Log unmapped proteins
- Manual mapping table for edge cases

### Risk 5: Database Size Growth
**Mitigation**:
- Partition sequence_metadata by division
- Archive old releases to cold storage
- Implement data retention policy

## Success Metrics

### Phase 1-2 (Foundation + Parser):
- [ ] Parse sample GenBank file with 100% accuracy
- [ ] Extract all CDS features with protein_id
- [ ] Pass all parser unit tests

### Phase 3-4 (FTP + Storage):
- [ ] Successfully download phage division (~20MB)
- [ ] Upload sequences to S3
- [ ] Batch insert 1000 records in <1 second

### Phase 5-6 (Pipeline + Orchestrator):
- [ ] Process phage division end-to-end
- [ ] Achieve <5 minutes for viral division
- [ ] Parallel processing shows 3-4x speedup

### Phase 7-9 (Integration + Testing + Docs):
- [ ] Search returns GenBank sequences
- [ ] Retrieve sequences by accession
- [ ] Download FASTA files
- [ ] All tests passing
- [ ] Documentation complete

## Timeline

**Optimistic**: 6 days
**Realistic**: 8-10 days
**Conservative**: 12-14 days

Factors:
- Parser complexity (biggest unknown)
- FTP reliability
- Testing coverage
- Integration polish

## Next Steps

1. **Immediate**: Create database migration
2. **Day 1**: Build module structure and models
3. **Day 1-2**: Implement parser
4. **Day 2**: Build FTP client
5. **Day 2-3**: Implement storage layer
6. **Day 3-4**: Build pipeline
7. **Day 4-5**: Build orchestrator
8. **Day 5**: Integration
9. **Day 5-6**: Testing
10. **Day 6**: Documentation

## Commands to Run

```bash
# Run database migration
sqlx migrate run

# Test parser
cargo test genbank_parser

# Test batch operations
cargo test genbank_batch

# Run small ingestion (phage division)
cargo run --bin genbank_test_phage

# Run full viral division
cargo run --bin genbank_test_viral

# Run full release (all divisions)
cargo run --bin genbank_orchestrator -- --release 259
```

## Notes

- Follow exact patterns from NCBI Taxonomy implementation
- Reuse batch operation logic (500 chunk size)
- Reuse parallel processing logic (buffer_unordered)
- S3 storage identical to UniProt pattern
- Transaction per file for safety
- Extensive logging for debugging
- Progress tracking for long operations
