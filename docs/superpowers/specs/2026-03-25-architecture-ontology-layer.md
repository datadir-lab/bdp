# BDP Architecture — Ontology Layer & Knowledge Graph Design Spec

**Date:** 2026-03-25
**Status:** Approved
**Covers:** Workspace restructure, DB schema redesign, Apache AGE, agent-oriented architecture

---

## Overview

BDP is growing from a bioinformatics package registry into a full biological knowledge
graph platform powering auto-research agents. This spec defines the target architecture
for that transition: how the workspace, database, ingestion pipelines, and graph layer
should be structured to scale to all of bioinformatics — billions of entities, hundreds
of millions of edges, and agents doing continuous data mining.

This document is the authoritative design reference. Implementation plans reference it.

---

## Goals

1. Migrate all ingestion to `bdp-ingest` as a proper workspace crate (not a stub)
2. Eliminate JSONB from all queryable domain data — full relational design
3. Replace the `source_type` CHECK constraint anti-pattern with a FK table
4. Introduce `ont_term_xrefs` as the cross-pipeline entity resolution table
5. Add Apache AGE (PostgreSQL graph extension) for Cypher-based graph traversal
6. Design entity-typed graph node attribute tables (no JSONB properties)
7. Replace generic `graph_edges` with typed association tables (industry-standard KG pattern)
8. Separate `graph_layout` (WebGPU positions) from domain data
9. Define agent-oriented additions: MCP server, streaming queries, provenance
10. Define big data ingestion patterns: incremental, checkpoint/resume, backpressure

**Non-goals (this spec):**
- Vector embeddings implementation (covered in `2026-03-21-vectors-embedding-design.md`)
- WebGPU graph view implementation (covered in `2026-03-22-graph-view-webgpu-design.md`)
- Individual pipeline implementation (covered in per-pipeline plan files)

---

## Alignment with Data Mining & Web Research Best Practices

### What research agents need from BDP

Auto-research agents constantly doing data mining on biological knowledge require:

| Capability | Current State | Target |
|------------|--------------|--------|
| Multi-hop graph traversal | Recursive SQL CTEs (painful) | Apache AGE Cypher queries |
| Semantic entity search | Full-text only | pgvector HNSW (planned) |
| Entity resolution | None | `entity_aliases` table |
| Cross-db linking | Manual per-pipeline | `ont_term_xrefs` unified table |
| Query provenance | None | Dataset version stamps on results |
| Streaming results | None | Chunked HTTP responses for large result sets |
| Cached expensive queries | None | Redis TTL cache for graph queries |
| MCP interface | Not built | `bdp-mcp` (elevate priority from P2 to P1) |

### Apache AGE — confirmed

Apache AGE (Apache Graph Extension) adds Cypher query language directly to PostgreSQL.
It operates on the same tables, same connection pool, same ACID transactions. No separate
graph database process.

**Why this matters for research agents:**

Without AGE, a 3-hop traversal from a protein to related diseases is:
```sql
WITH RECURSIVE traversal AS (
  SELECT id, label, entity_type_id, 0 AS depth FROM graph_nodes WHERE external_id = 'P04637'
  UNION ALL
  SELECT n.id, n.label, n.entity_type_id, t.depth + 1
  FROM traversal t
  JOIN graph_edges e ON e.source_node_id = t.id OR e.target_node_id = t.id
  JOIN graph_nodes n ON n.id = CASE WHEN e.source_node_id = t.id THEN e.target_node_id ELSE e.source_node_id END
  WHERE t.depth < 3
)
SELECT * FROM traversal;
```

With AGE, the same query is:
```cypher
MATCH path = (p:protein {external_id: 'P04637'})-[*1..3]-(n)
RETURN n.label, labels(n)[0] AS type, length(path) AS hops
ORDER BY hops
LIMIT 100
```

This is the natural language for agents. The MCP server exposes a `graph_query` tool
that takes a Cypher string — agents compose these naturally.

**Integration plan:**
- Install `apache_age` PostgreSQL extension (migration `20260325000001_enable_age.sql`)
- The `graph_sync` job projects typed association tables → AGE edges (not a generic `graph_edges` table)
- AGE vertices come from `graph_nodes` (entity index); AGE edges come from typed association tables
- Expose `POST /api/v1/graph/cypher` endpoint (authenticated, read-only Cypher)
- BDP-81 (activate entity types) triggers AGE graph refresh

**Scale ceiling:** AGE has been tested to 1B+ edges on commodity hardware. Adequate
for STRING (2B interactions) with partitioned typed association tables and appropriate indexing.

### Big data ingestion patterns

Current pipelines do full-reload ingestion. At UniProt TrEMBL scale (250M proteins),
STRING scale (2B+ edges), this becomes impractical. Required patterns:

**Incremental ingestion:**
Every pipeline must track a `last_ingested_at` cursor (FTP modification date or
upstream version). On subsequent runs, only process records newer than the cursor.
The `ingestion_jobs` table already has `metadata JSONB` — cursor lives there.

**Checkpoint / resume:**
Long-running jobs (GenBank, STRING) write progress checkpoints every N records.
On restart after failure, resume from last checkpoint rather than beginning.
Add `checkpoint_data JSONB` to `ingestion_jobs` (diagnostic context — JSONB acceptable here).

**Streaming decompression:**
Already implemented for GenBank. Apply the same pattern to STRING (~40GB gzipped),
UniProt TrEMBL (~200GB XML). Never decompress to disk.

**Backpressure:**
Ingestion pipelines must yield to API traffic. Implement via tokio semaphore with
configurable max-concurrent-db-connections for ingest vs. API.

**Dead letter queue:**
Records that fail parsing after N retries go to `ingest_failed_records` table with
raw bytes + error. Agents can query this for data quality auditing.

```sql
CREATE TABLE ingest_failed_records (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline    TEXT NOT NULL,          -- 'uniprot', 'string_db'
    batch_id    UUID,                   -- links to ingestion_job
    raw_data    BYTEA NOT NULL,         -- original bytes
    error_msg   TEXT NOT NULL,
    attempt_count SMALLINT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_ingest_failed_pipeline ON ingest_failed_records(pipeline, created_at);
```

