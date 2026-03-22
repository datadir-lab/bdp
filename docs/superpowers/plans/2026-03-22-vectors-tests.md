# Vectors Test Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add comprehensive test coverage across all four layers for the pgvector/vectors feature: Rust `#[sqlx::test]`, Rust E2E with testcontainers, Python pytest, and Frontend Vitest.

**Architecture:** Python prerequisites first (extract helpers, add dev extras), then frontend helper + tests, then Rust unit tests per handler, then Rust E2E tests using the existing `E2EEnvironment` harness. Each task is self-contained and produces a passing test commit.

**Tech Stack:** Rust/sqlx (pgvector), pytest, Vitest/jsdom, testcontainers (Postgres + MinIO), axum in-process HTTP

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `tools/bdp-embed/pyproject.toml` | Modify | Add `[project.optional-dependencies]` dev group |
| `tools/bdp-embed/bdp_embed/project.py` | Modify | Extract `compute_k_landmarks()` and `get_model_key()` |
| `tools/bdp-embed/tests/test_embed_text.py` | Modify | Add 2 new tests |
| `tools/bdp-embed/tests/test_tiles.py` | Modify | Add 2 new tests |
| `tools/bdp-embed/tests/test_project.py` | Create | 3 tests for extracted helpers |
| `web/lib/vectors/tile-loader.ts` | Modify | Export `clearTileCache()` |
| `web/lib/source-type-colors.test.ts` | Create | 4 Vitest tests |
| `web/lib/vectors/tile-loader.test.ts` | Create | 5 Vitest tests |
| `crates/bdp-server/src/features/vectors/queries/get_stats.rs` | Modify | Add 2 `#[sqlx::test]` tests |
| `crates/bdp-server/src/features/vectors/queries/semantic_search.rs` | Modify | Add 1 `#[sqlx::test]` test |
| `crates/bdp-server/src/features/vectors/queries/get_neighbors.rs` | Modify | Add 2 `#[sqlx::test]` tests |
| `crates/bdp-server/tests/e2e/vectors_tests.rs` | Create | 4 E2E HTTP tests |
| `crates/bdp-server/tests/e2e/mod.rs` | Modify | Register `vectors_tests` module |
| `crates/bdp-server/tests/e2e/harness.rs` | Modify | Add `get_request()` public helper |

---

## Task 1: Python prerequisites — dev extras + extract project.py helpers

**Files:**
- Modify: `tools/bdp-embed/pyproject.toml`
- Modify: `tools/bdp-embed/bdp_embed/project.py`

- [ ] **Step 1: Add dev extras to pyproject.toml**

Add after the `[project.scripts]` block:

```toml
[project.optional-dependencies]
dev = ["pytest>=8"]
```

- [ ] **Step 2: Extract `get_model_key` and `compute_k_landmarks` from project.py**

In `project.py`, find these two inline expressions:
- `model_key = f"vectors/models/{run_id}/umap.joblib"` (inside `_project`)
- `k = min(n_landmarks, len(vectors))` (inside `_project`, the `except` block)

Add these two module-level functions at the top of `project.py`, just before `@app.command()`:

```python
def get_model_key(run_id: str) -> str:
    return f"vectors/models/{run_id}/umap.joblib"


def compute_k_landmarks(n_entries: int, max_landmarks: int = 50_000) -> int:
    return min(max_landmarks, n_entries)
```

Then update `_project` to call them:
- Replace `model_key = f"vectors/models/{run_id}/umap.joblib"` with `model_key = get_model_key(run_id)`
- Replace `k = min(n_landmarks, len(vectors))` with `k = compute_k_landmarks(len(vectors), n_landmarks)`

- [ ] **Step 3: Verify existing tests still pass**

```bash
cd /c/personal/dev/bdp/.worktrees/feature-vectors/tools/bdp-embed
pip install -e ".[dev]"
pytest tests/ -v
```

Expected: All existing tests pass (test_embed_text.py, test_tiles.py).

- [ ] **Step 4: Commit**

