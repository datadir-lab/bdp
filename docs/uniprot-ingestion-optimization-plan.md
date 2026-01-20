# UniProt Ingestion Optimization Plan

**Date:** 2026-01-19
**Status:** Planning
**Priority:** Critical (0% storage success rate)

## Executive Summary

The UniProt ingestion pipeline is currently **failing to store any proteins** (0/1000 entries stored) and has severe performance bottlenecks:

1. **Storage errors silently swallowed** - Most critical issue
2. **Repeated TAR decompression** - 2,280+ decompressions of same 1.4GB file
3. **N+1 database queries** - 10,000 queries per 1000-entry batch
4. **Incorrect taxonomy classification** - All entities marked as "organism"
5. **Wrong slug format** - Using `organism-9606` instead of `homo-sapiens`

**Estimated speedup after fixes:** 10-100x faster ingestion

---

## Current Issues

### Issue 1: Storage Failures (CRITICAL ⚠️)

**Location:** `crates/bdp-server/src/ingest/uniprot/storage.rs:58-74`

**Problem:**
```rust
for entry in entries {
    if let Err(e) = self.store_entry(entry).await {
        debug!("Failed to store entry {}: {}", entry.accession, e);
        continue; // ← Silently skipping ALL entries
    }
    stored_count += 1;
}
```

**Result:** `Successfully stored 0/1000 entries` in logs

**Root cause:** Unknown (errors not logged at ERROR level)

**Fix:**
- Change `debug!` to `error!`
- Collect and report first 5 errors for debugging
- Add validation that `source_type` is never empty
- Ensure transactional integrity (registry_entry + data_source + metadata atomic)

---

### Issue 2: Repeated TAR Decompression

**Location:** `crates/bdp-server/src/ingest/uniprot/pipeline.rs:771-814`

**Problem:**
Each worker decompresses entire 1.4GB tar.gz → 3.1GB DAT for every batch:
- 4 workers × 570 batches = ~2,280 decompressions
- Each decompression takes 1-2 seconds
- Total wasted time: ~1 hour per ingestion

**Current behavior:**
```rust
// pipeline.rs:771
let dat_data = Arc::new(dat_data); // Shared compressed bytes

// But in parse_range (parser.rs:144):
let dat_data = self.extract_dat_data(data)?; // Re-decompresses every time!
```

**Solution:** Disk-based caching
```
$INGEST_CACHE_DIR/
  └── uniprot/
      ├── 2024_01.dat          # Decompressed, 3.1GB
      ├── 2024_02.dat
      └── 2025_01.dat
```

**Implementation:**
1. Check cache: `if exists($CACHE_DIR/uniprot/{version}.dat)`
2. If not cached: decompress TAR → write to cache
3. Memory-map cached file for workers
4. Auto-cleanup: delete files older than 7 days

**Benefits:**
- Single decompression per version (idempotent across restarts)
- Multiple workers share same file
- Multiple machines can use shared storage (NFS/EFS)
- Pre-warm cache for historical ingestion

---

### Issue 3: N+1 Database Queries

**Location:** `crates/bdp-server/src/ingest/uniprot/storage.rs:79-102`

**Problem:**
Each protein makes 7-10 sequential DB calls:
```rust
organism_id = get_or_create_organism()      // 2 queries (SELECT + INSERT)
entry_id = create_registry_entry()          // 1 query
create_data_source()                        // 1 query
sequence_id = get_or_create_sequence()      // 2 queries (SELECT + INSERT)
create_protein_metadata()                   // 1 query
version_id = create_version()               // 1 query
create_version_files() × 3 formats          // 3 queries
```

**Total:** 10,000 queries per 1000-entry batch

**Solution:** Batch operations + caching

