# BDP CLI End-to-End Tests

## Overview

This directory contains comprehensive E2E tests for the BDP CLI:

- **`search_e2e_tests.rs`** - Mock-based tests using wiremock (fast, no external dependencies)
- **`search_real_e2e.rs`** - Real PostgreSQL + axum server tests for search functionality
- **`workflow_e2e.rs`** - Full workflow tests (init → add → pull → clean)

## Status

### ✅ Working Tests

**`search_e2e_tests.rs`** - Fully functional mock-based tests
```bash
cargo test -p bdp-cli --test search_e2e_tests
```

### ⚠️ Blocked Tests

**`search_real_e2e.rs`** and **`workflow_e2e.rs`** are currently **blocked** due to compilation issues.

**Problem**: These tests previously depended on `bdp-server` which includes `apalis-postgres`. The apalis-postgres crate uses SQLx compile-time macros that require either:
1. A running PostgreSQL database during compilation
2. SQLx offline metadata cache (`.sqlx/` directory)

Neither is available in the current setup, causing compilation to fail.

## Solutions

### Option 1: Generate SQLx Offline Cache (Recommended)

```bash
# 1. Start PostgreSQL locally
docker run -d --name bdp-postgres \
  -e POSTGRES_USER=bdp \
  -e POSTGRES_PASSWORD=bdp_dev_password \
  -e POSTGRES_DB=bdp \
  -p 5432:5432 \
  postgres:16-alpine

# 2. Set DATABASE_URL
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"

# 3. Run migrations
cd crates/bdp-server
sqlx database create
sqlx migrate run

# 4. Generate offline cache
cd ../..
cargo sqlx prepare --workspace

# 5. Now tests compile with SQLX_OFFLINE=true
SQLX_OFFLINE=true cargo test -p bdp-cli --test search_real_e2e
SQLX_OFFLINE=true cargo test -p bdp-cli --test workflow_e2e
```

### Option 2: Refactor to Remove bdp-server Dependency

Remove `bdp-server` from dev-dependencies and inline the minimal server setup code directly in test files. This requires:

1. Copy `StorageConfig`, `Storage` types into test files
2. Build axum router manually without `features::router()`
3. Implement search endpoint handler directly

**Pros**: No external dependencies, compiles without database
**Cons**: Code duplication, maintenance burden

### Option 3: Feature Flag

Make E2E tests optional behind a feature flag:

```toml
[features]
e2e-tests = []

[dev-dependencies]
bdp-server = { path = "../bdp-server", optional = true }
```

```rust
#[cfg(feature = "e2e-tests")]
mod workflow_e2e;
```

Run with: `cargo test -p bdp-cli --features e2e-tests`

## Recommendation

**Option 1 (Generate Offline Cache)** is the best long-term solution:
- Works in CI/CD pipelines
- No code duplication
- Tests remain comprehensive
- One-time setup per schema change

## Test Coverage

### `workflow_e2e.rs` - 15 Test Functions

**Workflow Tests (6)**:
1. `test_init_creates_manifest` - bdp init
2. `test_source_add_and_list` - Add + list
3. `test_source_add_duplicate` - Duplicate handling
4. `test_source_remove` - Remove source
5. `test_pull_downloads_sources` - Full pull workflow
6. `test_pull_cached_skips_download` - Cache verification

**Clean + Re-pull (3)**:
7. `test_clean_all_removes_cache`
8. `test_pull_after_clean_redownloads`
9. `test_clean_search_cache`

**Integration (3)**:
10. `test_search_then_add_then_pull`
11. `test_pull_multiple_sources`
12. `test_pull_nonexistent_source`

**Edge Cases (3)**:
13. `test_pull_without_init`
14. `test_pull_empty_manifest`
15. `test_pull_force_redownloads`

---

**Last Updated**: 2026-02-06
**Status**: Awaiting offline cache generation or refactor
