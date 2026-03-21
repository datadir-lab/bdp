# BDP Vector Embeddings & /vectors Page — Design Spec

**Date:** 2026-03-21
**Status:** Approved
**Linear:** BDP-66 (MCP server — semantic search dependency)

---

## Overview

Add pgvector-based semantic embeddings for all BDP registry entries across all
bioinformatics databases, a `/vectors` page for interactive 2D visualization of
the embedding space, and vector similarity search for MCP tool integration.

**Scale target:** 10M+ registry entries initially (UniProt, NCBI RefSeq,
InterPro, GO, PDB, Taxonomy); 50M–250M+ at full scope including TrEMBL,
AlphaFold, PubMed literature, pathways, variants, compounds, and expression
data. See _Planned Data Domains_ section for the full type registry.

---

## Goals

1. Embed every `registry_entry` as a 512-dim text vector (name + description +
   organism + source_type + tags)
2. Pre-compute 2D UMAP projection + quadtree tiles for interactive visualization
3. Expose `/vectors` page using `regl-scatterplot` (handles 20M points in WebGL)
4. Expose semantic search endpoint powering MCP `search_sources` tool
5. Design schema to accommodate Phase 2 sequence embeddings (ESM-2) without
   migration pain

**Non-goals (Phase 1):**
- Sequence-level embeddings (ESM-2) — schema supports it, not implemented yet
- Real-time embedding of new entries (incremental batch job is sufficient)
- 3D visualization (2D is the proven approach at this scale; Nomic Atlas,
  WizMap, Jupyter Scatter all use 2D)

---

## Architecture

```
Registry entries
      |
      v
[bdp-embed CLI — Stage 1: Embed]
  OpenAI text-embedding-3-small, dimensions=512
  Batches of 2048, incremental (skip already embedded)
      |
      v
entry_embeddings (halfvec(512) + HNSW index ~10GB)
      |
      v
[bdp-embed CLI — Stage 2: Project]
  Landmark UMAP (50K landmarks, stable coords)
  New points projected onto fixed scaffold
      |
      v
entry_projections (x, y, denormalized display fields)
      |
      v
[bdp-embed CLI — Stage 3: Tiles]
  Quadtree build over 2D coords (WizMap approach)
  Zoom levels 0-14, tile JSON files
      |
      v
MinIO  vectors/tiles/{run_id}/{z}/{x}/{y}.json
       vectors/models/{run_id}/umap.joblib
      |
      v
Backend API (Rust/axum CQRS)         pgvector KNN
  GET /api/v1/vectors/tiles/{z}/{x}/{y}    |
  GET /api/v1/vectors/search?q=...  <------+
  GET /api/v1/vectors/{id}/neighbors
  GET /api/v1/vectors/stats
      |
      v
Frontend /vectors page (Next.js)
  regl-scatterplot — renders tile contents
  Viewport-based tile fetching (Leaflet model)
  Search → fly to result cluster
  Click → sidebar with neighbors + detail link
```

---

## Database Schema

### New migrations (three)

**Migration 1 — enable pgvector + entry_embeddings:**

```sql
CREATE EXTENSION IF NOT EXISTS vector;

-- Text embeddings: 512-dim Matryoshka via text-embedding-3-small
-- Matryoshka allows truncating 1536 dims → 512 with modest quality loss;
-- halfvec stores as float16 instead of float32 (50% storage savings).
-- Table size: 10M × 512 × 2 bytes = ~10GB on disk.
-- HNSW index RAM: ~5–8GB (graph links, not full vector data — separate from table).
CREATE TABLE entry_embeddings (
    entry_id     UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    model        VARCHAR(100) NOT NULL DEFAULT 'text-embedding-3-small',
    vector       halfvec(512) NOT NULL,
    embedded_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- HNSW for approximate nearest-neighbor search (cosine similarity)
-- m=16, ef_construction=64: standard tradeoff of recall (~97%) vs build time (~1-2h).
-- Online inserts are supported but large batch additions (>1M rows) should
-- be followed by REINDEX to restore graph balance and recall quality.
CREATE INDEX ON entry_embeddings
    USING hnsw (vector halfvec_cosine_ops)
    WITH (m = 16, ef_construction = 64);
```

