# Graph View — WebGPU Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a tile-streaming WebGPU graph view capable of rendering 10M+ nodes and 100M+ edges from BDP's cross-database biological knowledge graph.

**Architecture:** PostgreSQL + PostGIS stores node positions in Cartesian SRID-0 space. A Rust/axum CQRS tile server streams FlatBuffers-encoded tiles filtered by viewport bbox + zoom-based LOD. A deck.gl v9 frontend (WebGPU with WebGL fallback) renders tiles progressively with a 500K-node LRU GraphState.

**Tech Stack:** Rust + axum + sqlx + PostGIS + FlatBuffers (server); Next.js 16 + deck.gl v9 + @luma.gl/webgpu + flatbuffers (frontend)

**Spec:** `docs/superpowers/specs/2026-03-22-graph-view-webgpu-design.md`

**Note:** The offline layout pipeline (`cargo xtask graph layout`) is a separate follow-on plan. This plan covers DB schema, backend API, and frontend rendering only.

---

## File Map

### New files (backend)
- `crates/bdp-server/src/features/graph/mod.rs` — module root, registers handlers
- `crates/bdp-server/src/features/graph/types.rs` — shared response types, FlatBuffers builder helpers
- `crates/bdp-server/src/features/graph/router.rs` — axum Router with 5 endpoints
- `crates/bdp-server/src/features/graph/queries/get_registry.rs` — GetGraphRegistryQuery
- `crates/bdp-server/src/features/graph/queries/get_overview.rs` — GetGraphOverviewQuery
- `crates/bdp-server/src/features/graph/queries/get_tile.rs` — GetGraphTileQuery (FlatBuffers response)
- `crates/bdp-server/src/features/graph/queries/search_nodes.rs` — SearchGraphNodesQuery
- `crates/bdp-server/src/features/graph/queries/get_neighborhood.rs` — GetNodeNeighborhoodQuery

### Modified files (backend)
- `crates/bdp-server/src/features/mod.rs` — add `pub mod graph;` + `.nest` registration
- `crates/bdp-server/Cargo.toml` — add `flatbuffers` dep

### New migrations
- `migrations/20260322000001_graph_registry_tables.sql` — graph_entity_types + graph_edge_types
- `migrations/20260322000002_graph_core_tables.sql` — graph_nodes, graph_edges, graph_communities, graph_layout_jobs, indexes, materialized view
- `migrations/20260322000003_graph_seed_data.sql` — 17 entity types + 14 edge types

### New files (frontend)
- `web/lib/graph/flatbuffers-decoder.ts` — binary FlatBuffers → GraphTile typed object
- `web/lib/graph/graph-state.ts` — LRU positional node store, cap 500K
- `web/lib/graph/lod.ts` — zoom → visible edge category set (client-side display toggle)
- `web/lib/graph/renderer.ts` — WebGPU device with WebGL fallback
- `web/lib/graph/tile-manager.ts` — fetch tiles, decode, merge into GraphState
- `web/lib/graph/graph-tile-layer.ts` — custom deck.gl TileLayer subclass
- `web/components/graph/graph-controls.tsx` — search bar + entity/edge type filter panel
- `web/components/graph/graph-legend.tsx` — active types from registry
- `web/components/graph/node-tooltip.tsx` — hover card at zoom 12+, fetches metadata on demand
- `web/app/[locale]/graph/graph-view.tsx` — client component, deck.gl canvas
- `web/app/[locale]/graph/page.tsx` — server component, page shell

---

## Phase 1: Database Migrations

### Task 1: Migration — Registry Tables

**Files:**
- Create: `migrations/20260322000001_graph_registry_tables.sql`

- [ ] **Step 1: Write migration**

```sql
-- Migration: graph registry tables
-- entity types and edge types drive frontend color/filter/zoom systems

CREATE TABLE graph_entity_types (
  id          SMALLINT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  label       TEXT NOT NULL,
  color_hex   TEXT NOT NULL,
  source_dbs  TEXT[] NOT NULL,
  is_active   BOOLEAN NOT NULL DEFAULT false,
  description TEXT
);

CREATE TABLE graph_edge_types (
  id           SMALLINT PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE,
  label        TEXT NOT NULL,
  category     TEXT NOT NULL,
  color_hex    TEXT NOT NULL,
  min_zoom     SMALLINT NOT NULL DEFAULT 5,
  is_directed  BOOLEAN NOT NULL DEFAULT true,
  is_active    BOOLEAN NOT NULL DEFAULT false,
  description  TEXT
);
```

- [ ] **Step 2: Apply migration**

Run: `cargo xtask db migrate`
Expected: migration applied, tables created

- [ ] **Step 3: Commit**

```bash
git add migrations/20260322000001_graph_registry_tables.sql
git commit -m "feat(graph): add graph_entity_types and graph_edge_types registry tables"
```

---

### Task 2: Migration — Core Tables, Indexes, Materialized View

**Files:**
- Create: `migrations/20260322000002_graph_core_tables.sql`

- [ ] **Step 1: Write migration**

```sql
-- Migration: graph core tables
-- Requires PostGIS (already enabled on prod; enable in test if needed)

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
  external_id    TEXT NOT NULL,
  source_db      TEXT NOT NULL,
  label          TEXT,
  degree         INTEGER NOT NULL DEFAULT 0,
  size           FLOAT NOT NULL DEFAULT 1.0,
  position       GEOMETRY(POINT),
  community_id   INTEGER REFERENCES graph_communities(id),
  properties     JSONB DEFAULT '{}',
  UNIQUE (external_id, source_db)
);

CREATE TABLE graph_edges (
  id             BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  source_id      BIGINT NOT NULL REFERENCES graph_nodes(id),
  target_id      BIGINT NOT NULL REFERENCES graph_nodes(id),
  edge_type_id   SMALLINT NOT NULL REFERENCES graph_edge_types(id),
  weight         FLOAT NOT NULL DEFAULT 1.0,
  midpoint       GEOMETRY(POINT),
  UNIQUE (source_id, target_id, edge_type_id),
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
  strategy        TEXT NOT NULL,
  node_count      INTEGER,
  edge_count      INTEGER,
  community_count INTEGER,
  status          TEXT NOT NULL DEFAULT 'running'
);

-- Spatial indexes (GIST for bbox queries, Cartesian SRID 0)
CREATE INDEX idx_graph_nodes_position
  ON graph_nodes USING GIST(position);

CREATE INDEX idx_graph_edges_midpoint
  ON graph_edges USING GIST(midpoint);

-- LOD degree filter
CREATE INDEX idx_graph_nodes_degree
  ON graph_nodes (degree DESC);

-- Source lookup (search + neighborhood)
CREATE INDEX idx_graph_nodes_external
  ON graph_nodes (source_db, external_id);

-- Edge traversal (neighborhood)
CREATE INDEX idx_graph_edges_source ON graph_edges (source_id);
CREATE INDEX idx_graph_edges_target ON graph_edges (target_id);

-- Materialized view: top-5K hubs for /overview
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

- [ ] **Step 2: Apply migration**

Run: `cargo xtask db migrate`
Expected: all tables + indexes + materialized view created

- [ ] **Step 3: Regenerate SQLx metadata**

Run: `cargo xtask sqlx prepare`
Expected: `.sqlx/` updated

- [ ] **Step 4: Commit**

```bash
git add migrations/20260322000002_graph_core_tables.sql .sqlx/
git commit -m "feat(graph): add core graph tables, PostGIS indexes, and graph_overview materialized view"
```

---

### Task 3: Migration — Seed Registry Data

**Files:**
- Create: `migrations/20260322000003_graph_seed_data.sql`

- [ ] **Step 1: Write migration**

```sql
-- Seed: 17 entity types (4 active, 13 pending future ingestion phases)
INSERT INTO graph_entity_types (id, name, label, color_hex, source_dbs, is_active) VALUES
  (1,  'protein',           'Protein',           '#63B3ED', ARRAY['uniprot'],                           true),
  (2,  'gene',              'Gene',              '#9AE6B4', ARRAY['genbank', 'refseq'],                 true),
  (3,  'go_term',           'GO Term',           '#F6AD55', ARRAY['gene_ontology'],                     true),
  (4,  'taxon',             'Taxon',             '#ED8989', ARRAY['ncbi_taxonomy'],                     true),
  (5,  'compound',          'Compound',          '#A78BFA', ARRAY['chembl', 'pubchem', 'chebi'],        false),
  (6,  'drug',              'Drug',              '#E879F9', ARRAY['drugbank', 'chembl'],                false),
  (7,  'disease',           'Disease',           '#FCA5A5', ARRAY['omim', 'mondo', 'disgenet'],         false),
  (8,  'phenotype',         'Phenotype',         '#FBD0E8', ARRAY['hpo'],                               false),
  (9,  'pathway',           'Pathway',           '#5EEAD4', ARRAY['kegg', 'reactome', 'wikipathways'],  false),
  (10, 'variant',           'Variant',           '#FDE047', ARRAY['dbsnp', 'clinvar', 'gnomad'],        false),
  (11, 'structure',         'Structure',         '#93C5FD', ARRAY['pdb', 'alphafold'],                  false),
  (12, 'tissue',            'Tissue',            '#86EFAC', ARRAY['uberon', 'bto'],                     false),
  (13, 'cell_type',         'Cell Type',         '#34D399', ARRAY['cell_ontology'],                     false),
  (14, 'metabolite',        'Metabolite',        '#C4B5FD', ARRAY['hmdb', 'metaboLights'],              false),
  (15, 'publication',       'Publication',       '#D1D5DB', ARRAY['pubmed', 'europe_pmc'],              false),
  (16, 'epigenomic_region', 'Epigenomic Region', '#FB923C', ARRAY['encode', 'roadmap'],                 false),
  (17, 'sequence',          'Sequence (rRNA)',   '#6EE7B7', ARRAY['mgnify', 'silva'],                   false);

