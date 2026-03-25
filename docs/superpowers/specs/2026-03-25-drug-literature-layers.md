# Drug, Interaction & Literature Layer Design

**Date**: 2026-03-25
**Issues**: BDP-82 (Literature Layer), BDP-83 (Drug & Interaction Layer)
**Status**: Approved for implementation

---

## Goal

Implement five new ingestion pipelines to complete the BDP knowledge graph. Once live, AI agents can traverse the full biological chain autonomously via MCP:

```
gene → disease → phenotype → pathway → drug → clinical trial → literature
         ↑                      ↑         ↑
    Open Targets             STRING     ChEMBL
```

**Five pipelines**:
1. **Open Targets** — gene↔disease associations (replaces DisGeNET, which went paywalled in v7)
2. **ClinicalTrials.gov** — disease↔trial associations
3. **ChEMBL** — compound↔protein drug-target activities
4. **STRING** — protein↔protein interaction network (human only)
5. **PubMed/MEDLINE + PubTator3** — literature + entity annotations

---

## Why These Sources

| Source | License | Format | Notes |
|--------|---------|--------|-------|
| Open Targets 25.03 | Apache 2.0 | Parquet | MONDO IDs — zero MeSH bridging needed |
| ClinicalTrials.gov (AACT) | Public domain | Flat files / API | 2.23 GB initial dump, daily delta API |
| ChEMBL 36 | CC-BY-SA 4.0 | SQLite 5.2 GB | InChIKey bridge to compound_terms |
| STRING v12.0 | CC-BY 4.0 | TSV gz 130 MB | Human-only, scores 0–1000 |
| PubMed 2026 baseline | PMC/MEDLINE terms | 1 334 gz XML | 37 M records; entity annotations via PubTator3 |

**DisGeNET rejected**: v7+ requires paid subscription. Open Targets provides equivalent coverage with MONDO IDs directly and Apache 2.0 license.

---

## Pipeline 1 — Open Targets (gene↔disease)

### Source

- Base URL: `ftp.ebi.ac.uk/pub/databases/opentargets/platform/25.03/output/association_overall_direct/`
- Format: Parquet partitioned directory (Arrow schema)
- Key columns: `targetId` (Ensembl gene ID), `diseaseId` (MONDO/EFO), `score` (0.0–1.0 FLOAT4)
- Also download: `targets/` directory for gene symbol → Ensembl ID mapping

### Ensembl → UniProt bridge

Open Targets uses Ensembl gene IDs. BDP stores UniProt accessions. Bridge:
1. Download `targets/` Parquet (has `approvedSymbol`, `proteinIds` array with source=`uniprot_swissprot`)
2. Build in-memory `HashMap<ensembl_id, uniprot_acc>` before inserting associations
3. Only insert rows where UniProt accession exists in `data_sources` (inner join semantics)

### MONDO → internal ID bridge

BDP stores MONDO terms in `disease_terms`. Open Targets `diseaseId` is already `MONDO:XXXXXXX` format. Query `disease_terms.term_id` to get internal `disease_term_id`.

### Schema

```sql
CREATE TABLE gene_disease_associations (
    id               BIGSERIAL PRIMARY KEY,
    gene_id          INTEGER NOT NULL REFERENCES data_sources(id),   -- UniProt data_source
    disease_term_id  INTEGER NOT NULL REFERENCES disease_terms(id),
    association_type TEXT NOT NULL DEFAULT 'direct',
    score            FLOAT4,
    source           TEXT NOT NULL DEFAULT 'open_targets',
    source_version   TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(gene_id, disease_term_id, source)
);
CREATE INDEX ON gene_disease_associations(gene_id);
CREATE INDEX ON gene_disease_associations(disease_term_id);
CREATE INDEX ON gene_disease_associations(score DESC NULLS LAST);
```

### Crate structure

`crates/bdp-ingest/src/pipelines/open_targets/`
- `config.rs` — `OpenTargetsConfig { release: String, org_id: Uuid }`
- `downloader.rs` — HTTP + async download of Parquet files
- `parser.rs` — Arrow/Parquet reader, stream rows
- `runner.rs` — `OpenTargetsPipelineRunner::new(config, pool).run()` → `anyhow::Result<()>`

---