**Migration 2 — entry_projections:**

```sql
-- Pre-computed 2D UMAP coords for the /vectors page
-- Denormalized display fields avoid joins at query time for 10M+ rows
-- entry_type values: 'data_source' | 'tool' (mirrors registry_entries constraint)
CREATE TABLE entry_projections (
    entry_id     UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    x            FLOAT4 NOT NULL,
    y            FLOAT4 NOT NULL,
    label        TEXT NOT NULL,           -- entry name, display only
    entry_type   VARCHAR(50) NOT NULL,    -- 'data_source' or 'tool'
    source_type  VARCHAR(50),             -- protein | genome | annotation | etc
    org_slug     VARCHAR(100) NOT NULL,   -- for URL building
    slug         VARCHAR(255) NOT NULL,   -- for URL building
    projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ON entry_projections (x, y);
CREATE INDEX ON entry_projections (source_type);
CREATE INDEX ON entry_projections (entry_type, source_type);
```

**Migration 3 — vector_projection_runs:**

```sql
-- Tracks each completed bdp-embed pipeline run.
-- Frontend reads current_run_id from /stats to construct versioned tile URLs.
CREATE TABLE vector_projection_runs (
    run_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- status: 'pending' | 'embedding' | 'projecting' | 'tiling' | 'complete' | 'failed'
    stage_completed VARCHAR(20),          -- last successfully completed stage
    entry_count     BIGINT,               -- total registry_entries at run time
    embedded_count  BIGINT,               -- entries with embeddings
    projected_count BIGINT,               -- entries with projection coords
    tile_prefix     TEXT,                 -- MinIO prefix: vectors/tiles/{run_id}/
    error_message   TEXT,                 -- set on failure
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    projected_at    TIMESTAMPTZ,          -- set when project stage completes
    completed_at    TIMESTAMPTZ           -- set when all three stages complete
);
```

**Phase 2 (not in scope now) — sequence_embeddings:**
Separate table `sequence_embeddings` with `halfvec(1280)` (ESM-2 650M) and
its own HNSW index. Same `entry_id` FK. Added in a later migration when GPU
inference pipeline is ready.

### Tile storage

Tile files stored in MinIO under existing S3 bucket:
```
vectors/tiles/{run_id}/{z}/{x}/{y}.json
```

Empty spatial regions produce **no tile file** — a 404 is the canonical
"no points here" response. At zoom 14 the theoretical cell count is
2^14 × 2^14 but the actual written tile count equals the number of
non-empty grid cells, which is far lower for sparse bio data.

**Canonical tile record schema** (TypeScript):
```typescript
interface TilePoint {
  id:   string;   // entry_id (UUID)
  x:    number;   // projected x coord
  y:    number;   // projected y coord
  l:    string;   // label (entry name)
  et:   string;   // entry_type: 'data_source' | 'tool'
  st:   string;   // source_type: 'protein' | 'genome' | etc ('' if null)
  org:  string;   // org_slug
  slug: string;   // entry slug
}
type TileFile = TilePoint[];
```

`run_id` versions tiles — the frontend reads `current_run_id` from
`/api/v1/vectors/stats` at startup and constructs tile URLs as
`/api/v1/vectors/tiles/{run_id}/{z}/{x}/{y}`. Old tiles remain valid
while a new projection is being built.

---

## Embedding Pipeline — `bdp-embed`

A Python CLI (`tools/bdp-embed/`) invoked by the existing Rust job system after
bulk ingestion completes. Three subcommands:

### `bdp-embed embed`