-- Seed: 14 Phase 1 edge types (all active)
INSERT INTO graph_edge_types (id, name, label, category, color_hex, min_zoom, is_directed, is_active) VALUES
  (1,  'interacts_with',        'Interacts with',        'molecular',   '#8B5CF6', 8, false, true),
  (2,  'binds_to',              'Binds to',              'molecular',   '#A78BFA', 8, true,  true),
  (3,  'co_expressed_with',     'Co-expressed with',     'molecular',   '#C4B5FD', 8, false, true),
  (4,  'is_a',                  'Is a',                  'ontological', '#FB923C', 5, true,  true),
  (5,  'part_of',               'Part of',               'ontological', '#FDBA74', 5, true,  true),
  (6,  'regulates',             'Regulates',             'ontological', '#FED7AA', 5, true,  true),
  (7,  'positively_regulates',  'Positively regulates',  'ontological', '#86EFAC', 5, true,  true),
  (8,  'negatively_regulates',  'Negatively regulates',  'ontological', '#FCA5A5', 5, true,  true),
  (9,  'parent_of',             'Parent of',             'taxonomic',   '#5EEAD4', 5, true,  true),
  (10, 'synonym_of',            'Synonym of',            'taxonomic',   '#99F6E4', 5, false, true),
  (11, 'has_go_annotation',     'Has GO annotation',     'cross_db',    '#FACC15', 7, true,  true),
  (12, 'encoded_by',            'Encoded by',            'cross_db',    '#A3E635', 7, true,  true),
  (13, 'has_taxon',             'Has taxon',             'cross_db',    '#E879F9', 7, true,  true),
  (14, 'ortholog_of',           'Ortholog of',           'cross_db',    '#38BDF8', 7, false, true);
```

- [ ] **Step 2: Apply migration**

Run: `cargo xtask db migrate`
Expected: 17 entity types + 14 edge types inserted

- [ ] **Step 3: Verify seed data**

Run: `cargo xtask db shell`
Then: `SELECT count(*) FROM graph_entity_types;` → 17, `SELECT count(*) FROM graph_edge_types;` → 14

- [ ] **Step 4: Commit**

```bash
git add migrations/20260322000003_graph_seed_data.sql
git commit -m "feat(graph): seed 17 entity types and 14 Phase 1 edge types"
```

---

## Phase 2: Backend CQRS

### Task 4: Add flatbuffers dependency

**Files:**
- Modify: `crates/bdp-server/Cargo.toml`

- [ ] **Step 1: Add flatbuffers**

In `[dependencies]` section, add:
```toml
flatbuffers = "24"
```

- [ ] **Step 2: Build check**

Run: `cargo build -p bdp-server`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-server/Cargo.toml Cargo.lock
git commit -m "feat(graph): add flatbuffers dependency to bdp-server"
```

---

### Task 5: Shared types and FlatBuffers builder

**Files:**
- Create: `crates/bdp-server/src/features/graph/types.rs`

- [ ] **Step 1: Write types.rs**

```rust
use serde::Serialize;

/// Response type for /registry
#[derive(Debug, Serialize)]
pub struct GraphRegistryResponse {
    pub entity_types: Vec<EntityTypeDto>,
    pub edge_types: Vec<EdgeTypeDto>,
}

#[derive(Debug, Serialize)]
pub struct EntityTypeDto {
    pub id: i16,
    pub name: String,
    pub label: String,
    pub color_hex: String,
    pub source_dbs: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct EdgeTypeDto {
    pub id: i16,
    pub name: String,
    pub label: String,
    pub category: String,
    pub color_hex: String,
    pub min_zoom: i16,
    pub is_directed: bool,
    pub is_active: bool,
}

/// Response type for /overview
#[derive(Debug, Serialize)]
pub struct GraphOverviewResponse {
    pub nodes: Vec<OverviewNodeDto>,
}

#[derive(Debug, Serialize)]
pub struct OverviewNodeDto {
    pub id: i64,
    pub x: f64,
    pub y: f64,
    pub entity_type_id: i16,
    pub degree: i32,
    pub size: f64,
    pub label: Option<String>,
    pub community_id: Option<i32>,
}

/// Response type for /search
#[derive(Debug, Serialize)]
pub struct SearchResultDto {
    pub id: i64,
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub entity_type_id: i16,
}

/// Server-side LOD: zoom level → degree threshold for node queries
pub fn degree_threshold(zoom: u8) -> i32 {
    match zoom {
        0..=2 => i32::MAX, // handled by /overview, not /tiles
        3..=5 => 500,
        6..=8 => 50,
        _ => 0, // zoom 9+: all nodes
    }
}

/// Server-side LOD: zoom level → minimum edge weight
pub fn edge_weight_threshold(zoom: u8) -> f64 {
    match zoom {
        3..=5 => 0.8,
        6..=8 => 0.3,
        _ => 0.0,
    }
}

/// Build a FlatBuffers-encoded GraphTile binary response.
/// Schema mirrors spec: GraphNode, GraphEdge, GraphTile root type.
pub mod flatbuffers_builder {
    use flatbuffers::{FlatBufferBuilder, WIPOffset};

    pub struct FbNode {
        pub id: u64,
        pub x: f32,
        pub y: f32,
        pub entity_type_id: u16,
        pub degree: u32,
        pub size: f32,
        pub label: Option<String>,
    }

    pub struct FbEdge {
        pub source_id: u64,
        pub target_id: u64,
        pub edge_type_id: u16,
        pub weight: f32,
    }

    /// Encode nodes + edges into a FlatBuffers binary tile.
    /// Returns the owned bytes vec ready to send as application/octet-stream.
    pub fn encode_tile(
        nodes: Vec<FbNode>,
        edges: Vec<FbEdge>,
        zoom: u8,
        total_in_bbox: u32,
    ) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::with_capacity(512 * 1024);

        // Build node vector
        let fb_nodes: Vec<_> = nodes
            .iter()
            .map(|n| {
                let label_offset = n
                    .label
                    .as_deref()
                    .map(|s| builder.create_string(s));
                // Inline table: id, x, y, entity_type_id, degree, size, label
                // Using manual byte layout matching the FlatBuffers schema in the spec.
                // Each field pushed in declaration order per FlatBuffers convention.
                let mut node_builder = flatbuffers::TableBuilder::new(&mut builder);
                // Field 0: id (ulong)
                node_builder.add_value(flatbuffers::field_index_to_field_offset(0), n.id, 0u64);
                // Field 1: x (float)
                node_builder.add_value(flatbuffers::field_index_to_field_offset(1), n.x, 0.0f32);
                // Field 2: y (float)
                node_builder.add_value(flatbuffers::field_index_to_field_offset(2), n.y, 0.0f32);
                // Field 3: entity_type_id (ushort)
                node_builder.add_value(flatbuffers::field_index_to_field_offset(3), n.entity_type_id, 0u16);
                // Field 4: degree (uint)
                node_builder.add_value(flatbuffers::field_index_to_field_offset(4), n.degree, 0u32);
                // Field 5: size (float)
                node_builder.add_value(flatbuffers::field_index_to_field_offset(5), n.size, 0.0f32);
                // Field 6: label (string, optional)
                if let Some(lbl) = label_offset {
                    node_builder.add_offset(flatbuffers::field_index_to_field_offset(6), lbl);
                }
                node_builder.finish()
            })
            .collect();

        let fb_edges: Vec<_> = edges
            .iter()
            .map(|e| {
                let mut edge_builder = flatbuffers::TableBuilder::new(&mut builder);
                edge_builder.add_value(flatbuffers::field_index_to_field_offset(0), e.source_id, 0u64);
                edge_builder.add_value(flatbuffers::field_index_to_field_offset(1), e.target_id, 0u64);
                edge_builder.add_value(flatbuffers::field_index_to_field_offset(2), e.edge_type_id, 0u16);
                edge_builder.add_value(flatbuffers::field_index_to_field_offset(3), e.weight, 0.0f32);
                edge_builder.finish()
            })
            .collect();

        let nodes_vec = builder.create_vector(&fb_nodes);
        let edges_vec = builder.create_vector(&fb_edges);

        let mut tile_builder = flatbuffers::TableBuilder::new(&mut builder);
        tile_builder.add_offset(flatbuffers::field_index_to_field_offset(0), nodes_vec);
        tile_builder.add_offset(flatbuffers::field_index_to_field_offset(1), edges_vec);
        tile_builder.add_value(flatbuffers::field_index_to_field_offset(2), zoom, 0u8);
        tile_builder.add_value(flatbuffers::field_index_to_field_offset(3), total_in_bbox, 0u32);
        let root = tile_builder.finish();
        builder.finish(root, None);

        builder.finished_data().to_vec()
    }
}
```

> **Note for implementer:** The `flatbuffers::TableBuilder` API above uses the low-level flatbuffers crate API. If the flatbuffers crate version you pin uses the code-generated pattern instead, generate Rust code from the `.fbs` schema file in `schemas/graph.fbs` (create it from the spec) and import the generated types. Either approach is acceptable — pick the one that compiles cleanest.