## Pipeline 2 — ClinicalTrials.gov

### Source — initial load

- AACT flat files: `aact.ctti-clinicaltrials.org/downloads` (PostgreSQL dump or CSV, ~2.23 GB compressed)
- Key tables: `studies`, `conditions`, `interventions`, `browse_conditions`, `browse_interventions`

### Source — incremental delta

- ClinicalTrials.gov API v2: `https://clinicaltrials.gov/api/v2/studies?query.term=AREA[LastUpdatePostDate]RANGE[2026-01-01,MAX]&pageSize=1000`
- Paginate with `pageToken` until no next page
- Run daily via BDP cron/pipeline orchestrator

### AACT → MONDO bridge

AACT `conditions` uses free-text MeSH terms. Bridge via existing `disease_term_xrefs`:
- MONDO pipeline already imports MeSH cross-refs into `disease_term_xrefs`
- `SELECT dt.id FROM disease_terms dt JOIN disease_term_xrefs x ON x.disease_term_id = dt.id WHERE x.source = 'MESH' AND x.external_id = :mesh_id`
- Unmapped conditions stored with `disease_term_id = NULL` (soft link)

### Schema

```sql
CREATE TABLE clinical_trials (
    id                BIGSERIAL PRIMARY KEY,
    nct_id            TEXT NOT NULL UNIQUE,
    title             TEXT,
    status            TEXT,                        -- recruiting, completed, etc.
    phase             TEXT,                        -- Phase 1/2/3/4, N/A
    start_date        DATE,
    completion_date   DATE,
    sponsor           TEXT,
    source_version    TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE trial_disease_links (
    id              BIGSERIAL PRIMARY KEY,
    trial_id        BIGINT NOT NULL REFERENCES clinical_trials(id),
    disease_term_id INTEGER REFERENCES disease_terms(id),   -- NULL if unmapped
    raw_condition   TEXT NOT NULL,
    UNIQUE(trial_id, disease_term_id)
);

CREATE TABLE trial_intervention_links (
    id           BIGSERIAL PRIMARY KEY,
    trial_id     BIGINT NOT NULL REFERENCES clinical_trials(id),
    compound_id  INTEGER REFERENCES data_sources(id),        -- NULL if unmapped
    raw_name     TEXT NOT NULL,
    UNIQUE(trial_id, compound_id)
);

CREATE INDEX ON clinical_trials(status);
CREATE INDEX ON trial_disease_links(disease_term_id);
CREATE INDEX ON trial_intervention_links(compound_id);
```

### Crate structure

`crates/bdp-ingest/src/pipelines/clinical_trials/`
- `config.rs` — `ClinicalTrialsConfig { aact_dump_path: Option<PathBuf>, from_date: Option<NaiveDate> }`
- `aact_loader.rs` — parse CSV flat files, bulk insert
- `api_fetcher.rs` — paginated API fetch for delta
- `runner.rs` — `ClinicalTrialsPipelineRunner::new(config, pool).run()` → `anyhow::Result<()>`

---

## Pipeline 3 — ChEMBL (drug targets)

### Source

- Full SQLite: `ftp.ebi.ac.uk/pub/databases/chembl/ChEMBLdb/releases/chembl_36/chembl_36_sqlite.tar.gz` (5.2 GB)
- Compound-only TSV: `chembl_36_chemreps.txt.gz` (274 MB) for InChIKey matching
- UniProt mapping: `chembl_uniprot_mapping.txt` — `CHEMBL_TARGET_ID → UniProt_AC`

### Bridge to BDP

- **Compound bridge**: `compound_structures.standard_inchi_key = compound_terms.inchikey` (ChEBI → ChEMBL)
- **Target bridge**: `chembl_uniprot_mapping.txt` → lookup in `data_sources` by UniProt accession

Only insert `drug_target_activities` rows where both compound and target resolve in BDP.

### Schema