---

## Workspace Structure

### Target crate layout

```
crates/
  bdp-core/               # Rename from bdp-common
    src/
      error.rs            # BdpError, BdpResult
      types/              # shared value types (OntologyId, ExternalId, etc.)
      logging.rs
      checksum.rs

  bdp-ingest/             # ALL ingestion — library + binary
    src/
      lib.rs
      main.rs             # standalone binary: bdp-ingest [pipeline] [--catchup]

      common/
        obo.rs            # Generic OBO parser (extracted from gene_ontology)
        ftp.rs
        http.rs
        decompression.rs
        version_discovery.rs
        batch.rs          # BatchConfig, chunked inserts
        checkpoint.rs     # checkpoint/resume for long-running jobs

      framework/
        pipeline.rs       # PipelineRunner trait
        coordinator.rs    # existing
        worker.rs         # existing

      pipelines/
        mod.rs            # pipeline registry: Vec<Box<dyn PipelineRunner>>
        gene_ontology/    # refactored to use common/obo.rs
        uniprot/
        ncbi_taxonomy/
        genbank/
        interpro/
        ensembl/          # complete existing stub
        reactome/         # NEW
        mondo/            # NEW
        hpo/              # NEW
        chebi/            # NEW
        string_db/        # NEW

      orchestrator.rs     # registration-based JoinSet (not hardcoded conditionals)
      scheduler.rs
      config.rs           # all enable flags + cursor config

  bdp-graph/              # NEW — separate from server
    src/
      sync/
        node_sync.rs      # project domain tables → graph_nodes
        edge_sync.rs      # build cross-pipeline edges
        activation.rs     # set is_active=true as data arrives
      layout/
        louvain.rs        # community detection
        force_atlas2.rs   # per-community layout
        normalize.rs      # → [-1.0, 1.0]
      age/
        mod.rs            # Apache AGE integration
        cypher.rs         # safe Cypher query builder (parameterized)
      api/                # CQRS handlers for /api/v1/graph/* (moved from server)

  bdp-server/             # API only — no ingest logic, imports bdp-ingest + bdp-graph
    src/
      features/           # existing CQRS slices unchanged
      api/
      middleware/
      main.rs             # wires orchestrator + graph sync on startup

  bdp-mcp/                # NEW — MCP server for AI agent tool use (elevate to P1)
    src/
      tools/
        search_sources.rs     # semantic + full-text search
        graph_query.rs        # Cypher passthrough (read-only)
        entity_lookup.rs      # resolve aliases → canonical entity
        get_version.rs        # dataset version info for provenance

  bdp-cli/                # unchanged
```

### Why `bdp-graph` is separate

Graph sync, layout computation, and Cypher query handling are operationally distinct
from API request handling. `bdp-graph` can run as a library imported by both
`bdp-server` (for the /graph API endpoints) and `cargo xtask graph sync` (offline job).

### PipelineRunner trait (registration pattern)

```rust
// bdp-ingest/src/framework/pipeline.rs

pub trait PipelineRunner: Send + 'static {
    fn name(&self) -> &'static str;
    fn is_enabled(&self, config: &IngestConfig) -> bool { true }
    async fn run(self) -> anyhow::Result<PipelineStats>;
}

// orchestrator.rs builds the registry:
fn build_pipeline_registry(config: &IngestConfig, db: Arc<PgPool>, ...) -> Vec<Box<dyn PipelineRunner>> {
    vec![
        Box::new(UniProtPipeline::new(config, db.clone(), ...)),
        Box::new(NcbiTaxonomyPipeline::new(config, db.clone(), ...)),
        Box::new(GoPipeline::new(config, db.clone(), ...)),
        Box::new(ReactomePipeline::new(config, db.clone(), ...)),
        Box::new(MondoPipeline::new(config, db.clone(), ...)),
        Box::new(HpoPipeline::new(config, db.clone(), ...)),
        Box::new(ChebiPipeline::new(config, db.clone(), ...)),
        Box::new(StringDbPipeline::new(config, db.clone(), ...)),
        // adding a new pipeline = one line here + one module
    ]
}
```

---

## Database Schema

### 1. Kill the `source_type` CHECK constraint

**Current (wrong):** Every new pipeline requires a DDL migration to extend an enum.

```sql
-- Migrate away from:
source_type IN ('protein', 'taxonomy', 'go_term', 'interpro_entry', ...)

-- To:
CREATE TABLE source_types (
    name        TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed (and extend with INSERT, never ALTER TABLE):
INSERT INTO source_types VALUES
    ('protein',          'Protein',         'UniProt protein sequences and annotations'),
    ('taxonomy',         'Taxon',           'NCBI taxonomy nodes'),
    ('genomic_sequence', 'Genomic Sequence','GenBank/RefSeq nucleotide sequences'),
    ('go_term',          'GO Term',         'Gene Ontology terms'),
    ('interpro_entry',   'InterPro Entry',  'InterPro protein family/domain entries'),
    ('pathway',          'Pathway',         'Biological pathways (Reactome, KEGG)'),
    ('disease',          'Disease',         'Disease terms (MONDO)'),
    ('phenotype',        'Phenotype',       'Phenotype terms (HPO)'),
    ('compound',         'Compound',        'Chemical compounds (ChEBI, PubChem)'),
    ('drug',             'Drug',            'Drug entities (ChEMBL, DrugBank)'),
    ('variant',          'Variant',         'Genomic variants (ClinVar, dbSNP)'),
    ('structure',        'Structure',       'Protein structures (PDB, AlphaFold)'),
    ('bundle',           'Bundle',          'Aggregate data source bundle');

ALTER TABLE data_sources
    DROP CONSTRAINT check_source_type,
    ADD COLUMN source_type_name TEXT REFERENCES source_types(name),
    -- migrate data, then:
    DROP COLUMN source_type,
    RENAME COLUMN source_type_name TO source_type;
```

