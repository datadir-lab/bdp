# Foundation Migrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply Phase 0-2 DB migrations that unblock all new pipelines — source_types FK table, unified xref/synonym tables, entity aliases, agent query log, dead letter queue, GO JSONB cleanup, and protein xref JSONB cleanup.

**Architecture:** All migrations are additive first (add new tables/columns), then data-migrating (move JSONB to relational rows), then destructive (drop old columns). Rust code is updated to match schema changes. The `source_type` CHECK constraint is replaced with a FK to `source_types` table — adding a new pipeline thereafter requires only an INSERT, no DDL.

**Tech Stack:** PostgreSQL 16, SQLx, Rust (axum/CQRS), cargo xtask

---

## File Map

**New migration files** (all in `migrations/`):
- `20260325000002_source_types_table.sql` — create+seed source_types, migrate data_sources FK
- `20260325000003_ont_term_xrefs.sql` — unified cross-reference table
- `20260325000004_ont_term_synonyms.sql` — unified synonym table
- `20260325000005_entity_aliases.sql` — alias resolution table
- `20260325000006_agent_query_log.sql` — MCP query provenance
- `20260325000007_ingest_failed_records.sql` — dead letter queue
- `20260325000010_go_term_alt_ids.sql` — migrate alt_ids JSONB
- `20260325000011_go_annotation_extensions.sql` — migrate annotation_extension JSONB
- `20260325000012_go_term_remove_jsonb.sql` — DROP synonyms/xrefs/alt_ids JSONB columns
- `20260325000013_go_annotations_remove_jsonb.sql` — DROP annotation_extension JSONB
- `20260325000020_protein_xrefs_columns.sql` — add isoform/chain/additional columns
- `20260325000021_protein_xrefs_remove_jsonb.sql` — migrate then DROP metadata JSONB

**Modified Rust files:**
- `crates/bdp-server/src/features/shared/validation.rs` — remove VALID_SOURCE_TYPES constant + validate_source_type fn (replaced by DB FK)
- `crates/bdp-server/src/features/data_sources/commands/create.rs` — remove validate_source_type call
- `crates/bdp-server/src/ingest/gene_ontology/storage.rs` — write synonyms/xrefs/alt_ids to relational tables instead of JSONB
- `crates/bdp-server/src/ingest/uniprot/storage.rs` (or storage_adapter.rs) — update protein xref insert

---

## Task 1: source_types FK table (kills the CHECK constraint)

**Files:**
- Create: `migrations/20260325000002_source_types_table.sql`
- Modify: `crates/bdp-server/src/features/shared/validation.rs`
- Modify: `crates/bdp-server/src/features/data_sources/commands/create.rs`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260325000002_source_types_table.sql