```sql
CREATE TABLE drug_target_activities (
    id              BIGSERIAL PRIMARY KEY,
    compound_id     INTEGER NOT NULL REFERENCES data_sources(id),   -- ChEBI compound
    target_gene_id  INTEGER NOT NULL REFERENCES data_sources(id),   -- UniProt gene
    activity_type   TEXT,       -- IC50, Ki, Kd, EC50, etc.
    activity_value  FLOAT4,
    activity_unit   TEXT,       -- nM, µM, etc.
    relation        TEXT,       -- =, <, >, <=, >=
    assay_type      TEXT,       -- B (binding), F (functional), etc.
    chembl_assay_id TEXT,
    chembl_doc_id   TEXT,
    confidence      SMALLINT,   -- 0-9 ChEMBL confidence score
    source_version  TEXT NOT NULL DEFAULT 'chembl_36',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(compound_id, target_gene_id, chembl_assay_id)
);
CREATE INDEX ON drug_target_activities(compound_id);
CREATE INDEX ON drug_target_activities(target_gene_id);
CREATE INDEX ON drug_target_activities(activity_type, activity_value);
```

### Crate structure

`crates/bdp-ingest/src/pipelines/chembl/`
- `config.rs` — `ChemblConfig { sqlite_path: PathBuf, org_id: Uuid }`
- `extractor.rs` — rusqlite queries against ChEMBL SQLite
- `mapper.rs` — InChIKey + UniProt bridge resolution
- `runner.rs` — `ChemblPipelineRunner::new(config, pool).run()` → `anyhow::Result<()>`

**Dependency**: `rusqlite` (bundled feature). Process: download SQLite dump → extract → run ETL → optionally delete dump.

---

## Pipeline 4 — STRING (protein interactions)

### Source

- Human-only: `https://stringdb-downloads.org/download/protein.links.detailed.v12.0/9606.protein.links.detailed.v12.0.txt.gz` (129.7 MB)
- Alias mapping: `9606.protein.aliases.v12.0.txt.gz` — filter `source = BLAST_UniProt_AC` for UniProt mapping
- Columns: `protein1`, `protein2`, `neighborhood`, `fusion`, `cooccurence`, `coexpression`, `experimental`, `database`, `textmining`, `combined_score`

### Deduplication

STRING emits both `A B` and `B A` pairs. Store only where `protein1 < protein2` (lexicographic). Query logic in MCP tool does a two-way lookup.

### Bridge to BDP

`9606.ENSP00000XXXXXX` → UniProt via alias file → `data_sources` by accession.
Only insert rows where both proteins resolve in BDP.

### Schema

```sql
CREATE TABLE protein_interactions (
    id                BIGSERIAL PRIMARY KEY,
    protein_a_id      INTEGER NOT NULL REFERENCES data_sources(id),
    protein_b_id      INTEGER NOT NULL REFERENCES data_sources(id),
    -- channel scores (0-1000 SMALLINT)
    score_neighborhood    SMALLINT,
    score_fusion          SMALLINT,
    score_cooccurrence    SMALLINT,
    score_coexpression    SMALLINT,
    score_experimental    SMALLINT,
    score_database        SMALLINT,
    score_textmining      SMALLINT,
    combined_score        SMALLINT NOT NULL,
    source_version        TEXT NOT NULL DEFAULT 'string_v12',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(protein_a_id, protein_b_id)
);
CREATE INDEX ON protein_interactions(protein_a_id);
CREATE INDEX ON protein_interactions(protein_b_id);
CREATE INDEX ON protein_interactions(combined_score DESC);
```

### Crate structure

`crates/bdp-ingest/src/pipelines/string_db/`
- `config.rs` — `StringConfig { species_id: u32, min_score: i16, org_id: Uuid }`
- `downloader.rs` — HTTP download of links + aliases files
- `alias_mapper.rs` — parse alias file, build `HashMap<ensp_id, uniprot_acc>`
- `parser.rs` — streaming TSV parse, filter min_score, dedup A<B
- `runner.rs` — `StringPipelineRunner::new(config, pool).run()` → `anyhow::Result<()>`

---

## Pipeline 5 — PubMed/MEDLINE + PubTator3

### Scale

- 2026 baseline: 1,334 gz XML files (~30–33 GB compressed)
- Uncompressed: ~250 GB
- Record count: ~37 million
- PubTator3 entity annotations: ~600 MB TSV (gene/disease/chemical per PMID)

### Architecture decision: ParadeDB pg_search