Adding a new pipeline now = `INSERT INTO source_types` in the pipeline's migration.
No shared constraint to modify. No coordination between pipeline migrations.

### 2. Unified cross-reference table — the key design piece

This table is the spine of cross-pipeline entity resolution.

```sql
CREATE TABLE ont_term_xrefs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Source term (the entity that HAS this xref)
    term_id     UUID NOT NULL,          -- PK of the owning row
    term_table  TEXT NOT NULL,          -- 'go_term_metadata', 'disease_terms', 'protein_metadata'
    -- Target (the thing being referenced)
    source_db   TEXT NOT NULL,          -- 'OMIM', 'CHEBI', 'PMID', 'UniProtKB', 'Reactome'
    source_id   TEXT NOT NULL,          -- '604606', '33709', '12345678', 'P04637', 'R-HSA-123'
    xref_type   TEXT,                   -- 'exact', 'related', 'broader', 'narrower', null=unspecified
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ont_xrefs_term     ON ont_term_xrefs(term_id, term_table);
CREATE INDEX idx_ont_xrefs_source   ON ont_term_xrefs(source_db, source_id);
CREATE INDEX idx_ont_xrefs_reverse  ON ont_term_xrefs(source_db, source_id, term_table);
```

**This table enables:**
- "Find all GO terms, MONDO diseases, HPO phenotypes, and UniProt proteins
  that reference OMIM:604606" → single index scan on `(source_db='OMIM', source_id='604606')`
- "What does GO:0006955 cross-reference?" → scan on `(term_id, term_table='go_term_metadata')`
- Cross-pipeline identity: Reactome's `UniProt2Reactome` file provides protein→pathway
  links via UniProt accession — stored here, resolved at graph sync time

### 3. Unified ontology synonym table

```sql
CREATE TABLE ont_term_synonyms (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id     UUID NOT NULL,
    term_table  TEXT NOT NULL,          -- 'go_term_metadata', 'disease_terms', 'phenotype_terms'
    scope       TEXT NOT NULL CHECK (scope IN ('EXACT','BROAD','NARROW','RELATED')),
    text        TEXT NOT NULL,
    synonym_type TEXT,                  -- 'systematic_synonym', 'abbreviation', 'layperson_term'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ont_synonyms_term  ON ont_term_synonyms(term_id, term_table);
CREATE INDEX idx_ont_synonyms_text  ON ont_term_synonyms USING GIN (to_tsvector('english', text));
CREATE UNIQUE INDEX idx_ont_synonyms_dedup ON ont_term_synonyms(term_id, term_table, scope, text);
```

### 4. JSONB elimination — GO tables

```sql
-- Migration: remove synonyms/xrefs/alt_ids JSONB from go_term_metadata
-- Data moves to ont_term_synonyms, ont_term_xrefs, go_term_alt_ids

CREATE TABLE go_term_alt_ids (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    go_term_id  UUID NOT NULL REFERENCES go_term_metadata(id) ON DELETE CASCADE,
    alt_go_id   TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(go_term_id, alt_go_id)
);
CREATE INDEX idx_go_alt_ids_term    ON go_term_alt_ids(go_term_id);
CREATE INDEX idx_go_alt_ids_alt     ON go_term_alt_ids(alt_go_id);
-- Note: alt_ids are GO-specific (GO:XXXXXXX format) so they stay in a GO-specific table.
-- General xrefs (CHEBI:, Wikipedia:, PMID:) move to ont_term_xrefs.

-- Replace annotation_extension JSONB with proper table
CREATE TABLE go_annotation_extensions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    annotation_id   UUID NOT NULL REFERENCES go_annotations(id) ON DELETE CASCADE,
    relation        TEXT NOT NULL,      -- 'occurs_in', 'has_input', 'part_of', 'has_output'
    filler_db       TEXT NOT NULL,      -- 'CL', 'CHEBI', 'GO', 'UBERON', 'EMAPA', 'PR'
    filler_id       TEXT NOT NULL,      -- '0000236', '33709', '0006955'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_go_ann_ext_ann     ON go_annotation_extensions(annotation_id);
CREATE INDEX idx_go_ann_ext_filler  ON go_annotation_extensions(filler_db, filler_id);
```

### 5. JSONB elimination — protein tables

```sql
-- Replace protein_cross_references.metadata JSONB with explicit columns
-- UniProt xref metadata is structured — each DB has known fields
ALTER TABLE protein_cross_references
    DROP COLUMN metadata,
    ADD COLUMN isoform    TEXT,     -- isoform-specific xref (e.g., P04637-1)
    ADD COLUMN chain      TEXT,     -- PDB chain (e.g., 'A', 'B')
    ADD COLUMN additional TEXT;     -- rare overflow, plain text, not queried

-- protein_cross_references also moves to ont_term_xrefs for unified lookups
-- Keep protein_cross_references for UniProt-specific extra fields (isoform, chain)
-- but join through ont_term_xrefs for cross-pipeline queries
```

### 6. Sequence features — proper genomic schema

```sql
-- Replace sequence features JSONB with hstore + range index
-- Enable hstore extension (lightweight key/value, fully indexable per key)
CREATE EXTENSION IF NOT EXISTS hstore;

CREATE TABLE sequence_features (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sequence_id     UUID NOT NULL REFERENCES genomic_sequences(id) ON DELETE CASCADE,
    feature_type    TEXT NOT NULL,      -- 'CDS', 'gene', 'mRNA', 'rRNA', 'misc_feature'
    start_pos       BIGINT,
    end_pos         BIGINT,
    strand          SMALLINT CHECK (strand IN (-1, 0, 1)),
    phase           SMALLINT CHECK (phase IN (0, 1, 2)),
    -- Structured key/value attributes from GFF3/GenBank FT lines
    -- e.g., gene_id=>"GENE001", transcript_id=>"ENST000", product=>"insulin"
    attributes      HSTORE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Standard indexes
CREATE INDEX idx_seq_features_seq   ON sequence_features(sequence_id);
CREATE INDEX idx_seq_features_type  ON sequence_features(feature_type);

-- Range index for overlap queries: "find all features in chr1:1000000-2000000"
CREATE INDEX idx_seq_features_range ON sequence_features
    USING GIST (int8range(start_pos, end_pos, '[]'));

-- hstore index for attribute queries: "find all features with gene_id = 'TP53'"
CREATE INDEX idx_seq_features_attrs ON sequence_features USING GIN (attributes);
```

