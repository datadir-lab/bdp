# Graph View — WebGPU Design Spec

**Date:** 2026-03-22
**Status:** Draft
**Author:** Sebastian Stupak

---

## Overview

Design for a WebGPU-accelerated interactive graph view capable of rendering 10M+ nodes and 100M+ edges from BDP's cross-database biological knowledge graph. The view supports three interaction modes simultaneously: overview (see the whole graph), search-driven (fly to an entity), and neighborhood exploration (expand from a node).

---

## Goals

- Render 10M+ nodes and 100M+ edges in the browser at interactive frame rates
- Support all four current entity types (protein, gene, go_term, taxon) and all future types via an extensible registry
- Differentiate nodes by color (entity type) and size (log degree)
- Differentiate edges by color and width (edge type), with zoom-based visibility thresholds
- Progressive loading: meaningful content within 200ms of page open
- Graceful fallback from WebGPU to WebGL transparently

---

## Non-Goals

- In-browser force simulation (layout is precomputed offline)
- 3D rendering (noted as potential future extension)
- Editing the graph (read-only view)
- Real-time graph updates (layout refreshes weekly)

---

## Coordinate System

All node positions are stored in a **flat Cartesian coordinate space** normalized to `[-1.0, 1.0]` on both axes. This is NOT geographic data — WGS-84 (SRID 4326) must NOT be used as it applies spherical Earth math to synthetic coordinates, corrupting all bbox queries.

All PostGIS geometry columns use `GEOMETRY(POINT)` with no SRID (defaults to SRID 0, i.e., Cartesian). Tile bbox requests use the same `[-1.0, 1.0]` coordinate space. The client and server must use identical units for all bbox parameters.

---

## Architecture Overview

```
Browser (Next.js + deck.gl v9)
  └─ GraphView
       ├─ OverviewLayer         static top-5K hubs, loaded on mount
       ├─ GraphTileLayer        custom deck.gl TileLayer, streams tiles by viewport
       ├─ NeighborhoodLayer     on-demand subgraph on node click
       ├─ SearchBar             flies camera to entity position
       ├─ GraphState            merged node store, LRU eviction at 500K positional records
       └─ EdgeTypeFilterPanel + NodeTypeLegend

bdp-server (Rust/axum, CQRS)
  ├─ GET /api/v1/graph/overview          top-5K hubs, JSON, Redis-cached 1hr
  ├─ GET /api/v1/graph/tiles             bbox + zoom → FlatBuffers binary
  ├─ GET /api/v1/graph/nodes/:id/neighborhood
  ├─ GET /api/v1/graph/search            returns entity + (x, y) for camera fly-to
  └─ GET /api/v1/graph/registry          entity types + edge types (fetched once on load)

PostgreSQL + PostGIS
  ├─ graph_entity_types     registry, drives frontend filter + color system
  ├─ graph_edge_types       registry, drives edge rendering + zoom thresholds
  ├─ graph_nodes            positions (PostGIS POINT, SRID 0), degree, community, properties
  ├─ graph_edges            source, target, type, weight, midpoint (PostGIS POINT, SRID 0)
  ├─ graph_communities      community metadata
  ├─ graph_layout_jobs      layout pipeline run history
  └─ graph_overview (mat. view)  top-5K hubs by degree

Offline Layout Pipeline (cargo xtask graph layout)
  └─ Louvain community detection (pure-Rust: louvain-rs crate or igraph via CLI)
     → community macro-layout (force-directed on community graph)
     → per-community ForceAtlas2 (Rayon parallel)
     → normalize positions to [-1.0, 1.0]
     → write positions + midpoints back to DB
     → rebuild PostGIS spatial indexes
```

---

## Database Schema

### Registry tables

