# Vectors Feature — Test Suite Design

**Date**: 2026-03-22
**Branch**: `feature/vectors`
**Scope**: All test layers for the pgvector embeddings, `/vectors` page, and `bdp-embed` Python CLI

---

## Overview

Four test layers, all runnable locally with Docker (testcontainers):

| Layer | Tool | Target |
|-------|------|--------|
| Rust unit/integration | `#[sqlx::test]` | Query handlers |
| Rust E2E | testcontainers (Postgres + MinIO) + axum in-process | HTTP endpoints |
| Python unit | pytest | embed_text, tiles, project |
| Frontend unit | Vitest + jsdom | source-type-colors, tile-loader |

---

## Layer 1: Rust `#[sqlx::test]` Tests

All tests use `sqlx::query` (non-macro) for test data insertion to avoid requiring offline metadata.

### `get_stats.rs` — add 2 tests alongside the existing one

**`test_stats_counts_registry_entries`**
- Insert 1 org + 3 `registry_entries` using `sqlx::query`
- Call `handle(pool, GetVectorStatsQuery)`
- Assert: `entry_count == Some(3)`, `current_run_id.is_none()`, `embedded_count == Some(0)`

**`test_stats_with_complete_run`**
- Insert 1 org + 3 entries + 2 rows in `entry_embeddings` (any vector values) + 1 `vector_projection_runs` row with `status='complete'`, `entry_count=3`, `embedded_count=2`, `projected_count=1`, `tile_prefix='vectors/tiles/run123'`
- Call `handle(pool, GetVectorStatsQuery)`
- Assert all 7 `VectorStatsResponse` fields are non-null and match inserted values

### `semantic_search.rs` — add 1 `#[sqlx::test]`

The existing 3 tests cover `validate()`. This test covers the handler's OpenAI path:

**`test_semantic_search_embedding_unavailable_without_api_key`**
- `std::env::remove_var("OPENAI_API_KEY")` before calling
- Call `handle(pool, SemanticSearchQuery { q: "ribosome".into(), k: 10 })`
- Assert: `Err(SemanticSearchError::EmbeddingUnavailable(_))`
- Restore env var after test (use `temp_env` crate or `defer` pattern)

### `get_neighbors.rs` — add 2 `#[sqlx::test]`

The existing test covers `validate_k`.

**`test_get_neighbors_not_found_for_unknown_entry`**
- Call `handle(pool, GetNeighborsQuery { entry_id: Uuid::new_v4(), k: 5 })` on empty DB
- Assert: `Err(GetNeighborsError::NotFound)`

**`test_get_neighbors_returns_knn_ordered_by_similarity`**
- Insert 1 org + 4 `registry_entries` with UUIDs `[e0, e1, e2, e3]`
- Insert `entry_embeddings` for all 4 entries:
  - `e0`: seed vector — unit vector along dim 0: `[1.0, 0.0, ..., 0.0]`
  - `e1`: near neighbor — `[0.95, 0.05, ..., 0.0]` (high cosine similarity to e0)
  - `e2`: medium neighbor — `[0.5, 0.5, ..., 0.0]`
  - `e3`: far neighbor — `[0.0, 1.0, 0.0, ..., 0.0]` (orthogonal to e0)
  - All vectors must be normalized to unit length before insertion
- Call `handle(pool, GetNeighborsQuery { entry_id: e0, k: 3 })`
- Assert: `neighbors.len() == 3`
- Assert: neighbors do NOT include `e0` (self excluded)
- Assert: `neighbors[0].entry_id == e1` (most similar first)
- Assert: all similarity values are in `(0.0, 1.0]`

### `get_tile.rs`

No additional tests needed — the handler has no DB interaction, and Storage mocking requires E2E. Covered in Layer 2.

---

## Layer 2: Rust E2E Tests

**File**: `crates/bdp-server/tests/e2e/vectors_tests.rs`

**Approach**: Spin up Postgres + MinIO via testcontainers, start axum app in-process using the existing `bdp_server` app builder (avoids needing a Docker image). Use `reqwest::Client` for HTTP calls.

**Setup pattern** (per test):
```rust
#[tokio::test]
#[serial]
async fn test_name() -> Result<()> {
    let pg = Postgres::default().start().await?;
    let db_url = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", pg.get_host_port_ipv4(5432).await?);
    // run migrations
    // start minio
    // build app and bind to random port
    // run assertions via reqwest
}
```

### Test cases

**`test_vectors_stats_empty`**
- Fresh DB with migrations applied, no data
- `GET /api/v1/vectors/stats`
- Assert: HTTP 200, `data.current_run_id == null`, `data.entry_count == 0`

**`test_vectors_tile_not_found`**
- `GET /api/v1/vectors/tiles/nonexistent-run-id/0/0/0`
- Assert: HTTP 404

**`test_vectors_search_returns_503_without_api_key`**
- Ensure `OPENAI_API_KEY` is unset for the process
- `GET /api/v1/vectors/search?q=ribosome`
- Assert: HTTP 503

**`test_vectors_neighbors_returns_404_for_missing_entry`**
- `GET /api/v1/vectors/00000000-0000-0000-0000-000000000000/neighbors`
- Assert: HTTP 404 (entry has no embedding → NotFound)

---

## Layer 3: Python pytest

### `tools/bdp-embed/tests/test_embed_text.py` — add 3 tests

**`test_all_source_types_produce_non_empty_output`**
- Define a rich entry dict: `{"name": "X", "description": "Y", "organism": "E. coli"}`
- Call `build_embed_text(entry, source_type)` for all 12 source types
- Assert each result is a non-empty string with no leading/trailing whitespace