Note: `HSTORE` (not JSONB) — flat key/value, GIN-indexed per key, perfect for GFF3
attributes. No nesting, no arrays — genomic feature attributes don't need them.

### 7. Per-pipeline data source chain — the BDP registry model

**Answer to: "will each ingest vertical slice have its own proper data_source → metadata tables?"**

**Yes, always.** Every pipeline follows the same registration chain before writing domain data:

```
registry_entries (slug, name, entry_type)
    └── data_sources (source_type FK → source_types)
           └── versions (version, release_date, data_url)
                  └── [domain-specific metadata tables]
```

This is non-negotiable because it is BDP's core value proposition: every dataset is versioned,
auditable, and reproducible. The chain also enables cross-pipeline version stamps on agent queries.

**Per-pipeline domain tables (what they own, nobody else touches):**

| Pipeline | Registry entry | Domain tables |
|----------|---------------|---------------|
| UniProt | `protein` | `protein_metadata`, `protein_cross_references`, `protein_comments`, `protein_features` |
| NCBI Taxonomy | `taxonomy` | `taxon_nodes`, `taxon_names` |
| GenBank | `genomic_sequence` | `genomic_sequences`, `sequence_features` |
| Gene Ontology | `go_term` | `go_term_metadata`, `go_relationships`, `go_annotations`, `go_term_alt_ids`, `go_annotation_extensions` |
| InterPro | `interpro_entry` | `interpro_entries`, `interpro_proteins`, `interpro_cross_refs` |
| Reactome | `pathway` | `pathway_terms`, `pathway_hierarchy`, `pathway_reactions` |
| MONDO | `disease` | `disease_terms`, `disease_relationships` |
| HPO | `phenotype` | `phenotype_terms`, `phenotype_hierarchy` |
| ChEBI | `compound` | `compound_terms`, `compound_relationships` |
| STRING | `protein_interaction` | *(edges only — `protein_interactions` association table, no "compound" entity)* |
| Ensembl | `gene` | `gene_metadata`, `transcript_metadata`, `gene_cross_refs` |

Association tables (edges between domains) are populated by pipelines that provide the linking data:
- Reactome provides `protein_pathway_associations`
- HPO provides `gene_disease_associations` + `disease_phenotype_associations`
- STRING provides `protein_interactions`
- GO annotations provide `protein_go_annotations` (already `go_annotations`)

### 8. Graph layer — industry research verdict

**Answer to: "do we need graph_nodes and graph_edges? should it be in a layer or between domain models?"**

Research across all major production bioinformatics knowledge graphs:

| System | Entity count | Edge count | Edge architecture |
|--------|-------------|------------|-------------------|
| **Hetionet** | 47K across 11 types | 2.25M across 24 types | **Typed edge tables per relationship** |
| **SPOKE** | 27M across 21 types | 635M across 55 types | **Typed association tables** |
| **PharMeBINet** | 2.9M | 15.9M across 208 types | **Typed domain tables as edges** |
| **RTX-KG2** (NCATS) | 10M+ | 150M+ | **Biolink Model typed predicates** |
| **Monarch KG** | — | — | **OWL associations, typed** |

**Industry consensus: typed association tables ARE the edges.** No production bioinformatics KG
uses a generic `(source_node_id, target_node_id, edge_type_id, properties JSONB)` table.

**Why generic graph_edges fail at BDP scale:**

1. STRING alone has 2B protein interactions — a generic edges table with JSONB properties
   cannot be efficiently indexed for `combined_score > 900` filters
2. Gene-disease associations have 15+ evidence columns (score, association_type, source,
   pubmed_ids, evidence_code) — these are meaningless as JSONB in a generic table
3. Protein-pathway links need to be queryable by species, pathway level, evidence — impossible
   without typed columns

**The correct architecture:**

```
Domain layer (owns entities + associations between them)
  protein_metadata          ← entity: protein
  pathway_terms             ← entity: pathway
  protein_pathway_associations ← EDGE: protein participates_in pathway
  protein_interactions      ← EDGE: protein interacts_with protein (STRING)
  gene_disease_associations ← EDGE: gene associated_with disease (HPO/ClinVar)
  disease_phenotype_associations ← EDGE: disease has_phenotype phenotype (HPO)

Graph index layer (for traversal + rendering only)
  graph_nodes               ← entity index (deduped, typed, for AGE vertex creation)
  graph_layout              ← WebGPU positions (x,y,community_id,size) — separate from entities

Graph query layer
  Apache AGE virtual graph  ← projects domain tables → Cypher-queryable vertices + edges
```

**`graph_nodes` is an entity index, not the entities.** It exists for:
- Deduplication across pipelines (same protein from UniProt and STRING = one node)
- AGE vertex creation (AGE needs a stable integer ID)
- Graph traversal starting points
- `entity_aliases` resolution (alias → `graph_nodes.id` → domain table row)

It does NOT store entity attributes (those live in domain tables + `gn_*_attrs` for denormalized graph queries).

**`graph_layout` is rendering metadata only.** WebGPU positions, community membership,
visual size — nothing that belongs in a domain query lives here.

### 8a. Typed association tables (edges between domain entities)