```sql
-- Entity types: all future types pre-defined with is_active=false
CREATE TABLE graph_entity_types (
  id          SMALLINT PRIMARY KEY,   -- starts at 1
  name        TEXT NOT NULL UNIQUE,
  label       TEXT NOT NULL,
  color_hex   TEXT NOT NULL,
  source_dbs  TEXT[] NOT NULL,
  is_active   BOOLEAN NOT NULL DEFAULT false,
  description TEXT
);

-- Edge types: driven by registry, not hardcoded enums
CREATE TABLE graph_edge_types (
  id           SMALLINT PRIMARY KEY,   -- starts at 1
  name         TEXT NOT NULL UNIQUE,
  label        TEXT NOT NULL,
  category     TEXT NOT NULL,   -- molecular | ontological | taxonomic | cross_db
  color_hex    TEXT NOT NULL,
  min_zoom     SMALLINT NOT NULL DEFAULT 5,
  is_directed  BOOLEAN NOT NULL DEFAULT true,
  is_active    BOOLEAN NOT NULL DEFAULT false,
  description  TEXT
);
```

### Core tables

```sql
CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE graph_communities (
  id           INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  name         TEXT,
  center_x     FLOAT NOT NULL,
  center_y     FLOAT NOT NULL,
  node_count   INTEGER NOT NULL,
  dominant_entity_type SMALLINT REFERENCES graph_entity_types(id)
);

CREATE TABLE graph_nodes (
  id             BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  entity_type_id SMALLINT NOT NULL REFERENCES graph_entity_types(id),
  external_id    TEXT NOT NULL,         -- original ID in source DB (e.g. P04637)
  source_db      TEXT NOT NULL,         -- 'uniprot', 'chembl', etc.
  label          TEXT,
  degree         INTEGER NOT NULL DEFAULT 0,
  size           FLOAT NOT NULL DEFAULT 1.0,   -- log10(degree+1), normalized [1,20]
  position       GEOMETRY(POINT),              -- SRID 0, Cartesian [-1.0, 1.0]
  community_id   INTEGER REFERENCES graph_communities(id),
  properties     JSONB DEFAULT '{}',    -- type-specific metadata (NOT cached client-side)
  UNIQUE (external_id, source_db)
);

CREATE TABLE graph_edges (
  id             BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  source_id      BIGINT NOT NULL REFERENCES graph_nodes(id),
  target_id      BIGINT NOT NULL REFERENCES graph_nodes(id),
  edge_type_id   SMALLINT NOT NULL REFERENCES graph_edge_types(id),
  weight         FLOAT NOT NULL DEFAULT 1.0,
  midpoint       GEOMETRY(POINT),              -- SRID 0, Cartesian, midpoint of source+target
  -- Uniqueness: prevent duplicate edges across ingestion runs.
  -- For undirected edge types, canonical form enforces source_id < target_id (see constraint below).
  UNIQUE (source_id, target_id, edge_type_id),
  -- Enforce canonical ordering for undirected edges:
  -- source_id < target_id when the edge type is undirected.
  -- Directed edge types have no ordering requirement.
  CONSTRAINT undirected_canonical_order
    CHECK (
      source_id < target_id
      OR (SELECT is_directed FROM graph_edge_types WHERE id = edge_type_id)
    )
);

CREATE TABLE graph_layout_jobs (
  id              INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  started_at      TIMESTAMPTZ NOT NULL,
  completed_at    TIMESTAMPTZ,
  strategy        TEXT NOT NULL,        -- 'full' | 'incremental'
  node_count      INTEGER,
  edge_count      INTEGER,
  community_count INTEGER,
  status          TEXT NOT NULL DEFAULT 'running'  -- 'running' | 'done' | 'failed'
);

-- Materialized view for /overview endpoint
CREATE MATERIALIZED VIEW graph_overview AS
  SELECT id, ST_X(position) as x, ST_Y(position) as y,
         entity_type_id, degree, size, label, community_id
  FROM graph_nodes
  WHERE degree > (
    SELECT percentile_disc(0.999) WITHIN GROUP (ORDER BY degree)
    FROM graph_nodes
  )
  ORDER BY degree DESC
  LIMIT 5000;
```

### Indexes

```sql
-- Tile bbox queries (Cartesian SRID 0)
CREATE INDEX CONCURRENTLY idx_graph_nodes_position
  ON graph_nodes USING GIST(position);

CREATE INDEX CONCURRENTLY idx_graph_edges_midpoint
  ON graph_edges USING GIST(midpoint);

-- LOD degree filter (used on every tile query)
CREATE INDEX idx_graph_nodes_degree
  ON graph_nodes (degree DESC);

-- Source lookup (for search + neighborhood)
CREATE INDEX idx_graph_nodes_external
  ON graph_nodes (source_db, external_id);

-- Edge traversal (neighborhood expansion)
CREATE INDEX idx_graph_edges_source ON graph_edges (source_id);
CREATE INDEX idx_graph_edges_target ON graph_edges (target_id);
```