```bash
git add tools/bdp-embed/pyproject.toml tools/bdp-embed/bdp_embed/project.py
git commit -m "refactor(bdp-embed): extract get_model_key and compute_k_landmarks helpers; add dev extras"
```

---

## Task 2: Python tests — test_embed_text.py, test_tiles.py, test_project.py

**Files:**
- Modify: `tools/bdp-embed/tests/test_embed_text.py`
- Modify: `tools/bdp-embed/tests/test_tiles.py`
- Create: `tools/bdp-embed/tests/test_project.py`

- [ ] **Step 1: Write failing tests first**

Add to the END of `tools/bdp-embed/tests/test_embed_text.py`:

```python
SOURCE_TYPES_12 = [
    "protein", "genome", "taxonomy", "transcript", "annotation",
    "structure", "domain", "pathway", "ontology_term", "compound",
    "variant", "literature",
]

def test_all_source_types_produce_non_empty_output():
    entry = {"name": "X", "description": "Y", "organism": "E. coli"}
    for source_type in SOURCE_TYPES_12:
        result = build_embed_text(entry, source_type)
        assert isinstance(result, str) and result.strip(), \
            f"source_type '{source_type}' returned empty string"
        assert not result.startswith(" "), \
            f"source_type '{source_type}' has leading space"
        assert not result.endswith(" "), \
            f"source_type '{source_type}' has trailing space"


def test_no_double_spaces_when_fields_empty():
    result = build_embed_text({"name": "Insulin"}, "protein")
    assert "  " not in result, "double spaces found in result"
```

Add to the END of `tools/bdp-embed/tests/test_tiles.py`:

```python
def test_lod_z8_keeps_all_points():
    """At z>=8, max_per_cell = len(points), so no downsampling."""
    pts = [make_point(float(i % 10), float(i // 10), i) for i in range(200)]
    tiles = build_quadtree(pts, run_id="test", zoom_min=8, zoom_max=8)
    total = sum(len(t["points"]) for t in tiles if t["z"] == 8)
    assert total == 200, f"Expected 200 points at z=8, got {total}"


def test_empty_cells_not_in_output():
    """Tiles with no points are never emitted."""
    # 4 tightly clustered points + 1 sentinel far away
    cluster = [
        make_point(-0.50, 0.50, 0),
        make_point(-0.48, 0.52, 1),
        make_point(-0.49, 0.51, 2),
        make_point(-0.47, 0.51, 3),
    ]
    sentinel = [make_point(10.0, 10.0, 4)]
    pts = cluster + sentinel

    tiles = build_quadtree(pts, run_id="test", zoom_min=1, zoom_max=1)
    z1_tiles = [t for t in tiles if t["z"] == 1]

    # No tile is empty
    assert all(len(t["points"]) > 0 for t in z1_tiles), \
        "Found empty tile in output"
    # At z=1 with 5 total points: max_per_cell = max(1, 5//4) = 1
    # Cluster cell and sentinel cell each have 1 point → 2 tiles
    assert len(z1_tiles) == 2, f"Expected 2 tiles at z=1, got {len(z1_tiles)}"
```

Create `tools/bdp-embed/tests/test_project.py`:

```python
from bdp_embed.project import compute_k_landmarks, get_model_key


def test_k_landmarks_caps_at_entry_count():
    assert compute_k_landmarks(n_entries=100, max_landmarks=50_000) == 100


def test_k_landmarks_uses_max_when_sufficient_data():
    assert compute_k_landmarks(n_entries=100_000, max_landmarks=50_000) == 50_000


def test_model_key_format():
    assert get_model_key("abc-123") == "vectors/models/abc-123/umap.joblib"
```

- [ ] **Step 2: Run to verify tests fail (for new tests) or pass (imports fine)**

```bash
cd /c/personal/dev/bdp/.worktrees/feature-vectors/tools/bdp-embed
pytest tests/test_project.py -v
```

Expected: All 3 tests PASS (functions are now extracted). If any ImportError → verify Task 1 was done correctly.

```bash
pytest tests/test_embed_text.py::test_all_source_types_produce_non_empty_output -v
pytest tests/test_tiles.py::test_lod_z8_keeps_all_points -v
pytest tests/test_tiles.py::test_empty_cells_not_in_output -v
```