```
bdp-embed embed \
  --db-url $DATABASE_URL \
  --openai-key $OPENAI_API_KEY \
  --model text-embedding-3-small \
  --dimensions 512 \
  --batch-size 2048 \
  --workers 8
```

- Reads `registry_entries` not yet in `entry_embeddings` (incremental)
- Builds embed text: `f"{name} {description or ''} {source_type or ''} {organism or ''} {tags or ''}"`
- Calls OpenAI embeddings API in parallel batches
- Writes `halfvec(512)` rows to `entry_embeddings`
- Cost estimate: ~$0.02 per 1M tokens; 10M entries × ~100 tokens ≈ **$20 total**

### `bdp-embed project`

```
bdp-embed project \
  --db-url $DATABASE_URL \
  --run-id $RUN_ID \
  --landmarks 50000 \
  --method landmark-umap
```

- Selects 50K landmark points via k-means centroids from `entry_embeddings`
- Runs full UMAP on landmarks only and **serializes the fitted UMAP model**
  to MinIO (`vectors/models/{run_id}/umap.joblib`) — this is critical for
  coordinate stability. Subsequent runs reload this model to project new
  entries onto the same scaffold rather than re-fitting from scratch.
- Projects all remaining entries onto the fixed landmark scaffold via
  `umap_model.transform()` — existing coordinates are stable as long as the
  same model is reused. The model is only re-fitted when the landmark set
  itself needs to change (e.g., after a major schema change or full re-ingestion),
  which intentionally shifts all coordinates.
- Writes x, y + denormalized fields to `entry_projections`
- Runtime: ~30-60 min for 10M entries on standard CPU; faster with GPU

### `bdp-embed tiles`

```
bdp-embed tiles \
  --db-url $DATABASE_URL \
  --s3-bucket bdp \
  --zoom-min 0 \
  --zoom-max 14 \
  --output-prefix vectors/tiles/{run_id}/
```

- Builds quadtree from `entry_projections` (WizMap approach)
- At each zoom level: tile = 256×256 logical grid cell
  - Zoom 0-3: 1 representative per cluster (coarse overview)
  - Zoom 4-9: progressive density
  - Zoom 10-14: full density within cell
- Writes tile JSON files to MinIO
- Runtime: ~10 min for 10M entries
- Updates a `vector_projection_runs` metadata table with `run_id`,
  `projected_at`, `entry_count`, `tile_prefix`

### Error handling

| Error | Behaviour |
|---|---|
| OpenAI rate limit (429) | Exponential backoff, max 10 retries per batch |
| OpenAI API key missing | Fail immediately with clear error message |
| OpenAI unreachable | Abort run, set `vector_projection_runs.status = 'failed'` |
| Empty embed text (NULL name + NULL description) | Skip entry, log warning, do not embed |
| Entry text > 8191 tokens | Truncate to 8191 tokens before sending |
| MinIO unavailable during tiles | Abort tiles stage, mark run as failed |
| k-means fails to converge | Retry with increased max_iter, fallback to random landmark selection |

### Python dependencies (`tools/bdp-embed/pyproject.toml`)

```toml
[project]
requires-python = ">=3.11"
dependencies = [
    "openai>=1.30",
    "umap-learn>=0.5",
    "scikit-learn>=1.4",    # k-means for landmarks
    "numpy>=1.26",
    "psycopg[binary]>=3.1", # async postgres
    "boto3>=1.34",           # MinIO/S3
    "joblib>=1.3",           # UMAP model serialization
    "tqdm>=4.66",            # progress bars
    "typer>=0.12",           # CLI framework
]
```

### Invocation from job system

Each stage is tracked separately in `vector_projection_runs`. The Rust job
system runs stages sequentially, updating status after each:

```rust
// In ingestion job completion handler
// run_id is created here and passed to all three subcommands
let run_id = create_projection_run(&pool).await?;

// bdp-embed embed (incremental, no --run-id needed)
run_embed_stage(run_id, &pool).await
    .map_err(|e| mark_run_failed(run_id, e))?;

// bdp-embed project --run-id {run_id}  (writes umap.joblib to MinIO)
run_project_stage(run_id, &pool).await
    .map_err(|e| mark_run_failed(run_id, e))?;

// bdp-embed tiles --run-id {run_id}  (writes tiles to MinIO)
run_tiles_stage(run_id, &pool).await
    .map_err(|e| mark_run_failed(run_id, e))?;

mark_run_complete(run_id, &pool).await?;
```

If a stage fails, the next trigger skips completed stages by checking
`stage_completed` on the most recent run. `embed` is always incremental;
`project` and `tiles` resume from scratch but are fast enough (~1h total)
that this is acceptable.

---

## Backend API — `features/vectors/`

New CQRS feature following existing patterns.

### File structure

```
crates/bdp-server/src/features/vectors/
  mod.rs
  queries/
    mod.rs
    get_tile.rs          — proxies MinIO tile, adds cache headers
    semantic_search.rs   — embeds query + pgvector KNN
    get_neighbors.rs     — KNN from an existing entry's vector
    get_stats.rs         — coverage stats + last projection run info
  routes.rs
```

### Endpoints

All endpoints are public (no auth required for read-only vector data).
`semantic_search` is rate-limited to 60 req/min per IP (each call triggers
an OpenAI API request if the query is not cached).

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/vectors/tiles/{run_id}/{z}/{x}/{y}` | Serve pre-built tile from MinIO |
| GET | `/api/v1/vectors/search?q=&k=20` | Semantic search via pgvector KNN |
| GET | `/api/v1/vectors/{entry_id}/neighbors?k=10` | KNN for a specific entry |
| GET | `/api/v1/vectors/stats` | Coverage stats + current run metadata |

### `semantic_search` query

**Rust handler flow:**
1. Receive query string `q`
2. Check in-process LRU cache (128 entries, keyed by query string)
3. Cache miss: call OpenAI `client.embeddings.create(model="text-embedding-3-small", input=q, dimensions=512)` via `async-openai` crate. Returns `Vec<f32>` (512 floats).
4. Cast to `halfvec` for SQLx bind: `pgvector::HalfVector::from(vec_f32)`
5. If OpenAI unreachable: return `503 Service Unavailable` with message "Embedding service unavailable"
6. Run SQL, return results

```sql
-- Note: data_sources uses table inheritance — data_sources.id IS registry_entries.id
-- (shared primary key). The LEFT JOIN on ds.id = re.id is therefore correct.
SELECT
    re.slug,
    re.name,
    re.entry_type,
    ds.source_type,
    o.slug        AS org_slug,
    ep.x,
    ep.y,
    1 - (e.vector <=> $1) AS similarity
FROM entry_embeddings e
JOIN registry_entries re ON re.id = e.entry_id
JOIN organizations o ON o.id = re.organization_id
LEFT JOIN data_sources ds ON ds.id = re.id
LEFT JOIN entry_projections ep ON ep.entry_id = e.entry_id
ORDER BY e.vector <=> $1
LIMIT $2
```

`x` and `y` may be NULL if a projection run has not yet completed for this
entry — the frontend handles this by skipping the camera-fly step.

### `get_neighbors` query

```sql
-- Two-step: fetch seed vector, then KNN excluding self
SELECT
    re.slug,
    re.name,
    re.entry_type,
    ds.source_type,
    o.slug AS org_slug,
    ep.x,
    ep.y,
    1 - (e.vector <=> seed.vector) AS similarity