---

## Registry Seed Data

### Entity types (IDs start at 1; all future types pre-defined with is_active=false)

| id | name | label | source_dbs | color_hex | is_active |
|----|------|-------|------------|-----------|-----------|
| 1 | protein | Protein | uniprot | #63B3ED | true |
| 2 | gene | Gene | genbank, refseq | #9AE6B4 | true |
| 3 | go_term | GO Term | gene_ontology | #F6AD55 | true |
| 4 | taxon | Taxon | ncbi_taxonomy | #ED8989 | true |
| 5 | compound | Compound | chembl, pubchem, chebi | #A78BFA | false |
| 6 | drug | Drug | drugbank, chembl | #E879F9 | false |
| 7 | disease | Disease | omim, mondo, disgenet | #FCA5A5 | false |
| 8 | phenotype | Phenotype | hpo | #FBD0E8 | false |
| 9 | pathway | Pathway | kegg, reactome, wikipathways | #5EEAD4 | false |
| 10 | variant | Variant | dbsnp, clinvar, gnomad | #FDE047 | false |
| 11 | structure | Structure | pdb, alphafold | #93C5FD | false |
| 12 | tissue | Tissue | uberon, bto | #86EFAC | false |
| 13 | cell_type | Cell Type | cell_ontology | #34D399 | false |
| 14 | metabolite | Metabolite | hmdb, metaboLights | #C4B5FD | false |
| 15 | publication | Publication | pubmed, europe_pmc | #D1D5DB | false |
| 16 | epigenomic_region | Epigenomic Region | encode, roadmap | #FB923C | false |
| 17 | sequence | Sequence (rRNA) | mgnify, silva | #6EE7B7 | false |

### Edge types — Phase 1 seed (is_active=true; future phases add rows with is_active=false)

| id | name | label | category | color_hex | min_zoom | is_directed | is_active |
|----|------|-------|----------|-----------|----------|-------------|-----------|
| 1 | interacts_with | Interacts with | molecular | #8B5CF6 | 8 | false | true |
| 2 | binds_to | Binds to | molecular | #A78BFA | 8 | true | true |
| 3 | co_expressed_with | Co-expressed with | molecular | #C4B5FD | 8 | false | true |
| 4 | is_a | Is a | ontological | #FB923C | 5 | true | true |
| 5 | part_of | Part of | ontological | #FDBA74 | 5 | true | true |
| 6 | regulates | Regulates | ontological | #FED7AA | 5 | true | true |
| 7 | positively_regulates | Positively regulates | ontological | #86EFAC | 5 | true | true |
| 8 | negatively_regulates | Negatively regulates | ontological | #FCA5A5 | 5 | true | true |
| 9 | parent_of | Parent of | taxonomic | #5EEAD4 | 5 | true | true |
| 10 | synonym_of | Synonym of | taxonomic | #99F6E4 | 5 | false | true |
| 11 | has_go_annotation | Has GO annotation | cross_db | #FACC15 | 7 | true | true |
| 12 | encoded_by | Encoded by | cross_db | #A3E635 | 7 | true | true |
| 13 | has_taxon | Has taxon | cross_db | #E879F9 | 7 | true | true |
| 14 | ortholog_of | Ortholog of | cross_db | #38BDF8 | 7 | false | true |

Future phases append rows to this table with `is_active=false` until the corresponding ingestion pipeline is built. No code changes are needed — the frontend reads the registry at startup via `/api/v1/graph/registry`.

### Edge type category zoom thresholds (reference, authoritative values are per-row above)

| category | default min_zoom | rationale |
|----------|-----------------|-----------|
| molecular | 8 | Dense, visually noisy at overview |
| ontological | 5 | Sparse hierarchy, readable at medium zoom |
| taxonomic | 5 | Tree structure, readable at medium zoom |
| cross_db | 7 | Cross-database links, meaningful only at locality |

---

## LOD Strategy

LOD filtering is **server-side only**. The server translates `zoom` to a degree threshold before querying. The client does not apply additional LOD filtering — it renders everything it receives.