Expected: All PASS.

- [ ] **Step 3: Run full Python test suite**

```bash
pytest tests/ -v
```

Expected: All tests pass (no failures, no errors).

- [ ] **Step 4: Commit**

```bash
git add tools/bdp-embed/tests/
git commit -m "test(bdp-embed): add test_project.py and extend embed_text/tiles test coverage"
```

---

## Task 3: Frontend — add clearTileCache export to tile-loader.ts

**Files:**
- Modify: `web/lib/vectors/tile-loader.ts`

The in-module `tileCache` Map is not exported and cannot be reset between Vitest tests. Add a test helper at the end of the file.

- [ ] **Step 1: Add clearTileCache export**

Append to the end of `web/lib/vectors/tile-loader.ts`:

```typescript
/** Reset the in-memory tile cache. Intended for use in tests only. */
export function clearTileCache(): void {
  tileCache.clear();
}
```

- [ ] **Step 2: Verify TypeScript still compiles**

```bash
cd /c/personal/dev/bdp/.worktrees/feature-vectors/web
npx tsc --noEmit 2>&1
```

Expected: No output (zero errors).

- [ ] **Step 3: Commit**

```bash
git add web/lib/vectors/tile-loader.ts
git commit -m "feat(web): export clearTileCache test helper from tile-loader"
```

---

## Task 4: Frontend Vitest tests — source-type-colors + tile-loader

**Files:**
- Create: `web/lib/source-type-colors.test.ts`
- Create: `web/lib/vectors/tile-loader.test.ts`

- [ ] **Step 1: Create source-type-colors.test.ts**

```typescript
// web/lib/source-type-colors.test.ts
import { describe, it, expect } from 'vitest';
import {
  getSourceTypeColor,
  SOURCE_TYPE_COLORS,
  DEFAULT_POINT_COLOR,
} from './source-type-colors';

describe('getSourceTypeColor', () => {
  it('returns correct hex for protein', () => {
    expect(getSourceTypeColor('protein')).toBe('#3b82f6');
  });

  it('returns DEFAULT_POINT_COLOR for unknown type', () => {
    expect(getSourceTypeColor('unknown_xyz')).toBe(DEFAULT_POINT_COLOR);
  });

  it('returns DEFAULT_POINT_COLOR for null, undefined, and empty string', () => {
    expect(getSourceTypeColor(null)).toBe(DEFAULT_POINT_COLOR);
    expect(getSourceTypeColor(undefined)).toBe(DEFAULT_POINT_COLOR);
    expect(getSourceTypeColor('')).toBe(DEFAULT_POINT_COLOR);
  });

  it('all 17 known source types return a non-default color', () => {
    const types = Object.keys(SOURCE_TYPE_COLORS);
    expect(types).toHaveLength(17);
    types.forEach((type) => {
      expect(getSourceTypeColor(type)).not.toBe(DEFAULT_POINT_COLOR);
    });
  });
});
```

- [ ] **Step 2: Run source-type-colors tests to verify they pass**

```bash
cd /c/personal/dev/bdp/.worktrees/feature-vectors/web
npx vitest run lib/source-type-colors.test.ts 2>&1
```

Expected: 4 tests PASS.

- [ ] **Step 3: Create tile-loader.test.ts**