-- 1. Create lookup table
CREATE TABLE source_types (
    name        TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Seed all current + future types
INSERT INTO source_types (name, label, description) VALUES
    ('protein',          'Protein',          'UniProt protein sequences and annotations'),
    ('taxonomy',         'Taxon',            'NCBI taxonomy nodes'),
    ('organism',         'Organism',         'Organism entries'),
    ('genomic_sequence', 'Genomic Sequence', 'GenBank/RefSeq nucleotide sequences'),
    ('genome',           'Genome',           'Assembled genome entries'),
    ('go_term',          'GO Term',          'Gene Ontology terms'),
    ('interpro_entry',   'InterPro Entry',   'InterPro protein family/domain entries'),
    ('pathway',          'Pathway',          'Biological pathways (Reactome, KEGG)'),
    ('disease',          'Disease',          'Disease terms (MONDO)'),
    ('phenotype',        'Phenotype',        'Phenotype terms (HPO)'),
    ('compound',         'Compound',         'Chemical compounds (ChEBI, PubChem)'),
    ('drug',             'Drug',             'Drug entities (ChEMBL, DrugBank)'),
    ('variant',          'Variant',          'Genomic variants (ClinVar, dbSNP)'),
    ('structure',        'Structure',        'Protein structures (PDB, AlphaFold)'),
    ('gene',             'Gene',             'Gene entries (Ensembl)'),
    ('transcript',       'Transcript',       'Transcript entries (Ensembl)'),
    ('annotation',       'Annotation',       'Annotation entries'),
    ('bundle',           'Bundle',           'Aggregate data source bundle'),
    ('other',            'Other',            'Uncategorized source type');

-- 3. Add FK column alongside existing TEXT column
ALTER TABLE data_sources ADD COLUMN source_type_fk TEXT REFERENCES source_types(name);

-- 4. Populate FK from existing TEXT column (all existing values are in the seed above)
UPDATE data_sources SET source_type_fk = source_type;

-- 5. Make FK NOT NULL now that data is migrated
ALTER TABLE data_sources ALTER COLUMN source_type_fk SET NOT NULL;

-- 6. Drop the CHECK-constrained column and rename FK column
ALTER TABLE data_sources DROP COLUMN source_type;
ALTER TABLE data_sources RENAME COLUMN source_type_fk TO source_type;
```

- [ ] **Step 2: Apply migration against dev database**

```bash
cargo xtask db migrate
```
Expected: migration applies without error.

- [ ] **Step 3: Remove validate_source_type from Rust**

In `crates/bdp-server/src/features/shared/validation.rs`, delete the `VALID_SOURCE_TYPES` constant and `validate_source_type` function entirely. The DB FK now enforces validity — invalid source types produce a FK violation error from PostgreSQL.

Also remove the imports and test cases for `validate_source_type` from the same file.

- [ ] **Step 4: Remove validation call from create command**

In `crates/bdp-server/src/features/data_sources/commands/create.rs`:
- Remove the `validate_source_type` import
- Remove the call to `validate_source_type(&self.source_type)?` in `validate()`
- Keep other validations (slug, name) unchanged

- [ ] **Step 5: Verify it compiles**

```bash
SQLX_OFFLINE=true cargo check -p bdp-server 2>&1 | grep "^error" | head -20
```
Expected: zero `error:` lines (warnings about unused imports are OK, fix those).

- [ ] **Step 6: Run data_sources tests**

```bash
cargo test -p bdp-server --test query_tests 2>&1 | tail -20
```
Expected: all pass. If `validate_source_type` test is gone — that's correct.

- [ ] **Step 7: Commit**

```bash
git add migrations/20260325000002_source_types_table.sql \
        crates/bdp-server/src/features/shared/validation.rs \
        crates/bdp-server/src/features/data_sources/commands/create.rs
git commit -m "feat(db): replace source_type CHECK constraint with source_types FK table"
```

---

## Task 2: Unified cross-reference table

**Files:**
- Create: `migrations/20260325000003_ont_term_xrefs.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260325000003_ont_term_xrefs.sql

CREATE TABLE ont_term_xrefs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id     UUID NOT NULL,
    term_table  TEXT NOT NULL,
    source_db   TEXT NOT NULL,
    source_id   TEXT NOT NULL,
    xref_type   TEXT,           -- 'exact', 'related', 'broader', 'narrower'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ont_xrefs_term    ON ont_term_xrefs(term_id, term_table);
CREATE INDEX idx_ont_xrefs_source  ON ont_term_xrefs(source_db, source_id);
CREATE INDEX idx_ont_xrefs_reverse ON ont_term_xrefs(source_db, source_id, term_table);
```

- [ ] **Step 2: Apply and verify**

```bash
cargo xtask db migrate
# Verify table exists:
# psql $DATABASE_URL -c "\d ont_term_xrefs"
```

- [ ] **Step 3: Commit**

```bash
git add migrations/20260325000003_ont_term_xrefs.sql
git commit -m "feat(db): add ont_term_xrefs unified cross-reference table"
```

---

## Task 3: Unified synonym table

**Files:**
- Create: `migrations/20260325000004_ont_term_synonyms.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260325000004_ont_term_synonyms.sql

CREATE TABLE ont_term_synonyms (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id      UUID NOT NULL,
    term_table   TEXT NOT NULL,
    scope        TEXT NOT NULL CHECK (scope IN ('EXACT','BROAD','NARROW','RELATED')),
    text         TEXT NOT NULL,
    synonym_type TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ont_synonyms_term ON ont_term_synonyms(term_id, term_table);
CREATE INDEX idx_ont_synonyms_text ON ont_term_synonyms
    USING GIN (to_tsvector('english', text));
CREATE UNIQUE INDEX idx_ont_synonyms_dedup ON ont_term_synonyms(term_id, term_table, scope, text);
```

- [ ] **Step 2: Apply and commit**

```bash
cargo xtask db migrate
git add migrations/20260325000004_ont_term_synonyms.sql
git commit -m "feat(db): add ont_term_synonyms unified synonym table"
```

---

## Task 4: Entity aliases table

**Files:**
- Create: `migrations/20260325000005_entity_aliases.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260325000005_entity_aliases.sql

CREATE EXTENSION IF NOT EXISTS pg_trgm;  -- for trigram index below

CREATE TABLE entity_aliases (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canonical_db TEXT NOT NULL,     -- 'uniprot', 'gene_ontology', 'mondo'
    canonical_id TEXT NOT NULL,     -- 'P04637', 'GO:0006955', 'MONDO:0007254'
    alias_db     TEXT NOT NULL,     -- 'hgnc', 'entrez_gene', 'ensembl', 'symbol'
    alias_id     TEXT NOT NULL,     -- 'HGNC:11998', '7157', 'ENSG00000141510', 'TP53'
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(alias_db, alias_id)
);

CREATE INDEX idx_aliases_canonical ON entity_aliases(canonical_db, canonical_id);
CREATE INDEX idx_aliases_alias     ON entity_aliases(alias_db, alias_id);
-- Trigram index for fuzzy symbol search ("TP5" → "TP53")
CREATE INDEX idx_aliases_symbol_trgm ON entity_aliases
    USING GIN (alias_id gin_trgm_ops)
    WHERE alias_db = 'symbol';
```

- [ ] **Step 2: Apply and commit**

```bash
cargo xtask db migrate
git add migrations/20260325000005_entity_aliases.sql
git commit -m "feat(db): add entity_aliases table for agent entity resolution"
```

---

## Task 5: Agent query log + dead letter queue

**Files:**
- Create: `migrations/20260325000006_agent_query_log.sql`
- Create: `migrations/20260325000007_ingest_failed_records.sql`

- [ ] **Step 1: Write agent_query_log migration**

```sql
-- migrations/20260325000006_agent_query_log.sql

CREATE TABLE agent_query_log (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id         TEXT,
    tool_name        TEXT NOT NULL,
    query_params     JSONB NOT NULL,
    dataset_versions JSONB NOT NULL,
    result_count     INTEGER,
    duration_ms      INTEGER,
    executed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_log_agent ON agent_query_log(agent_id, executed_at);
CREATE INDEX idx_agent_log_tool  ON agent_query_log(tool_name, executed_at);
```

- [ ] **Step 2: Write ingest_failed_records migration**

```sql
-- migrations/20260325000007_ingest_failed_records.sql

CREATE TABLE ingest_failed_records (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline      TEXT NOT NULL,
    batch_id      UUID,
    raw_data      BYTEA NOT NULL,
    error_msg     TEXT NOT NULL,
    attempt_count SMALLINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ingest_failed_pipeline ON ingest_failed_records(pipeline, created_at);
```

- [ ] **Step 3: Apply and commit**

```bash
cargo xtask db migrate
git add migrations/20260325000006_agent_query_log.sql \
        migrations/20260325000007_ingest_failed_records.sql
git commit -m "feat(db): add agent_query_log and ingest_failed_records tables"
```

---

## Task 6: GO — alt_ids relational table (replaces JSONB)

**Files:**
- Create: `migrations/20260325000010_go_term_alt_ids.sql`

- [ ] **Step 1: Write migration with data migration**

```sql
-- migrations/20260325000010_go_term_alt_ids.sql

CREATE TABLE go_term_alt_ids (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    go_term_id UUID NOT NULL REFERENCES go_term_metadata(id) ON DELETE CASCADE,
    alt_go_id  TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(go_term_id, alt_go_id)
);

CREATE INDEX idx_go_alt_ids_term ON go_term_alt_ids(go_term_id);
CREATE INDEX idx_go_alt_ids_alt  ON go_term_alt_ids(alt_go_id);

-- Data migration: move existing alt_ids JSONB → relational rows
-- alt_ids JSONB is an array of text: ["GO:0006955", "GO:1234567"]
INSERT INTO go_term_alt_ids (go_term_id, alt_go_id)
SELECT
    g.id,
    alt_id.value::TEXT
FROM go_term_metadata g
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(g.alt_ids, '[]'::jsonb)
) AS alt_id(value)
WHERE g.alt_ids IS NOT NULL
  AND jsonb_array_length(g.alt_ids) > 0
ON CONFLICT (go_term_id, alt_go_id) DO NOTHING;
```

- [ ] **Step 2: Apply migration**

```bash
cargo xtask db migrate
```

- [ ] **Step 3: Verify row count matches (sanity check)**

Run against dev DB:
```sql
-- Should be > 0 if GO data is loaded
SELECT COUNT(*) FROM go_term_alt_ids;
-- Should equal: SELECT SUM(jsonb_array_length(alt_ids)) FROM go_term_metadata WHERE alt_ids IS NOT NULL;
```

- [ ] **Step 4: Commit**

```bash
git add migrations/20260325000010_go_term_alt_ids.sql
git commit -m "feat(db): add go_term_alt_ids table, migrate alt_ids JSONB"
```

---

## Task 7: GO — annotation extensions relational table

**Files:**
- Create: `migrations/20260325000011_go_annotation_extensions.sql`

The annotation_extension JSONB in GAF format stores `relation(DB:ID)` tuples like
`occurs_in(CL:0000236)`. The JSONB in go_annotations stores these parsed into objects.

- [ ] **Step 1: Write migration with data migration**

```sql
-- migrations/20260325000011_go_annotation_extensions.sql

CREATE TABLE go_annotation_extensions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    annotation_id UUID NOT NULL REFERENCES go_annotations(id) ON DELETE CASCADE,
    relation      TEXT NOT NULL,   -- 'occurs_in', 'has_input', 'part_of', 'has_output'
    filler_db     TEXT NOT NULL,   -- 'CL', 'CHEBI', 'GO', 'UBERON'
    filler_id     TEXT NOT NULL,   -- '0000236', '33709', '0006955'
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_go_ann_ext_ann    ON go_annotation_extensions(annotation_id);
CREATE INDEX idx_go_ann_ext_filler ON go_annotation_extensions(filler_db, filler_id);

-- Data migration: annotation_extension JSONB is stored as array of objects:
-- [{"relation": "occurs_in", "filler_db": "CL", "filler_id": "0000236"}, ...]
-- If the format is different in your data, adjust the JSON path accordingly.
INSERT INTO go_annotation_extensions (annotation_id, relation, filler_db, filler_id)
SELECT
    a.id,
    ext->>'relation',
    ext->>'filler_db',
    ext->>'filler_id'
FROM go_annotations a
CROSS JOIN LATERAL jsonb_array_elements(
    COALESCE(a.annotation_extension, '[]'::jsonb)
) AS ext
WHERE a.annotation_extension IS NOT NULL
  AND jsonb_array_length(a.annotation_extension) > 0
  AND ext->>'relation' IS NOT NULL
ON CONFLICT DO NOTHING;
```

- [ ] **Step 2: Apply migration**

```bash
cargo xtask db migrate
```

- [ ] **Step 3: Commit**

```bash
git add migrations/20260325000011_go_annotation_extensions.sql
git commit -m "feat(db): add go_annotation_extensions table, migrate annotation_extension JSONB"
```

---

## Task 8: GO — migrate synonyms+xrefs to unified tables, then drop JSONB columns

**Files:**
- Create: `migrations/20260325000012_go_term_migrate_jsonb.sql`
- Create: `migrations/20260325000013_go_term_drop_jsonb_columns.sql`
- Modify: `crates/bdp-server/src/ingest/gene_ontology/storage.rs`

- [ ] **Step 1: Write data migration (synonyms → ont_term_synonyms, xrefs → ont_term_xrefs)**

```sql
-- migrations/20260325000012_go_term_migrate_jsonb.sql

-- Migrate GO synonyms → ont_term_synonyms
-- JSONB schema: [{"type": "EXACT", "text": "immune response"}, ...]
INSERT INTO ont_term_synonyms (term_id, term_table, scope, text)
SELECT
    g.id,
    'go_term_metadata',
    UPPER(syn->>'type'),    -- 'EXACT', 'BROAD', 'NARROW', 'RELATED'
    syn->>'text'
FROM go_term_metadata g
CROSS JOIN LATERAL jsonb_array_elements(
    COALESCE(g.synonyms, '[]'::jsonb)
) AS syn
WHERE g.synonyms IS NOT NULL
  AND jsonb_array_length(g.synonyms) > 0
  AND syn->>'text' IS NOT NULL
  AND UPPER(syn->>'type') IN ('EXACT', 'BROAD', 'NARROW', 'RELATED')
ON CONFLICT (term_id, term_table, scope, text) DO NOTHING;

-- Migrate GO xrefs → ont_term_xrefs
-- JSONB schema: ["Wikipedia:Immune_response", "KEGG:ko04620", "Reactome:R-HSA-1", ...]
-- Split "DB:ID" → source_db='Wikipedia', source_id='Immune_response'
INSERT INTO ont_term_xrefs (term_id, term_table, source_db, source_id)
SELECT
    g.id,
    'go_term_metadata',
    CASE
        WHEN xref::TEXT LIKE '%:%' THEN split_part(xref::TEXT, ':', 1)
        ELSE 'unknown'
    END,
    CASE
        WHEN xref::TEXT LIKE '%:%' THEN substring(xref::TEXT from position(':' in xref::TEXT) + 1)
        ELSE xref::TEXT
    END
FROM go_term_metadata g
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(g.xrefs, '[]'::jsonb)
) AS xref
WHERE g.xrefs IS NOT NULL
  AND jsonb_array_length(g.xrefs) > 0
ON CONFLICT DO NOTHING;
```

- [ ] **Step 2: Apply migration**

```bash
cargo xtask db migrate
```

- [ ] **Step 3: Update GO storage.rs to write to relational tables**

In `crates/bdp-server/src/ingest/gene_ontology/storage.rs`, update the GO term insert to write synonyms and xrefs to `ont_term_synonyms` / `ont_term_xrefs` / `go_term_alt_ids` instead of JSONB columns.

Remove the `synonyms`, `xrefs`, `alt_ids` from the `INSERT INTO go_term_metadata` query.

Add a separate chunked insert for each:

```rust
// After inserting go_term_metadata rows and getting back their UUIDs,
// batch insert synonyms:
if !synonyms_batch.is_empty() {
    let mut q = QueryBuilder::new(
        "INSERT INTO ont_term_synonyms (term_id, term_table, scope, text) "
    );
    q.push_values(&synonyms_batch, |mut b, (term_id, scope, text)| {
        b.push_bind(term_id)
         .push_bind("go_term_metadata")
         .push_bind(scope)
         .push_bind(text);
    });
    q.push(" ON CONFLICT (term_id, term_table, scope, text) DO NOTHING");
    q.build().execute(&mut *tx).await?;
}
// Similar for xrefs → ont_term_xrefs and alt_ids → go_term_alt_ids
```

- [ ] **Step 4: Compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-server 2>&1 | grep "^error" | head -20
```

- [ ] **Step 5: Write drop migration (separate — apply after verifying storage.rs works)**

```sql
-- migrations/20260325000013_go_term_drop_jsonb_columns.sql

-- Only run after confirming storage.rs no longer writes to these columns
ALTER TABLE go_term_metadata
    DROP COLUMN IF EXISTS synonyms,
    DROP COLUMN IF EXISTS xrefs,
    DROP COLUMN IF EXISTS alt_ids;

ALTER TABLE go_annotations
    DROP COLUMN IF EXISTS annotation_extension;
```

- [ ] **Step 6: Regenerate SQLx offline cache (if running with live DB)**

```bash
cargo xtask sqlx prepare
```

- [ ] **Step 7: Apply drop migration**

```bash
cargo xtask db migrate
```

- [ ] **Step 8: Final compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-server 2>&1 | grep "^error" | head -20
```

- [ ] **Step 9: Commit**

```bash
git add migrations/20260325000012_go_term_migrate_jsonb.sql \
        migrations/20260325000013_go_term_drop_jsonb_columns.sql \
        crates/bdp-server/src/ingest/gene_ontology/storage.rs
git commit -m "feat(db): migrate GO synonyms/xrefs/alt_ids from JSONB to relational tables"
```

---

## Task 9: Protein xrefs — remove metadata JSONB

**Files:**
- Create: `migrations/20260325000020_protein_xrefs_columns.sql`
- Create: `migrations/20260325000021_protein_xrefs_drop_jsonb.sql`
- Modify: `crates/bdp-server/src/ingest/uniprot/storage.rs` (or `storage_adapter.rs`)

- [ ] **Step 1: Add explicit columns and migrate data**

```sql
-- migrations/20260325000020_protein_xrefs_columns.sql

-- Add typed columns for the known metadata fields from UniProt xrefs
ALTER TABLE protein_cross_references
    ADD COLUMN isoform   TEXT,      -- isoform-specific xref (e.g., P04637-1)
    ADD COLUMN chain     TEXT,      -- PDB chain identifier (e.g., 'A')
    ADD COLUMN additional TEXT;     -- rare overflow text, not queried

-- Data migration: extract known fields from metadata JSONB
UPDATE protein_cross_references
SET
    isoform   = metadata->>'isoform',
    chain     = metadata->>'chain',
    additional = CASE
        WHEN (metadata - 'isoform' - 'chain') <> '{}'::jsonb
        THEN (metadata - 'isoform' - 'chain')::text
        ELSE NULL
    END
WHERE metadata IS NOT NULL;
```

- [ ] **Step 2: Apply migration**

```bash
cargo xtask db migrate
```

- [ ] **Step 3: Update uniprot storage.rs**

Find where `protein_cross_references` is inserted (in `storage.rs` or `storage_adapter.rs`). Replace the `metadata` JSONB bind with separate `isoform`, `chain`, `additional` binds.

The UniProt XML `<dbReference>` elements have child `<property>` elements. Map:
- `type="molecule ID"` → `isoform`
- `type="chains"` → `chain`
- Anything else → `additional`

- [ ] **Step 4: Compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-server 2>&1 | grep "^error" | head -20
```

- [ ] **Step 5: Drop JSONB column**

```sql
-- migrations/20260325000021_protein_xrefs_drop_jsonb.sql

ALTER TABLE protein_cross_references DROP COLUMN IF EXISTS metadata;
```

- [ ] **Step 6: Regenerate SQLx cache + apply**

```bash
cargo xtask sqlx prepare
cargo xtask db migrate
SQLX_OFFLINE=true cargo check -p bdp-server 2>&1 | grep "^error" | head -20
```

- [ ] **Step 7: Commit**

```bash
git add migrations/20260325000020_protein_xrefs_columns.sql \
        migrations/20260325000021_protein_xrefs_drop_jsonb.sql \
        crates/bdp-server/src/ingest/uniprot/storage.rs
git commit -m "feat(db): replace protein_cross_references metadata JSONB with typed columns"
```

---

## Task 10: Final verification

- [ ] **Step 1: Full workspace build**

```bash
SQLX_OFFLINE=true cargo build --workspace 2>&1 | grep "^error" | head -20
```

- [ ] **Step 2: Run all tests**

```bash
cargo xtask test all 2>&1 | tail -30
```

- [ ] **Step 3: Verify no JSONB in domain tables (sanity check query)**

```sql
-- Run against dev DB — expect 0 rows for target tables
SELECT table_name, column_name, data_type
FROM information_schema.columns
WHERE table_name IN ('go_term_metadata', 'go_annotations', 'protein_cross_references')
  AND data_type = 'jsonb';
```
Expected: only `go_term_metadata` columns `comments` (if kept) — no synonyms/xrefs/alt_ids/annotation_extension/metadata.

- [ ] **Step 4: Commit any remaining cleanup**

```bash
git add -A
git commit -m "chore: foundation migrations complete — JSONB eliminated from domain tables"
```

---

**Plan complete and saved to `docs/superpowers/plans/2026-03-25-foundation-migrations.md`.**