```
Zoom 0–2  (world)       /overview endpoint — top 5K hubs by degree, no edges, cached
Zoom 3–5  (continent)   degree > 500  — ~50K nodes globally, ontological+taxonomic edges
Zoom 6–8  (city)        degree > 50   — ~500K nodes, all edge categories visible
Zoom 9–11 (street)      degree >= 0   — all nodes in viewport (including degree-0), labels at zoom 10
Zoom 12+  (building)    degree >= 0   — full metadata, hover cards with properties JSONB
```

### Server-side zoom → degree threshold mapping

```rust
fn degree_threshold(zoom: u8) -> u32 {
    match zoom {
        0..=2  => u32::MAX,  // handled by /overview, not /tiles
        3..=5  => 500,
        6..=8  => 50,
        _      => 0,         // zoom 9+: all nodes (degree >= 0), query uses WHERE degree >= threshold
    }
}

fn edge_weight_threshold(zoom: u8) -> f32 {
    match zoom {
        3..=5 => 0.8,   // hub-to-hub only
        6..=8 => 0.3,
        _     => 0.0,
    }
}
```

---

## Tile Server

### Registry endpoint (fetched once on page load)

```
GET /api/v1/graph/registry
→ JSON: { entity_types: [...], edge_types: [...] }
   cached client-side in memory for the session lifetime
   client uses integer IDs in all subsequent requests
```

### Tile request

```
GET /api/v1/graph/tiles
  ?x_min=&y_min=&x_max=&y_max=          (Cartesian [-1.0, 1.0] space)
  &zoom=
  &entity_type_ids=1,3                   (optional, registry integer IDs)
  &edge_type_ids=4,5,9                   (optional, registry integer IDs)
```

Client sends integer IDs (not names) — names are only for display. This avoids a name-to-ID lookup on every tile request.

### FlatBuffers response schema

```flatbuffers
table GraphNode {
  id:             ulong;
  x:              float;
  y:              float;
  entity_type_id: ushort;   // ushort, not ubyte — registry may exceed 255 entries
  degree:         uint;
  size:           float;
  label:          string;   // null at zoom < 10
}

table GraphEdge {
  source_id:    ulong;
  target_id:    ulong;
  edge_type_id: ushort;     // ushort — future edge types will exceed 255
  weight:       float;
}

table GraphTile {
  nodes:          [GraphNode];
  edges:          [GraphEdge];
  zoom:           ubyte;
  total_in_bbox:  uint;     // node count before degree filter, for UI indicator
}

root_type GraphTile;
```

Content-Type: `application/octet-stream`
Expected size at zoom 7 typical viewport: ~400–600KB (vs ~5MB JSON equivalent).

### Cross-tile edge rule

Each edge is stored with its `midpoint` geometry (average of source and target position). The tile query fetches edges whose midpoint falls within the bbox — each edge appears in exactly one tile.

**Known trade-off:** a tile may return edges whose one endpoint is outside the loaded viewport. The client skips rendering any such edge (both endpoints must be in `GraphState`). At zoom 6–8 with typical viewports this wastes ~5–15% of edge bandwidth — acceptable given the midpoint rule's simplicity and the avoidance of duplicate edge delivery.

### CQRS query handlers

```
crates/bdp-server/src/features/graph/
  mod.rs
  queries/
    get_tile.rs
      GetGraphTileQuery {
        x_min: f64, y_min: f64, x_max: f64, y_max: f64,
        zoom: u8,
        entity_type_ids: Option<Vec<i16>>,
        edge_type_ids: Option<Vec<i16>>,
      }
      -- applies degree_threshold(zoom) and edge_weight_threshold(zoom) server-side
      -- uses ST_MakeEnvelope(x_min, y_min, x_max, y_max) with SRID 0

    get_neighborhood.rs
      GetNodeNeighborhoodQuery { node_id: i64, depth: u8 }
      -- for undirected edge types: fetches both (node→neighbor) and (neighbor→node)

    search_nodes.rs
      SearchGraphNodesQuery { query: String, limit: u8 }
      -- returns { id, x, y, label, entity_type_id } for camera fly-to

    get_overview.rs
      GetGraphOverviewQuery
      -- reads graph_overview materialized view
      -- Redis cache key: "graph:overview", TTL 1hr, warmed on server startup

    get_registry.rs
      GetGraphRegistryQuery
      -- reads graph_entity_types + graph_edge_types where is_active=true
      -- Redis cache key: "graph:registry", TTL 24hr, invalidated on registry update

  router.rs
  types.rs    -- EntityType, EdgeType, FlatBuffers generated types
```