```typescript
// web/lib/vectors/tile-loader.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  fetchTile,
  fetchStats,
  fetchSemanticSearch,
  clearTileCache,
} from './tile-loader';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

beforeEach(() => {
  mockFetch.mockReset();
  clearTileCache();
});

describe('fetchTile', () => {
  it('makes one fetch call and caches result for repeated calls', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [
        { id: '1', x: 0, y: 0, l: 'P1', et: 'ds', st: 'protein', org: 'org', slug: 's1' },
      ],
    });

    const first = await fetchTile('run1', 0, 0, 0);
    const second = await fetchTile('run1', 0, 0, 0);

    expect(mockFetch).toHaveBeenCalledTimes(1);
    expect(first).toEqual(second);
    expect(first).toHaveLength(1);
  });

  it('returns empty array and caches 404', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 });

    const result = await fetchTile('run1', 0, 0, 99);
    expect(result).toEqual([]);

    // Second call: cache hit, no additional fetch
    await fetchTile('run1', 0, 0, 99);
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });
});

describe('fetchStats', () => {
  it('returns VectorStats with expected shape', async () => {
    const stats = {
      current_run_id: null,
      status: null,
      entry_count: 0,
      embedded_count: 0,
      projected_count: 0,
      projected_at: null,
      tile_prefix: null,
    };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: stats }),
    });

    const result = await fetchStats();
    expect(result.current_run_id).toBeNull();
    expect(result.entry_count).toBe(0);
    expect(result.tile_prefix).toBeNull();
  });
});

describe('fetchSemanticSearch', () => {
  it('URL-encodes spaces in the q parameter', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: [] }),
    });

    await fetchSemanticSearch('ribosome function');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('ribosome%20function'),
    );
  });
});
```

- [ ] **Step 4: Run tile-loader tests to verify they pass**

```bash
cd /c/personal/dev/bdp/.worktrees/feature-vectors/web
npx vitest run lib/vectors/tile-loader.test.ts 2>&1
```

Expected: 5 tests PASS.

- [ ] **Step 5: Run full frontend test suite**

```bash
npx vitest run lib/ 2>&1
```

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add web/lib/source-type-colors.test.ts web/lib/vectors/tile-loader.test.ts
git commit -m "test(web): add Vitest tests for source-type-colors and tile-loader"
```

---

## Task 5: Rust — get_stats.rs tests

**Files:**
- Modify: `crates/bdp-server/src/features/vectors/queries/get_stats.rs`

**Important**: All test data insertion uses `sqlx::query` (not `sqlx::query!`) to avoid needing offline metadata regeneration.

- [ ] **Step 1: Write the two new tests**

Append inside the existing `#[cfg(test)] mod tests { ... }` block in `get_stats.rs`, after the existing `test_stats_returns_nulls_with_no_data` test:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_stats_counts_registry_entries(pool: PgPool) -> sqlx::Result<()> {
    let org_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, created_at, updated_at)
         VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(org_id)
    .bind("test-org")
    .bind("Test Org")
    .execute(&pool)
    .await?;

    for i in 0..3u32 {
        sqlx::query(
            "INSERT INTO registry_entries (id, organization_id, slug, name, entry_type, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(org_id)
        .bind(format!("entry-{i}"))
        .bind(format!("Entry {i}"))
        .bind("data_source")
        .execute(&pool)
        .await?;
    }

    let stats = handle(pool, GetVectorStatsQuery).await.unwrap();
    assert_eq!(stats.entry_count, Some(3));
    assert!(stats.current_run_id.is_none());
    assert_eq!(stats.embedded_count, Some(0));
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_stats_with_complete_run(pool: PgPool) -> sqlx::Result<()> {
    let org_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, created_at, updated_at)
         VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(org_id)
    .bind("test-org2")
    .bind("Test Org 2")
    .execute(&pool)
    .await?;

    for i in 0..3u32 {
        sqlx::query(
            "INSERT INTO registry_entries (id, organization_id, slug, name, entry_type, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(org_id)
        .bind(format!("r-{i}"))
        .bind(format!("R {i}"))
        .bind("data_source")
        .execute(&pool)
        .await?;
    }

    let run_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO vector_projection_runs
         (run_id, status, entry_count, embedded_count, projected_count,
          projected_at, tile_prefix, started_at)
         VALUES ($1, $2, $3, $4, $5, NOW(), $6, NOW())",
    )
    .bind(run_id)
    .bind("complete")
    .bind(3i64)
    .bind(2i64)
    .bind(1i64)
    .bind("vectors/tiles/run123")
    .execute(&pool)
    .await?;

    let stats = handle(pool, GetVectorStatsQuery).await.unwrap();
    // run fields come from vector_projection_runs
    assert!(stats.current_run_id.is_some());
    assert_eq!(stats.status, Some("complete".to_string()));
    assert_eq!(stats.tile_prefix, Some("vectors/tiles/run123".to_string()));
    assert_eq!(stats.projected_count, Some(1));
    // live counts come from tables
    assert_eq!(stats.entry_count, Some(3));    // COUNT(*) FROM registry_entries
    assert_eq!(stats.embedded_count, Some(0)); // COUNT(*) FROM entry_embeddings (none inserted)
    Ok(())
}
```

- [ ] **Step 2: Run these two tests**

```bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bdp_test"
cd /c/personal/dev/bdp/.worktrees/feature-vectors
cargo test --package bdp-server \
  "features::vectors::queries::get_stats::tests::test_stats_counts_registry_entries" \
  "features::vectors::queries::get_stats::tests::test_stats_with_complete_run" \
  -- --test-threads=1 2>&1 | tail -20