Plain `tsvector` full-text search degrades at 37M records. Use **ParadeDB** `pg_search` extension (BM25 over PostgreSQL). If ParadeDB is not available on the VPS, fall back to `tsvector` + GIN index with a plan to migrate.

**Storage requirement**: ~220–280 GB for full literature tables + indexes. This **cannot** fit on the current 8 GB VPS (disk constraint). Options:
1. Separate PostgreSQL instance (500 GB+ SSD) — recommended
2. Subset load (PubMed Central Open Access only, ~4M records)
3. PubMed API only (no bulk, 3 req/s limit)

**Decision for v1**: Load PubMed Central Open Access subset (~4M records) for initial launch. Add full baseline as optional env flag `INGEST_PUBMED_FULL=true`. Infrastructure ticket required for dedicated literature node.

### XML parsing

- Crate: `quick-xml` async streaming
- Key elements per `PubmedArticle`:
  - `PMID` — primary key
  - `ArticleTitle` — title
  - `AbstractText` — abstract (may be structured with `Label`)
  - `AuthorList/Author` — LastName, ForeName, CollectiveName, AffiliationInfo
  - `MeshHeadingList` — descriptor + qualifier UIDs
  - `KeywordList` — free-text keywords
  - `ELocationID` (EIdType="doi") — DOI
  - `ArticleId` (IdType="pmc") — PMCID
  - `PubDate` — Year, Month, Day (normalize to DATE)
  - `Journal/ISSN`, `Journal/Title`

### PubTator3 entity annotations

- Download: `ftp.ncbi.nlm.nih.gov/pub/lu/PubTator3/bioconcepts2pubtator3.gz`
- Format: TSV `pmid | type | concept_id | name | mentions`
- Types: `Gene` (NCBI Gene ID), `Disease` (MONDO/MESH), `Chemical` (ChEBI/MESH)
- Insert into `publication_entities` table after publications are loaded

### Schema