---

## Frontend Structure

```
web/app/[locale]/graph/
  page.tsx                 server component
  graph-view.tsx           client component, deck.gl canvas

web/lib/graph/
  tile-manager.ts          fetch tiles (sends integer IDs), decode FlatBuffers
  graph-state.ts           merged positional node Map, LRU eviction at 500K
  flatbuffers-decoder.ts   binary → GraphTile typed object
  lod.ts                   zoom level → edge category filter (client-side display toggle)
  renderer.ts              WebGPU device with WebGL fallback

web/components/graph/
  graph-controls.tsx       search bar, entity type filter, edge type filter
  node-tooltip.tsx         hover card at zoom 12+, fetches properties JSONB on demand
  graph-legend.tsx         active entity types + edge types from registry
```

### Client-side node record (stored in GraphState)

`GraphState` stores only the **lightweight positional record** per node. Full metadata (`properties` JSONB, full label) is **fetched on demand** when the user hovers at zoom 12+, not cached in `GraphState`.

```typescript
// Stored per node in GraphState — ~48 bytes each, cap at 500K = ~24MB
interface PositionalNode {
  id:             bigint;
  x:              number;
  y:              number;
  entityTypeId:   number;
  degree:         number;
  size:           number;
  label:          string | null;   // present only at zoom >= 10
}

// Fetched on hover/click, NOT stored in GraphState
interface NodeMetadata {
  id:         bigint;
  label:      string;
  externalId: string;
  sourceDb:   string;
  properties: Record<string, unknown>;  // type-specific JSONB fields
}
```

### GraphState LRU eviction

```typescript
export class GraphState {
  private nodes = new Map<bigint, PositionalNode>();
  private readonly MAX_NODES = 500_000;  // ~24MB positional records

  merge(tile: GraphTile): void {
    for (const node of tile.nodes) {
      this.nodes.set(node.id, node);
    }
    this.evictIfNeeded();
  }

  evictOldestTile(tileNodes: PositionalNode[]): void {
    for (const n of tileNodes) this.nodes.delete(n.id);
  }

  private evictIfNeeded(): void {
    if (this.nodes.size <= this.MAX_NODES) return;
    const overflow = this.nodes.size - this.MAX_NODES;
    const iter = this.nodes.keys();
    for (let i = 0; i < overflow; i++) this.nodes.delete(iter.next().value);
  }

  has(id: bigint): boolean { return this.nodes.has(id); }
  get(id: bigint): PositionalNode | undefined { return this.nodes.get(id); }
}
```

### Key frontend behaviors

**Initial load:** fetch `/registry` and `/overview` in parallel on mount. Render 5K hubs immediately from overview. TileLayer activates once overview is painted.

**Viewport change:** deck.gl TileLayer requests tiles for visible bbox at current zoom. Sends integer entity/edge type IDs from registry. Cancels stale in-flight requests. LRU cache holds up to 100 tiles.

**Search:** `GET /search?q=TP53` → returns `{ id, x, y, label, entity_type_id }` → `FlyToInterpolator` animates camera to `(x, y)` at zoom 10.

**Node click:** fetches neighborhood at depth 2, merges into `GraphState`. For undirected edge types, the neighborhood endpoint returns edges in both directions.

**Hover at zoom 12+:** fetches `NodeMetadata` (including `properties` JSONB) on demand. Not stored in `GraphState`.

**WebGPU fallback:**
```typescript
try {
  device = await createWebGPUDevice();
} catch {
  device = await createWebGLDevice();  // same deck.gl code, different backend
}
```

---

## Offline Layout Pipeline

Invoked via: `cargo xtask graph layout [--incremental] [--dry-run]`

**Infrastructure requirement:** needs 32GB+ RAM. Must run on the dedicated ingestion server, not the web server. Coordinate with ops before scheduling.

### Layout algorithm — pure Rust, no Python dependency