#### Strategy 1: Pre-cache Organisms
```rust
struct OrganismCache {
    cache: HashMap<i32, Uuid>,  // taxonomy_id → organism_id
    last_refreshed: SystemTime,
    refresh_interval: Duration,  // 5 minutes
}

impl OrganismCache {
    async fn get_or_create(&mut self, taxonomy_id: i32) -> Result<Uuid> {
        // Check cache first
        if let Some(id) = self.cache.get(&taxonomy_id) {
            return Ok(*id);
        }

        // Refresh cache if stale (debounced)
        if self.last_refreshed.elapsed()? > self.refresh_interval {
            self.refresh().await?;

            // Try again after refresh
            if let Some(id) = self.cache.get(&taxonomy_id) {
                return Ok(*id);
            }
        }

        // Create new organism
        let id = self.create_organism(taxonomy_id).await?;
        self.cache.insert(taxonomy_id, id);
        Ok(id)
    }

    async fn refresh(&mut self) -> Result<()> {
        let organisms = sqlx::query!("SELECT taxonomy_id, data_source_id FROM organism_metadata")
            .fetch_all(&self.db)
            .await?;

        self.cache.clear();
        for org in organisms {
            self.cache.insert(org.taxonomy_id, org.data_source_id);
        }
        self.last_refreshed = SystemTime::now();
        Ok(())
    }
}
```

**Reduces:** 1000 organism lookups → 0 queries (cached)

#### Strategy 2: Batch Insert with QueryBuilder
```rust
pub async fn store_entries(&self, entries: &[UniProtEntry]) -> Result<usize> {
    let mut tx = self.db.begin().await?;

    // 1. Batch insert registry entries (1 query for 1000 rows)
    let mut query = QueryBuilder::new("INSERT INTO registry_entries (organization_id, slug, name, description, entry_type) VALUES ");
    query.push_values(entries.iter().take(500), |mut b, entry| {
        b.push_bind(self.organization_id)
         .push_bind(&entry.accession)
         .push_bind(format!("{} [{}]", entry.protein_name, entry.organism_name))
         .push_bind(format!("UniProt protein: {}", entry.protein_name))
         .push_bind("data_source");
    });
    query.push(" ON CONFLICT (slug) DO UPDATE SET updated_at = NOW() RETURNING id");
    let entry_ids = query.build_query_as::<(Uuid,)>().fetch_all(&mut *tx).await?;

    // 2. Batch insert data sources (1 query for 1000 rows)
    // ... similar QueryBuilder pattern

    // 3. Batch insert sequences with deduplication (1 query)
    // ... similar QueryBuilder pattern

    // 4. Batch insert protein_metadata (1 query)
    // ... similar QueryBuilder pattern

    // 5. Batch insert versions (1 query)
    // ... similar QueryBuilder pattern

    // 6. Batch insert version_files (3 queries for 3000 rows, chunked)
    // ... similar QueryBuilder pattern

    tx.commit().await?;
    Ok(entries.len())
}
```

**PostgreSQL limits:** Max 65,535 parameters per query → chunk into batches of 500 rows

**Reduces:** 10,000 queries → ~20-30 queries per 1000-entry batch = **300-500x improvement**

---

### Issue 4: Incorrect Taxonomy Classification

**Location:** `crates/bdp-server/src/ingest/uniprot/storage.rs:146, 206`

**Problem:**
```rust
// Line 146: ALL taxonomy entities marked as "organism"
INSERT INTO data_sources (id, source_type) VALUES ($1, 'organism')

// Even viruses get source_type = 'organism'!
```

**UniProt DAT Format (NOT currently parsed):**
```
OS   Human immunodeficiency virus 1 (HIV-1).
OX   NCBI_TaxID=11676;
OC   Viruses; Riboviria; Orthornavirae; Kitrinoviricota;     ← NOT PARSED!
OC   Flasuviricetes; Amarillovirales; Flaviviridae; Flavivirus.
```

**Solution:** Parse OC (Organism Classification) line

The OC line contains taxonomic lineage with domain as first element:
- `Eukaryota` → `source_type = "organism"`
- `Viruses` → `source_type = "virus"`
- `Bacteria` → `source_type = "bacteria"`
- `Archaea` → `source_type = "archaea"`

**Implementation:**
```rust
// parser.rs: Add to UniProtEntry
pub struct UniProtEntry {
    // ... existing fields
    pub taxonomy_lineage: Vec<String>,  // NEW: ["Viruses", "Riboviria", ...]
}

// parser.rs: Parse OC line
fn parse_oc_line(&mut self, line: &str) -> Result<()> {
    let oc_part = line.trim_start_matches("OC   ");
    for taxon in oc_part.split(';') {
        let trimmed = taxon.trim().trim_end_matches('.');
        if !trimmed.is_empty() {
            self.taxonomy_lineage.push(trimmed.to_string());
        }
    }
    Ok(())
}

// storage.rs: Classify based on lineage
fn classify_source_type(lineage: &[String]) -> &'static str {
    match lineage.first().map(|s| s.as_str()) {
        Some("Viruses") => "virus",
        Some("Bacteria") => "bacteria",
        Some("Archaea") => "archaea",
        Some("Eukaryota") => "organism",
        _ => "organism", // Fallback for unknown/malformed
    }
}
```