```sql
-- EDGE: protein participates_in pathway (Reactome UniProt2Reactome)
CREATE TABLE protein_pathway_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id      UUID NOT NULL REFERENCES protein_metadata(id) ON DELETE CASCADE,
    pathway_id      UUID NOT NULL REFERENCES pathway_terms(id) ON DELETE CASCADE,
    source_db       TEXT NOT NULL DEFAULT 'reactome',
    evidence_type   TEXT,               -- 'inferred_from_experiment', 'inferred_from_sequence_model'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(protein_id, pathway_id, source_db)
);
CREATE INDEX idx_ppa_protein   ON protein_pathway_associations(protein_id);
CREATE INDEX idx_ppa_pathway   ON protein_pathway_associations(pathway_id);

-- EDGE: protein interacts_with protein (STRING DB, 2B+ rows)
CREATE TABLE protein_interactions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_a_id    UUID NOT NULL REFERENCES protein_metadata(id) ON DELETE CASCADE,
    protein_b_id    UUID NOT NULL REFERENCES protein_metadata(id) ON DELETE CASCADE,
    combined_score  SMALLINT NOT NULL,          -- 0-1000 (STRING scale)
    experimental_score  SMALLINT,
    coexpression_score  SMALLINT,
    database_score      SMALLINT,
    textmining_score    SMALLINT,
    source_db       TEXT NOT NULL DEFAULT 'string_db',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- enforce canonical ordering (a <= b) to avoid duplicate symmetric edges
    CONSTRAINT protein_interactions_ordered CHECK (protein_a_id <= protein_b_id),
    UNIQUE(protein_a_id, protein_b_id, source_db)
);
CREATE INDEX idx_pi_protein_a   ON protein_interactions(protein_a_id);
CREATE INDEX idx_pi_protein_b   ON protein_interactions(protein_b_id);
CREATE INDEX idx_pi_score       ON protein_interactions(combined_score DESC);

-- EDGE: gene/protein associated_with disease (HPO, ClinVar, OMIM, DisGeNET)
CREATE TABLE gene_disease_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id       UUID NOT NULL,              -- FK to protein_metadata or gene_metadata
    entity_table    TEXT NOT NULL,              -- 'protein_metadata' | 'gene_metadata'
    disease_id      UUID NOT NULL REFERENCES disease_terms(id) ON DELETE CASCADE,
    association_type TEXT NOT NULL,             -- 'causal', 'susceptibility', 'biomarker', 'modifier'
    score           FLOAT,                      -- DisGeNET GDA score 0-1
    source_db       TEXT NOT NULL,              -- 'hpo', 'clinvar', 'omim', 'disgenet'
    evidence_code   TEXT,                       -- 'IEA', 'PCS', 'TAS'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_gda_entity     ON gene_disease_associations(entity_id, entity_table);
CREATE INDEX idx_gda_disease    ON gene_disease_associations(disease_id);
CREATE INDEX idx_gda_source     ON gene_disease_associations(source_db);

-- EDGE: disease has_phenotype phenotype (HPO annotations)
CREATE TABLE disease_phenotype_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    disease_id      UUID NOT NULL REFERENCES disease_terms(id) ON DELETE CASCADE,
    phenotype_id    UUID NOT NULL REFERENCES phenotype_terms(id) ON DELETE CASCADE,
    frequency_hpo   TEXT,           -- 'HP:0040280' (Obligate) → 'HP:0040285' (Excluded)
    onset_modifier  TEXT,           -- HPO onset term
    source_db       TEXT NOT NULL DEFAULT 'hpo',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(disease_id, phenotype_id, source_db)
);
CREATE INDEX idx_dpa_disease    ON disease_phenotype_associations(disease_id);
CREATE INDEX idx_dpa_phenotype  ON disease_phenotype_associations(phenotype_id);

-- EDGE: protein has_structure structure (PDB / AlphaFold)
CREATE TABLE protein_structure_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id      UUID NOT NULL REFERENCES protein_metadata(id) ON DELETE CASCADE,
    structure_id    UUID NOT NULL REFERENCES structure_entries(id) ON DELETE CASCADE,
    chain_id        TEXT,           -- PDB chain (A, B, ...)
    coverage_start  INTEGER,        -- residue range covered
    coverage_end    INTEGER,
    source_db       TEXT NOT NULL,  -- 'pdb', 'alphafold'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_psa_protein    ON protein_structure_associations(protein_id);
CREATE INDEX idx_psa_structure  ON protein_structure_associations(structure_id);
```

**Biolink Model edge type vocabulary:**
Association table names map to Biolink Model predicates. This aligns BDP with the NCATS
Translator ecosystem and is compatible with RTX-KG2 / SPOKE query patterns.

| Table | Biolink Predicate |
|-------|------------------|
| `protein_pathway_associations` | `biolink:participates_in` |
| `protein_interactions` | `biolink:interacts_with` |
| `gene_disease_associations` | `biolink:associated_with` |
| `disease_phenotype_associations` | `biolink:has_phenotype` |
| `protein_structure_associations` | `biolink:has_3D_structure` |
| `go_annotations` (existing) | `biolink:enables` / `biolink:involved_in` |

Apache AGE maps these tables to Cypher edge labels at graph sync time:
```sql
-- graph_sync creates AGE edges from protein_pathway_associations:
SELECT * FROM cypher('bdp', $$
    MATCH (p:protein {node_id: $protein_node_id})
    MATCH (pw:pathway {node_id: $pathway_node_id})
    CREATE (p)-[:participates_in {source_db: $source_db, evidence_type: $evidence_type}]->(pw)
$$) AS (r agtype);
```

The Cypher API then enables multi-hop traversal across all typed edge tables:
```cypher
MATCH path = (p:protein {uniprot_acc: 'P04637'})-[:participates_in]->(pw:pathway)
             -[:has_phenotype_association]-(d:disease)
RETURN p.label AS protein, pw.label AS pathway, d.label AS disease
LIMIT 50
```

### 8b. graph_layout table — WebGPU positions only

```sql
-- Rendering metadata only — no domain data here
CREATE TABLE graph_layout (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    x               FLOAT NOT NULL DEFAULT 0.0,
    y               FLOAT NOT NULL DEFAULT 0.0,
    community_id    INTEGER,        -- Louvain community assignment
    size            FLOAT NOT NULL DEFAULT 1.0,  -- visual weight (degree-based)
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_graph_layout_community ON graph_layout(community_id);
-- No spatial index needed — positions are computed + sent to GPU as buffer arrays
```