Community detection uses a pure-Rust Louvain implementation (evaluate `louvain-rs` or implement directly using `petgraph`). This avoids a cross-language subprocess boundary and integrates cleanly with the xtask pipeline.

If a third-party tool proves necessary for scale (e.g., igraph for very large graphs), it is invoked via CLI with a well-defined contract:
- **Input:** temp file of edge list CSV (`source_id,target_id,weight`) written by the pipeline
- **Output:** temp file of community assignments CSV (`node_id,community_id`)
- **Error handling:** non-zero exit code → pipeline fails with structured error, layout job marked `failed`

### Stages

```
1. Extract      — stream all nodes + edges from PostgreSQL into memory (~2.5GB RAM)
2. Detect       — Louvain community detection (Rust, ~30 min for 10M nodes / 100M edges)
3. Macro layout — force-directed on community graph (~1K community nodes, seconds)
4. Per-community ForceAtlas2 — Rayon parallel across communities (~10–30 min)
                  high-degree nodes pinned at community center
                  periphery spreads outward proportional to degree
5. Normalize    — all positions to [-1.0, 1.0] Cartesian space
                  size = log10(degree+1), normalized to [1.0, 20.0]
6. Write back   — positions, community_id, degree, size → graph_nodes
7. Midpoints    — midpoint = ((src.x+tgt.x)/2, (src.y+tgt.y)/2) → graph_edges.midpoint
8. Indexes      — REINDEX CONCURRENTLY on all spatial indexes
9. Mat. view    — REFRESH MATERIALIZED VIEW CONCURRENTLY graph_overview
10. Job record  — mark graph_layout_jobs row as 'done'
```

### Refresh strategy

```
After each ingestion cycle:
  if new_node_count < 5% of total:
    → incremental: assign new nodes to nearest community centroid + gaussian jitter
    → compute midpoints for new edges only
    → partial spatial index rebuild
  else:
    → full recompute (off-peak, ~1–2 hours total)
```

---

## Roadmap — Ingestion Domain Phases

Each phase flips `is_active = true` in the registry for the relevant entity and edge types. No schema migrations are needed — all types are pre-declared. The layout pipeline automatically incorporates new nodes on its next run.

### Phase 1 — Current (active)
- Proteins (UniProt)
- Genes (GenBank / RefSeq)
- Gene Ontology terms
- Taxa (NCBI Taxonomy)

### Phase 1b — Protein Interaction Networks
**Sources:** STRING, BioGRID, IntAct
**New entity types:** none (proteins already active)
**New edge types:** `interacts_with` (already seeded), adds confidence-scored PPI edges
**Note:** STRING alone adds ~11B interaction pairs at full confidence. Edge count will exceed 1B at this phase — midpoint spatial index performance must be benchmarked before enabling.

### Phase 2 — Chemical & Drug Intelligence
**Sources:** ChEMBL, DrugBank, PubChem, ChEBI
**New entity types:** `compound` (id=5), `drug` (id=6)
**New edge types:** `targets` (drug→protein), `inhibits`, `activates`
**Value:** drug-target interaction network, compound-structure clustering

### Phase 3 — Disease & Phenotype
**Sources:** OMIM, MONDO Disease Ontology, DisGeNET, HPO, ClinVar
**New entity types:** `disease` (id=7), `phenotype` (id=8), `variant` (id=10)
**New edge types:** `causes` (variant→disease), `associated_with` (gene→disease), `has_phenotype`, `treats` (drug→disease)
**Value:** complete genotype-phenotype-disease axis, clinical relevance scoring

### Phase 4 — Pathways & Metabolomics
**Sources:** KEGG, Reactome, WikiPathways, HMDB, MetaboLights
**New entity types:** `pathway` (id=9), `metabolite` (id=14)
**New edge types:** `participates_in` (protein/gene→pathway), `metabolized_to`, `produced_by`, `found_in`
**Value:** systems biology view — from gene to pathway to metabolite

### Phase 5 — Protein Structure
**Sources:** PDB, AlphaFold DB
**New entity types:** `structure` (id=11)
**New edge types:** `has_structure` (protein→structure)
**Properties additions:** proteins gain `{ "alphafold_confidence": 0.92, "pdb_ids": ["1TUP"] }` in `properties` JSONB
**Value:** structure-function relationships, confidence-annotated AlphaFold predictions