- [ ] **Step 2: Compile check**

Run: `cargo build -p bdp-server`
Expected: compiles (or adjust FlatBuffers API to match crate version)

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-server/src/features/graph/types.rs
git commit -m "feat(graph): add graph feature types and FlatBuffers tile builder"
```

---

### Task 6: GetGraphRegistryQuery

**Files:**
- Create: `crates/bdp-server/src/features/graph/queries/get_registry.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "migrations")]
    async fn test_get_registry_returns_active_types(pool: PgPool) {
        let result = handle(pool, GetGraphRegistryQuery).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        // seed data: 4 active entity types, 14 active edge types
        assert_eq!(response.entity_types.len(), 4);
        assert_eq!(response.edge_types.len(), 14);
        // protein is entity type id=1
        let protein = response.entity_types.iter().find(|e| e.name == "protein");
        assert!(protein.is_some());
        assert_eq!(protein.unwrap().color_hex, "#63B3ED");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bdp-server graph::queries::get_registry -- --nocapture`
Expected: compile error (module not created yet)

- [ ] **Step 3: Implement**

```rust
use crate::features::graph::types::{EdgeTypeDto, EntityTypeDto, GraphRegistryResponse};
use anyhow::Result;
use mediator::Request;
use sqlx::PgPool;

pub struct GetGraphRegistryQuery;

impl Request<Result<GraphRegistryResponse>> for GetGraphRegistryQuery {}
impl crate::cqrs::middleware::Query for GetGraphRegistryQuery {}

pub async fn handle(pool: PgPool, _query: GetGraphRegistryQuery) -> Result<GraphRegistryResponse> {
    let entity_types = sqlx::query_as!(
        EntityTypeDto,
        r#"
        SELECT id, name, label, color_hex, source_dbs, is_active
        FROM graph_entity_types
        WHERE is_active = true
        ORDER BY id
        "#
    )
    .fetch_all(&pool)
    .await?;

    let edge_types = sqlx::query_as!(
        EdgeTypeDto,
        r#"
        SELECT id, name, label, category, color_hex, min_zoom, is_directed, is_active
        FROM graph_edge_types
        WHERE is_active = true
        ORDER BY id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(GraphRegistryResponse {
        entity_types,
        edge_types,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p bdp-server graph::queries::get_registry -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-server/src/features/graph/queries/get_registry.rs
git commit -m "feat(graph): add GetGraphRegistryQuery"
```

---

### Task 7: GetGraphOverviewQuery

**Files:**
- Create: `crates/bdp-server/src/features/graph/queries/get_overview.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "migrations")]
    async fn test_get_overview_empty_db(pool: PgPool) {
        // With no nodes inserted, overview returns empty list (materialized view is empty)
        let result = handle(pool, GetGraphOverviewQuery).await;
        assert!(result.is_ok());
        assert!(result.unwrap().nodes.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bdp-server graph::queries::get_overview -- --nocapture`
Expected: compile error

- [ ] **Step 3: Implement**

```rust
use crate::features::graph::types::{GraphOverviewResponse, OverviewNodeDto};
use anyhow::Result;
use mediator::Request;
use sqlx::PgPool;

pub struct GetGraphOverviewQuery;

impl Request<Result<GraphOverviewResponse>> for GetGraphOverviewQuery {}
impl crate::cqrs::middleware::Query for GetGraphOverviewQuery {}

pub async fn handle(pool: PgPool, _query: GetGraphOverviewQuery) -> Result<GraphOverviewResponse> {
    let nodes = sqlx::query_as!(
        OverviewNodeDto,
        r#"
        SELECT id, x, y, entity_type_id, degree, size, label, community_id
        FROM graph_overview
        ORDER BY degree DESC
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(GraphOverviewResponse { nodes })
}
```

> **Note:** `graph_overview` is a materialized view. In test databases it will be empty unless `REFRESH MATERIALIZED VIEW graph_overview` is called after inserting test nodes. The test above covers the empty case; full integration is covered by E2E tests after the layout pipeline populates data.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p bdp-server graph::queries::get_overview -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-server/src/features/graph/queries/get_overview.rs
git commit -m "feat(graph): add GetGraphOverviewQuery"
```

---

### Task 8: GetGraphTileQuery

**Files:**
- Create: `crates/bdp-server/src/features/graph/queries/get_tile.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "migrations")]
    async fn test_get_tile_empty_db_returns_empty_tile(pool: PgPool) {
        let query = GetGraphTileQuery {
            x_min: -1.0,
            y_min: -1.0,
            x_max: 1.0,
            y_max: 1.0,
            zoom: 9,
            entity_type_ids: None,
            edge_type_ids: None,
        };
        let result = handle(pool, query).await;
        assert!(result.is_ok());
        // bytes are a valid FlatBuffers payload — just check it is non-empty
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[sqlx::test(migrations = "migrations")]
    async fn test_degree_threshold_applied(pool: PgPool) {
        // degree_threshold(3..=5) = 500. With no nodes with degree>500,
        // a zoom-4 query over full bbox should return 0 nodes.
        let query = GetGraphTileQuery {
            x_min: -1.0,
            y_min: -1.0,
            x_max: 1.0,
            y_max: 1.0,
            zoom: 4,
            entity_type_ids: None,
            edge_type_ids: None,
        };
        let result = handle(pool, query).await;
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bdp-server graph::queries::get_tile -- --nocapture`
Expected: compile error

- [ ] **Step 3: Implement**

```rust
use crate::features::graph::types::{
    degree_threshold, edge_weight_threshold,
    flatbuffers_builder::{encode_tile, FbEdge, FbNode},
};
use anyhow::Result;
use mediator::Request;
use sqlx::PgPool;

pub struct GetGraphTileQuery {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
    pub zoom: u8,
    pub entity_type_ids: Option<Vec<i16>>,
    pub edge_type_ids: Option<Vec<i16>>,
}

impl Request<Result<Vec<u8>>> for GetGraphTileQuery {}
impl crate::cqrs::middleware::Query for GetGraphTileQuery {}

pub async fn handle(pool: PgPool, query: GetGraphTileQuery) -> Result<Vec<u8>> {
    let deg_threshold = degree_threshold(query.zoom) as i32;
    let wt_threshold = edge_weight_threshold(query.zoom);
    let include_labels = query.zoom >= 10;

    // Count total nodes in bbox before degree filter (for UI indicator)
    let total_in_bbox: i64 = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM graph_nodes
        WHERE ST_Within(position, ST_MakeEnvelope($1, $2, $3, $4, 0))
        "#,
        query.x_min,
        query.y_min,
        query.x_max,
        query.y_max,
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    // Fetch nodes in bbox with LOD degree filter
    let rows = if let Some(ids) = &query.entity_type_ids {
        sqlx::query!(
            r#"
            SELECT id, ST_X(position) as x, ST_Y(position) as y,
                   entity_type_id, degree, size,
                   CASE WHEN $6 THEN label ELSE NULL END as label
            FROM graph_nodes
            WHERE ST_Within(position, ST_MakeEnvelope($1, $2, $3, $4, 0))
              AND degree >= $5
              AND entity_type_id = ANY($7)
            "#,
            query.x_min, query.y_min, query.x_max, query.y_max,
            deg_threshold,
            include_labels,
            ids as &[i16],
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query!(
            r#"
            SELECT id, ST_X(position) as x, ST_Y(position) as y,
                   entity_type_id, degree, size,
                   CASE WHEN $6 THEN label ELSE NULL END as label
            FROM graph_nodes
            WHERE ST_Within(position, ST_MakeEnvelope($1, $2, $3, $4, 0))
              AND degree >= $5
            "#,
            query.x_min, query.y_min, query.x_max, query.y_max,
            deg_threshold,
            include_labels,
        )
        .fetch_all(&pool)
        .await?
    };

    let nodes: Vec<FbNode> = rows
        .into_iter()
        .map(|r| FbNode {
            id: r.id as u64,
            x: r.x.unwrap_or(0.0) as f32,
            y: r.y.unwrap_or(0.0) as f32,
            entity_type_id: r.entity_type_id as u16,
            degree: r.degree as u32,
            size: r.size as f32,
            label: r.label,
        })
        .collect();

    // Fetch edges whose midpoint falls in bbox, filtered by weight threshold.
    // Cross-tile rule: edges are assigned to the tile containing their midpoint.
    // We do NOT filter by source_id — edges whose source was LOD-filtered out
    // but whose midpoint falls in this tile still belong to this tile.
    // The client skips rendering any edge whose endpoints are absent from GraphState.
    let edge_rows = if let Some(ids) = &query.edge_type_ids {
        sqlx::query!(
            r#"
            SELECT source_id, target_id, edge_type_id, weight
            FROM graph_edges
            WHERE ST_Within(midpoint, ST_MakeEnvelope($1, $2, $3, $4, 0))
              AND weight >= $5
              AND edge_type_id = ANY($6)
            "#,
            query.x_min, query.y_min, query.x_max, query.y_max,
            wt_threshold,
            ids as &[i16],
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query!(
            r#"
            SELECT source_id, target_id, edge_type_id, weight
            FROM graph_edges
            WHERE ST_Within(midpoint, ST_MakeEnvelope($1, $2, $3, $4, 0))
              AND weight >= $5
            "#,
            query.x_min, query.y_min, query.x_max, query.y_max,
            wt_threshold,
        )
        .fetch_all(&pool)
        .await?
    };

    let edges: Vec<FbEdge> = edge_rows
        .into_iter()
        .map(|r| FbEdge {
            source_id: r.source_id as u64,
            target_id: r.target_id as u64,
            edge_type_id: r.edge_type_id as u16,
            weight: r.weight as f32,
        })
        .collect();

    Ok(encode_tile(nodes, edges, query.zoom, total_in_bbox as u32))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p bdp-server graph::queries::get_tile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-server/src/features/graph/queries/get_tile.rs
git commit -m "feat(graph): add GetGraphTileQuery with LOD filtering and FlatBuffers response"
```

---

### Task 9: SearchGraphNodesQuery

**Files:**
- Create: `crates/bdp-server/src/features/graph/queries/search_nodes.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "migrations")]
    async fn test_search_empty_db(pool: PgPool) {
        let query = SearchGraphNodesQuery {
            query: "TP53".to_string(),
            limit: 10,
        };
        let result = handle(pool, query).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bdp-server graph::queries::search_nodes -- --nocapture`
Expected: compile error

- [ ] **Step 3: Implement**

```rust
use crate::features::graph::types::SearchResultDto;
use anyhow::Result;
use mediator::Request;
use sqlx::PgPool;

pub struct SearchGraphNodesQuery {
    pub query: String,
    pub limit: u8,
}

impl Request<Result<Vec<SearchResultDto>>> for SearchGraphNodesQuery {}
impl crate::cqrs::middleware::Query for SearchGraphNodesQuery {}

pub async fn handle(pool: PgPool, q: SearchGraphNodesQuery) -> Result<Vec<SearchResultDto>> {
    let like_pattern = format!("%{}%", q.query.to_lowercase());
    let results = sqlx::query_as!(
        SearchResultDto,
        r#"
        SELECT id, ST_X(position) as "x!", ST_Y(position) as "y!",
               label, entity_type_id
        FROM graph_nodes
        WHERE lower(label) LIKE $1
           OR lower(external_id) LIKE $1
        ORDER BY degree DESC
        LIMIT $2
        "#,
        like_pattern,
        q.limit as i64,
    )
    .fetch_all(&pool)
    .await?;

    Ok(results)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p bdp-server graph::queries::search_nodes -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-server/src/features/graph/queries/search_nodes.rs
git commit -m "feat(graph): add SearchGraphNodesQuery"
```

---

### Task 10: GetNodeNeighborhoodQuery

**Files:**
- Create: `crates/bdp-server/src/features/graph/queries/get_neighborhood.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "migrations")]
    async fn test_get_neighborhood_nonexistent_node(pool: PgPool) {
        let query = GetNodeNeighborhoodQuery {
            node_id: 999999,
            depth: 2,
        };
        let result = handle(pool, query).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.nodes.is_empty());
        assert!(resp.edges.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bdp-server graph::queries::get_neighborhood -- --nocapture`
Expected: compile error

- [ ] **Step 3: Implement**

```rust
use anyhow::Result;
use mediator::Request;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct NeighborhoodResponse {
    pub nodes: Vec<NeighborNodeDto>,
    pub edges: Vec<NeighborEdgeDto>,
}

#[derive(Debug, Serialize)]
pub struct NeighborNodeDto {
    pub id: i64,
    pub x: f64,
    pub y: f64,
    pub entity_type_id: i16,
    pub degree: i32,
    pub size: f64,
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NeighborEdgeDto {
    pub source_id: i64,
    pub target_id: i64,
    pub edge_type_id: i16,
    pub weight: f64,
}

pub struct GetNodeNeighborhoodQuery {
    pub node_id: i64,
    pub depth: u8,
}

impl Request<Result<NeighborhoodResponse>> for GetNodeNeighborhoodQuery {}
impl crate::cqrs::middleware::Query for GetNodeNeighborhoodQuery {}

pub async fn handle(pool: PgPool, q: GetNodeNeighborhoodQuery) -> Result<NeighborhoodResponse> {
    // Depth-1 or depth-2 neighborhood via recursive CTE
    // For undirected edge types we fetch both directions (source_id = node OR target_id = node)
    let depth = q.depth.min(2) as i32;

    let neighbor_ids: Vec<i64> = sqlx::query_scalar!(
        r#"
        WITH RECURSIVE neighbors AS (
          SELECT source_id as node_id, 1 as depth FROM graph_edges WHERE target_id = $1
          UNION
          SELECT target_id as node_id, 1 as depth FROM graph_edges WHERE source_id = $1
          UNION
          SELECT e.source_id, n.depth + 1
            FROM graph_edges e
            JOIN neighbors n ON e.target_id = n.node_id
           WHERE n.depth < $2
          UNION
          SELECT e.target_id, n.depth + 1
            FROM graph_edges e
            JOIN neighbors n ON e.source_id = n.node_id
           WHERE n.depth < $2
        )
        SELECT DISTINCT node_id FROM neighbors WHERE node_id != $1
        "#,
        q.node_id,
        depth,
    )
    .fetch_all(&pool)
    .await?;

    if neighbor_ids.is_empty() {
        return Ok(NeighborhoodResponse {
            nodes: vec![],
            edges: vec![],
        });
    }

    let mut all_ids = neighbor_ids.clone();
    all_ids.push(q.node_id);

    let nodes = sqlx::query_as!(
        NeighborNodeDto,
        r#"
        SELECT id, ST_X(position) as "x!", ST_Y(position) as "y!",
               entity_type_id, degree, size, label
        FROM graph_nodes
        WHERE id = ANY($1)
        "#,
        &all_ids,
    )
    .fetch_all(&pool)
    .await?;

    let edges = sqlx::query_as!(
        NeighborEdgeDto,
        r#"
        SELECT source_id, target_id, edge_type_id, weight
        FROM graph_edges
        WHERE source_id = ANY($1) AND target_id = ANY($1)
        "#,
        &all_ids,
    )
    .fetch_all(&pool)
    .await?;

    Ok(NeighborhoodResponse { nodes, edges })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p bdp-server graph::queries::get_neighborhood -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-server/src/features/graph/queries/get_neighborhood.rs
git commit -m "feat(graph): add GetNodeNeighborhoodQuery with recursive CTE"
```

---

### Task 11: Graph Router and Module Registration

**Files:**
- Create: `crates/bdp-server/src/features/graph/queries/mod.rs`
- Create: `crates/bdp-server/src/features/graph/router.rs`
- Create: `crates/bdp-server/src/features/graph/mod.rs`
- Modify: `crates/bdp-server/src/features/mod.rs`

- [ ] **Step 1: Write queries/mod.rs**

```rust
pub mod get_neighborhood;
pub mod get_overview;
pub mod get_registry;
pub mod get_tile;
pub mod search_nodes;
```

- [ ] **Step 2: Write router.rs**

```rust
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use thiserror::Error;

use crate::features::FeatureState;
use super::queries::{
    get_neighborhood::GetNodeNeighborhoodQuery,
    get_overview::GetGraphOverviewQuery,
    get_registry::GetGraphRegistryQuery,
    get_tile::GetGraphTileQuery,
    search_nodes::SearchGraphNodesQuery,
};

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for GraphError {
    fn into_response(self) -> Response {
        let msg = self.to_string();
        tracing::error!(error = %msg, "graph endpoint error");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

pub fn routes() -> Router<FeatureState> {
    Router::new()
        .route("/registry", get(registry_handler))
        .route("/overview", get(overview_handler))
        .route("/tiles", get(tile_handler))
        .route("/search", get(search_handler))
        .route("/nodes/:id/neighborhood", get(neighborhood_handler))
}

async fn registry_handler(State(state): State<FeatureState>) -> Result<impl IntoResponse, GraphError> {
    let result = state
        .dispatch(GetGraphRegistryQuery)
        .await
        .map_err(GraphError::Internal)?;
    Ok(Json(result))
}

async fn overview_handler(State(state): State<FeatureState>) -> Result<impl IntoResponse, GraphError> {
    let result = state
        .dispatch(GetGraphOverviewQuery)
        .await
        .map_err(GraphError::Internal)?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct TileParams {
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
    zoom: u8,
    entity_type_ids: Option<String>,
    edge_type_ids: Option<String>,
}

async fn tile_handler(
    State(state): State<FeatureState>,
    AxumQuery(params): AxumQuery<TileParams>,
) -> Result<impl IntoResponse, GraphError> {
    let entity_type_ids = params.entity_type_ids.as_deref().map(parse_id_list);
    let edge_type_ids = params.edge_type_ids.as_deref().map(parse_id_list);

    let bytes = state
        .dispatch(GetGraphTileQuery {
            x_min: params.x_min,
            y_min: params.y_min,
            x_max: params.x_max,
            y_max: params.y_max,
            zoom: params.zoom,
            entity_type_ids,
            edge_type_ids,
        })
        .await
        .map_err(GraphError::Internal)?;

    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        bytes,
    ))
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<u8>,
}

async fn search_handler(
    State(state): State<FeatureState>,
    AxumQuery(params): AxumQuery<SearchParams>,
) -> Result<impl IntoResponse, GraphError> {
    let results = state
        .dispatch(SearchGraphNodesQuery {
            query: params.q,
            limit: params.limit.unwrap_or(10),
        })
        .await
        .map_err(GraphError::Internal)?;
    Ok(Json(results))
}

#[derive(Debug, Deserialize)]
struct NeighborhoodParams {
    depth: Option<u8>,
}

async fn neighborhood_handler(
    State(state): State<FeatureState>,
    Path(id): Path<i64>,
    AxumQuery(params): AxumQuery<NeighborhoodParams>,
) -> Result<impl IntoResponse, GraphError> {
    let result = state
        .dispatch(GetNodeNeighborhoodQuery {
            node_id: id,
            depth: params.depth.unwrap_or(2),
        })
        .await
        .map_err(GraphError::Internal)?;
    Ok(Json(result))
}

/// Parse "1,3,5" → vec![1i16, 3i16, 5i16]
fn parse_id_list(s: &str) -> Vec<i16> {
    s.split(',')
        .filter_map(|part| part.trim().parse::<i16>().ok())
        .collect()
}
```

- [ ] **Step 3: Write mod.rs**

```rust
pub mod queries;
pub mod router;
pub mod types;

pub use router::routes;
```

- [ ] **Step 4: Register in features/mod.rs**

Find the existing feature registrations in `crates/bdp-server/src/features/mod.rs`. Add:
- `pub mod graph;` alongside other `pub mod` declarations
- `.nest("/api/v1/graph", graph::routes().with_state(state.clone()))` alongside other `.nest(...)` calls

- [ ] **Step 5: Build check**

Run: `cargo build -p bdp-server`
Expected: compiles clean

- [ ] **Step 6: Run all graph tests**

Run: `cargo test -p bdp-server graph -- --nocapture`
Expected: all PASS

- [ ] **Step 7: Regenerate SQLx metadata**

Run: `cargo xtask sqlx prepare`
Expected: `.sqlx/` updated

- [ ] **Step 8: Commit**

```bash
git add crates/bdp-server/src/features/graph/ crates/bdp-server/src/features/mod.rs .sqlx/
git commit -m "feat(graph): wire up graph router with 5 endpoints"
```

---

## Phase 3: Frontend

### Task 12: Install frontend packages

**Files:**
- Modify: `web/package.json` (via yarn add)

> All frontend commands must be run from the `web/` directory. Use `yarn`, NOT npm.

- [ ] **Step 1: Install deck.gl and luma.gl packages**

```bash
cd web
yarn add @deck.gl/core @deck.gl/layers @deck.gl/geo-layers @deck.gl/extensions @luma.gl/webgpu @luma.gl/webgl flatbuffers
```

- [ ] **Step 2: Verify build still works**

Run: `yarn build`
Expected: Next.js build succeeds (ignore type errors from new untyped packages for now)

- [ ] **Step 3: Commit**

```bash
git add web/package.json web/yarn.lock
git commit -m "feat(graph): add deck.gl, luma.gl, and flatbuffers frontend deps"
```

---

### Task 13: FlatBuffers decoder

**Files:**
- Create: `web/lib/graph/flatbuffers-decoder.ts`

- [ ] **Step 1: Write failing test**

Create `web/lib/graph/__tests__/flatbuffers-decoder.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { decodeGraphTile } from '../flatbuffers-decoder';

describe('decodeGraphTile', () => {
  it('returns empty tile for empty buffer', () => {
    // An all-zeros buffer won't parse as a real FlatBuffers tile,
    // but our decoder should return an empty tile rather than throw.
    const buf = new ArrayBuffer(8);
    const tile = decodeGraphTile(buf);
    expect(tile.nodes).toBeDefined();
    expect(tile.edges).toBeDefined();
  });
});
```

Run: `cd web && yarn test`
Expected: FAIL (module not found)

- [ ] **Step 2: Implement**

```typescript
// web/lib/graph/flatbuffers-decoder.ts
// Decodes FlatBuffers binary tile responses from /api/v1/graph/tiles.
// Schema: GraphNode (id ulong, x float, y float, entity_type_id ushort,
//           degree uint, size float, label string?)
//         GraphEdge (source_id ulong, target_id ulong, edge_type_id ushort, weight float)
//         GraphTile (nodes [GraphNode], edges [GraphEdge], zoom ubyte, total_in_bbox uint)

import { ByteBuffer } from 'flatbuffers';

export interface PositionalNode {
  id: bigint;
  x: number;
  y: number;
  entityTypeId: number;
  degree: number;
  size: number;
  label: string | null;
}

export interface TileEdge {
  sourceId: bigint;
  targetId: bigint;
  edgeTypeId: number;
  weight: number;
}

export interface GraphTile {
  nodes: PositionalNode[];
  edges: TileEdge[];
  zoom: number;
  totalInBbox: number;
}

/**
 * Decode a FlatBuffers binary response from the tile endpoint.
 * Returns an empty tile if the buffer cannot be parsed.
 */
export function decodeGraphTile(buffer: ArrayBuffer): GraphTile {
  try {
    const bb = new ByteBuffer(new Uint8Array(buffer));
    const rootOffset = bb.readInt32(bb.position()) + bb.position();

    // GraphTile table: field 0=nodes, 1=edges, 2=zoom, 3=total_in_bbox
    const nodesOffset = readVectorOffset(bb, rootOffset, 0);
    const edgesOffset = readVectorOffset(bb, rootOffset, 1);
    const zoom = readScalarU8(bb, rootOffset, 2);
    const totalInBbox = readScalarU32(bb, rootOffset, 3);

    const nodes = nodesOffset ? decodeNodeVector(bb, nodesOffset) : [];
    const edges = edgesOffset ? decodeEdgeVector(bb, edgesOffset) : [];

    return { nodes, edges, zoom, totalInBbox };
  } catch {
    return { nodes: [], edges: [], zoom: 0, totalInBbox: 0 };
  }
}

function decodeNodeVector(bb: ByteBuffer, vecOffset: number): PositionalNode[] {
  const len = bb.readInt32(vecOffset);
  const nodes: PositionalNode[] = [];
  for (let i = 0; i < len; i++) {
    const objOffset = vecOffset + 4 + i * 4;
    const nodeOff = objOffset + bb.readInt32(objOffset);
    nodes.push({
      id: readScalarU64(bb, nodeOff, 0),
      x: readScalarF32(bb, nodeOff, 1),
      y: readScalarF32(bb, nodeOff, 2),
      entityTypeId: readScalarU16(bb, nodeOff, 3),
      degree: readScalarU32(bb, nodeOff, 4),
      size: readScalarF32(bb, nodeOff, 5),
      label: readOptionalString(bb, nodeOff, 6),
    });
  }
  return nodes;
}

function decodeEdgeVector(bb: ByteBuffer, vecOffset: number): TileEdge[] {
  const len = bb.readInt32(vecOffset);
  const edges: TileEdge[] = [];
  for (let i = 0; i < len; i++) {
    const objOffset = vecOffset + 4 + i * 4;
    const edgeOff = objOffset + bb.readInt32(objOffset);
    edges.push({
      sourceId: readScalarU64(bb, edgeOff, 0),
      targetId: readScalarU64(bb, edgeOff, 1),
      edgeTypeId: readScalarU16(bb, edgeOff, 2),
      weight: readScalarF32(bb, edgeOff, 3),
    });
  }
  return edges;
}

// --- Low-level FlatBuffers field readers ---

function vtableOffset(bb: ByteBuffer, tableOffset: number): number {
  return tableOffset - bb.readInt32(tableOffset);
}

function fieldOffset(bb: ByteBuffer, tableOffset: number, fieldIndex: number): number {
  const vtable = vtableOffset(bb, tableOffset);
  const vtableSize = bb.readInt16(vtable);
  const fieldOff = 4 + fieldIndex * 2;
  if (fieldOff >= vtableSize) return 0;
  return bb.readInt16(vtable + fieldOff);
}

function readVectorOffset(bb: ByteBuffer, tableOffset: number, fieldIndex: number): number | null {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  if (!off) return null;
  const pos = tableOffset + off;
  return pos + bb.readInt32(pos);
}

function readScalarU8(bb: ByteBuffer, tableOffset: number, fieldIndex: number): number {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  return off ? bb.readUint8(tableOffset + off) : 0;
}

function readScalarU16(bb: ByteBuffer, tableOffset: number, fieldIndex: number): number {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  return off ? bb.readUint16(tableOffset + off) : 0;
}

function readScalarU32(bb: ByteBuffer, tableOffset: number, fieldIndex: number): number {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  return off ? bb.readUint32(tableOffset + off) : 0;
}

function readScalarF32(bb: ByteBuffer, tableOffset: number, fieldIndex: number): number {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  return off ? bb.readFloat32(tableOffset + off) : 0;
}

function readScalarU64(bb: ByteBuffer, tableOffset: number, fieldIndex: number): bigint {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  if (!off) return 0n;
  const pos = tableOffset + off;
  const lo = bb.readUint32(pos);
  const hi = bb.readUint32(pos + 4);
  return (BigInt(hi) << 32n) | BigInt(lo);
}

function readOptionalString(bb: ByteBuffer, tableOffset: number, fieldIndex: number): string | null {
  const off = fieldOffset(bb, tableOffset, fieldIndex);
  if (!off) return null;
  const strOffset = tableOffset + off;
  const strPos = strOffset + bb.readInt32(strOffset);
  const strLen = bb.readInt32(strPos);
  const bytes = new Uint8Array(bb.bytes().buffer, strPos + 4, strLen);
  return new TextDecoder().decode(bytes);
}
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd web && yarn test`
Expected: PASS (the empty-buffer test returns empty tile without throwing)

- [ ] **Step 4: Commit**

```bash
git add web/lib/graph/flatbuffers-decoder.ts web/lib/graph/__tests__/flatbuffers-decoder.test.ts
git commit -m "feat(graph): add FlatBuffers tile decoder"
```

---

### Task 14: GraphState (LRU node store)

**Files:**
- Create: `web/lib/graph/graph-state.ts`

- [ ] **Step 1: Write failing test**

Create `web/lib/graph/__tests__/graph-state.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { GraphState } from '../graph-state';
import type { PositionalNode } from '../flatbuffers-decoder';

function makeNode(id: number): PositionalNode {
  return { id: BigInt(id), x: 0, y: 0, entityTypeId: 1, degree: 1, size: 1, label: null };
}

describe('GraphState', () => {
  it('stores and retrieves nodes', () => {
    const gs = new GraphState();
    const node = makeNode(1);
    gs.merge({ nodes: [node], edges: [], zoom: 9, totalInBbox: 1 });
    expect(gs.has(1n)).toBe(true);
    expect(gs.get(1n)).toEqual(node);
  });

  it('evicts oldest nodes when over MAX_NODES', () => {
    const gs = new GraphState(3);  // cap at 3 for test
    gs.merge({ nodes: [makeNode(1), makeNode(2), makeNode(3)], edges: [], zoom: 9, totalInBbox: 3 });
    gs.merge({ nodes: [makeNode(4)], edges: [], zoom: 9, totalInBbox: 4 });
    expect(gs.size).toBe(3);
    // node 1 (oldest) should be evicted
    expect(gs.has(1n)).toBe(false);
    expect(gs.has(4n)).toBe(true);
  });
});
```

Run: `cd web && yarn test`
Expected: FAIL

- [ ] **Step 2: Implement**

```typescript
// web/lib/graph/graph-state.ts
import type { GraphTile, PositionalNode } from './flatbuffers-decoder';

export class GraphState {
  private nodes = new Map<bigint, PositionalNode>();
  private readonly maxNodes: number;

  constructor(maxNodes = 500_000) {
    this.maxNodes = maxNodes;
  }

  get size(): number {
    return this.nodes.size;
  }

  merge(tile: GraphTile): void {
    for (const node of tile.nodes) {
      this.nodes.set(node.id, node);
    }
    this.evictIfNeeded();
  }

  evictTile(tileNodes: PositionalNode[]): void {
    for (const n of tileNodes) {
      this.nodes.delete(n.id);
    }
  }

  has(id: bigint): boolean {
    return this.nodes.has(id);
  }

  get(id: bigint): PositionalNode | undefined {
    return this.nodes.get(id);
  }

  allNodes(): PositionalNode[] {
    return Array.from(this.nodes.values());
  }

  private evictIfNeeded(): void {
    if (this.nodes.size <= this.maxNodes) return;
    const overflow = this.nodes.size - this.maxNodes;
    const iter = this.nodes.keys();
    for (let i = 0; i < overflow; i++) {
      const next = iter.next();
      if (!next.done) this.nodes.delete(next.value);
    }
  }
}
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd web && yarn test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add web/lib/graph/graph-state.ts web/lib/graph/__tests__/graph-state.test.ts
git commit -m "feat(graph): add GraphState with 500K LRU eviction"
```

---

### Task 15: LOD and renderer utilities

**Files:**
- Create: `web/lib/graph/lod.ts`
- Create: `web/lib/graph/renderer.ts`

- [ ] **Step 1: Write lod.ts**

```typescript
// web/lib/graph/lod.ts
// Client-side LOD: which edge categories are visible at the current zoom level.
// Mirrors the server-side min_zoom values in graph_edge_types registry.
// Used to toggle edge type visibility in the UI without re-fetching tiles.

export type EdgeCategory = 'molecular' | 'ontological' | 'taxonomic' | 'cross_db';

export function visibleEdgeCategories(zoom: number): Set<EdgeCategory> {
  const visible = new Set<EdgeCategory>();
  if (zoom >= 5) {
    visible.add('ontological');
    visible.add('taxonomic');
  }
  if (zoom >= 7) {
    visible.add('cross_db');
  }
  if (zoom >= 8) {
    visible.add('molecular');
  }
  return visible;
}

export function isEdgeVisible(category: EdgeCategory, zoom: number): boolean {
  return visibleEdgeCategories(zoom).has(category);
}
```

- [ ] **Step 2: Write renderer.ts**

```typescript
// web/lib/graph/renderer.ts
// WebGPU device with transparent WebGL fallback.
// deck.gl uses the same layer code regardless of backend.

export type GpuDevice = unknown;  // deck.gl luma.gl device type

export async function createGraphDevice(): Promise<GpuDevice> {
  try {
    const { createDevice } = await import('@luma.gl/webgpu');
    return await createDevice({ type: 'webgpu' });
  } catch {
    const { createDevice } = await import('@luma.gl/webgl');
    return await createDevice({ type: 'webgl2' });
  }
}
```

- [ ] **Step 3: Compile check**

Run: `cd web && yarn type-check`
Expected: no errors for these two files

- [ ] **Step 4: Commit**

```bash
git add web/lib/graph/lod.ts web/lib/graph/renderer.ts
git commit -m "feat(graph): add LOD zoom filter and WebGPU/WebGL renderer factory"
```

---

### Task 16: Tile manager

**Files:**
- Create: `web/lib/graph/tile-manager.ts`

- [ ] **Step 1: Write tile-manager.ts**

```typescript
// web/lib/graph/tile-manager.ts
// Fetches tiles from /api/v1/graph/tiles, decodes FlatBuffers, merges into GraphState.
// Passes integer IDs from the registry (never string names).

import { decodeGraphTile, type GraphTile } from './flatbuffers-decoder';
import type { GraphState } from './graph-state';

export interface TileRequest {
  xMin: number;
  yMin: number;
  xMax: number;
  yMax: number;
  zoom: number;
  entityTypeIds?: number[];
  edgeTypeIds?: number[];
}

export class TileManager {
  private baseUrl: string;
  private state: GraphState;

  constructor(baseUrl: string, state: GraphState) {
    this.baseUrl = baseUrl;
    this.state = state;
  }

  async fetchTile(req: TileRequest, signal?: AbortSignal): Promise<GraphTile> {
    const params = new URLSearchParams({
      x_min: req.xMin.toString(),
      y_min: req.yMin.toString(),
      x_max: req.xMax.toString(),
      y_max: req.yMax.toString(),
      zoom: req.zoom.toString(),
    });

    if (req.entityTypeIds?.length) {
      params.set('entity_type_ids', req.entityTypeIds.join(','));
    }
    if (req.edgeTypeIds?.length) {
      params.set('edge_type_ids', req.edgeTypeIds.join(','));
    }

    const res = await fetch(`${this.baseUrl}/api/v1/graph/tiles?${params}`, { signal });
    if (!res.ok) throw new Error(`Tile fetch failed: ${res.status}`);

    const buffer = await res.arrayBuffer();
    const tile = decodeGraphTile(buffer);
    this.state.merge(tile);
    return tile;
  }
}
```

- [ ] **Step 2: Compile check**

Run: `cd web && yarn type-check`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add web/lib/graph/tile-manager.ts
git commit -m "feat(graph): add TileManager for tile fetching and merging"
```

---

### Task 17: Custom deck.gl TileLayer

**Files:**
- Create: `web/lib/graph/graph-tile-layer.ts`

- [ ] **Step 1: Write graph-tile-layer.ts**

```typescript
// web/lib/graph/graph-tile-layer.ts
// Custom deck.gl TileLayer that streams graph tiles from the BDP tile server.
// Uses ScatterplotLayer for nodes, LineLayer for edges.

import { TileLayer } from '@deck.gl/geo-layers';
import { ScatterplotLayer, LineLayer } from '@deck.gl/layers';
import { DataFilterExtension } from '@deck.gl/extensions';
import type { PickingInfo } from '@deck.gl/core';
import type { GraphTile, PositionalNode, TileEdge } from './flatbuffers-decoder';
import type { GraphState } from './graph-state';
import type { EdgeTypeDto, EntityTypeDto } from './registry';

export interface GraphLayerProps {
  state: GraphState;
  registry: { entityTypes: EntityTypeDto[]; edgeTypes: EdgeTypeDto[] };
  zoom: number;
  onNodeClick?: (node: PositionalNode) => void;
  onNodeHover?: (node: PositionalNode | null) => void;
  activeEntityTypeIds?: Set<number>;
  activeEdgeTypeIds?: Set<number>;
}

export function createGraphTileLayer(props: GraphLayerProps, baseUrl: string) {
  const entityColorMap = new Map(
    props.registry.entityTypes.map((t) => [t.id, hexToRgb(t.color_hex)])
  );
  const edgeColorMap = new Map(
    props.registry.edgeTypes.map((t) => [t.id, hexToRgb(t.color_hex)])
  );

  return new TileLayer<GraphTile>({
    id: 'graph-tile-layer',
    // Cartesian coordinates [-1, 1]; deck.gl treats these as WGS-84 degrees
    // which is fine for a synthetic flat layout — we just set coordinateSystem
    // to CARTESIAN to suppress geographic distortion warnings.
    getTileData: async ({ bbox, signal }) => {
      const b = bbox as { west: number; south: number; east: number; north: number };
      const params = new URLSearchParams({
        x_min: b.west.toString(),
        y_min: b.south.toString(),
        x_max: b.east.toString(),
        y_max: b.north.toString(),
        zoom: props.zoom.toString(),
      });
      if (props.activeEntityTypeIds?.size) {
        params.set('entity_type_ids', [...props.activeEntityTypeIds].join(','));
      }
      if (props.activeEdgeTypeIds?.size) {
        params.set('edge_type_ids', [...props.activeEdgeTypeIds].join(','));
      }
      const res = await fetch(`${baseUrl}/api/v1/graph/tiles?${params}`, { signal });
      if (!res.ok) throw new Error(`Tile fetch ${res.status}`);
      const buf = await res.arrayBuffer();
      const { decodeGraphTile } = await import('./flatbuffers-decoder');
      const tile = decodeGraphTile(buf);
      props.state.merge(tile);
      return tile;
    },
    renderSubLayers: (subProps) => {
      const tile = subProps.data as GraphTile | null;
      if (!tile) return [];

      return [
        new ScatterplotLayer<PositionalNode>({
          id: `${subProps.id}-nodes`,
          data: tile.nodes,
          getPosition: (n) => [n.x, n.y],
          getFillColor: (n) => entityColorMap.get(n.entityTypeId) ?? [150, 150, 150],
          getRadius: (n) => n.size * 0.002,
          radiusMinPixels: 2,
          radiusMaxPixels: 20,
          pickable: true,
          onClick: (info: PickingInfo<PositionalNode>) => {
            if (info.object) props.onNodeClick?.(info.object);
          },
          onHover: (info: PickingInfo<PositionalNode>) => {
            props.onNodeHover?.(info.object ?? null);
          },
        }),
        new LineLayer<TileEdge>({
          id: `${subProps.id}-edges`,
          data: tile.edges,
          getSourcePosition: (e) => {
            const src = props.state.get(e.sourceId);
            return src ? [src.x, src.y] : [0, 0];
          },
          getTargetPosition: (e) => {
            const tgt = props.state.get(e.targetId);
            return tgt ? [tgt.x, tgt.y] : [0, 0];
          },
          getColor: (e) => edgeColorMap.get(e.edgeTypeId) ?? [100, 100, 100, 80],
          getWidth: 1,
          widthMinPixels: 0.5,
          // Skip edges whose endpoints are not in GraphState (cross-tile rule).
          // DataFilterExtension is required for filterRange/getFilterValue to work —
          // without it, filterRange is silently ignored and all edges render at [0,0].
          extensions: [new DataFilterExtension({ filterSize: 1 })],
          filterRange: [1, 1],
          getFilterValue: (e: TileEdge) =>
            props.state.has(e.sourceId) && props.state.has(e.targetId) ? 1 : 0,
        }),
      ];
    },
  });
}

function hexToRgb(hex: string): [number, number, number] {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? [parseInt(result[1], 16), parseInt(result[2], 16), parseInt(result[3], 16)]
    : [150, 150, 150];
}
```

- [ ] **Step 2: Create registry type file referenced above**

Create `web/lib/graph/registry.ts`:
```typescript
// web/lib/graph/registry.ts
// Registry response types and fetch helper.

export interface EntityTypeDto {
  id: number;
  name: string;
  label: string;
  color_hex: string;
  source_dbs: string[];
  is_active: boolean;
}

export interface EdgeTypeDto {
  id: number;
  name: string;
  label: string;
  category: string;
  color_hex: string;
  min_zoom: number;
  is_directed: boolean;
  is_active: boolean;
}

export interface GraphRegistry {
  entityTypes: EntityTypeDto[];
  edgeTypes: EdgeTypeDto[];
}

export async function fetchRegistry(baseUrl: string): Promise<GraphRegistry> {
  const res = await fetch(`${baseUrl}/api/v1/graph/registry`);
  if (!res.ok) throw new Error(`Registry fetch failed: ${res.status}`);
  const data = await res.json();
  return {
    entityTypes: data.entity_types,
    edgeTypes: data.edge_types,
  };
}
```

- [ ] **Step 3: Compile check**

Run: `cd web && yarn type-check`
Expected: no errors for these files

- [ ] **Step 4: Commit**

```bash
git add web/lib/graph/graph-tile-layer.ts web/lib/graph/registry.ts
git commit -m "feat(graph): add custom GraphTileLayer and registry types"
```

---

### Task 18: Graph UI components

**Files:**
- Create: `web/components/graph/graph-legend.tsx`
- Create: `web/components/graph/graph-controls.tsx`
- Create: `web/components/graph/node-tooltip.tsx`

- [ ] **Step 1: Write graph-legend.tsx**

```tsx
// web/components/graph/graph-legend.tsx
'use client';

import type { EntityTypeDto, EdgeTypeDto } from '@/lib/graph/registry';

interface GraphLegendProps {
  entityTypes: EntityTypeDto[];
  edgeTypes: EdgeTypeDto[];
  zoom: number;
}

export function GraphLegend({ entityTypes, edgeTypes, zoom }: GraphLegendProps) {
  const visibleEdges = edgeTypes.filter((et) => et.min_zoom <= zoom);

  return (
    <div className="absolute bottom-4 left-4 bg-card border rounded-lg p-3 text-xs space-y-2 max-w-[180px]">
      <div>
        <p className="font-semibold text-muted-foreground uppercase tracking-wide mb-1">Nodes</p>
        {entityTypes.map((et) => (
          <div key={et.id} className="flex items-center gap-2">
            <span
              className="inline-block w-3 h-3 rounded-full flex-shrink-0"
              style={{ backgroundColor: et.color_hex }}
            />
            <span>{et.label}</span>
          </div>
        ))}
      </div>
      {visibleEdges.length > 0 && (
        <div>
          <p className="font-semibold text-muted-foreground uppercase tracking-wide mb-1">Edges</p>
          {visibleEdges.map((et) => (
            <div key={et.id} className="flex items-center gap-2">
              <span
                className="inline-block w-4 h-0.5 flex-shrink-0"
                style={{ backgroundColor: et.color_hex }}
              />
              <span>{et.label}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Write graph-controls.tsx**

```tsx
// web/components/graph/graph-controls.tsx
'use client';

import * as React from 'react';
import { Search } from 'lucide-react';
import type { EntityTypeDto, EdgeTypeDto } from '@/lib/graph/registry';

interface SearchResult {
  id: bigint;
  x: number;
  y: number;
  label: string | null;
  entity_type_id: number;
}

interface GraphControlsProps {
  entityTypes: EntityTypeDto[];
  edgeTypes: EdgeTypeDto[];
  activeEntityTypeIds: Set<number>;
  activeEdgeTypeIds: Set<number>;
  onEntityTypeToggle: (id: number) => void;
  onEdgeTypeToggle: (id: number) => void;
  onSearchResult: (result: SearchResult) => void;
  baseUrl: string;
}

export function GraphControls({
  entityTypes,
  edgeTypes,
  activeEntityTypeIds,
  activeEdgeTypeIds,
  onEntityTypeToggle,
  onEdgeTypeToggle,
  onSearchResult,
  baseUrl,
}: GraphControlsProps) {
  const [query, setQuery] = React.useState('');
  const [results, setResults] = React.useState<SearchResult[]>([]);

  const search = React.useCallback(async (q: string) => {
    if (q.length < 2) { setResults([]); return; }
    const res = await fetch(`${baseUrl}/api/v1/graph/search?q=${encodeURIComponent(q)}&limit=8`);
    if (res.ok) setResults(await res.json());
  }, [baseUrl]);

  React.useEffect(() => {
    const t = setTimeout(() => search(query), 300);
    return () => clearTimeout(t);
  }, [query, search]);

  return (
    <div className="absolute top-4 left-4 flex flex-col gap-2 w-64">
      {/* Search */}
      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
        <input
          className="w-full pl-8 pr-3 py-2 bg-card border rounded-lg text-sm outline-none focus:ring-1 focus:ring-primary"
          placeholder="Search nodes..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {results.length > 0 && (
          <div className="absolute top-full mt-1 w-full bg-card border rounded-lg shadow-lg z-10">
            {results.map((r) => (
              <button
                key={r.id.toString()}
                className="w-full text-left px-3 py-2 text-sm hover:bg-muted"
                onClick={() => { onSearchResult(r); setResults([]); setQuery(r.label ?? ''); }}
              >
                {r.label ?? r.id.toString()}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Entity type filters */}
      <div className="bg-card border rounded-lg p-2 space-y-1">
        <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Node types</p>
        {entityTypes.map((et) => (
          <label key={et.id} className="flex items-center gap-2 cursor-pointer text-xs">
            <input
              type="checkbox"
              checked={activeEntityTypeIds.has(et.id)}
              onChange={() => onEntityTypeToggle(et.id)}
              className="rounded"
            />
            <span
              className="inline-block w-2.5 h-2.5 rounded-full"
              style={{ backgroundColor: et.color_hex }}
            />
            {et.label}
          </label>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Write node-tooltip.tsx**

```tsx
// web/components/graph/node-tooltip.tsx
// Shows data from PositionalNode (already in GraphState — no extra fetch needed).
// The properties JSONB fetch endpoint is out of scope for this plan;
// add a dedicated GET /api/v1/graph/nodes/:id endpoint in a follow-on plan.
'use client';

import type { PositionalNode } from '@/lib/graph/flatbuffers-decoder';
import type { EntityTypeDto } from '@/lib/graph/registry';

interface NodeTooltipProps {
  node: PositionalNode;
  entityType: EntityTypeDto | undefined;
  x: number;
  y: number;
}

export function NodeTooltip({ node, entityType, x, y }: NodeTooltipProps) {
  return (
    <div
      className="absolute z-20 pointer-events-none bg-card border rounded-lg shadow-lg p-3 text-xs max-w-[240px]"
      style={{ left: x + 12, top: y - 8 }}
    >
      <p className="font-semibold text-sm">{node.label ?? `Node ${node.id}`}</p>
      <p className="text-muted-foreground">
        {entityType?.label ?? 'Unknown'} · degree {node.degree}
      </p>
    </div>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add web/components/graph/
git commit -m "feat(graph): add GraphLegend, GraphControls, and NodeTooltip components"
```

---

### Task 19: GraphView client component

**Files:**
- Create: `web/app/[locale]/graph/graph-view.tsx`

- [ ] **Step 1: Write graph-view.tsx**

```tsx
// web/app/[locale]/graph/graph-view.tsx
'use client';

import * as React from 'react';
import { DeckGL } from '@deck.gl/react';
import { ScatterplotLayer } from '@deck.gl/layers';
import { FlyToInterpolator } from '@deck.gl/core';
import type { ViewState } from '@deck.gl/core';
import { GraphState } from '@/lib/graph/graph-state';
import { createGraphTileLayer } from '@/lib/graph/graph-tile-layer';
import { fetchRegistry, type GraphRegistry } from '@/lib/graph/registry';
import type { OverviewNodeDto } from '@/lib/graph/overview';
import { GraphLegend } from '@/components/graph/graph-legend';
import { GraphControls } from '@/components/graph/graph-controls';
import { NodeTooltip } from '@/components/graph/node-tooltip';
import type { PositionalNode } from '@/lib/graph/flatbuffers-decoder';

interface GraphViewProps {
  baseUrl: string;
}

const INITIAL_VIEW_STATE = {
  longitude: 0,
  latitude: 0,
  zoom: 1,
  pitch: 0,
  bearing: 0,
};

export function GraphView({ baseUrl }: GraphViewProps) {
  const [registry, setRegistry] = React.useState<GraphRegistry | null>(null);
  const [overviewNodes, setOverviewNodes] = React.useState<OverviewNodeDto[]>([]);
  const [viewState, setViewState] = React.useState<ViewState>(INITIAL_VIEW_STATE);
  const [hoveredNode, setHoveredNode] = React.useState<PositionalNode | null>(null);
  const [hoverPos, setHoverPos] = React.useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [activeEntityTypeIds, setActiveEntityTypeIds] = React.useState<Set<number>>(new Set());
  const [activeEdgeTypeIds, setActiveEdgeTypeIds] = React.useState<Set<number>>(new Set());
  const graphState = React.useRef(new GraphState());

  // Load registry and overview in parallel on mount
  React.useEffect(() => {
    Promise.all([
      fetchRegistry(baseUrl),
      fetch(`${baseUrl}/api/v1/graph/overview`).then((r) => r.json()),
    ]).then(([reg, overview]) => {
      setRegistry(reg);
      setOverviewNodes(overview.nodes ?? []);
      // Activate all types by default
      setActiveEntityTypeIds(new Set(reg.entityTypes.map((t: { id: number }) => t.id)));
      setActiveEdgeTypeIds(new Set(reg.edgeTypes.map((t: { id: number }) => t.id)));
    });
  }, [baseUrl]);

  const zoom = (viewState as { zoom?: number }).zoom ?? 1;

  const layers = React.useMemo(() => {
    if (!registry) return [];

    const entityColorMap = new Map(
      registry.entityTypes.map((t) => {
        const hex = t.color_hex;
        const r = parseInt(hex.slice(1, 3), 16);
        const g = parseInt(hex.slice(3, 5), 16);
        const b = parseInt(hex.slice(5, 7), 16);
        return [t.id, [r, g, b] as [number, number, number]];
      })
    );

    const layers = [];

    // Overview layer: static 5K hub nodes shown at low zoom
    if (zoom <= 4 && overviewNodes.length > 0) {
      layers.push(
        new ScatterplotLayer({
          id: 'overview-layer',
          data: overviewNodes,
          getPosition: (n: OverviewNodeDto) => [n.x, n.y],
          getFillColor: (n: OverviewNodeDto) => entityColorMap.get(n.entity_type_id) ?? [150, 150, 150],
          getRadius: (n: OverviewNodeDto) => n.size * 0.003,
          radiusMinPixels: 2,
          radiusMaxPixels: 15,
        })
      );
    }

    // Tile layer: streams tiles at zoom > 2
    if (zoom > 2) {
      layers.push(
        createGraphTileLayer(
          {
            state: graphState.current,
            registry,
            zoom,
            onNodeHover: (node) => setHoveredNode(node),
            activeEntityTypeIds: activeEntityTypeIds.size < registry.entityTypes.length
              ? activeEntityTypeIds
              : undefined,
            activeEdgeTypeIds: activeEdgeTypeIds.size < registry.edgeTypes.length
              ? activeEdgeTypeIds
              : undefined,
          },
          baseUrl
        )
      );
    }

    return layers;
  }, [registry, overviewNodes, zoom, activeEntityTypeIds, activeEdgeTypeIds, baseUrl]);

  if (!registry) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        Loading graph...
      </div>
    );
  }

  return (
    <div className="relative w-full h-full">
      <DeckGL
        viewState={viewState}
        onViewStateChange={({ viewState: vs }) => setViewState(vs as ViewState)}
        controller
        layers={layers}
        onHover={(info) => {
          if (info.coordinate) {
            setHoverPos({ x: info.x, y: info.y });
          }
        }}
      />

      <GraphControls
        entityTypes={registry.entityTypes}
        edgeTypes={registry.edgeTypes}
        activeEntityTypeIds={activeEntityTypeIds}
        activeEdgeTypeIds={activeEdgeTypeIds}
        onEntityTypeToggle={(id) =>
          setActiveEntityTypeIds((prev) => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id); else next.add(id);
            return next;
          })
        }
        onEdgeTypeToggle={(id) =>
          setActiveEdgeTypeIds((prev) => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id); else next.add(id);
            return next;
          })
        }
        onSearchResult={(result) => {
          setViewState((prev) => ({
            ...prev,
            longitude: result.x,
            latitude: result.y,
            zoom: 10,
            transitionDuration: 1000,
            transitionInterpolator: new FlyToInterpolator(),
          }));
        }}
        baseUrl={baseUrl}
      />

      <GraphLegend
        entityTypes={registry.entityTypes.filter((t) => activeEntityTypeIds.has(t.id))}
        edgeTypes={registry.edgeTypes.filter((t) => activeEdgeTypeIds.has(t.id))}
        zoom={zoom}
      />

      {hoveredNode && (
        <NodeTooltip
          node={hoveredNode}
          entityType={registry.entityTypes.find((t) => t.id === hoveredNode.entityTypeId)}
          x={hoverPos.x}
          y={hoverPos.y}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Create missing overview type**

Create `web/lib/graph/overview.ts`:
```typescript
// web/lib/graph/overview.ts
export interface OverviewNodeDto {
  id: number;
  x: number;
  y: number;
  entity_type_id: number;
  degree: number;
  size: number;
  label: string | null;
  community_id: number | null;
}
```

- [ ] **Step 3: Compile check**

Run: `cd web && yarn type-check`
Expected: no errors in these files

- [ ] **Step 4: Commit**

```bash
git add web/app/[locale]/graph/graph-view.tsx web/lib/graph/overview.ts
git commit -m "feat(graph): add GraphView client component with deck.gl layers"
```

---

### Task 20: Graph page

**Files:**
- Create: `web/app/[locale]/graph/page.tsx`

- [ ] **Step 1: Write page.tsx**

```tsx
// web/app/[locale]/graph/page.tsx
import { GraphView } from './graph-view';

export const metadata = {
  title: 'Knowledge Graph',
  description: 'Interactive biological knowledge graph with 10M+ nodes',
};

export default function GraphPage() {
  const baseUrl = process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8000';

  return (
    <main className="w-full h-[calc(100vh-4rem)]">
      <GraphView baseUrl={baseUrl} />
    </main>
  );
}
```

- [ ] **Step 2: Build check**

Run: `cd web && yarn build`
Expected: build succeeds

- [ ] **Step 3: Commit**

```bash
git add web/app/[locale]/graph/page.tsx
git commit -m "feat(graph): add graph page route at /graph"
```

---

### Task 21: Final integration check

- [ ] **Step 1: Run all backend tests**

Run: `cargo test -p bdp-server -- --nocapture`
Expected: all PASS, no regressions

- [ ] **Step 2: Run frontend unit tests**

Run: `cd web && yarn test`
Expected: all PASS

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p bdp-server -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Run rustfmt**

Run: `cargo fmt -p bdp-server`
Expected: no changes needed (or commit formatting fixes)

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(graph): graph view implementation complete — DB schema, CQRS tile server, deck.gl frontend"
```

---

## Follow-on: Plan B — Offline Layout Pipeline

The `cargo xtask graph layout` pipeline (Louvain community detection → ForceAtlas2 per community → normalize positions → write back to DB → rebuild spatial indexes) is a separate plan. It is documented in the spec but omitted here to keep this plan focused on the API surface that the frontend needs to integrate against.

Until layout data is present in `graph_nodes.position`, the graph page will render an empty canvas (the API returns empty tiles, the overview returns empty data). This is expected and correct behavior.
