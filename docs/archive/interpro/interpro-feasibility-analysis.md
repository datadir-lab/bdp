# InterPro Integration Feasibility Analysis

**Date**: 2026-01-28
**Status**: High Feasibility - Recommended for Implementation

## Executive Summary

InterPro is **highly feasible** for integration into BDP's ingestion pipeline system. The data source aligns well with existing architecture patterns, has permissive licensing, and would provide significant value by adding protein domain/family annotations that complement your UniProt protein data.

**Key Finding**: InterPro → UniProt cross-references mirror your existing UniProt → NCBI Taxonomy pattern, making it a natural architectural fit.

---

## 1. License & Legal Compliance

### License Terms
- **Data License**: [CC0 1.0 Universal](https://interpro-documentation.readthedocs.io/en/latest/license.html) (Public Domain Dedication)
- **Software**: Apache License for InterProScan
- **Commercial Use**: ✅ Permitted without special licensing
- **Redistribution**: ✅ Fully permitted under CC0

### Citation Requirements
**Primary Citation** (2025):
```bibtex
Blum, M., et al. (2025). InterPro: the protein sequence classification resource in 2025.
Nucleic Acids Research. PMID: 39565202.
```

[Full citation requirements](https://interpro-documentation.readthedocs.io/en/latest/citing.html)

### BDP Compliance
- ✅ **No barriers** to ingestion and redistribution
- ✅ Citation policy table already exists in your architecture
- ✅ Can use existing `setup_citation_policy()` infrastructure (see `crates/bdp-server/src/ingest/citations.rs`)

**Action Required**: Add InterPro citation policy configuration (similar to UniProt policy)

---

## 2. Data Characteristics

### What is InterPro?
InterPro is a **protein signature database** that integrates multiple member databases:
- **Pfam**: Protein families (now hosted by InterPro)
- **SMART**: Domains and signaling motifs
- **PROSITE**: Protein domains, families, and functional sites
- **CDD**: Conserved domains
- And 10+ other specialized databases

### Data Coverage
- **81.3%** of UniProtKB sequences have InterPro annotations (~50M+ sequences)
- Provides: Domain boundaries, family classifications, functional sites, GO term mappings
- **Release Frequency**: Every **8 weeks**, coordinated with UniProt releases

### Cross-References with UniProt
InterPro calculates signatures for **all proteins in UniProtKB**. This creates a natural dependency:

```
NCBI Taxonomy (organisms)
    ↓ (referenced by)
UniProt Proteins
    ↓ (referenced by)
InterPro Domains/Families
```

This matches your existing architecture pattern where UniProt references NCBI Taxonomy.

**Source**: [InterPro-UniProt Integration](https://academic.oup.com/nar/article/49/D1/D344/5958491)

---

## 3. FTP Structure & File Formats

### FTP Location
**Base URL**: `ftp.ebi.ac.uk/pub/databases/interpro/`

**Directory Structure**:
```
interpro/
├── current_release/           # Latest version
├── releases/
│   ├── 98.0/                  # Historical releases
│   ├── 99.0/
│   ├── 100.0/
│   ├── 101.0/
│   ├── 102.0/
│   └── 103.0/                 # Latest (Dec 2024)
└── README
```

**Source**: [InterPro FTP Index](https://ftp.ebi.ac.uk/pub/databases/interpro/)

### Key Data Files

#### 1. `protein2ipr.dat.gz` (Primary file for BDP)
- **Format**: Tab-delimited (TSV)
- **Content**: Maps UniProt accessions → InterPro entries
- **Columns**: `UniProt_Accession`, `InterPro_ID`, `Entry_Name`, `Signature_Accession`, `Start`, `End`, `E-value`, etc.
- **Size**: ~2-4 GB compressed (estimated based on coverage)
- **Use Case**: Create individual data sources per InterPro entry with protein lists

#### 2. `entry.list`
- **Format**: Tab-delimited
- **Content**: InterPro entry metadata
- **Columns**: `IPR_ID`, `Entry_Type`, `Entry_Name`
- **Entry Types**: Family, Domain, Repeat, Site, Homologous Superfamily
- **Use Case**: Metadata for InterPro entries

#### 3. `names.dat` / `short_names.dat`
- **Format**: Tab-delimited
- **Content**: InterPro ID → human-readable names
- **Use Case**: Display names in UI

#### 4. JSON/XML exports
- **Format**: JSON or XML
- **Content**: Complete InterPro data via API
- **Use Case**: Optional for rich metadata

**Source**: [InterPro Download Documentation](https://interpro-documentation.readthedocs.io/en/latest/download.html)

---

## 4. Versioning Strategy

### InterPro Version Numbers
- **Format**: `X.0` (e.g., 98.0, 99.0, 100.0, 101.0, 102.0, 103.0)
- **Frequency**: Every **8 weeks** (coordinated with UniProt)
- **Latest**: 103.0 (December 2024)

**Recent Version Timeline**:
- **98.0**: January 2024
- **99.0**: April 2024
- **100.0**: May/June 2024
- **101.0**: July 2024
- **103.0**: December 2024

**Source**: [InterPro Blog](https://proteinswebteam.github.io/interpro-blog/)

### BDP Version Mapping Strategy

Your existing `versioning` module can handle InterPro well:

```rust
impl VersioningStrategy {
    pub fn interpro() -> Self {
        Self {
            major_triggers: vec![
                VersionTrigger::new(
                    VersionChangeType::Removed,
                    "entries",
                    "InterPro entries obsoleted or removed"
                ),
                VersionTrigger::new(
                    VersionChangeType::Modified,
                    "signatures",
                    "Signature boundaries significantly changed"
                ),
            ],
            minor_triggers: vec![
                VersionTrigger::new(
                    VersionChangeType::Added,
                    "entries",
                    "New InterPro entries added"
                ),
                VersionTrigger::new(
                    VersionChangeType::Modified,
                    "annotations",
                    "Entry descriptions or GO mappings updated"
                ),
                VersionTrigger::new(
                    VersionChangeType::Dependency,
                    "uniprot",
                    "Updated to match new UniProt release"
                ),
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: true,
            cascade_on_minor: false, // InterPro updates shouldn't cascade widely
        }
    }
}
```

**External Version**: `103.0` → **Internal Version**: `1.0`, `1.1`, `1.2` (semantic)

---

## 5. Architectural Fit Analysis

### Your Existing Patterns

Looking at your codebase, you have:

1. **Versioning Module** (`crates/bdp-server/src/ingest/versioning/`):
   - ✅ Semantic versioning with major/minor bumps
   - ✅ Changelog generation
   - ✅ Dependency cascade tracking
   - ✅ Per-organization versioning strategies

2. **Cross-Reference Support**:
   - ✅ UniProt references NCBI Taxonomy via `taxonomy_helper.rs`
   - ✅ Foreign key relationships in database
   - ✅ Dependency tracking in `dependencies` table

3. **Generic ETL Framework** (`crates/bdp-server/src/ingest/framework/`):
   - ✅ FTP downloader (`common/ftp.rs`)
   - ✅ Checksum verification (`framework/checksum.rs`)
   - ✅ Decompression support (`common/decompression.rs`)
   - ✅ Batch processing (`framework/coordinator.rs`)

4. **Citation Infrastructure** (`crates/bdp-server/src/ingest/citations.rs`):
   - ✅ `setup_citation_policy()` function
   - ✅ `uniprot_policy()` template

### How InterPro Fits

InterPro would be the **fourth major data source**, following your established pattern:

```
Data Source Pipeline Pattern (existing):
1. NCBI Taxonomy (organisms) ← base dependency
2. UniProt (proteins) ← depends on NCBI Taxonomy
3. Gene Ontology (terms) ← referenced by UniProt
4. GenBank/RefSeq (nucleotides) ← independent

InterPro Addition:
5. InterPro (domains) ← depends on UniProt
```

### Database Schema Additions Needed

You'll need new tables for InterPro (similar to your `protein_metadata`, `taxonomy_metadata` pattern):

```sql
-- InterPro entries (domains, families, sites)
CREATE TABLE interpro_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    interpro_id VARCHAR(20) NOT NULL UNIQUE, -- e.g., IPR000001
    entry_type VARCHAR(50) NOT NULL, -- Family, Domain, Repeat, Site, etc.
    name TEXT NOT NULL,
    short_name VARCHAR(255),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Protein-to-InterPro mappings (the main data)
CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id UUID REFERENCES protein_metadata(id) ON DELETE CASCADE,
    interpro_entry_id UUID REFERENCES interpro_entries(id) ON DELETE CASCADE,
    signature_accession VARCHAR(50), -- e.g., PF00001 (Pfam ID)
    signature_database VARCHAR(50), -- e.g., Pfam, SMART, PROSITE
    start_position INTEGER NOT NULL,
    end_position INTEGER NOT NULL,
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_protein_interpro_protein ON protein_interpro_matches(protein_id);
CREATE INDEX idx_protein_interpro_entry ON protein_interpro_matches(interpro_entry_id);
CREATE INDEX idx_interpro_type ON interpro_entries(entry_type);
```

**Reasoning**: This matches your existing pattern where:
- `taxonomy_metadata` stores organism data
- `protein_metadata` stores protein data
- `protein_interpro_matches` would store domain/family annotations

---

## 6. Implementation Proposal

### Module Structure (following your patterns)

```
crates/bdp-server/src/ingest/interpro/
├── mod.rs                    # Module exports
├── config.rs                 # InterProFtpConfig
├── ftp.rs                    # InterProFtp downloader
├── models.rs                 # InterProEntry, ProteinMatch
├── parser.rs                 # Parse protein2ipr.dat.gz
├── storage.rs                # InterProStorage (database writes)
├── pipeline.rs               # InterProPipeline (orchestration)
├── version_discovery.rs      # Discover available versions on FTP
└── orchestrator.rs           # Job orchestration
```

This mirrors your existing:
- `uniprot/` (6,600+ lines)
- `ncbi_taxonomy/` (3,100+ lines)
- `genbank/` (2,500+ lines)
- `gene_ontology/` (2,800+ lines)

### Estimated Complexity

| Component | Complexity | Reasoning |
|-----------|------------|-----------|
| **Parser** | Low | TSV format, simpler than UniProt DAT or GenBank flat files |
| **Storage** | Medium | Need protein cross-reference lookups (similar to taxonomy helper) |
| **Versioning** | Low | Simple X.0 format, predictable release schedule |
| **FTP Download** | Low | Reuse existing `common/ftp.rs` infrastructure |
| **Testing** | Medium | Need to verify cross-references work correctly |

**Estimated LOC**: ~2,000-3,000 lines (similar to GenBank/Gene Ontology)

**Estimated Time**: 1-2 weeks for experienced developer familiar with your codebase

---

## 7. Dependency Modeling

### Cross-Reference Architecture

InterPro entries should **depend on UniProt**:

```rust
// In InterProPipeline::run()
async fn run(&self, pool: &PgPool) -> Result<()> {
    // 1. Check that UniProt organization exists
    let uniprot_org = get_organization_by_slug(pool, "uniprot").await?;

    // 2. Check that we have a compatible UniProt version
    //    (InterPro 103.0 should match UniProt release from same timeframe)
    let uniprot_version = get_latest_version(pool, uniprot_org.id).await?;

    // 3. Download and parse protein2ipr.dat.gz
    let matches = self.parser.parse_protein2ipr(&data).await?;

    // 4. For each match, look up protein by accession
    for match in matches {
        let protein = lookup_protein_by_accession(pool, &match.uniprot_acc).await?;
        // Store protein_interpro_matches row
    }

    // 5. Create dependency link
    create_dependency(pool, interpro_version_id, uniprot_version_id).await?;
}
```

### Version Cascade Behavior

When UniProt version bumps (e.g., new proteins added):
1. **MAJOR UniProt change** (proteins removed) → Triggers InterPro re-ingestion (cascade)
2. **MINOR UniProt change** (proteins added) → InterPro can optionally re-ingest for new annotations

Configuration:
```rust
VersioningStrategy::interpro().cascade_on_major = true;  // Re-ingest if UniProt breaks
VersioningStrategy::interpro().cascade_on_minor = false; // Don't cascade for additions
```

This is similar to how Gene Ontology behaves (`cascade_on_minor: false`).

---

## 8. Data Modeling Philosophy

### Your Current Approach (from analysis)

Looking at `uniprot/storage.rs` and `ncbi_taxonomy/storage.rs`, your philosophy is:

1. **Individual Data Sources per Entry**: Each protein is a separate data source in `registry_entries`
2. **Metadata Tables**: Type-specific data in `protein_metadata`, `taxonomy_metadata`
3. **Relationships via Foreign Keys**: `protein_metadata.organism_id` → `taxonomy_metadata.id`
4. **Deduplication**: Shared sequences in `protein_sequences` (SHA-256 hashing)

### InterPro Modeling Options

**Option A: Individual Data Sources per InterPro Entry** (matches your pattern)
- Each InterPro entry (e.g., `IPR000001`) becomes a data source
- Registry entry: `interpro:IPR000001-matches@1.0`
- Version files: TSV/JSON lists of UniProt accessions with match coordinates
- **Pros**: Consistent with your architecture, easy to version independently
- **Cons**: ~40,000+ InterPro entries = 40,000+ data sources

**Option B: Single Aggregate Data Source** (simpler)
- One data source: `interpro:all-matches@103.0`
- Contains all protein-to-InterPro mappings for that release
- Version files: Complete `protein2ipr.dat.gz` or per-protein JSON
- **Pros**: Simpler versioning, matches how InterPro distributes data
- **Cons**: Less granular, can't track individual entry changes

**Recommendation**: **Option B** (Single Aggregate)

**Reasoning**:
1. InterPro releases as a cohesive unit (all entries updated together)
2. Users typically want "all InterPro annotations for version X"
3. Reduces database bloat (40K+ entries is excessive)
4. Simpler dependency tracking (one InterPro version depends on one UniProt version)
5. Can still expose individual entries via API filtering

### Storage Pattern

```rust
// Create one aggregate data source per release
registry_entry: "interpro:all-matches"
version: "1.0" (internal) / "103.0" (external)

// Metadata tables for domain definitions
interpro_entries:
  - IPR000001, "Kringle", "Domain", ...
  - IPR000002, "Cation transporter/ATPase", "Family", ...

// Relationship table for protein annotations
protein_interpro_matches:
  - protein_id: <P01308 UUID>, interpro_entry_id: <IPR000001 UUID>, start: 120, end: 180
  - protein_id: <P01308 UUID>, interpro_entry_id: <IPR000234 UUID>, start: 45, end: 98
```

This matches your `gene_ontology` module pattern:
- GO terms in `go_terms` table (like `interpro_entries`)
- Protein annotations in `protein_go_annotations` (like `protein_interpro_matches`)

---

## 9. File Versioning Strategy

### File Formats to Offer

Based on your existing patterns, InterPro data sources should provide:

1. **TSV** (primary): Filtered `protein2ipr.dat.gz` containing matches
   ```
   uniprot:all-proteins@1.0 → files/
     ├── all-proteins-1.0.fasta.gz
     ├── all-proteins-1.0.json.gz
     └── metadata.json
   ```

2. **JSON** (structured): Per-protein annotations
   ```json
   {
     "uniprot_accession": "P01308",
     "interpro_matches": [
       {
         "interpro_id": "IPR000001",
         "name": "Kringle",
         "type": "Domain",
         "start": 120,
         "end": 180,
         "signature": "PF00051",
         "signature_database": "Pfam",
         "e_value": 1.2e-45
       }
     ]
   }
   ```

3. **Entry-specific TSV**: Just matches for a specific InterPro entry (via API filter)

### S3 Storage Path

Following your `storage/mod.rs` pattern:
```
s3://bdp-storage/data-sources/interpro/all-matches/103.0/
  ├── protein2ipr-103.0.dat.gz        # Raw data from FTP
  ├── protein2ipr-103.0.json.gz       # Converted to JSON
  ├── entries-103.0.tsv.gz            # InterPro entry metadata
  └── checksums.sha256                # Checksums for verification
```

---

## 10. Risks & Mitigations

### Risk 1: Large File Size
**Risk**: `protein2ipr.dat.gz` is ~2-4 GB compressed, ~10-20 GB uncompressed
**Impact**: Memory usage during parsing, storage costs
**Mitigation**:
- Stream parsing (don't load entire file into memory)
- Batch inserts with `DB_MICRO_BATCH_SIZE` (your existing pattern)
- Compress stored files (gzip)
- Use existing `MAX_INSERT_BATCH_SIZE` constants

### Risk 2: Cross-Reference Failures
**Risk**: UniProt accessions in InterPro may not exist in BDP database
**Impact**: Orphaned annotations, data loss
**Mitigation**:
- Ingest UniProt **before** InterPro (dependency order)
- Skip missing accessions with warning logs
- Track "orphaned annotations" count in changelog
- Provide re-ingestion capability when UniProt is updated

### Risk 3: Version Coordination
**Risk**: InterPro 103.0 may reference proteins from UniProt 2024_03, but BDP has 2024_02
**Impact**: Incomplete annotations
**Mitigation**:
- Check UniProt version compatibility before ingestion
- Store `compatible_uniprot_versions` in InterPro metadata
- Warn users if version mismatch exists

### Risk 4: Update Frequency (Every 8 Weeks)
**Risk**: Frequent updates increase operational burden
**Impact**: More versions to store, more ingestion jobs
**Mitigation**:
- Automated ingestion with `IngestOrchestrator` (you already have this)
- Differential updates (only re-ingest changed entries)
- Changelog-based versioning (minor bumps for most updates)

---

## 11. Benefits to BDP Platform

### User Value

1. **Functional Annotations**: Users can discover protein domains/families
   ```bash
   bdp source add interpro:all-matches@1.0
   # Get domain annotations for all proteins in UniProt
   ```

2. **Domain-Level Analysis**: Researchers can:
   - Filter proteins by domain type (e.g., "all proteins with Kringle domains")
   - Understand protein function from domain composition
   - Link to Pfam, PROSITE, SMART databases

3. **Enhanced Search**: Web UI can search by:
   - InterPro ID (`IPR000001`)
   - Domain name (`Kringle`)
   - Protein family (`Insulin family`)

### Platform Value

1. **Ecosystem Completeness**:
   ```
   BDP Data Sources:
   - NCBI Taxonomy (organisms) ✅
   - UniProt (proteins) ✅
   - Gene Ontology (function) ✅
   - GenBank/RefSeq (genomes) ✅
   - InterPro (domains) ← NEW
   ```

2. **Cross-Reference Network**: InterPro creates a bridge:
   ```
   UniProt → InterPro → Pfam/SMART/PROSITE (external DBs)
   ```

3. **Research Use Cases**:
   - "Give me all human proteins with WD40 repeats"
   - "Which proteins in my dataset have the same domain architecture?"
   - "Has the domain composition of protein X changed across versions?"

---

## 12. Recommended Implementation Plan

### Phase 1: Database Schema (1-2 days)
- [ ] Create migration for `interpro_entries` table
- [ ] Create migration for `protein_interpro_matches` table
- [ ] Add indexes for performance
- [ ] Test with sample data

### Phase 2: FTP & Parser (3-4 days)
- [ ] Implement `InterProFtp` downloader (reuse `common/ftp.rs`)
- [ ] Implement `Protein2IprParser` for TSV parsing
- [ ] Implement `EntryListParser` for metadata
- [ ] Unit tests for parsing

### Phase 3: Storage & Cross-References (3-4 days)
- [ ] Implement `InterProStorage` for database writes
- [ ] Add UniProt accession lookup (similar to `TaxonomyHelper`)
- [ ] Handle missing proteins (log warnings)
- [ ] Batch insert optimization

### Phase 4: Pipeline & Orchestration (2-3 days)
- [ ] Implement `InterProPipeline` end-to-end flow
- [ ] Add version discovery from FTP
- [ ] Integrate with `IngestOrchestrator`
- [ ] Create job definition for apalis queue

### Phase 5: Versioning & Citations (1-2 days)
- [ ] Add `VersioningStrategy::interpro()`
- [ ] Implement change detection (compare releases)
- [ ] Add InterPro citation policy
- [ ] Test dependency cascade behavior

### Phase 6: Testing & Validation (2-3 days)
- [ ] Integration tests with real InterPro data
- [ ] Test cross-reference lookup performance
- [ ] Validate version bumps work correctly
- [ ] Test with missing UniProt proteins

### Phase 7: Documentation (1 day)
- [ ] Update ROADMAP.md
- [ ] Add InterPro section to docs
- [ ] Document CLI usage (`bdp source add interpro:all-matches@1.0`)
- [ ] API endpoint documentation

**Total Estimated Time**: 2-3 weeks

---

## 13. Conclusion & Recommendation

### Feasibility Score: ✅ **9/10** (Highly Feasible)

**Why High Score**:
- ✅ Permissive license (CC0)
- ✅ Well-structured FTP (simple TSV files)
- ✅ Natural dependency on UniProt (matches your architecture)
- ✅ Predictable versioning (8-week cycle)
- ✅ High user value (domain annotations)
- ✅ Fits existing ETL framework perfectly

**Deductions**:
- ⚠️ Large file size requires streaming parsing
- ⚠️ Cross-reference failures need careful handling

### Recommendation

**PROCEED with InterPro integration** as the **5th major data source** in BDP.

**Priority**: Medium-High (after current pipeline stabilization)

**Sequence**:
1. Complete production data ingestion for existing pipelines (UniProt, NCBI Taxonomy, GenBank, GO)
2. Implement InterPro pipeline (2-3 weeks)
3. Ingest historical InterPro versions (starting with 103.0)
4. Add to web UI search/browse

---

## 14. Alternative Considerations

### Similar Databases (if InterPro is delayed)

1. **Pfam** (now part of InterPro): Protein families - but InterPro subsumes this
2. **SMART**: Domains - also integrated into InterPro
3. **PDB** (Protein Data Bank): 3D structures - different use case, much larger complexity
4. **STRING**: Protein-protein interactions - valuable but different domain
5. **EnsemblCompara**: Comparative genomics - complex, genome-level

**Verdict**: InterPro is the **best next choice** - it consolidates multiple domain/family databases.

---

## References

- [InterPro License](https://interpro-documentation.readthedocs.io/en/latest/license.html)
- [InterPro Citation Guide](https://interpro-documentation.readthedocs.io/en/latest/citing.html)
- [InterPro Download Documentation](https://interpro-documentation.readthedocs.io/en/latest/download.html)
- [InterPro FTP Site](https://ftp.ebi.ac.uk/pub/databases/interpro/)
- [InterPro 2025 Paper](https://academic.oup.com/nar/article/53/D1/D444/7905301)
- [InterPro Blog (Version History)](https://proteinswebteam.github.io/interpro-blog/)
- [InterPro-UniProt Integration](https://academic.oup.com/nar/article/49/D1/D344/5958491)
- [InterMine InterPro Documentation](https://intermine.readthedocs.io/en/latest/database/data-sources/library/proteins/interpro/)