**TODO:** Later, integrate full NCBI taxonomy database for advanced classification

**References:**
- [UniProt Knowledgebase User Manual](https://web.expasy.org/docs/userman.html)
- [UniProt Taxonomic Lineage Help](https://www.uniprot.org/help/taxonomic_lineage)

---

### Issue 5: Wrong Slug Format

**Location:** `crates/bdp-server/src/ingest/uniprot/storage.rs:124`

**Problem:**
```rust
let slug = format!("organism-{}", taxonomy_id);  // "organism-9606"
```

**Expected format (per CLI spec):**
- Individual protein: `uniprot:P12345@1.0`
- Organism bundle: `uniprot:homo-sapiens@1.0`
- Database bundle: `uniprot:swissprot@1.0`

**Fix:**
```rust
fn taxonomy_to_slug(organism_name: &str, taxonomy_id: i32) -> String {
    let name = organism_name
        .split('(').next()  // Remove "(Human)" suffix
        .unwrap_or(organism_name)
        .trim()
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    if name.is_empty() || name.len() > 100 {
        format!("taxon-{}", taxonomy_id)  // Fallback
    } else {
        name
    }
}

// Examples:
// "Homo sapiens (Human)" → "homo-sapiens"
// "Escherichia coli K-12" → "escherichia-coli-k-12"
// "Human immunodeficiency virus 1" → "human-immunodeficiency-virus-1"
```

---

### Issue 6: Missing Bundles

**Current:** Only creates `uniprot-all` bundle
**Expected:** Create organism-specific and database bundles

**Bundle Structure:**

```
uniprot:swissprot@1.0                    # All reviewed proteins (570k)
  └─ dependencies: [all individual proteins]

uniprot:homo-sapiens@1.0                 # All human proteins (20k)
  └─ dependencies: [P12345, P67890, ...]

uniprot:mus-musculus@1.0                 # All mouse proteins (15k)
  └─ dependencies: [Q8VDD5, Q91VR5, ...]

uniprot:human-immunodeficiency-virus-1@1.0  # All HIV proteins
  └─ dependencies: [P03366, P03367, ...]
```

**Implementation:**
```rust
impl UniProtStorage {
    /// Create bundles after all proteins stored
    pub async fn create_bundles(&self, entries: &[UniProtEntry]) -> Result<()> {
        // 1. Group proteins by organism
        let mut by_organism: HashMap<i32, Vec<Uuid>> = HashMap::new();
        for entry in entries {
            by_organism
                .entry(entry.taxonomy_id)
                .or_default()
                .push(entry_id);
        }

        // 2. Create organism bundles (no minimum threshold)
        for (taxonomy_id, protein_ids) in by_organism {
            let organism_name = self.get_organism_name(taxonomy_id).await?;
            let slug = taxonomy_to_slug(&organism_name, taxonomy_id);

            self.create_bundle(
                &slug,
                &format!("{} (UniProt Proteins)", organism_name),
                &protein_ids,
                "bundle",  // or inherit organism type?
            ).await?;
        }

        // 3. Create swissprot bundle (all proteins)
        let all_protein_ids: Vec<_> = by_organism.values().flatten().copied().collect();
        self.create_bundle(
            "swissprot",
            "UniProt Swiss-Prot (Reviewed Proteins)",
            &all_protein_ids,
            "bundle",
        ).await?;

        Ok(())
    }
}
```

**Question:** Should organism bundles inherit `source_type` from organism classification?
- Option A: All bundles → `source_type = "bundle"` (simpler)
- Option B: Human bundle → `"organism"`, HIV bundle → `"virus"` (more semantic)

**Recommendation:** Option A for simplicity

---

## Configuration Changes

### Increase Worker Count
**Location:** `crates/bdp-server/src/ingest/uniprot/pipeline.rs:780`

**Current:**
```rust
let num_workers = std::cmp::min(4, total_records / self.batch_config.parse_batch_size + 1);
```

**New:**
```rust
let num_workers = std::cmp::min(16, total_records / self.batch_config.parse_batch_size + 1);
```

### Increase Batch Size
**Location:** `crates/bdp-server/src/ingest/config.rs:244-247`

**Current:**
```rust
batch_size: std::env::var("INGEST_UNIPROT_BATCH_SIZE")
    .unwrap_or_else(|| "1000".to_string())
    .parse()
    .unwrap_or(1000),
```

**New:**
```rust
batch_size: std::env::var("INGEST_UNIPROT_BATCH_SIZE")
    .unwrap_or_else(|| "5000".to_string())
    .parse()
    .unwrap_or(5000),
```

---

## Implementation Plan

### Phase 1: Critical Fixes (Agents 1-2)
**Priority:** CRITICAL
**Duration:** Parallel execution

#### Agent 1: Storage Error Handling + Config
**Files:**
- `crates/bdp-server/src/ingest/uniprot/storage.rs`
- `crates/bdp-server/src/ingest/uniprot/pipeline.rs`
- `crates/bdp-server/src/ingest/config.rs`

**Tasks:**
1. Fix error swallowing in `store_entries()` (lines 58-74)
2. Add validation: `source_type` must be non-empty
3. Ensure transactional integrity (add `tx.begin()` / `tx.commit()`)
4. Change worker count: 4 → 16 (line 780)
5. Change batch size: 1000 → 5000 (config.rs)
6. Add error aggregation and logging

**Acceptance criteria:**
- Errors logged at ERROR level with full context
- Storage reports actual success/failure counts
- Transactions rollback on any error
- Config changes applied

---

#### Agent 2: TAR Decompression Caching
**Files:**
- `crates/bdp-server/src/ingest/uniprot/parser.rs`
- `crates/bdp-server/src/ingest/uniprot/pipeline.rs`
- `crates/bdp-server/src/ingest/config.rs`

**Tasks:**
1. Add `INGEST_CACHE_DIR` env var (default: `/tmp/bdp-ingest-cache`)
2. Implement cache check/write in `download_phase()`
3. Add `parse_range_predecompressed()` method to skip extraction
4. Memory-map or read cached file for workers
5. Add cache cleanup job (delete files >7 days old)

**Cache structure:**
```
$INGEST_CACHE_DIR/
  └── uniprot/
      ├── 2024_01.dat          # Decompressed DAT
      ├── 2024_01.dat.lock     # Lock file for atomic writes
      └── .cleanup_marker      # Last cleanup timestamp
```

**Acceptance criteria:**
- Single decompression per version (idempotent)
- Workers read from cached file (no re-decompression)
- Cache cleanup runs automatically
- Logs show cache hits/misses

---

### Phase 2: Performance Optimization (Agents 3-4)
**Priority:** HIGH
**Duration:** Parallel execution

#### Agent 3: Organism Cache + Batch Operations
**Files:**
- `crates/bdp-server/src/ingest/uniprot/storage.rs`
- `crates/bdp-server/src/ingest/uniprot/pipeline.rs`

**Tasks:**
1. Implement `OrganismCache` struct with refresh logic
2. Add cache invalidation on organism updates (debounced, 5 min refresh)
3. Refactor `store_entries()` to use `sqlx::QueryBuilder` for batch inserts
4. Group operations: registry_entries, data_sources, sequences, protein_metadata, versions, version_files
5. Maintain transaction boundaries
6. Handle sequence deduplication within batch

**Batch sizes:**
- Registry entries: 500 rows per query
- Data sources: 500 rows per query
- Protein metadata: 500 rows per query
- Version files: 500 rows per query (3 queries for 1500 files)

**Acceptance criteria:**
- Organism lookups: 1000 queries → 0 queries (cached)
- Total queries: 10,000 → 20-30 per batch
- Transactions ensure atomicity
- Sequence deduplication works correctly

---

#### Agent 4: Taxonomy Classification
**Files:**
- `crates/bdp-server/src/ingest/uniprot/models.rs`
- `crates/bdp-server/src/ingest/uniprot/parser.rs`
- `crates/bdp-server/src/ingest/uniprot/storage.rs`

**Tasks:**
1. Add `taxonomy_lineage: Vec<String>` to `UniProtEntry`
2. Parse OC (Organism Classification) line in parser
3. Handle multi-line OC entries
4. Implement `classify_source_type(lineage)` helper
5. Update organism creation to use classified type
6. Add TODO comment for NCBI taxonomy integration

**Classification logic:**
```rust
match lineage.first().map(|s| s.as_str()) {
    Some("Viruses") => "virus",
    Some("Bacteria") => "bacteria",
    Some("Archaea") => "archaea",
    Some("Eukaryota") => "organism",
    _ => "organism", // Fallback
}
```

**Acceptance criteria:**
- OC lines parsed correctly (including multi-line)
- Viruses classified as "virus"
- Bacteria classified as "bacteria"
- Archaea classified as "archaea"
- Eukaryotes classified as "organism"
- Fallback to "organism" for unknown

---

### Phase 3: Bundle Creation (Agent 5)
**Priority:** MEDIUM
**Duration:** Sequential (after Phase 1-2 complete)

#### Agent 5: Bundle Creation System
**Files:**
- `crates/bdp-server/src/ingest/uniprot/storage.rs`
- `crates/bdp-server/src/ingest/uniprot/pipeline.rs`

**Tasks:**
1. Implement `taxonomy_to_slug()` helper (human-readable slugs)
2. Implement `create_bundles()` method (organism + swissprot)
3. Create organism bundles (one per unique organism, no threshold)
4. Create swissprot bundle (all proteins)
5. Fix dependencies to reference correct slugs
6. Remove `uniprot-all` bundle (replaced by `swissprot`)
7. Call `create_bundles()` at end of storage phase

**Bundle format:**
```yaml
slug: "homo-sapiens"
name: "Homo sapiens (Human) - UniProt Proteins"
source_type: "bundle"
dependencies: [uniprot:P12345@1.0, uniprot:P67890@1.0, ...]
```

**Acceptance criteria:**
- Organism bundles use human-readable slugs
- Swissprot bundle created with all proteins
- Dependencies reference correct protein slugs
- No duplicate bundles
- Bundles queryable via CLI: `bdp source add uniprot:homo-sapiens-fasta@1.0`

---

## Testing Plan

### Unit Tests
1. `taxonomy_to_slug()` edge cases
   - Unicode characters
   - Very long names (>100 chars)
   - Names with special characters
   - Fallback to `taxon-{id}`

2. `classify_source_type()` coverage
   - All four domains (Eukaryota, Viruses, Bacteria, Archaea)
   - Empty lineage
   - Unknown domain

3. OC line parsing
   - Single-line OC
   - Multi-line OC
   - Malformed OC

4. Batch insert logic
   - Chunk size boundaries
   - Transaction rollback on error
   - Sequence deduplication

### Integration Tests
1. Cache behavior
   - First ingestion (cache miss)
   - Second ingestion (cache hit)
   - Cache cleanup

2. Organism cache
   - Initial load
   - Cache miss → refresh
   - Debounced refresh

3. Bundle creation
   - Organism bundles for all taxonomy IDs
   - Swissprot bundle completeness
   - Dependency links

### E2E Test
1. Ingest sample UniProt version (1000 proteins)
2. Verify all 1000 stored
3. Verify bundles created
4. Query via CLI: `bdp source add uniprot:homo-sapiens-fasta@1.0`
5. Verify dependency resolution

---

## Monitoring & Observability

### Metrics to Track
1. Storage success rate (currently 0%)
2. Decompression cache hit rate
3. Organism cache hit rate
4. Queries per batch
5. Ingestion throughput (proteins/sec)
6. Bundle creation time

### Logging Improvements
- Add structured logging with tracing spans
- Log error details at ERROR level
- Track progress: "Stored 1000/570000 proteins (0.18%)"

---

## Missing UniProt DAT Fields (Future Enhancement)

### Currently Parsed Fields ✓
- **AC** - Accession number (primary identifier)
- **ID** - Entry name
- **DE** - Description (RecName only)
- **GN** - Gene name
- **OS** - Organism name
- **OX** - NCBI Taxonomy ID
- **DT** - Release date
- **SQ** - Sequence info (length, mass)
- Amino acid sequence

### High Priority Missing Fields

#### 1. **FT (Feature Table)** - Structural/Functional Annotations
Most requested by bioinformaticians for:
- Protein domains (e.g., "DOMAIN 50..150; Kinase")
- Active sites (e.g., "ACT_SITE 75; Catalytic")
- Binding sites (e.g., "BINDING 120; ATP")
- Post-translational modifications (e.g., "MOD_RES 45; Phosphoserine")
- Signal peptides, transmembrane regions
- Disease variants (e.g., "VARIANT 123; A->T; Cancer-associated")

**Storage suggestion:** New table `protein_features`
```sql
CREATE TABLE protein_features (
    id UUID PRIMARY KEY,
    protein_metadata_id UUID REFERENCES protein_metadata(id),
    feature_type VARCHAR(50),  -- 'DOMAIN', 'BINDING', 'ACT_SITE', etc.
    start_position INT,
    end_position INT,
    description TEXT,
    evidence_level VARCHAR(20)
);
```

#### 2. **DR (Database Cross-References)** - Integration with External DBs
Links to 100+ databases:
- **PDB** - 3D structures (most important!)
- **GO** - Gene Ontology functional annotations
- **InterPro** - Protein family classification
- **KEGG** - Metabolic pathways
- **Pfam** - Protein domain databases
- **RefSeq** - NCBI reference sequences

Example:
```
DR   PDB; 1A2B; X-ray; 2.50 A; A/B=1-120.
DR   GO; GO:0005524; F:ATP binding; IEA:UniProtKB-KW.
DR   InterPro; IPR000719; Protein_kinase_dom.
```

**Storage suggestion:** New table `protein_cross_references`
```sql
CREATE TABLE protein_cross_references (
    id UUID PRIMARY KEY,
    protein_metadata_id UUID REFERENCES protein_metadata(id),
    database_name VARCHAR(50),  -- 'PDB', 'GO', 'InterPro', etc.
    database_id VARCHAR(255),   -- External identifier
    metadata JSONB              -- Database-specific fields
);
```

#### 3. **CC (Comments)** - Structured Functional Annotations
Topic-based annotations:
- **FUNCTION** - Biological role
- **CATALYTIC ACTIVITY** - Enzymatic reactions
- **SUBCELLULAR LOCATION** - Where protein is found
- **INTERACTION** - Protein-protein interactions
- **DISEASE** - Associated pathologies
- **SIMILARITY** - Evolutionary relationships

Example:
```
CC   -!- FUNCTION: Catalyzes the phosphorylation of proteins at serine/threonine
CC       residues. Plays a role in cell cycle regulation.
CC   -!- SUBCELLULAR LOCATION: Nucleus. Cytoplasm.
CC   -!- DISEASE: Mutations are associated with cancer.
```

**Storage suggestion:** Add `comments` JSONB field to `protein_metadata`
```sql
ALTER TABLE protein_metadata ADD COLUMN comments JSONB;
-- Structure: {"function": "...", "subcellular_location": "...", "disease": "..."}
```

#### 4. **PE (Protein Existence)** - Evidence Level
Quality indicator (1-5):
- **1** - Experimental evidence at protein level
- **2** - Experimental evidence at transcript level
- **3** - Inferred from homology
- **4** - Predicted
- **5** - Uncertain

**Storage suggestion:** Add to `protein_metadata`
```sql
ALTER TABLE protein_metadata ADD COLUMN protein_existence INT CHECK (protein_existence BETWEEN 1 AND 5);
```

#### 5. **KW (Keywords)** - Controlled Vocabulary
Functional classification terms:
- "ATP-binding"
- "Kinase"
- "Transmembrane"
- "Signal"
- "Glycoprotein"

**Storage suggestion:** Many-to-many relationship
```sql
CREATE TABLE keywords (
    id UUID PRIMARY KEY,
    keyword VARCHAR(100) UNIQUE
);

CREATE TABLE protein_keywords (
    protein_metadata_id UUID REFERENCES protein_metadata(id),
    keyword_id UUID REFERENCES keywords(id),
    PRIMARY KEY (protein_metadata_id, keyword_id)
);
```

#### 6. **OC (Organism Classification)** - Full Lineage
Already in current plan! Parse multi-line taxonomic lineage.

#### 7. **DE Alternative Names** - AltName, SubName
Currently only parsing **RecName** (recommended name). Missing:
- **AltName** - Alternative names
- **SubName** - Submitted names (for TrEMBL)
- Short names, EC numbers

Example:
```
DE   RecName: Full=Epidermal growth factor receptor;
DE            Short=EGFR;
DE            EC=2.7.10.1;
DE   AltName: Full=Proto-oncogene c-ErbB-1;
```

**Storage suggestion:** Add to `protein_metadata`
```sql
ALTER TABLE protein_metadata
    ADD COLUMN alternative_names TEXT[],
    ADD COLUMN ec_numbers TEXT[];
```

### Medium Priority Missing Fields

#### 8. **OG (Organelle)** - Subcellular Origin
Indicates if protein is from:
- Mitochondrion
- Plastid (chloroplast)
- Plasmid

Rare but useful for filtering.

#### 9. **OH (Organism Host)** - Viral Hosts
Only present for viruses - lists host organisms.

### Lower Priority Fields

#### 10. **References (RN, RP, RC, RX, RG, RA, RT, RL)**
Publication metadata - useful for provenance but bulky.

---

## Recommended Implementation Priority

### Phase 1 (This PR) ✅
- Fix storage errors
- TAR caching
- Batch operations
- OC (organism classification)
- Bundle creation

### Phase 2 (Next PR)
- **PE** - Protein existence (easy, single field)
- **KW** - Keywords (moderate, many-to-many)
- **DE** - Alternative names (easy, array field)
- **OC** - Full lineage parsing (update from current plan)

### Phase 3 (Separate Feature)
- **DR** - Database cross-references (complex, 100+ databases)
- **FT** - Feature table (complex parsing, position ranges)
- **CC** - Comments (complex, topic-based parsing)

### Phase 4 (Advanced)
- **OG** - Organelle
- **OH** - Organism host
- References parsing

---

## Future Work (Not in This PR)

### NCBI Taxonomy Integration
- Full taxonomic hierarchy
- Nested bundles (e.g., `uniprot:mammalia@1.0` → `uniprot:homo-sapiens@1.0`)
- Proper domain/kingdom/phylum classification

### NCBI Ingestion
- Similar pipeline structure
- Reuse caching and batch operations
- Different bundle structure (by assembly, chromosome, etc.)

### Performance Enhancements
- Parallel bundle creation
- Incremental ingestion (only new proteins)
- Streaming ingestion (avoid loading full file in memory)

### Additional UniProt Features
- Parse FT (feature table) for domains and modifications
- Parse DR (cross-references) for PDB, GO, InterPro integration
- Parse CC (comments) for functional annotations
- Add PE (protein existence) quality indicator
- Add KW (keywords) for better search/filtering

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Batch operations break sequence deduplication | HIGH | Careful testing, maintain hash-based lookup |
| Cache corruption | MEDIUM | Atomic writes with lock files, checksums |
| Organism cache stale | LOW | 5-minute refresh interval, debounced |
| Slug collisions | LOW | Fallback to `taxon-{id}`, unique constraints in DB |
| Bundle creation failure | MEDIUM | Retry logic, separate transaction from storage |

---

## Success Criteria

1. **Storage success rate:** 0% → 100%
2. **Ingestion speed:** 10-100x faster (measure before/after)
3. **Cache utilization:** Single decompression per version
4. **Query reduction:** 10,000 queries → ~30 queries per batch
5. **Correct classification:** Viruses, bacteria, archaea properly typed
6. **Bundle completeness:** All organisms + swissprot bundles created
7. **CLI integration:** `bdp source add uniprot:homo-sapiens-fasta@1.0` works

---

## Agent Execution Order

**Parallel (Phase 1):**
- Agent 1: Storage errors + config
- Agent 2: TAR decompression caching

**Parallel (Phase 2):**
- Agent 3: Organism cache + batch operations
- Agent 4: Taxonomy classification

**Sequential (Phase 3):**
- Agent 5: Bundle creation (depends on Phase 1-2)

**Total estimated time:** 2-3 hours (parallel execution)

---

## References

- [UniProt Knowledgebase User Manual](https://web.expasy.org/docs/userman.html)
- [UniProt Taxonomic Lineage Help](https://www.uniprot.org/help/taxonomic_lineage)
- [BDP File Formats Specification](../docs/agents/design/file-formats.md)
- [BDP Database Schema](../docs/agents/design/database-schema.md)