**`test_no_double_spaces_when_fields_empty`**
- Call `build_embed_text({"name": "X"}, "protein")` (most fields absent)
- Assert `"  "` not in result (no consecutive spaces from missing joins)

**`test_pathway_gene_list_truncated_at_20`**
- Entry with `gene_list` of 50 items
- Assert `"gene19" in result` and `"gene20" not in result`
- (Already tested, keep or extend to check exact boundary `gene19`/`gene20`)

### `tools/bdp-embed/tests/test_tiles.py` — add 2 tests

**`test_lod_z8_keeps_all_points`**
- Create 200 points spread across a 10×10 grid
- Build quadtree with `zoom_max=8`
- At zoom level 8, each tile's `points` list equals all points that fall in that cell (no downsampling)
- Verify: total points across all z=8 tiles == total input points

**`test_empty_cells_not_in_output`**
- Create 4 points all in the top-left quadrant `(x<0, y>0)` of a `[-1,1]×[-1,1]` grid
- Build quadtree at zoom_max=1 (4 cells)
- Assert only 1 tile returned at z=1 (the cell containing the points)
- Assert no tiles with `points == []` in output

### `tools/bdp-embed/tests/test_project.py` — new file

**`test_k_landmarks_caps_at_entry_count`**
- Call the k-landmarks calculation with `n_entries=100`, `max_landmarks=50000`
- Assert returned k == 100 (capped at data size)

**`test_k_landmarks_uses_max_when_sufficient_data`**
- Call with `n_entries=100_000`, `max_landmarks=50_000`
- Assert returned k == 50_000

**`test_model_key_format`**
- Call `get_model_key(run_id="abc-123")`
- Assert result == `"vectors/models/abc-123/umap.joblib"`

> **Note**: `test_project.py` tests only pure functions extracted from `project.py`. If the k-landmarks and model-key logic is inline in the `project` command, extract them as module-level helpers before testing.

---

## Layer 4: Frontend Vitest

### `web/lib/source-type-colors.test.ts`

```typescript
import { describe, it, expect } from 'vitest';
import { getSourceTypeColor, SOURCE_TYPE_COLORS, DEFAULT_POINT_COLOR } from '../source-type-colors';

describe('getSourceTypeColor', () => {
  it('returns correct hex for protein', () => {
    expect(getSourceTypeColor('protein')).toBe('#3b82f6');
  });

  it('returns DEFAULT_POINT_COLOR for unknown type', () => {
    expect(getSourceTypeColor('unknown_xyz')).toBe(DEFAULT_POINT_COLOR);
  });

  it('returns DEFAULT_POINT_COLOR for null, undefined, empty string', () => {
    expect(getSourceTypeColor(null)).toBe(DEFAULT_POINT_COLOR);
    expect(getSourceTypeColor(undefined)).toBe(DEFAULT_POINT_COLOR);
    expect(getSourceTypeColor('')).toBe(DEFAULT_POINT_COLOR);
  });

  it('all 17 known types return a non-default color', () => {
    Object.keys(SOURCE_TYPE_COLORS).forEach(type => {
      expect(getSourceTypeColor(type)).not.toBe(DEFAULT_POINT_COLOR);
    });
    expect(Object.keys(SOURCE_TYPE_COLORS)).toHaveLength(17);
  });
});
```

### `web/lib/vectors/tile-loader.test.ts`

Export a `clearTileCache()` function from `tile-loader.ts` (test-only helper) to reset the in-module Map between tests.

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fetchTile, fetchStats, fetchSemanticSearch, clearTileCache } from './tile-loader';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

beforeEach(() => {
  mockFetch.mockReset();
  clearTileCache();
});

describe('fetchTile', () => {
  it('makes one fetch and caches result on repeated calls', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => [{ id: '1', x: 0, y: 0, l: 'P1', et: 'ds', st: 'protein', org: 'org', slug: 's1' }],
    });
    await fetchTile('run1', 0, 0, 0);
    await fetchTile('run1', 0, 0, 0);
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });

  it('returns empty array and caches for 404', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 });
    const result = await fetchTile('run1', 0, 0, 99);
    expect(result).toEqual([]);
    await fetchTile('run1', 0, 0, 99);
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });
});

describe('fetchStats', () => {
  it('returns VectorStats shape with null fields on empty DB', async () => {
    const stats = {
      current_run_id: null, status: null, entry_count: 0,
      embedded_count: 0, projected_count: 0, projected_at: null, tile_prefix: null,
    };
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({ data: stats }) });
    const result = await fetchStats();
    expect(result.current_run_id).toBeNull();
    expect(result.entry_count).toBe(0);
  });
});

describe('fetchSemanticSearch', () => {
  it('URL-encodes the q parameter', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({ data: [] }) });
    await fetchSemanticSearch('ribosome function');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('ribosome%20function'),
    );
  });
});
```

---

## What Is NOT Tested Here

- `semantic_search` SQL correctness (requires mocking OpenAI or a real key — deferred to manual integration)
- `get_tile` storage error mapping (no MinIO mock; covered by E2E tile-not-found test)
- `bdp-embed embed` and `bdp-embed project` commands end-to-end (require real DB + OpenAI; CI-only)
- `/vectors` React component rendering (no React component tests in this project)

---

## Running the Tests

```bash
# Rust sqlx tests (requires DATABASE_URL pointing to a Postgres instance)
cargo test --package bdp-server --lib

# Rust E2E tests (requires Docker for testcontainers)
cargo test --package bdp-server --test e2e -- vectors

# Python tests
cd tools/bdp-embed && pip install -e ".[dev]" && pytest tests/ -v

# Frontend
cd web && yarn vitest run lib/
```