FROM entry_embeddings e
CROSS JOIN (
    SELECT vector FROM entry_embeddings WHERE entry_id = $1
) seed
JOIN registry_entries re ON re.id = e.entry_id
JOIN organizations o ON o.id = re.organization_id
LEFT JOIN data_sources ds ON ds.id = re.id
LEFT JOIN entry_projections ep ON ep.entry_id = e.entry_id
WHERE e.entry_id != $1
ORDER BY e.vector <=> seed.vector
LIMIT $2
```

Returns 404 if `$1` (entry_id) has no embedding yet.

### `get_stats` response

```json
{
  "current_run_id": "uuid | null",
  "status": "pending | embedding | projecting | tiling | complete | failed | null",
  "entry_count": 10420000,
  "embedded_count": 8200000,
  "projected_count": 8150000,
  "projected_at": "2026-03-21T14:00:00Z | null",
  "tile_prefix": "vectors/tiles/{run_id}/ | null"
}
```

`null` values indicate no completed run exists yet.

### `get_tile` handler

```rust
// Proxy MinIO tile — no DB query
// MinIO path: {tile_prefix}{z}/{x}/{y}.json  (tile_prefix from run_id in route)
// Response: Cache-Control: public, max-age=86400, immutable
// 404 if tile doesn't exist — normal for empty spatial regions or deep zoom
// The run_id in the URL path ensures old tiles remain valid during a rebuild
```

Sparse tiles (empty spatial regions) are **not written** to MinIO — a 404
response is the canonical signal for "no points in this tile". The frontend
silently skips 404 tiles.

---

## Frontend — `/vectors` page

### Tech additions

- `regl-scatterplot` — WebGL scatter plot, up to 20M points, pan/zoom/select
- No Three.js, no deck.gl required

### Page structure

```
/vectors
  ├── stats bar (top): "8.2M of 10.4M entries embedded · projected 2h ago"
  ├── search bar (overlay): semantic search input
  ├── legend (overlay): toggle by source_type (protein/genome/annotation/tool/…)
  ├── canvas: regl-scatterplot instance
  └── sidebar (right, on click): label, type, org, nearest neighbors, "Open" link
```

### Tile loading model

Follows the Leaflet/MapLibre tile model:
1. Page init: fetch `/api/v1/vectors/stats` → get `current_run_id`
2. Determine initial viewport tiles (zoom=3, center of projection space)
3. Fetch tile JSONs → pass points to regl-scatterplot
4. On pan/zoom (debounced 150ms): diff current viewport vs loaded tiles,
   fetch missing tiles, append to point set
5. Tiles are cached in-memory for the session (avoid re-fetching on pan-back)

### Color mapping

Use the canonical `SOURCE_TYPE_COLORS` constant defined in the
_Planned Data Domains_ section. Do not define colors locally in the page
component — import from a shared constants file.

### Search flow

1. User types query → debounced 300ms
2. Call `GET /api/v1/vectors/search?q=<query>&k=20`
3. Results returned with x, y coords (from `entry_projections`)
4. Fly camera to centroid of result cluster
5. Highlight matching points (cyan ring, same as Veles approach)
6. Non-matching points dimmed to 20% opacity

### Sidebar (on point click)

- Entry name, type badge, org name
- "Open" link → existing detail page (`/sources/{org}/{slug}`)
- "Nearest neighbors" section: calls `GET /api/v1/vectors/{id}/neighbors?k=6`
  → shows 6 nearest entries with similarity score + type badge

### Empty/loading states

- No projections yet: "No embeddings yet. Run `bdp-embed embed` to get started."
- Partial coverage: "3.1M of 10.4M entries embedded. More appearing as ingestion runs."
- Tile 404: silently skip (normal for deep zoom in sparse regions)

---

## MCP Integration

The `search_sources` tool in BDP-66 calls `semantic_search` directly. No extra
work required — the vector endpoint is a drop-in semantic upgrade to text search:

```
User: "Find me the latest UniProt SwissProt FASTA"
AI:   calls search_sources(query="uniprot swissprot fasta")
         → server embeds query (or uses LRU cache)
         → pgvector KNN returns top-5 with similarity scores
         → MCP tool returns formatted results