```sql
CREATE TABLE publications (
    id           BIGSERIAL PRIMARY KEY,
    pmid         INTEGER NOT NULL UNIQUE,
    pmcid        TEXT,
    doi          TEXT,
    title        TEXT NOT NULL,
    abstract     TEXT,
    pub_date     DATE,
    journal      TEXT,
    source       TEXT NOT NULL DEFAULT 'pubmed',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    search_vec   TSVECTOR GENERATED ALWAYS AS (
                     to_tsvector('english', coalesce(title,'') || ' ' || coalesce(abstract,''))
                 ) STORED
);
CREATE INDEX ON publications USING GIN(search_vec);
CREATE INDEX ON publications(pmid);
CREATE INDEX ON publications(pub_date DESC NULLS LAST);

CREATE TABLE publication_authors (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    position       SMALLINT NOT NULL,
    last_name      TEXT,
    fore_name      TEXT,
    collective     TEXT,
    affiliation    TEXT
);
CREATE INDEX ON publication_authors(publication_id);

CREATE TABLE publication_mesh (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    mesh_ui        TEXT NOT NULL,   -- e.g. D009369
    descriptor     TEXT NOT NULL,
    is_major_topic BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX ON publication_mesh(publication_id);
CREATE INDEX ON publication_mesh(mesh_ui);

CREATE TABLE publication_entities (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    entity_type    TEXT NOT NULL,   -- 'gene', 'disease', 'chemical'
    external_id    TEXT NOT NULL,   -- NCBI Gene ID, MONDO:XXXXXXX, CHEBI:XXXXXXX
    entity_name    TEXT,
    -- resolved BDP foreign keys (NULL if unresolved)
    gene_id        INTEGER REFERENCES data_sources(id),
    disease_term_id INTEGER REFERENCES disease_terms(id),
    compound_id    INTEGER REFERENCES data_sources(id)
);
CREATE INDEX ON publication_entities(publication_id);
CREATE INDEX ON publication_entities(entity_type, external_id);
CREATE INDEX ON publication_entities(gene_id) WHERE gene_id IS NOT NULL;
CREATE INDEX ON publication_entities(disease_term_id) WHERE disease_term_id IS NOT NULL;
CREATE INDEX ON publication_entities(compound_id) WHERE compound_id IS NOT NULL;

CREATE TABLE pubmed_ingest_files (
    id            BIGSERIAL PRIMARY KEY,
    filename      TEXT NOT NULL UNIQUE,
    record_count  INTEGER,
    status        TEXT NOT NULL DEFAULT 'pending',   -- pending, processing, done, error
    error_message TEXT,
    processed_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Crate structure

`crates/bdp-ingest/src/pipelines/pubmed/`
- `config.rs` — `PubmedConfig { open_access_only: bool, worker_count: usize, batch_size: usize, org_id: Uuid }`
- `manifest.rs` — fetch file list from NCBI FTP; track in `pubmed_ingest_files`
- `downloader.rs` — parallel download with `JoinSet`, `worker_count` concurrency
- `parser.rs` — `quick-xml` async streaming; `PubmedArticle` struct
- `entity_linker.rs` — PubTator3 TSV parser; resolves gene/disease/compound to BDP IDs
- `runner.rs` — `PubmedPipelineRunner::new(config, pool).run()` → `anyhow::Result<()>`

---

## MCP Tool Activation Map

When each pipeline is implemented, the corresponding stub tools in `bdp-mcp` become live:

| Pipeline | MCP tool activated |
|----------|--------------------|
| Open Targets | `get_gene_diseases` (in `genes.rs`) |
| ClinicalTrials.gov | `get_disease_trials` (in `diseases.rs`), `get_compound_trials` (in `compounds.rs`) |
| ChEMBL | `get_compound_targets` (in `compounds.rs`) |
| STRING | `get_gene_interactions` — new tool to add |
| PubMed | `search_literature`, `get_publication` (in `literature.rs`) |
| PubMed entities | `get_gene_literature`, traversal paths involving literature |

Activation means: replace `common::stub_result(...)` with real DB queries. No tool schema change needed.

---

## Crate Layout in bdp-ingest

All new pipelines live inside the existing `crates/bdp-ingest` crate:

```
crates/bdp-ingest/src/pipelines/
├── open_targets/    (new)
├── clinical_trials/ (new)
├── chembl/          (new)
├── string_db/       (new)
└── pubmed/          (new)
```

Each pipeline is gated by an env flag following the existing pattern:
- `INGEST_OPEN_TARGETS_ENABLED=true`
- `INGEST_CLINICAL_TRIALS_ENABLED=true`
- `INGEST_CHEMBL_ENABLED=true`
- `INGEST_STRING_ENABLED=true`
- `INGEST_PUBMED_ENABLED=true`

The `IngestOrchestrator` (`src/orchestrator.rs`) spawns all enabled pipelines in a `JoinSet`.

---

## Migration Strategy

All migrations follow the existing pattern in `migrations/`:
- Numbered sequentially after the last existing migration
- Idempotent (`CREATE TABLE IF NOT EXISTS`)
- No data migrations — only schema

Migration order:
1. `gene_disease_associations`
2. `clinical_trials` + `trial_disease_links` + `trial_intervention_links`
3. `drug_target_activities`
4. `protein_interactions`
5. `publications` + `publication_authors` + `publication_mesh` + `publication_entities` + `pubmed_ingest_files`

---

## Infrastructure Notes

| Pipeline | Peak memory | Peak disk | Notes |
|----------|-------------|-----------|-------|
| Open Targets | ~500 MB | ~2 GB | Arrow/Parquet in memory |
| ClinicalTrials | ~200 MB | ~3 GB | AACT dump download |
| ChEMBL | ~800 MB | ~6 GB | SQLite ETL |
| STRING | ~300 MB | ~200 MB | Streaming TSV |
| PubMed (OA only) | ~1 GB | ~50 GB | 4M records, async streaming |
| PubMed (full) | ~2 GB | ~280 GB | 37M records — separate node required |

**Current VPS**: 8 GB RAM / unknown disk. PubMed full baseline requires dedicated infrastructure.
**Recommended**: Deploy full PubMed to a separate PostgreSQL node with 500+ GB SSD and 16+ GB RAM.

---

## Out of Scope (Future)

- **Europe PMC**: Enrichment layer (full-text, preprints). Add after PubMed baseline is live.
- **ClinGen**: Clinical variant-gene associations. Post-launch.
- **DrugBank**: Drug interactions. License cost — evaluate post-launch.
- **OMIM full-text**: Requires license agreement.