```

Expected: Both PASS. If the `migrations` path fails, try without the `migrations` argument (sqlx::test picks up `./migrations` automatically when `DATABASE_URL` is set).

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-server/src/features/vectors/queries/get_stats.rs
git commit -m "test(vectors): add sqlx::test coverage for get_stats with registry entries and full run"
```

---

## Task 6: Rust — semantic_search.rs test

**Files:**
- Modify: `crates/bdp-server/src/features/vectors/queries/semantic_search.rs`

- [ ] **Step 1: Write the failing test**

Append inside `#[cfg(test)] mod tests { ... }` in `semantic_search.rs`:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_semantic_search_embedding_unavailable_without_api_key(
    pool: PgPool,
) -> sqlx::Result<()> {
    // Remove the key so embed_query proceeds with an empty key,
    // which OpenAI rejects → EmbeddingUnavailable.
    let prev = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("OPENAI_API_KEY");

    let query = SemanticSearchQuery {
        q: "ribosome".to_string(),
        k: 5,
    };
    let result = handle(pool, query).await;

    // Restore to avoid poisoning other tests
    if let Some(key) = prev {
        std::env::set_var("OPENAI_API_KEY", key);
    }

    assert!(
        matches!(result, Err(SemanticSearchError::EmbeddingUnavailable(_))),
        "Expected EmbeddingUnavailable, got: {:?}",
        result
    );
    Ok(())
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test --package bdp-server \
  "features::vectors::queries::semantic_search::tests::test_semantic_search_embedding_unavailable_without_api_key" \
  -- --test-threads=1 2>&1 | tail -20
```

Expected: PASS. The empty API key causes OpenAI to return a 401, which maps to `EmbeddingUnavailable`.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-server/src/features/vectors/queries/semantic_search.rs
git commit -m "test(vectors): add sqlx::test for semantic_search EmbeddingUnavailable path"
```

---

## Task 7: Rust — get_neighbors.rs tests

**Files:**
- Modify: `crates/bdp-server/src/features/vectors/queries/get_neighbors.rs`

- [ ] **Step 1: Write the two new tests**

Append inside `#[cfg(test)] mod tests { ... }` in `get_neighbors.rs`:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_get_neighbors_not_found_for_unknown_entry(
    pool: PgPool,
) -> sqlx::Result<()> {
    let query = GetNeighborsQuery {
        entry_id: uuid::Uuid::new_v4(),
        k: 5,
    };
    let result = handle(pool, query).await;
    assert!(
        matches!(result, Err(GetNeighborsError::NotFound)),
        "Expected NotFound, got: {:?}",
        result
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_neighbors_returns_knn_ordered_by_similarity(
    pool: PgPool,
) -> sqlx::Result<()> {
    use pgvector::HalfVector;

    // Insert org
    let org_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, created_at, updated_at)
         VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(org_id)
    .bind("nbr-org")
    .bind("Nbr Org")
    .execute(&pool)
    .await?;

    // Insert 4 registry entries
    let entry_ids: Vec<uuid::Uuid> = (0..4).map(|_| uuid::Uuid::new_v4()).collect();
    for (i, eid) in entry_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO registry_entries (id, organization_id, slug, name, entry_type, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
        )
        .bind(eid)
        .bind(org_id)
        .bind(format!("e-{i}"))
        .bind(format!("Entry {i}"))
        .bind("data_source")
        .execute(&pool)
        .await?;
    }

    // Build 4 unit vectors with known cosine similarity to e0=[1,0,...,0]:
    //   e0: dim0=1.0                  (seed)
    //   e1: dim0=0.95, dim1=0.3122    (cos_sim ≈ 0.95, normalized)
    //   e2: dim0=0.5,  dim1=0.8660    (cos_sim ≈ 0.5,  normalized)
    //   e3: dim0=0.0,  dim1=1.0       (cos_sim = 0.0,  orthogonal)
    let make_vec = |d0: f32, d1: f32| -> Vec<f32> {
        let mut v = vec![0.0f32; 512];
        v[0] = d0;
        v[1] = d1;
        // already unit length for these pairs
        v
    };

    let vecs = [
        make_vec(1.0, 0.0),
        make_vec(0.9501, 0.3122),  // normalized: sqrt(0.95²+0.31²)≈1.0
        make_vec(0.5, 0.8660),     // normalized: sqrt(0.25+0.75)=1.0
        make_vec(0.0, 1.0),
    ];

    for (eid, v) in entry_ids.iter().zip(vecs.iter()) {
        let hv = HalfVector::from(v.clone());
        sqlx::query(
            "INSERT INTO entry_embeddings (entry_id, vector) VALUES ($1, $2::halfvec)",
        )
        .bind(eid)
        .bind(hv)
        .execute(&pool)
        .await?;
    }

    // Query neighbors for e0 with k=3
    let query = GetNeighborsQuery {
        entry_id: entry_ids[0],
        k: 3,
    };
    let result = handle(pool, query).await.unwrap();
    let neighbors = result.neighbors;

    assert_eq!(neighbors.len(), 3, "Expected 3 neighbors");
    // Self (e0) must not be in results
    assert!(
        neighbors.iter().all(|n| n.entry_id != entry_ids[0]),
        "Seed entry should not appear in neighbors"
    );
    // Most similar neighbor first (e1 has cos_sim ≈ 0.95)
    assert_eq!(
        neighbors[0].entry_id, entry_ids[1],
        "First neighbor should be e1 (highest similarity)"
    );
    // All similarities in valid range
    assert!(
        neighbors.iter().all(|n| n.similarity >= 0.0 && n.similarity <= 1.0),
        "All similarities should be in [0, 1]"
    );
    Ok(())
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo test --package bdp-server \
  "features::vectors::queries::get_neighbors::tests" \
  -- --test-threads=1 2>&1 | tail -20
```

Expected: 3 tests PASS (existing `test_invalid_k` + 2 new).

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-server/src/features/vectors/queries/get_neighbors.rs
git commit -m "test(vectors): add sqlx::test for get_neighbors NotFound and KNN ordering"
```

---

## Task 8: Rust E2E — vectors_tests.rs

**Files:**
- Create: `crates/bdp-server/tests/e2e/vectors_tests.rs`
- Modify: `crates/bdp-server/tests/e2e/mod.rs`
- Modify: `crates/bdp-server/tests/e2e/harness.rs`

**Note**: The E2E tests reuse `E2EEnvironment::new()` from the existing harness (Postgres + MinIO + BDP server via Docker). First, add a `get_request` helper method so vectors tests can call arbitrary GET endpoints.

- [ ] **Step 1: Read the existing harness to find how to add a method**

Read `crates/bdp-server/tests/e2e/harness.rs` to find the `impl E2EEnvironment` block and the `server_url` field. Add the following public method inside `impl E2EEnvironment`:

```rust
/// Make a GET request to the BDP server at the given path.
/// Path should start with `/`, e.g. `/api/v1/vectors/stats`.
pub async fn get_request(&self, path: &str) -> Result<reqwest::Response> {
    let url = format!("{}{}", self.server_url, path);
    self.http_client
        .get(&url)
        .send()
        .await
        .context(format!("GET {path} failed"))
}
```

- [ ] **Step 2: Register the new test module in mod.rs**

Find `crates/bdp-server/tests/e2e/mod.rs` and add:
```rust
mod vectors_tests;
```
alongside the existing `mod ingestion_tests;` line.

- [ ] **Step 3: Write the failing tests**

Create `crates/bdp-server/tests/e2e/vectors_tests.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! E2E tests for the /api/v1/vectors endpoints.

use super::*;
use anyhow::Result;
use serial_test::serial;

/// GET /api/v1/vectors/stats on a fresh DB returns 200 with zero counts.
#[tokio::test]
#[serial]
async fn test_vectors_stats_empty() -> Result<()> {
    let env = E2EEnvironment::new().await?;

    let res = env.get_request("/api/v1/vectors/stats").await?;
    assert_eq!(res.status().as_u16(), 200);

    let body: serde_json::Value = res.json().await?;
    assert!(
        body["data"]["current_run_id"].is_null(),
        "current_run_id should be null on fresh DB"
    );
    assert_eq!(
        body["data"]["entry_count"], 0,
        "entry_count should be 0 on fresh DB"
    );
    Ok(())
}

/// GET a tile key that doesn't exist in MinIO returns 404.
#[tokio::test]
#[serial]
async fn test_vectors_tile_not_found() -> Result<()> {
    let env = E2EEnvironment::new().await?;

    let res = env
        .get_request("/api/v1/vectors/tiles/nonexistent-run-id/0/0/0")
        .await?;
    assert_eq!(
        res.status().as_u16(),
        404,
        "Missing tile should return 404"
    );
    Ok(())
}

/// GET /search without OPENAI_API_KEY returns 503.
#[tokio::test]
#[serial]
async fn test_vectors_search_returns_503_without_api_key() -> Result<()> {
    // The E2E test process should not have OPENAI_API_KEY set.
    // If it is, this test may fail — remove the var for this test.
    let prev = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("OPENAI_API_KEY");

    let env = E2EEnvironment::new().await?;
    let res = env.get_request("/api/v1/vectors/search?q=ribosome").await?;

    if let Some(key) = prev {
        std::env::set_var("OPENAI_API_KEY", key);
    }

    assert_eq!(
        res.status().as_u16(),
        503,
        "Search without API key should return 503"
    );
    Ok(())
}

/// GET neighbors for a UUID with no embedding returns 404.
#[tokio::test]
#[serial]
async fn test_vectors_neighbors_returns_404_for_missing_entry() -> Result<()> {
    let env = E2EEnvironment::new().await?;

    let res = env
        .get_request("/api/v1/vectors/00000000-0000-0000-0000-000000000000/neighbors")
        .await?;
    assert_eq!(
        res.status().as_u16(),
        404,
        "Entry with no embedding should return 404"
    );
    Ok(())
}
```

- [ ] **Step 4: Run the E2E tests (requires Docker)**

```bash
cargo test --package bdp-server --test e2e -- vectors 2>&1 | tail -40
```

Expected: All 4 tests PASS. If `E2EEnvironment::new()` fails because the BDP Docker image isn't built, build it first: `docker build -t bdp-server:latest .` from the repo root.

- [ ] **Step 5: Run full Rust library tests to ensure nothing broke**

```bash
cargo test --package bdp-server --lib 2>&1 | tail -20
```

Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add crates/bdp-server/tests/e2e/vectors_tests.rs \
        crates/bdp-server/tests/e2e/mod.rs \
        crates/bdp-server/tests/e2e/harness.rs
git commit -m "test(vectors): add E2E tests for stats, tile 404, search 503, and neighbors 404"
```

---

## Final Verification

After all 8 tasks complete, run the full test suite:

```bash
# Python
cd /c/personal/dev/bdp/.worktrees/feature-vectors/tools/bdp-embed
pytest tests/ -v

# Frontend
cd /c/personal/dev/bdp/.worktrees/feature-vectors/web
npx vitest run lib/

# Rust library (sqlx::test)
cd /c/personal/dev/bdp/.worktrees/feature-vectors
cargo test --package bdp-server --lib

# Rust E2E (requires Docker)
cargo test --package bdp-server --test e2e -- vectors
```

Push updated branch:
```bash
git push origin feature/vectors
```