```

Both text search (existing) and semantic search (new) run in parallel for MCP
queries; results are merged and ranked by combined score.

---

## Operations

### Routine workflow

```
bulk ingestion completes
    → job triggers: bdp-embed embed   (~17h for 10M at Tier 3 rate limits)
    → job triggers: bdp-embed project (~30-60 min)
    → job triggers: bdp-embed tiles   (~10 min)
    → frontend picks up new run_id from /stats on next load
```

### Coordinate stability

Coordinate stability depends on **reusing the serialized UMAP model** across
runs (stored in MinIO at `vectors/models/{run_id}/umap.joblib`). New entries
are projected via `umap_model.transform()` onto the fixed scaffold — their
coordinates are deterministic and existing points are unaffected.

Coordinates shift globally only when:
- The landmark set is re-selected (major schema change or full re-ingestion)
- A new UMAP model is fitted from scratch

This is an intentional, infrequent operation. The frontend has no mechanism
to detect coordinate shifts between runs — users may notice visual jumps
if they have bookmarked a region. This is acceptable for Phase 1.

### Index build

HNSW build on `halfvec(512)` at 10M rows:
- Estimated build time: 1-2h offline (not blocking API reads)
- Table storage: ~10GB on disk
- HNSW index in RAM: ~5-8GB (graph links, not the full vector data)
- Online inserts after initial build are supported but large batch additions
  (>1M rows) should be followed by `REINDEX CONCURRENTLY` to restore recall

### Sizing

| Component | Estimate |
|-----------|---------|
| `entry_embeddings` table (disk) | ~10GB (halfvec(512) × 10M) |
| HNSW index (RAM) | ~5–8GB |
| `entry_projections` table | ~1.5GB (x, y, text fields × 10M) |
| Tile files in MinIO | ~2–5GB per projection run (sparse tiles not written) |
| UMAP model in MinIO | ~500MB per run |
| Embedding cost (OpenAI) | ~$20 for 10M entries (one-time) |

---

## Planned Data Domains

This section documents the full intended scope of BDP data types so that the
embedding pipeline, `source_type` registry, color legend, and embed text
builders are designed to accommodate them from day one — even if ingestion
pipelines for some don't exist yet.

### Source type registry

The `source_type` column on `data_sources` is an open `VARCHAR(50)`. The
following values are the full planned contract. Ingestion pipelines and embed
text builders should be added incrementally; the schema requires no changes.

| source_type | Primary sources | Phase | Embed text strategy |
|---|---|---|---|
| `protein` | UniProt Swiss-Prot, TrEMBL | 1 (active) | name + description + gene_name + organism + function + GO terms |
| `genome` | NCBI RefSeq, Ensembl, UCSC | 1 (active) | assembly name + organism + assembly level + annotation source |
| `annotation` | ENCODE, Roadmap Epigenomics | 1 (active) | dataset name + description + assay type + organism + tissue |
| `structure` | PDB | 1 (active) | entry title + organism + method + resolution + molecule names |
| `taxonomy` | NCBI Taxonomy, GTDB | 1 (active) | scientific name + common name + lineage + rank |
| `transcript` | Ensembl, RefSeq | 1 (active) | transcript name + gene name + biotype + organism |
| `domain` | InterPro, Pfam, PROSITE | 1 (active) | domain name + description + type + member databases |
| `ontology_term` | GO, ChEBI, HPO, Uberon, Cell Ontology, SO | 1 (planned) | term name + definition + synonyms + namespace + parent terms |
| `pathway` | KEGG, Reactome, WikiPathways, MetaCyc | 1 (planned) | pathway name + organism + description + gene list (top 20) |
| `interaction` | STRING, BioGRID, IntAct | 2 (planned) | protein A name + protein B name + interaction type + evidence |
| `variant` | ClinVar, dbSNP, gnomAD, GWAS Catalog | 2 (planned) | rsID + gene + consequence + clinical significance + trait |
| `compound` | ChEMBL, PubChem, DrugBank, ChEBI | 2 (planned) | compound name + synonyms + bioactivity + targets + InChI key |
| `expression` | GEO, GTEx, ArrayExpress, TCGA | 2 (planned) | dataset title + organism + tissue/condition + assay type |
| `predicted_structure` | AlphaFold DB (~200M entries) | 2 (planned) | protein name + organism + confidence score + UniProt accession |
| `metagenome` | SILVA, MGnify, Human Microbiome Project | 2 (planned) | sample description + environment + taxonomy summary |
| `literature` | PubMed, bioRxiv, Europe PMC | special (see below) | title + abstract (raw text, no prefix) |

### Literature is a special case

PubMed alone has 36M+ abstracts — 3× the current BDP entry count. Literature
embeddings act as a **semantic backbone**: they bridge proteins, pathways,
variants, and compounds through the natural language of science. A researcher
searching "BRCA1 homologous recombination repair" should surface both proteins
and the papers that describe them in proximity in the vector space.

Design implications:
- Literature gets its own `source_type = 'literature'` with no truncation in
  embed text (full abstract, up to 512 tokens, truncated at token limit)
- `entry_projections` for literature points will cluster by research topic
  rather than data type — expected behavior
- Phase 1 scope: title + abstract only. Phase 2: citation graph edges as
  additional signal
- Scale: 36M PubMed + ~500K bioRxiv ≈ ~37M additional entries — largest single
  source type. Pipeline must handle this incrementally

### AlphaFold scale note

AlphaFold DB has ~200M predicted structures (one per UniProt entry). These
overlap heavily with `protein` entries — the same UniProt accession gets both a
`protein` entry (metadata) and a `predicted_structure` entry (3D coords +
confidence). At full scale this doubles the UniProt entry count. Plan
accordingly for HNSW index sizing in Phase 2.

### Source-type-aware embed text builders

The `bdp-embed embed` subcommand uses a pluggable builder per `source_type`
rather than a single generic template. This produces significantly higher
quality embeddings because the most semantically meaningful fields differ per
type:

```python
def build_embed_text(entry: dict, source_type: str) -> str:
    match source_type:
        case "protein":
            return f"{entry['name']} {entry.get('gene_name','')} " \
                   f"{entry.get('organism','')} {entry.get('function','')} " \
                   f"{entry.get('go_terms','')}"
        case "pathway":
            genes = " ".join(entry.get('gene_list', [])[:20])
            return f"{entry['name']} {entry.get('organism','')} " \
                   f"{entry.get('description','')} genes: {genes}"
        case "ontology_term":
            return f"{entry['name']} {entry.get('definition','')} " \
                   f"synonyms: {entry.get('synonyms','')} " \
                   f"namespace: {entry.get('namespace','')}"
        case "compound":
            return f"{entry['name']} {entry.get('synonyms','')} " \
                   f"{entry.get('bioactivity','')} targets: {entry.get('targets','')}"
        case "variant":
            return f"{entry.get('gene','')} {entry.get('consequence','')} " \
                   f"{entry.get('clinical_significance','')} {entry.get('trait','')}"
        case "genome":
            return f"{entry['name']} {entry.get('organism','')} " \
                   f"{entry.get('assembly_level','')} {entry.get('annotation_source','')}"
        case "taxonomy":
            return f"{entry['name']} {entry.get('common_name','')} " \
                   f"{entry.get('lineage','')} {entry.get('rank','')}"
        case "transcript":
            return f"{entry['name']} {entry.get('gene_name','')} " \
                   f"{entry.get('biotype','')} {entry.get('organism','')}"
        case "annotation":
            return f"{entry['name']} {entry.get('description','')} " \
                   f"{entry.get('assay_type','')} {entry.get('organism','')} " \
                   f"{entry.get('tissue','')}"
        case "structure":
            return f"{entry['name']} {entry.get('organism','')} " \
                   f"{entry.get('method','')} {entry.get('molecule_names','')}"
        case "domain":
            return f"{entry['name']} {entry.get('description','')} " \
                   f"{entry.get('domain_type','')} {entry.get('member_dbs','')}"
        case "literature":
            return f"{entry['title']} {entry.get('abstract','')}"  # raw text, no prefix
        case _:
            # Generic fallback for any type not yet explicitly handled
            return f"{entry['name']} {entry.get('description','')} " \
                   f"{source_type} {entry.get('organism','')}"