### Phase 6 — Anatomy, Expression & Cell Biology
**Sources:** UBERON, Cell Ontology (CL), BTO, GTEx, Expression Atlas
**New entity types:** `tissue` (id=12), `cell_type` (id=13)
**New edge types:** `expressed_in` (gene→tissue), `contains` (tissue→cell_type), `derived_from`, `located_in`
**Note:** `tissue` and `cell_type` are pre-declared in the registry but NOT used as embedded properties in any earlier phase. Expression context is stored as JSONB on gene nodes (e.g., `{ "high_expression_tissues": ["liver", "kidney"] }`) until Phase 6 activates them as first-class node types.
**Value:** anatomical context for expression and disease data, tissue-specific expression overlays

### Phase 7 — Literature
**Sources:** PubMed, Europe PMC
**New entity types:** `publication` (id=15)
**New edge types:** `cited_by`, `co_mentioned_with` (NLP co-occurrence), `supports_association`
**Value:** evidence layer — every cross-DB edge can be traced to a supporting publication

### Phase 8 — Microbiome & Environmental Genomics
**Sources:** MGnify, SILVA, IMG/M
**New entity types:** `sequence` (id=17, rRNA / metagenome sequences)
**New edge types:** `co_occurs_with` (in microbiome samples), `similar_to` (sequence identity > threshold)
**Value:** host-microbiome interaction, environmental genomics context

### Phase 9 — Epigenomics
**Sources:** ENCODE, Roadmap Epigenomics
**New entity types:** `epigenomic_region` (id=16)
**New edge types:** `epigenetically_regulates` (epigenomic_region→gene), `methylated_in`, `open_chromatin_in`
**Note:** `epigenetically_regulates` is a distinct name from the Phase 1 ontological `regulates` edge type — both must have unique names in `graph_edge_types`.
**Value:** regulatory layer connecting epigenome to gene expression and disease

---

## Open Questions

1. **Layout pipeline server:** the pipeline needs 32GB+ RAM. Is the dedicated ingestion server provisioned for this, or does it require a cloud burst job (e.g., a spot instance triggered post-ingestion)?

2. **Phase 1b edge count ceiling:** STRING + BioGRID at full confidence pushes edges toward 1B+. The `graph_edges.midpoint` GiST index at that scale needs benchmarking before Phase 1b ships. Consider a write-time partial index (`WHERE weight > 0.5`) to keep the index size manageable.

3. **cosmos.gl fallback:** cosmos.gl `disableSimulation` + `setPointPositions(Float32Array)` is a viable alternative renderer if deck.gl TileLayer proves insufficient for any reason. Keep as a documented fallback option.

4. **3D future:** UMAP embeddings of protein sequence space could warrant a 3D view. Schema `(x, y)` could extend to `(x, y, z)` via a column addition + `GEOMETRY(POINTZ)` migration. No action now.

5. **Registry cache invalidation:** when a new ingestion phase flips `is_active=true`, the server must invalidate the Redis `graph:registry` cache. Define the invalidation hook (post-migration step? admin endpoint? automatic on deploy?).

---

## References

- [cosmos.gl — GPU graph rendering, disableSimulation, setPointPositions](https://github.com/cosmosgl/graph)
- [GraphWaGu — first WebGPU graph system, Barnes-Hut in compute shaders](https://par.nsf.gov/biblio/10384648-graphwagu-gpu-powered-large-scale-graph-layout-computation-rendering-web)
- [Interactive Graph Layout of a Million Nodes](https://www.mdpi.com/2227-9709/3/4/23)
- [Louvain — Scalable Distributed Algorithm (1B+ edges)](https://cse.unl.edu/~yu/homepage/publications/paper/2018.A%20Scalable%20Distributed%20Louvain%20Algorithm%20for%20Large-scale%20Graph%20Community%20Detection.pdf)
- [Fast Multipole Methods for Force-Directed Layout — O(n), 7M vertices](https://ieeexplore.ieee.org/document/6341510/)
- [Interactive LOD Rendering — edge bundling + node aggregation](https://lago.hs8.de/)
- [FlatBuffers — zero-copy binary serialization, Rust + TS](https://flatbuffers.dev/)