This replaces the `position GEOMETRY(POINT)` and `community_id`/`size` columns that were
on `graph_nodes`. Entity data and rendering data are now fully separated:
- `graph_nodes` — who the entity is
- `graph_layout` — where it lives in the visualization

### 9. Graph node attribute tables — denormalized for AGE

`graph_nodes` holds position + traversal metadata only. Entity-specific attributes
live in typed 1:1 tables:

```sql
-- graph_nodes: entity index for AGE traversal + alias resolution
-- NO positions (→ graph_layout), NO JSONB properties (→ gn_*_attrs)
CREATE TABLE graph_nodes (
    id              BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    entity_type_id  SMALLINT NOT NULL REFERENCES graph_entity_types(id),
    external_id     TEXT NOT NULL,
    source_db       TEXT NOT NULL,
    label           TEXT NOT NULL,
    degree          INTEGER NOT NULL DEFAULT 0,  -- updated by graph_sync
    UNIQUE (source_db, external_id)
);

-- Entity-type attribute tables (1:1 with graph_nodes)
CREATE TABLE gn_protein_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    uniprot_acc     TEXT NOT NULL,
    gene_name       TEXT,
    organism_taxid  BIGINT,
    length_aa       INTEGER,
    is_reviewed     BOOLEAN,            -- Swiss-Prot=true, TrEMBL=false
    mass_da         BIGINT
);
CREATE INDEX idx_gn_protein_reviewed ON gn_protein_attrs(is_reviewed);
CREATE INDEX idx_gn_protein_organism ON gn_protein_attrs(organism_taxid);

CREATE TABLE gn_go_term_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    go_id           TEXT NOT NULL,
    namespace       TEXT NOT NULL,
    is_obsolete     BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_gn_go_namespace    ON gn_go_term_attrs(namespace);

CREATE TABLE gn_taxon_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    ncbi_taxid      BIGINT NOT NULL,
    rank            TEXT,               -- 'species', 'genus', 'family', 'phylum'
    division        TEXT                -- 'Vertebrates', 'Bacteria', 'Plants'
);
CREATE INDEX idx_gn_taxon_rank      ON gn_taxon_attrs(rank);

CREATE TABLE gn_pathway_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    reactome_id     TEXT NOT NULL,
    species_taxid   BIGINT,
    is_top_level    BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_gn_pathway_top     ON gn_pathway_attrs(is_top_level) WHERE is_top_level;
CREATE INDEX idx_gn_pathway_species ON gn_pathway_attrs(species_taxid);

CREATE TABLE gn_disease_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    mondo_id        TEXT NOT NULL,
    omim_id         TEXT,               -- denormalized from ont_term_xrefs for fast lookup
    orphanet_id     TEXT,
    icd10_code      TEXT
);
CREATE INDEX idx_gn_disease_omim    ON gn_disease_attrs(omim_id) WHERE omim_id IS NOT NULL;

CREATE TABLE gn_phenotype_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    hpo_id          TEXT NOT NULL,
    category        TEXT                -- 'Abnormality of the nervous system', etc.
);

CREATE TABLE gn_compound_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    chebi_id        TEXT NOT NULL,
    inchikey        TEXT,
    smiles          TEXT,
    formula         TEXT,
    mass_mono       FLOAT
);
CREATE INDEX idx_gn_compound_inchi  ON gn_compound_attrs(inchikey) WHERE inchikey IS NOT NULL;

CREATE TABLE gn_variant_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    rsid            TEXT,               -- 'rs28897696'
    clinvar_id      TEXT,
    significance    TEXT,               -- 'Pathogenic', 'Benign', 'VUS'
    consequence     TEXT                -- 'missense', 'nonsense', 'splice'
);
CREATE INDEX idx_gn_variant_sig     ON gn_variant_attrs(significance);

CREATE TABLE gn_structure_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    pdb_id          TEXT,
    method          TEXT,               -- 'X-RAY DIFFRACTION', 'ELECTRON MICROSCOPY', 'NMR'
    resolution_ang  FLOAT,
    is_predicted    BOOLEAN NOT NULL DEFAULT FALSE   -- false=PDB, true=AlphaFold
);

CREATE TABLE gn_drug_attrs (
    node_id         BIGINT PRIMARY KEY REFERENCES graph_nodes(id) ON DELETE CASCADE,
    chembl_id       TEXT,
    drugbank_id     TEXT,
    max_phase       SMALLINT,           -- clinical trial phase (0-4)
    mol_formula     TEXT
);
CREATE INDEX idx_gn_drug_phase      ON gn_drug_attrs(max_phase);
```

### 11. Entity alias resolution

Research agents refer to entities by many names. This table is the global resolution layer:

```sql
CREATE TABLE entity_aliases (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- What this resolves TO (canonical BDP entity)
    canonical_db    TEXT NOT NULL,      -- 'uniprot', 'gene_ontology', 'mondo'
    canonical_id    TEXT NOT NULL,      -- 'P04637', 'GO:0006955', 'MONDO:0007254'
    -- What this is known AS (alternative identifier)
    alias_db        TEXT NOT NULL,      -- 'hgnc', 'entrez_gene', 'ensembl', 'symbol'
    alias_id        TEXT NOT NULL,      -- 'HGNC:11998', '7157', 'ENSG00000141510', 'TP53'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(alias_db, alias_id)          -- each alias resolves to exactly one canonical
);

CREATE INDEX idx_aliases_canonical  ON entity_aliases(canonical_db, canonical_id);
CREATE INDEX idx_aliases_alias      ON entity_aliases(alias_db, alias_id);
-- Trigram index for fuzzy gene symbol search
CREATE INDEX idx_aliases_symbol_trgm ON entity_aliases USING GIN (alias_id gin_trgm_ops)
    WHERE alias_db = 'symbol';
```

Agent can ask: "resolve TP53" → `alias_id='TP53', alias_db='symbol'` → `canonical_id='P04637'`
→ all graph_nodes, go_annotations, disease_terms connected to that protein.

### 12. Agent query provenance