```

New source types get a fallback automatically. A dedicated builder is added
when that type's ingestion pipeline ships.

### Color legend expansion

The `/vectors` page legend must accommodate all planned types. The full color
map (add to frontend constants):

```typescript
export const SOURCE_TYPE_COLORS: Record<string, string> = {
  protein:             '#3b82f6',  // blue
  genome:              '#22c55e',  // green
  annotation:          '#f97316',  // orange
  structure:           '#06b6d4',  // cyan
  predicted_structure: '#0891b2',  // darker cyan
  taxonomy:            '#a855f7',  // purple
  transcript:          '#84cc16',  // lime
  domain:              '#f59e0b',  // amber
  ontology_term:       '#8b5cf6',  // violet
  pathway:             '#10b981',  // emerald
  interaction:         '#ef4444',  // red
  variant:             '#f43f5e',  // rose
  compound:            '#d946ef',  // fuchsia
  expression:          '#14b8a6',  // teal
  metagenome:          '#78716c',  // stone
  literature:          '#e2e8f0',  // slate-200 (light, distinct from data)
  tool:                '#64748b',  // slate
};
```

---

## Phase 2 — Sequence Embeddings (future)

When ESM-2 GPU inference pipeline is ready:

1. Add `sequence_embeddings` table with `halfvec(1280)` (ESM-2 650M model)
2. Add `bdp-embed embed-sequences` subcommand (reads protein sequences, runs
   ESM-2 in batches on GPU)
3. Add separate UMAP projection for sequence space
4. `/vectors` page gets a toggle: "Metadata view" vs "Sequence similarity view"
5. MCP `search_sources` gains `search_by_sequence` parameter

---

## Testing

- Unit tests for `semantic_search` query handler validation
- Integration test: embed 100 entries → project → verify KNN returns expected
  neighbors
- Tile API test: verify 404 for nonexistent tiles, 200 with correct JSON for
  built tiles
- Frontend: test tile loading, search flight, sidebar neighbor display

---

## Checklist

- [ ] Migration 1: enable pgvector, create `entry_embeddings`
- [ ] Migration 2: create `entry_projections`
- [ ] Migration 3: create `vector_projection_runs`
- [ ] `bdp-embed embed` subcommand (Python, source-type-aware builders)
  - [ ] Builders for all Phase 1 active types (protein, genome, annotation, structure, taxonomy, transcript, domain)
  - [ ] Generic fallback builder for planned types not yet active
- [ ] `bdp-embed project` subcommand (Python, landmark UMAP)
- [ ] `bdp-embed tiles` subcommand (Python, quadtree → MinIO)
- [ ] Backend: `features/vectors/` CQRS feature
  - [ ] `get_tile` query (MinIO proxy)
  - [ ] `semantic_search` query (pgvector KNN + LRU cache)
  - [ ] `get_neighbors` query
  - [ ] `get_stats` query
  - [ ] Routes registered
- [ ] Frontend: `/vectors` page
  - [ ] regl-scatterplot integration
  - [ ] Tile loading (viewport-based)
  - [ ] Search bar + camera fly
  - [ ] Legend + type toggles
  - [ ] Click sidebar + neighbors
- [ ] MCP: wire `search_sources` to semantic search endpoint
- [ ] Tests (unit + integration)
- [ ] `bdp-embed` documented in deployment guide