```sql
-- Every agent query gets a version stamp — core to BDP's reproducibility mission
CREATE TABLE agent_query_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Who asked
    agent_id        TEXT,               -- MCP client identifier
    tool_name       TEXT NOT NULL,      -- 'graph_query', 'search_sources', 'entity_lookup'
    -- What was asked (JSONB is fine: diagnostic, not queried)
    query_params    JSONB NOT NULL,
    -- What data was in scope (provenance)
    dataset_versions JSONB NOT NULL,    -- {"uniprot": "2026_01", "reactome": "114", "mondo": "2026-03"}
    -- Result metadata
    result_count    INTEGER,
    duration_ms     INTEGER,
    executed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_agent_log_agent    ON agent_query_log(agent_id, executed_at);
CREATE INDEX idx_agent_log_tool     ON agent_query_log(tool_name, executed_at);
```

### 13. Big data table partitioning (phase in as data warrants)

There is **no generic `graph_edges` table**. Edges live in typed association tables (see §8a).
Partitioning applies to the two high-volume tables:

```sql
-- Partition protein_metadata when TrEMBL ingestion begins (250M rows)
CREATE TABLE protein_metadata (
    -- ... same columns ...
    is_reviewed BOOLEAN NOT NULL DEFAULT FALSE
) PARTITION BY LIST (is_reviewed);

CREATE TABLE protein_metadata_swissprot
    PARTITION OF protein_metadata FOR VALUES IN (true);   -- ~570K rows
CREATE TABLE protein_metadata_trembl
    PARTITION OF protein_metadata FOR VALUES IN (false);  -- ~250M rows

-- Partition protein_interactions when STRING is fully loaded (2B+ rows)
-- Partition by human/mouse/other to support per-species queries efficiently
CREATE TABLE protein_interactions (
    -- ... same columns ...
    species_group TEXT NOT NULL  -- 'human', 'mouse', 'other'
) PARTITION BY LIST (species_group);

CREATE TABLE protein_interactions_human PARTITION OF protein_interactions
    FOR VALUES IN ('human');     -- ~80M edges (all human-human interactions)
CREATE TABLE protein_interactions_mouse PARTITION OF protein_interactions
    FOR VALUES IN ('mouse');
CREATE TABLE protein_interactions_other PARTITION OF protein_interactions
    FOR VALUES IN ('other');
-- Each partition has its own combined_score index for threshold filtering
```

---

## Apache AGE Integration

### Installation

```sql
-- Migration: 20260325000001_enable_age.sql
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

-- Create BDP graph
SELECT create_graph('bdp');
```

### Graph sync projection

The `graph_sync` job creates AGE vertices and edges mirroring the relational tables:

```sql
-- Each entity type maps to an AGE vertex label
SELECT * FROM cypher('bdp', $$
    CREATE (p:protein {
        node_id: $node_id,
        external_id: $external_id,
        label: $label,
        uniprot_acc: $uniprot_acc,
        is_reviewed: $is_reviewed
    })
$$) AS (v agtype);
```

AGE stores these in its internal format, but queries can join back to relational tables
via `node_id`. Agents get Cypher expressiveness; analysts get SQL precision — both
operating on the same data.

### Cypher API endpoint

```
POST /api/v1/graph/cypher
Content-Type: application/json
Authorization: Bearer <token>

{
  "query": "MATCH (p:protein)-[:participates_in]->(pw:pathway) WHERE pw.reactome_id = 'R-HSA-9612973' RETURN p.label, p.uniprot_acc LIMIT 100",
  "params": {}
}
```

Parameterized queries only. No DDL (CREATE/DROP/SET) permitted. Read-only enforced
at the query analysis layer before execution.

---

## Common OBO Parser

All four OBO-based pipelines (GO, MONDO, HPO, ChEBI) share a single parser in
`bdp-ingest/src/common/obo.rs`. Each pipeline provides a thin adapter.

```rust
// bdp-ingest/src/common/obo.rs

pub struct RawOboTerm {
    pub id: String,                            // "GO:0008150", "MONDO:0004992"
    pub name: String,
    pub namespace: Option<String>,             // raw string, pipeline interprets
    pub definition: Option<String>,
    pub is_obsolete: bool,
    pub synonyms: Vec<RawOboSynonym>,
    pub xrefs: Vec<String>,                    // raw: "CHEBI:33709", "OMIM:123456"
    pub alt_ids: Vec<String>,
    pub comments: Option<String>,
    pub is_a: Vec<String>,                     // parent IDs, raw
    pub relationships: Vec<RawOboRelationship>,
    pub property_values: Vec<(String, String)>, // for ChEBI InChI, SMILES
}

pub struct RawOboSynonym { pub scope: String, pub text: String, pub synonym_type: Option<String> }
pub struct RawOboRelationship { pub rel_type: String, pub target: String }

pub struct OboParser;
impl OboParser {
    pub fn parse(content: &str) -> Result<Vec<RawOboTerm>, OboParseError> { ... }
    // Splits "CHEBI:33709" → ("CHEBI", "33709") for xref storage
    pub fn split_xref(xref: &str) -> (String, String) { ... }
}
```

GO pipeline adapter: `RawOboTerm → (GoTerm, Vec<GoRelationship>)` — thin, ~50 lines.
MONDO adapter: `RawOboTerm → (DiseaseTerm, Vec<DiseaseRelationship>)` — identical shape.
HPO adapter: `RawOboTerm → (PhenotypeTerm, Vec<PhenotypeRelationship>)` — identical shape.
ChEBI adapter: `RawOboTerm → (CompoundTerm, Vec<CompoundRelationship>)` — extracts InChI, SMILES from `property_values`.

---

## MCP Server — Elevated to P1

The MCP server (`bdp-mcp`) is the primary interface for AI research agents. Without it,
agents cannot query BDP programmatically. Elevated from P2 to P1 alongside the
ontology pipelines.

**Tools exposed:**

| Tool | Description | Backed by |
|------|-------------|-----------|
| `search_entities` | Semantic + full-text entity search | pgvector + tsvector |
| `graph_query` | Read-only Cypher query | Apache AGE |
| `resolve_entity` | Alias → canonical ID | `entity_aliases` |
| `get_entity` | Full entity details by canonical ID | domain tables |
| `list_versions` | What dataset versions are available | `versions` table |
| `get_provenance` | Version stamps for last query | `agent_query_log` |

---

## Migration Sequence

Migrations must be applied in this order. Each is additive / backward-compatible.

```
Phase 0 (foundation — do first, unblocks everything):
  20260325000001_enable_age.sql
  20260325000002_source_types_table.sql          -- kill CHECK constraint
  20260325000003_ont_term_xrefs.sql              -- unified xref table
  20260325000004_ont_term_synonyms.sql           -- unified synonym table
  20260325000005_entity_aliases.sql
  20260325000006_agent_query_log.sql
  20260325000007_ingest_failed_records.sql

Phase 1 (GO cleanup):
  20260325000010_go_term_alt_ids.sql             -- migrate alt_ids JSONB
  20260325000011_go_annotation_extensions.sql    -- migrate annotation_extension JSONB
  20260325000012_go_term_remove_jsonb.sql        -- DROP synonyms/xrefs/alt_ids columns

Phase 2 (protein cleanup):
  20260325000020_protein_xrefs_remove_jsonb.sql  -- remove metadata JSONB
  20260325000021_sequence_features_hstore.sql    -- replace features JSONB with hstore

Phase 3 (graph tables — no generic graph_edges):
  20260325000030_graph_registry_tables.sql       -- entity/edge type registries
  20260325000031_graph_nodes.sql                 -- graph_nodes (lean entity index, no JSONB, no positions)
  20260325000032_graph_layout.sql                -- graph_layout (WebGPU positions only)
  20260325000033_graph_node_attr_tables.sql      -- gn_protein_attrs, gn_go_term_attrs, etc.
  20260325000034_graph_seed_data.sql             -- 17 entity types + 14 edge type labels (for AGE)

Phase 4 (new pipeline domain + association tables):
  20260326000001_reactome_tables.sql             -- pathway_terms, pathway_hierarchy, protein_pathway_associations
  20260326000002_mondo_tables.sql                -- disease_terms, disease_relationships
  20260326000003_hpo_tables.sql                  -- phenotype_terms, gene_disease_associations, disease_phenotype_associations
  20260326000004_chebi_tables.sql                -- compound_terms, compound_relationships
  20260326000005_string_db_tables.sql            -- protein_interactions (typed edge, no generic graph_edges)
  20260326000006_ensembl_tables.sql              -- gene_metadata, transcript_metadata

Phase 5 (partitioning — when data warrants):
  20260401000001_partition_protein_metadata.sql       -- when TrEMBL ingestion begins
  20260401000002_partition_protein_interactions.sql   -- when STRING fully loaded (2B+ rows)
```

---

## JSONB Policy Going Forward

**Acceptable JSONB uses (diagnostic / append-only context):**
- `audit_log.changes`, `audit_log.metadata` — audit records are schema-free by nature
- `ingestion_jobs.metadata`, `ingestion_jobs.checkpoint_data` — job-specific diagnostics
- `agent_query_log.query_params`, `agent_query_log.dataset_versions` — provenance context
- `versions.dependency_cache` — performance cache, not primary data

**Prohibited JSONB uses (queryable domain data):**
- Any array of strings that could be searched (use a normalized table)
- Any structured object with known fields (use typed columns)
- Cross-references and synonyms (use `ont_term_xrefs`, `ont_term_synonyms`)
- Entity attributes on graph nodes (use `gn_*_attrs` tables)

**Rule of thumb:** if an agent would ever want to filter or search on it, it must be relational.

---

## Resolved Design Decisions

These were open questions — now closed with evidence.

1. **Generic graph_edges vs. typed association tables** → **Typed tables win.**
   All production bioinformatics KGs (Hetionet 24 types, SPOKE 55 types, PharMeBINet 208 types,
   RTX-KG2 Biolink) use typed association tables. Zero use generic edge tables. Decision: eliminate
   `graph_edges`, use `protein_interactions`, `gene_disease_associations`, etc.

2. **Per-pipeline data source chain** → **Yes, always.**
   Every pipeline registers its own `registry_entries → data_sources → versions → domain_tables`.
   This is the BDP registry model. Never share tables across pipelines for primary data.

3. **graph_nodes as entity index** → **Yes, but lean.**
   `graph_nodes` exists for AGE vertex creation, alias resolution, and degree tracking only.
   Attributes live in domain tables + `gn_*_attrs`. Positions live in `graph_layout`.

## Open Questions

1. **Apache AGE vs. FalkorDB** — FalkorDB (successor to RedisGraph) shows 10-500x faster
   graph traversal benchmarks than Neo4j; AGE trades performance for PostgreSQL integration.
   Decision for now: proceed with AGE (same connection pool, same ACID, no extra process).
   Re-evaluate if Cypher query latency is unacceptable after STRING is ingested.

2. **Apache AGE vs. pg_graphql** — AGE for traversal, pg_graphql for schema introspection?
   Both can coexist. Evaluate after STRING is ingested and agent query patterns are known.

3. **TrEMBL ingestion timing** — 250M proteins requires partition migration first.
   Decision: ingest Swiss-Prot (570K) first, apply partition migration, then TrEMBL.

4. **STRING species scope** — All 11M proteins or human/mouse/yeast first?
   Recommendation: human (9606) + mouse (10090) + yeast (559292) first (~30M edges),
   then expand. Validates pipeline before committing to full 2B edge ingest.

5. **`bdp-mcp` auth model** — API key or OAuth? For research labs, API key with
   per-key rate limiting and audit logging is sufficient for phase 1.

---

**Last Updated:** 2026-03-25
**Related specs:**
- `docs/superpowers/specs/2026-03-21-vectors-embedding-design.md`
- `docs/superpowers/specs/2026-03-22-graph-view-webgpu-design.md`
**Related plans:**
- `docs/superpowers/plans/2026-03-22-graph-view-webgpu.md`
