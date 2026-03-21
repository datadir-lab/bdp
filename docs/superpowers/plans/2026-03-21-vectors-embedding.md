# Vector Embeddings & /vectors Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add pgvector semantic embeddings for all BDP registry entries, a `/vectors` exploration page using regl-scatterplot with WizMap-style quadtree tiles, and semantic search for the MCP server.

**Architecture:** Text embeddings (512-dim Matryoshka via OpenAI) stored in pgvector halfvec column with HNSW index. A Python CLI (`bdp-embed`) generates embeddings, runs landmark UMAP projection, and builds quadtree tile files stored in MinIO. The Rust API serves tiles and KNN search. The Next.js frontend renders points via regl-scatterplot with viewport-based tile fetching.

**Spec:** `docs/superpowers/specs/2026-03-21-vectors-embedding-design.md`

**Tech Stack:** pgvector 0.7+, halfvec(512), HNSW index, Python (umap-learn, openai, psycopg, boto3, typer), regl-scatterplot, async-openai (Rust), moka (Rust LRU cache)

---

## File Map

### New files — Migrations
- `migrations/20260322000001_enable_pgvector.sql`
- `migrations/20260322000002_entry_embeddings.sql`
- `migrations/20260322000003_entry_projections.sql`
- `migrations/20260322000004_vector_projection_runs.sql`

### New files — Python CLI (`tools/bdp-embed/`)
- `tools/bdp-embed/pyproject.toml` — package config + dependencies
- `tools/bdp-embed/bdp_embed/__init__.py`
- `tools/bdp-embed/bdp_embed/cli.py` — typer app entry point, subcommand wiring
- `tools/bdp-embed/bdp_embed/db.py` — postgres connection helpers (psycopg3 async)
- `tools/bdp-embed/bdp_embed/embed_text.py` — source-type-aware text builders (pure logic)
- `tools/bdp-embed/bdp_embed/embed.py` — `embed` subcommand: OpenAI batching → entry_embeddings
- `tools/bdp-embed/bdp_embed/project.py` — `project` subcommand: landmark UMAP → entry_projections + model serialization
- `tools/bdp-embed/bdp_embed/tiles.py` — `tiles` subcommand: quadtree build → MinIO
- `tools/bdp-embed/tests/__init__.py`
- `tools/bdp-embed/tests/test_embed_text.py`
- `tools/bdp-embed/tests/test_tiles.py`

### New files — Rust backend (`crates/bdp-server/src/features/vectors/`)
- `crates/bdp-server/src/features/vectors/mod.rs`
- `crates/bdp-server/src/features/vectors/routes.rs`
- `crates/bdp-server/src/features/vectors/queries/mod.rs`
- `crates/bdp-server/src/features/vectors/queries/get_stats.rs`
- `crates/bdp-server/src/features/vectors/queries/semantic_search.rs`
- `crates/bdp-server/src/features/vectors/queries/get_neighbors.rs`
- `crates/bdp-server/src/features/vectors/queries/get_tile.rs`

### Modified files — Rust backend
- `crates/bdp-server/Cargo.toml` — add pgvector, async-openai, moka
- `crates/bdp-server/src/features/mod.rs` — add `pub mod vectors;` + mount route
- `crates/bdp-server/src/cqrs/mod.rs` — register 4 vector query handlers

### New files — Frontend
- `web/lib/source-type-colors.ts` — canonical `SOURCE_TYPE_COLORS` + `ENTRY_TYPE_COLORS`
- `web/lib/vectors/tile-loader.ts` — tile URL construction, fetch, in-session cache
- `web/app/[locale]/vectors/page.tsx` — thin Next.js page shell
- `web/app/[locale]/vectors/vectors-view.tsx` — regl-scatterplot canvas + HUD
- `web/app/[locale]/vectors/vector-sidebar.tsx` — click sidebar (neighbors + links)
- `web/app/[locale]/vectors/vector-search-bar.tsx` — semantic search input

### Modified files — Frontend
- `web/components/layout/header.tsx` — add /vectors nav link

---

## Phase A: Database Migrations

### Task 1: Enable pgvector and create entry_embeddings

**Files:**
- Create: `migrations/20260322000001_enable_pgvector.sql`
- Create: `migrations/20260322000002_entry_embeddings.sql`

- [ ] **Step 1: Write migration 1 — enable pgvector**

```sql
-- migrations/20260322000001_enable_pgvector.sql
CREATE EXTENSION IF NOT EXISTS vector;
```

- [ ] **Step 2: Write migration 2 — entry_embeddings table**

```sql
-- migrations/20260322000002_entry_embeddings.sql

-- Text embeddings: 512-dim Matryoshka via text-embedding-3-small
-- halfvec = float16, saves 50% vs float32
-- Table disk: 10M × 512 × 2 bytes ≈ 10GB
-- HNSW index RAM: ~5-8GB (separate from table)
CREATE TABLE entry_embeddings (
    entry_id     UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    model        VARCHAR(100) NOT NULL DEFAULT 'text-embedding-3-small',
    vector       halfvec(512) NOT NULL,
    embedded_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- HNSW for cosine ANN search
-- m=16, ef_construction=64: ~97% recall, ~1-2h build at 10M rows
-- After large batch inserts (>1M rows): run REINDEX CONCURRENTLY to restore recall
CREATE INDEX entry_embeddings_vector_idx ON entry_embeddings
    USING hnsw (vector halfvec_cosine_ops)
    WITH (m = 16, ef_construction = 64);
```

- [ ] **Step 3: Run migrations**

```bash
cargo xtask db migrate
```

Expected: both migrations apply cleanly, `entry_embeddings` table visible in psql.

- [ ] **Step 4: Verify pgvector is active**

```bash
psql $DATABASE_URL -c "SELECT extname, extversion FROM pg_extension WHERE extname = 'vector';"
```

Expected: one row with `vector` and a version like `0.7.x`.

- [ ] **Step 5: Commit**

```bash
git add migrations/20260322000001_enable_pgvector.sql migrations/20260322000002_entry_embeddings.sql
git commit -m "feat(db): enable pgvector and add entry_embeddings table with HNSW index"
```

---

### Task 2: Add entry_projections and vector_projection_runs

**Files:**
- Create: `migrations/20260322000003_entry_projections.sql`
- Create: `migrations/20260322000004_vector_projection_runs.sql`

- [ ] **Step 1: Write migration 3 — entry_projections**

```sql
-- migrations/20260322000003_entry_projections.sql

-- Pre-computed 2D UMAP coords for the /vectors page.
-- Denormalized display fields (label, entry_type, etc.) avoid joins at
-- query time when serving 10M+ rows.
-- entry_type values: 'data_source' | 'tool' (mirrors registry_entries constraint)
CREATE TABLE entry_projections (
    entry_id     UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    x            FLOAT4 NOT NULL,
    y            FLOAT4 NOT NULL,
    label        TEXT NOT NULL,
    entry_type   VARCHAR(50) NOT NULL,
    source_type  VARCHAR(50),
    org_slug     VARCHAR(100) NOT NULL,
    slug         VARCHAR(255) NOT NULL,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX entry_projections_xy_idx ON entry_projections (x, y);
CREATE INDEX entry_projections_source_type_idx ON entry_projections (source_type);
CREATE INDEX entry_projections_type_source_idx ON entry_projections (entry_type, source_type);
```

- [ ] **Step 2: Write migration 4 — vector_projection_runs**

```sql
-- migrations/20260322000004_vector_projection_runs.sql

-- Tracks each bdp-embed pipeline run (embed → project → tiles).
-- status values: 'pending' | 'embedding' | 'projecting' | 'tiling' | 'complete' | 'failed'
-- Frontend reads current_run_id from /api/v1/vectors/stats to build tile URLs.
CREATE TABLE vector_projection_runs (
    run_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    stage_completed VARCHAR(20),
    entry_count     BIGINT,
    embedded_count  BIGINT,
    projected_count BIGINT,
    tile_prefix     TEXT,
    error_message   TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    projected_at    TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ
);
```

- [ ] **Step 3: Run migrations**

```bash
cargo xtask db migrate
```

Expected: migrations apply cleanly, both tables visible.

- [ ] **Step 4: Regenerate SQLx metadata**

```bash
cargo xtask sqlx prepare
```

- [ ] **Step 5: Commit**

```bash
git add migrations/20260322000003_entry_projections.sql migrations/20260322000004_vector_projection_runs.sql
git commit -m "feat(db): add entry_projections and vector_projection_runs tables"
```

---

## Phase B: bdp-embed Python CLI

### Task 3: Project scaffold + embed text builders

**Files:**
- Create: `tools/bdp-embed/pyproject.toml`
- Create: `tools/bdp-embed/bdp_embed/__init__.py`
- Create: `tools/bdp-embed/bdp_embed/cli.py`
- Create: `tools/bdp-embed/bdp_embed/embed_text.py`
- Create: `tools/bdp-embed/tests/__init__.py`
- Create: `tools/bdp-embed/tests/test_embed_text.py`

- [ ] **Step 1: Write pyproject.toml**

```toml
# tools/bdp-embed/pyproject.toml
[project]
name = "bdp-embed"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "openai>=1.30",
    "umap-learn>=0.5",
    "scikit-learn>=1.4",
    "numpy>=1.26",
    "psycopg[binary]>=3.1",
    "boto3>=1.34",
    "joblib>=1.3",
    "tqdm>=4.66",
    "typer>=0.12",
]

[project.scripts]
bdp-embed = "bdp_embed.cli:app"

[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.backends.legacy:build"
```

- [ ] **Step 2: Write embed_text.py (pure logic, no I/O)**

```python
# tools/bdp-embed/bdp_embed/embed_text.py

def build_embed_text(entry: dict, source_type: str) -> str:
    """Build the text to embed for a registry entry.

    Uses source-type-specific templates to produce the most semantically
    meaningful text. Unknown types fall through to the generic fallback.
    """
    def _join(*parts) -> str:
        return " ".join(p.strip() for p in parts if p and str(p).strip())

    match source_type:
        case "protein":
            return _join(
                entry.get("name", ""),
                entry.get("gene_name", ""),
                entry.get("organism", ""),
                entry.get("function", ""),
                entry.get("go_terms", ""),
            )
        case "genome":
            return _join(
                entry.get("name", ""),
                entry.get("organism", ""),
                entry.get("assembly_level", ""),
                entry.get("annotation_source", ""),
            )
        case "taxonomy":
            return _join(
                entry.get("name", ""),
                entry.get("common_name", ""),
                entry.get("lineage", ""),
                entry.get("rank", ""),
            )
        case "transcript":
            return _join(
                entry.get("name", ""),
                entry.get("gene_name", ""),
                entry.get("biotype", ""),
                entry.get("organism", ""),
            )
        case "annotation":
            return _join(
                entry.get("name", ""),
                entry.get("description", ""),
                entry.get("assay_type", ""),
                entry.get("organism", ""),
                entry.get("tissue", ""),
            )
        case "structure":
            return _join(
                entry.get("name", ""),
                entry.get("organism", ""),
                entry.get("method", ""),
                entry.get("molecule_names", ""),
            )
        case "domain":
            return _join(
                entry.get("name", ""),
                entry.get("description", ""),
                entry.get("domain_type", ""),
                entry.get("member_dbs", ""),
            )
        case "pathway":
            genes = " ".join(entry.get("gene_list", [])[:20])
            return _join(
                entry.get("name", ""),
                entry.get("organism", ""),
                entry.get("description", ""),
                f"genes: {genes}" if genes else "",
            )
        case "ontology_term":
            return _join(
                entry.get("name", ""),
                entry.get("definition", ""),
                f"synonyms: {entry.get('synonyms', '')}",
                f"namespace: {entry.get('namespace', '')}",
            )
        case "compound":
            return _join(
                entry.get("name", ""),
                entry.get("synonyms", ""),
                entry.get("bioactivity", ""),
                f"targets: {entry.get('targets', '')}",
            )
        case "variant":
            return _join(
                entry.get("gene", ""),
                entry.get("consequence", ""),
                entry.get("clinical_significance", ""),
                entry.get("trait", ""),
            )
        case "literature":
            # Raw text, no template prefix
            return _join(entry.get("title", ""), entry.get("abstract", ""))
        case _:
            # Generic fallback for types not yet explicitly handled
            return _join(
                entry.get("name", ""),
                entry.get("description", ""),
                source_type,
                entry.get("organism", ""),
            )
```

- [ ] **Step 3: Write the CLI entry point**

```python
# tools/bdp-embed/bdp_embed/__init__.py
# (empty)

# tools/bdp-embed/bdp_embed/cli.py
import typer

app = typer.Typer(name="bdp-embed", help="BDP embedding pipeline CLI")

# Subcommands registered in each module
from bdp_embed import embed, project, tiles  # noqa: E402, F401

if __name__ == "__main__":
    app()
```

- [ ] **Step 4: Write failing tests for embed_text**

```python
# tools/bdp-embed/tests/__init__.py
# (empty)

# tools/bdp-embed/tests/test_embed_text.py
from bdp_embed.embed_text import build_embed_text


def test_protein_includes_gene_and_organism():
    entry = {"name": "Insulin", "gene_name": "INS", "organism": "Homo sapiens",
             "function": "glucose metabolism", "go_terms": "GO:0005179"}
    result = build_embed_text(entry, "protein")
    assert "Insulin" in result
    assert "INS" in result
    assert "Homo sapiens" in result
    assert "glucose metabolism" in result


def test_protein_skips_empty_fields():
    entry = {"name": "Insulin"}
    result = build_embed_text(entry, "protein")
    assert result.strip() == "Insulin"
    assert "  " not in result  # no double spaces from empty joins


def test_genome_includes_assembly_level():
    entry = {"name": "GRCh38", "organism": "Homo sapiens", "assembly_level": "Chromosome"}
    result = build_embed_text(entry, "genome")
    assert "GRCh38" in result
    assert "Chromosome" in result


def test_pathway_limits_gene_list():
    entry = {"name": "Glycolysis", "gene_list": [f"gene{i}" for i in range(50)]}
    result = build_embed_text(entry, "pathway")
    # Only first 20 genes included
    assert "gene19" in result
    assert "gene20" not in result


def test_literature_uses_raw_text():
    entry = {"title": "BRCA1 and DNA repair", "abstract": "We studied..."}
    result = build_embed_text(entry, "literature")
    assert result == "BRCA1 and DNA repair We studied..."


def test_unknown_type_uses_generic_fallback():
    entry = {"name": "Foo", "description": "Bar", "organism": "E. coli"}
    result = build_embed_text(entry, "novel_future_type")
    assert "Foo" in result
    assert "Bar" in result
    assert "E. coli" in result


def test_empty_entry_does_not_crash():
    result = build_embed_text({}, "protein")
    assert isinstance(result, str)
```

- [ ] **Step 5: Install and run tests**

```bash
cd tools/bdp-embed
pip install -e ".[dev]" 2>/dev/null || pip install -e .
python -m pytest tests/test_embed_text.py -v
```

Expected: all 7 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tools/bdp-embed/
git commit -m "feat(bdp-embed): scaffold CLI + source-type-aware embed text builders"
```

---

### Task 4: embed subcommand (OpenAI → entry_embeddings)

**Files:**
- Create: `tools/bdp-embed/bdp_embed/db.py`
- Create: `tools/bdp-embed/bdp_embed/embed.py`

- [ ] **Step 1: Write db.py — postgres helpers**

```python
# tools/bdp-embed/bdp_embed/db.py
import psycopg
from typing import AsyncGenerator
from contextlib import asynccontextmanager


@asynccontextmanager
async def get_conn(db_url: str) -> AsyncGenerator[psycopg.AsyncConnection, None]:
    async with await psycopg.AsyncConnection.connect(db_url) as conn:
        yield conn
```

- [ ] **Step 2: Write embed.py**

```python
# tools/bdp-embed/bdp_embed/embed.py
import asyncio
import time
from typing import Annotated
import psycopg
import typer
import openai
from tqdm import tqdm
from bdp_embed.cli import app
from bdp_embed.db import get_conn
from bdp_embed.embed_text import build_embed_text

EMBED_MODEL = "text-embedding-3-small"
EMBED_DIMS = 512
MAX_TOKENS = 8191


def _truncate_text(text: str, max_chars: int = 32000) -> str:
    """Rough char-based truncation before sending to API (avoids token count calls)."""
    return text[:max_chars]


@app.command()
def embed(
    db_url: Annotated[str, typer.Option(envvar="DATABASE_URL")],
    openai_key: Annotated[str, typer.Option(envvar="OPENAI_API_KEY")],
    batch_size: int = 2048,
    workers: int = 8,
):
    """Generate text embeddings for all uningested registry entries."""
    asyncio.run(_embed(db_url, openai_key, batch_size, workers))


async def _embed(db_url: str, openai_key: str, batch_size: int, workers: int):
    client = openai.AsyncOpenAI(api_key=openai_key)

    async with get_conn(db_url) as conn:
        # Fetch entries not yet embedded (incremental)
        rows = await conn.execute(
            """
            SELECT re.id, re.name, re.description, re.entry_type,
                   ds.source_type, re.slug
            FROM registry_entries re
            LEFT JOIN data_sources ds ON ds.id = re.id
            WHERE re.id NOT IN (SELECT entry_id FROM entry_embeddings)
            ORDER BY re.created_at
            """,
            row_factory=psycopg.rows.dict_row,
        )
        entries = await rows.fetchall()

    if not entries:
        typer.echo("No new entries to embed.")
        return

    typer.echo(f"Embedding {len(entries)} entries in batches of {batch_size}...")
    semaphore = asyncio.Semaphore(workers)

    async def embed_batch(batch: list[dict]) -> list[tuple]:
        texts = [
            _truncate_text(build_embed_text(e, e.get("source_type") or e["entry_type"]))
            for e in batch
        ]
        # Skip entries with empty text
        valid = [(e, t) for e, t in zip(batch, texts) if t.strip()]
        if not valid:
            return []

        valid_entries, valid_texts = zip(*valid)

        for attempt in range(10):
            try:
                async with semaphore:
                    response = await client.embeddings.create(
                        model=EMBED_MODEL,
                        input=list(valid_texts),
                        dimensions=EMBED_DIMS,
                    )
                return [
                    (str(e["id"]), data.embedding)
                    for e, data in zip(valid_entries, response.data)
                ]
            except openai.RateLimitError:
                wait = 2 ** attempt
                typer.echo(f"Rate limited, waiting {wait}s...")
                await asyncio.sleep(wait)
            except openai.APIConnectionError as exc:
                typer.echo(f"OpenAI unreachable: {exc}", err=True)
                raise typer.Exit(1) from exc

        raise typer.Exit(1)

    # Process in batches
    batches = [entries[i:i+batch_size] for i in range(0, len(entries), batch_size)]
    results: list[tuple] = []
    for batch in tqdm(batches, desc="Batches"):
        results.extend(await embed_batch(batch))

    # Write to DB
    typer.echo(f"Writing {len(results)} embeddings to database...")
    async with get_conn(db_url) as conn:
        async with conn.pipeline():
            for entry_id, vector in results:
                await conn.execute(
                    """
                    INSERT INTO entry_embeddings (entry_id, model, vector)
                    VALUES (%s, %s, %s::halfvec)
                    ON CONFLICT (entry_id) DO UPDATE SET vector = EXCLUDED.vector,
                                                         embedded_at = NOW()
                    """,
                    (entry_id, EMBED_MODEL, str(vector)),
                )

    typer.echo(f"Done. {len(results)} embeddings written.")
```

- [ ] **Step 3: Verify CLI is importable**

```bash
cd tools/bdp-embed
python -c "from bdp_embed.embed import embed; print('OK')"
```

Expected: `OK`

- [ ] **Step 4: Commit**

```bash
git add tools/bdp-embed/bdp_embed/db.py tools/bdp-embed/bdp_embed/embed.py
git commit -m "feat(bdp-embed): add embed subcommand with OpenAI batching and incremental writes"
```

---

### Task 5: project subcommand (Landmark UMAP → entry_projections)

**Files:**
- Create: `tools/bdp-embed/bdp_embed/project.py`

- [ ] **Step 1: Write project.py**

```python
# tools/bdp-embed/bdp_embed/project.py
import asyncio
import uuid
from typing import Annotated
import psycopg
import numpy as np
import joblib
import boto3
import typer
import umap
from sklearn.cluster import MiniBatchKMeans
from tqdm import tqdm
from bdp_embed.cli import app
from bdp_embed.db import get_conn


@app.command()
def project(
    db_url: Annotated[str, typer.Option(envvar="DATABASE_URL")],
    run_id: Annotated[str, typer.Option(help="Run ID from vector_projection_runs")],
    s3_bucket: Annotated[str, typer.Option(envvar="S3_BUCKET", default_factory=lambda: "bdp")],
    s3_endpoint: Annotated[str, typer.Option(envvar="S3_ENDPOINT_URL", default_factory=lambda: "")],
    landmarks: int = 50000,
):
    """Project embeddings to 2D using landmark UMAP. Saves model to MinIO."""
    asyncio.run(_project(db_url, run_id, s3_bucket, s3_endpoint, landmarks))


async def _project(db_url: str, run_id: str, s3_bucket: str, s3_endpoint: str, n_landmarks: int):
    # Update status
    async with get_conn(db_url) as conn:
        await conn.execute(
            "UPDATE vector_projection_runs SET status='projecting' WHERE run_id=%s",
            (run_id,),
        )

    typer.echo("Loading vectors from database...")
    async with get_conn(db_url) as conn:
        rows = await conn.execute(
            """
            SELECT e.entry_id::text, e.vector::text,
                   re.name as label, re.entry_type, re.slug,
                   ds.source_type, o.slug as org_slug
            FROM entry_embeddings e
            JOIN registry_entries re ON re.id = e.entry_id
            JOIN organizations o ON o.id = re.organization_id
            LEFT JOIN data_sources ds ON ds.id = re.id
            ORDER BY e.embedded_at
            """,
            row_factory=psycopg.rows.dict_row,
        )
        all_rows = await rows.fetchall()

    if not all_rows:
        typer.echo("No embeddings found — run `bdp-embed embed` first.", err=True)
        raise typer.Exit(1)

    typer.echo(f"Loaded {len(all_rows)} vectors. Preparing for UMAP...")

    entry_ids = [r["entry_id"] for r in all_rows]
    vectors = np.array([
        list(map(float, r["vector"].strip("[]").split(",")))
        for r in all_rows
    ], dtype=np.float32)

    # Check if a prior model exists for this run (restart support)
    s3 = boto3.client("s3", endpoint_url=s3_endpoint or None)
    model_key = f"vectors/models/{run_id}/umap.joblib"

    umap_model = None
    try:
        s3.head_object(Bucket=s3_bucket, Key=model_key)
        typer.echo("Found existing UMAP model, downloading...")
        s3.download_file(s3_bucket, model_key, "/tmp/umap.joblib")
        umap_model = joblib.load("/tmp/umap.joblib")
        typer.echo("Reusing existing model (coordinate-stable).")
    except Exception:
        typer.echo(f"Fitting UMAP on {min(n_landmarks, len(vectors))} landmarks...")

        # Select landmarks via k-means centroids
        k = min(n_landmarks, len(vectors))
        kmeans = MiniBatchKMeans(n_clusters=k, random_state=42, n_init=3)
        kmeans.fit(vectors)
        landmark_indices = [
            np.argmin(np.linalg.norm(vectors - c, axis=1))
            for c in tqdm(kmeans.cluster_centers_, desc="Finding landmarks")
        ]
        landmarks = vectors[landmark_indices]

        umap_model = umap.UMAP(n_components=2, random_state=42, low_memory=True)
        umap_model.fit(landmarks)

        # Save model to MinIO for coordinate stability on future runs
        joblib.dump(umap_model, "/tmp/umap.joblib")
        s3.upload_file("/tmp/umap.joblib", s3_bucket, model_key)
        typer.echo(f"UMAP model saved to s3://{s3_bucket}/{model_key}")

    # Project all points onto the fixed scaffold
    typer.echo(f"Projecting {len(vectors)} points...")
    coords = umap_model.transform(vectors)

    # Write to entry_projections
    typer.echo("Writing projections to database...")
    async with get_conn(db_url) as conn:
        async with conn.pipeline():
            for i, row in enumerate(tqdm(all_rows, desc="Writing")):
                await conn.execute(
                    """
                    INSERT INTO entry_projections
                        (entry_id, x, y, label, entry_type, source_type, org_slug, slug)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                    ON CONFLICT (entry_id) DO UPDATE
                        SET x=EXCLUDED.x, y=EXCLUDED.y, projected_at=NOW()
                    """,
                    (
                        row["entry_id"],
                        float(coords[i, 0]),
                        float(coords[i, 1]),
                        row["label"] or row["slug"],
                        row["entry_type"],
                        row.get("source_type"),
                        row["org_slug"],
                        row["slug"],
                    ),
                )
        await conn.execute(
            """
            UPDATE vector_projection_runs
            SET status='tiling', stage_completed='project',
                projected_count=%s, projected_at=NOW()
            WHERE run_id=%s
            """,
            (len(all_rows), run_id),
        )

    typer.echo(f"Projection complete. {len(all_rows)} entries projected.")
```

- [ ] **Step 2: Verify import**

```bash
cd tools/bdp-embed
python -c "from bdp_embed.project import project; print('OK')"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add tools/bdp-embed/bdp_embed/project.py
git commit -m "feat(bdp-embed): add project subcommand with landmark UMAP and model persistence"
```

---

### Task 6: tiles subcommand (quadtree → MinIO)

**Files:**
- Create: `tools/bdp-embed/bdp_embed/tiles.py`
- Create: `tools/bdp-embed/tests/test_tiles.py`

- [ ] **Step 1: Write failing test for quadtree logic**

```python
# tools/bdp-embed/tests/test_tiles.py
import json
from bdp_embed.tiles import build_quadtree, get_tile_key, points_in_bounds


def make_point(x, y, i=0):
    return {"id": str(i), "x": x, "y": y, "l": f"P{i}", "et": "data_source",
            "st": "protein", "org": "uniprot", "slug": f"p{i}"}


def test_points_in_bounds_filters_correctly():
    pts = [make_point(1.0, 1.0), make_point(5.0, 5.0), make_point(-1.0, -1.0)]
    result = points_in_bounds(pts, x_min=0, x_max=3, y_min=0, y_max=3)
    assert len(result) == 1
    assert result[0]["x"] == 1.0


def test_get_tile_key_format():
    key = get_tile_key("abc123", z=3, tx=2, ty=1)
    assert key == "vectors/tiles/abc123/3/2/1.json"


def test_build_quadtree_returns_nonempty_tiles():
    pts = [make_point(float(i % 10), float(i // 10), i) for i in range(100)]
    tiles = build_quadtree(pts, run_id="test", zoom_min=0, zoom_max=3)
    # At least one tile at zoom 0
    assert any(t["z"] == 0 for t in tiles)
    # All tile keys end in .json
    assert all(t["key"].endswith(".json") for t in tiles)


def test_build_quadtree_coarse_tiles_have_fewer_points():
    pts = [make_point(float(i % 10), float(i // 10), i) for i in range(1000)]
    tiles = build_quadtree(pts, run_id="test", zoom_min=0, zoom_max=5)
    zoom0_tiles = [t for t in tiles if t["z"] == 0]
    zoom5_tiles = [t for t in tiles if t["z"] == 5]
    zoom0_count = sum(len(t["points"]) for t in zoom0_tiles)
    zoom5_count = sum(len(t["points"]) for t in zoom5_tiles)
    assert zoom0_count <= zoom5_count
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cd tools/bdp-embed
python -m pytest tests/test_tiles.py -v
```

Expected: ImportError — `tiles` module not found.

- [ ] **Step 3: Write tiles.py**

```python
# tools/bdp-embed/bdp_embed/tiles.py
import asyncio
import json
import io
from typing import Annotated
import psycopg
import numpy as np
import boto3
import typer
from tqdm import tqdm
from bdp_embed.cli import app
from bdp_embed.db import get_conn


def get_tile_key(run_id: str, z: int, tx: int, ty: int) -> str:
    return f"vectors/tiles/{run_id}/{z}/{tx}/{ty}.json"


def points_in_bounds(
    points: list[dict],
    x_min: float, x_max: float,
    y_min: float, y_max: float,
) -> list[dict]:
    return [
        p for p in points
        if x_min <= p["x"] < x_max and y_min <= p["y"] < y_max
    ]


def build_quadtree(
    points: list[dict],
    run_id: str,
    zoom_min: int = 0,
    zoom_max: int = 14,
) -> list[dict]:
    """Build quadtree tiles over projected 2D points.

    Returns list of dicts: {"key": str, "z": int, "points": list[dict]}
    Empty tiles are NOT included (404 = no points in cell).
    """
    if not points:
        return []

    xs = np.array([p["x"] for p in points])
    ys = np.array([p["y"] for p in points])
    x_min, x_max = float(xs.min()), float(xs.max())
    y_min, y_max = float(ys.min()), float(ys.max())

    # Add small padding
    pad_x = (x_max - x_min) * 0.01 or 1.0
    pad_y = (y_max - y_min) * 0.01 or 1.0
    x_min -= pad_x; x_max += pad_x
    y_min -= pad_y; y_max += pad_y

    tiles = []

    # Convert to numpy arrays for vectorized cell assignment (avoids O(N×cells) scan)
    all_xs = np.array([p["x"] for p in points])
    all_ys = np.array([p["y"] for p in points])

    for z in range(zoom_min, zoom_max + 1):
        n_cells = 2 ** z
        cell_w = (x_max - x_min) / n_cells
        cell_h = (y_max - y_min) / n_cells

        # Vectorized cell index assignment for every point at this zoom level
        tx_indices = np.clip(((all_xs - x_min) / cell_w).astype(int), 0, n_cells - 1)
        ty_indices = np.clip(((all_ys - y_min) / cell_h).astype(int), 0, n_cells - 1)

        # Downsample factor: show 1 per cluster at low zoom, all at high zoom
        max_per_cell = max(1, len(points) // (4 ** z)) if z < 8 else len(points)

        # Group point indices by (tx, ty) cell
        from collections import defaultdict
        cell_map: dict[tuple[int, int], list[int]] = defaultdict(list)
        for idx in range(len(points)):
            cell_map[(int(tx_indices[idx]), int(ty_indices[idx]))].append(idx)

        for (tx, ty), idx_list in cell_map.items():
            selected = [points[i] for i in idx_list[:max_per_cell]]
            tiles.append({
                "key": get_tile_key(run_id, z, tx, ty),
                "z": z,
                "points": selected,
            })

    return tiles


@app.command()
def tiles(
    db_url: Annotated[str, typer.Option(envvar="DATABASE_URL")],
    run_id: Annotated[str, typer.Option(help="Run ID from vector_projection_runs")],
    s3_bucket: Annotated[str, typer.Option(envvar="S3_BUCKET", default_factory=lambda: "bdp")],
    s3_endpoint: Annotated[str, typer.Option(envvar="S3_ENDPOINT_URL", default_factory=lambda: "")],
    zoom_min: int = 0,
    zoom_max: int = 14,
):
    """Build quadtree tile files from entry_projections and upload to MinIO."""
    asyncio.run(_tiles(db_url, run_id, s3_bucket, s3_endpoint, zoom_min, zoom_max))


async def _tiles(
    db_url: str, run_id: str, s3_bucket: str, s3_endpoint: str,
    zoom_min: int, zoom_max: int,
):
    typer.echo("Loading projections from database...")
    async with get_conn(db_url) as conn:
        rows = await conn.execute(
            """
            SELECT entry_id::text as id, x, y,
                   label as l, entry_type as et,
                   COALESCE(source_type, '') as st,
                   org_slug as org, slug
            FROM entry_projections
            ORDER BY entry_id
            """,
            row_factory=psycopg.rows.dict_row,
        )
        points = [dict(r) for r in await rows.fetchall()]

    if not points:
        typer.echo("No projections found — run `bdp-embed project` first.", err=True)
        raise typer.Exit(1)

    typer.echo(f"Building quadtree for {len(points)} points (zoom {zoom_min}-{zoom_max})...")
    tile_list = build_quadtree(points, run_id=run_id, zoom_min=zoom_min, zoom_max=zoom_max)

    typer.echo(f"Uploading {len(tile_list)} tiles to s3://{s3_bucket}/...")
    s3 = boto3.client("s3", endpoint_url=s3_endpoint or None)
    tile_prefix = f"vectors/tiles/{run_id}/"

    for tile in tqdm(tile_list, desc="Uploading tiles"):
        body = json.dumps(tile["points"], separators=(",", ":")).encode()
        s3.put_object(
            Bucket=s3_bucket,
            Key=tile["key"],
            Body=io.BytesIO(body),
            ContentType="application/json",
        )

    # Mark run as complete
    async with get_conn(db_url) as conn:
        await conn.execute(
            """
            UPDATE vector_projection_runs
            SET status='complete', stage_completed='tiles',
                tile_prefix=%s, completed_at=NOW()
            WHERE run_id=%s
            """,
            (tile_prefix, run_id),
        )

    typer.echo(f"Done. {len(tile_list)} tiles uploaded to {tile_prefix}.")
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cd tools/bdp-embed
python -m pytest tests/test_tiles.py -v
```

Expected: all 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add tools/bdp-embed/bdp_embed/tiles.py tools/bdp-embed/tests/test_tiles.py
git commit -m "feat(bdp-embed): add tiles subcommand with quadtree build and MinIO upload"
```

---

## Phase C: Rust Backend API

### Task 7: Add Rust dependencies

**Files:**
- Modify: `crates/bdp-server/Cargo.toml`

- [ ] **Step 1: Add new dependencies**

In `crates/bdp-server/Cargo.toml`, add under the `# Utilities` section:

```toml
# ============================================================================
# Vector Search
# ============================================================================
pgvector = { version = "0.4", features = ["sqlx"] }
async-openai = "0.27"
moka = { version = "0.12", features = ["future"] }
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build -p bdp-server 2>&1 | head -30
```

Expected: compiles without errors (may warn about unused imports, ignore for now).

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-server/Cargo.toml
git commit -m "chore(server): add pgvector, async-openai, moka dependencies"
```

---

### Task 8: get_stats query

**Files:**
- Create: `crates/bdp-server/src/features/vectors/queries/get_stats.rs`
- Create: `crates/bdp-server/src/features/vectors/queries/mod.rs`
- Create: `crates/bdp-server/src/features/vectors/mod.rs`

Start with `get_stats` — it's the simplest query and validates the scaffolding.

- [ ] **Step 1: Write the query struct and handler**

```rust
// crates/bdp-server/src/features/vectors/queries/get_stats.rs
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVectorStatsQuery;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStatsResponse {
    /// UUID of the most recent complete projection run, or null
    pub current_run_id: Option<String>,
    /// Current pipeline status
    pub status: Option<String>,
    /// Total registry entries
    pub entry_count: Option<i64>,
    /// Entries with embeddings
    pub embedded_count: Option<i64>,
    /// Entries with 2D projection coords
    pub projected_count: Option<i64>,
    /// When the last projection completed
    pub projected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// MinIO tile prefix for the current run
    pub tile_prefix: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetVectorStatsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<VectorStatsResponse, GetVectorStatsError>> for GetVectorStatsQuery {}
impl crate::cqrs::middleware::Query for GetVectorStatsQuery {}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    _query: GetVectorStatsQuery,
) -> Result<VectorStatsResponse, GetVectorStatsError> {
    // Get most recent run
    let run = sqlx::query!(
        r#"
        SELECT run_id::text, status, entry_count, embedded_count,
               projected_count, projected_at, tile_prefix
        FROM vector_projection_runs
        ORDER BY started_at DESC
        LIMIT 1
        "#
    )
    .fetch_optional(&pool)
    .await?;

    // Total entry count (fast, from registry_entries)
    let total_entries = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM registry_entries"
    )
    .fetch_one(&pool)
    .await?;

    // Embedded count
    let embedded_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM entry_embeddings"
    )
    .fetch_one(&pool)
    .await?;

    Ok(VectorStatsResponse {
        current_run_id: run.as_ref().map(|r| r.run_id.clone().unwrap_or_default()),
        status: run.as_ref().map(|r| r.status.clone()),
        entry_count: total_entries,
        embedded_count,
        projected_count: run.as_ref().and_then(|r| r.projected_count),
        projected_at: run.as_ref().and_then(|r| r.projected_at),
        tile_prefix: run.as_ref().and_then(|r| r.tile_prefix.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_stats_returns_nulls_with_no_data(pool: PgPool) -> sqlx::Result<()> {
        let result = handle(pool, GetVectorStatsQuery).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.current_run_id.is_none());
        assert!(stats.entry_count.unwrap_or(0) == 0);
        Ok(())
    }
}
```

- [ ] **Step 2: Write the queries mod.rs**

```rust
// crates/bdp-server/src/features/vectors/queries/mod.rs
pub mod get_stats;
pub mod semantic_search;
pub mod get_neighbors;
pub mod get_tile;

pub use get_stats::{GetVectorStatsError, GetVectorStatsQuery, VectorStatsResponse};
pub use semantic_search::{SemanticSearchError, SemanticSearchQuery, SemanticSearchResponse};
pub use get_neighbors::{GetNeighborsError, GetNeighborsQuery, GetNeighborsResponse};
pub use get_tile::{GetTileError, GetTileQuery, TileResponse};
```

- [ ] **Step 3: Write the vectors mod.rs**

```rust
// crates/bdp-server/src/features/vectors/mod.rs
pub mod queries;
pub mod routes;

pub use routes::vectors_routes;
```

- [ ] **Step 4: Run the unit test**

```bash
cargo test -p bdp-server features::vectors::queries::get_stats -- --nocapture
```

Expected: `test_stats_returns_nulls_with_no_data` PASSES.

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-server/src/features/vectors/
git commit -m "feat(vectors): add get_stats query and vectors feature module skeleton"
```

---

### Task 9: semantic_search and get_neighbors queries

**Files:**
- Create: `crates/bdp-server/src/features/vectors/queries/semantic_search.rs`
- Create: `crates/bdp-server/src/features/vectors/queries/get_neighbors.rs`

- [ ] **Step 1: Write semantic_search.rs**

```rust
// crates/bdp-server/src/features/vectors/queries/semantic_search.rs
use mediator::Request;
use moka::future::Cache;
use once_cell::sync::Lazy;
use pgvector::HalfVector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// In-process LRU cache: query string → halfvec(512)
// 128 entries × ~1KB each ≈ 128KB
static EMBED_CACHE: Lazy<Cache<String, Arc<Vec<f32>>>> = Lazy::new(|| {
    Cache::new(128)
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchQuery {
    pub q: String,
    #[serde(default = "default_k")]
    pub k: i64,
}

fn default_k() -> i64 { 20 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchItem {
    pub entry_id: Uuid,
    pub slug: String,
    pub name: String,
    pub entry_type: String,
    pub source_type: Option<String>,
    pub org_slug: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResponse {
    pub items: Vec<SemanticSearchItem>,
}

#[derive(Debug, thiserror::Error)]
pub enum SemanticSearchError {
    #[error("Query is required")]
    QueryEmpty,
    #[error("k must be between 1 and 100")]
    InvalidK,
    #[error("Embedding service unavailable: {0}")]
    EmbeddingUnavailable(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<SemanticSearchResponse, SemanticSearchError>> for SemanticSearchQuery {}
impl crate::cqrs::middleware::Query for SemanticSearchQuery {}

impl SemanticSearchQuery {
    pub fn validate(&self) -> Result<(), SemanticSearchError> {
        if self.q.trim().is_empty() {
            return Err(SemanticSearchError::QueryEmpty);
        }
        if !(1..=100).contains(&self.k) {
            return Err(SemanticSearchError::InvalidK);
        }
        Ok(())
    }
}

/// Embed a query string via OpenAI, using the in-process cache.
async fn embed_query(q: &str) -> Result<HalfVector, SemanticSearchError> {
    let cache_key = q.to_lowercase();

    if let Some(cached) = EMBED_CACHE.get(&cache_key).await {
        let hv = HalfVector::from(cached.as_slice().iter().map(|&f| f as f32).collect::<Vec<_>>());
        return Ok(hv);
    }

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let client = async_openai::Client::new().with_api_key(api_key);

    let request = async_openai::types::CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(q)
        .dimensions(512u32)
        .build()
        .map_err(|e| SemanticSearchError::EmbeddingUnavailable(e.to_string()))?;

    let response = client
        .embeddings()
        .create(request)
        .await
        .map_err(|e| SemanticSearchError::EmbeddingUnavailable(e.to_string()))?;

    let floats: Vec<f32> = response.data[0].embedding.iter().map(|&f| f as f32).collect();
    EMBED_CACHE.insert(cache_key, Arc::new(floats.clone())).await;

    Ok(HalfVector::from(floats))
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: SemanticSearchQuery,
) -> Result<SemanticSearchResponse, SemanticSearchError> {
    query.validate()?;

    let vector = embed_query(&query.q).await?;

    let rows = sqlx::query!(
        r#"
        SELECT
            e.entry_id               AS "entry_id!: Uuid",
            re.slug                  AS "slug!",
            re.name                  AS "name!",
            re.entry_type            AS "entry_type!",
            ds.source_type           AS "source_type?",
            o.slug                   AS "org_slug!",
            ep.x                     AS "x?: f32",
            ep.y                     AS "y?: f32",
            (1.0 - (e.vector <=> $1::halfvec))::float4 AS "similarity!"
        FROM entry_embeddings e
        JOIN registry_entries re ON re.id = e.entry_id
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        LEFT JOIN entry_projections ep ON ep.entry_id = e.entry_id
        ORDER BY e.vector <=> $1::halfvec
        LIMIT $2
        "#,
        vector as HalfVector,
        query.k,
    )
    .fetch_all(&pool)
    .await?;

    Ok(SemanticSearchResponse {
        items: rows.into_iter().map(|r| SemanticSearchItem {
            entry_id: r.entry_id,
            slug: r.slug,
            name: r.name,
            entry_type: r.entry_type,
            source_type: r.source_type,
            org_slug: r.org_slug,
            x: r.x,
            y: r.y,
            similarity: r.similarity,
        }).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_query() {
        let q = SemanticSearchQuery { q: "".to_string(), k: 20 };
        assert!(matches!(q.validate(), Err(SemanticSearchError::QueryEmpty)));
    }

    #[test]
    fn test_validate_invalid_k() {
        let q = SemanticSearchQuery { q: "insulin".to_string(), k: 0 };
        assert!(matches!(q.validate(), Err(SemanticSearchError::InvalidK)));
        let q2 = SemanticSearchQuery { q: "insulin".to_string(), k: 101 };
        assert!(matches!(q2.validate(), Err(SemanticSearchError::InvalidK)));
    }

    #[test]
    fn test_validate_ok() {
        let q = SemanticSearchQuery { q: "insulin".to_string(), k: 10 };
        assert!(q.validate().is_ok());
    }
}
```

- [ ] **Step 2: Write get_neighbors.rs**

```rust
// crates/bdp-server/src/features/vectors/queries/get_neighbors.rs
use mediator::Request;
use pgvector::HalfVector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::semantic_search::SemanticSearchItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNeighborsQuery {
    pub entry_id: Uuid,
    #[serde(default = "default_k")]
    pub k: i64,
}

fn default_k() -> i64 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNeighborsResponse {
    pub neighbors: Vec<SemanticSearchItem>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetNeighborsError {
    #[error("Entry not found or has no embedding")]
    NotFound,
    #[error("k must be between 1 and 100")]
    InvalidK,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<GetNeighborsResponse, GetNeighborsError>> for GetNeighborsQuery {}
impl crate::cqrs::middleware::Query for GetNeighborsQuery {}

impl GetNeighborsQuery {
    pub fn validate(&self) -> Result<(), GetNeighborsError> {
        if !(1..=100).contains(&self.k) {
            return Err(GetNeighborsError::InvalidK);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetNeighborsQuery,
) -> Result<GetNeighborsResponse, GetNeighborsError> {
    query.validate()?;

    // Fetch seed vector
    let seed = sqlx::query_scalar!(
        r#"SELECT vector AS "vector!: HalfVector" FROM entry_embeddings WHERE entry_id = $1"#,
        query.entry_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(GetNeighborsError::NotFound)?;

    // KNN excluding self
    let rows = sqlx::query!(
        r#"
        SELECT
            e.entry_id               AS "entry_id!: Uuid",
            re.slug                  AS "slug!",
            re.name                  AS "name!",
            re.entry_type            AS "entry_type!",
            ds.source_type           AS "source_type?",
            o.slug                   AS "org_slug!",
            ep.x                     AS "x?: f32",
            ep.y                     AS "y?: f32",
            (1.0 - (e.vector <=> $1::halfvec))::float4 AS "similarity!"
        FROM entry_embeddings e
        JOIN registry_entries re ON re.id = e.entry_id
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        LEFT JOIN entry_projections ep ON ep.entry_id = e.entry_id
        WHERE e.entry_id != $2
        ORDER BY e.vector <=> $1::halfvec
        LIMIT $3
        "#,
        seed as HalfVector,
        query.entry_id,
        query.k,
    )
    .fetch_all(&pool)
    .await?;

    Ok(GetNeighborsResponse {
        neighbors: rows.into_iter().map(|r| SemanticSearchItem {
            entry_id: r.entry_id,
            slug: r.slug,
            name: r.name,
            entry_type: r.entry_type,
            source_type: r.source_type,
            org_slug: r.org_slug,
            x: r.x,
            y: r.y,
            similarity: r.similarity,
        }).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_k() {
        let q = GetNeighborsQuery { entry_id: Uuid::new_v4(), k: 0 };
        assert!(matches!(q.validate(), Err(GetNeighborsError::InvalidK)));
    }
}
```

- [ ] **Step 3: Run unit tests**

```bash
cargo test -p bdp-server features::vectors::queries -- --nocapture
```

Expected: validation tests PASS. (Integration tests need a real DB — skip for now.)

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-server/src/features/vectors/queries/semantic_search.rs \
        crates/bdp-server/src/features/vectors/queries/get_neighbors.rs
git commit -m "feat(vectors): add semantic_search and get_neighbors queries"
```

---

### Task 10: get_tile handler + routes + mediator registration

**Files:**
- Create: `crates/bdp-server/src/features/vectors/queries/get_tile.rs`
- Create: `crates/bdp-server/src/features/vectors/routes.rs`
- Modify: `crates/bdp-server/src/features/mod.rs`
- Modify: `crates/bdp-server/src/cqrs/mod.rs`

- [ ] **Step 1: Write get_tile.rs**

```rust
// crates/bdp-server/src/features/vectors/queries/get_tile.rs
use mediator::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTileQuery {
    pub run_id: String,
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone)]
pub struct TileResponse {
    pub body: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetTileError {
    #[error("Tile not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(String),
}

impl Request<Result<TileResponse, GetTileError>> for GetTileQuery {}
impl crate::cqrs::middleware::Query for GetTileQuery {}

#[tracing::instrument(skip(storage))]
pub async fn handle(
    storage: crate::storage::Storage,
    query: GetTileQuery,
) -> Result<TileResponse, GetTileError> {
    let key = format!(
        "vectors/tiles/{}/{}/{}/{}.json",
        query.run_id, query.z, query.x, query.y
    );

    storage
        .get_bytes(&key)
        .await
        .map(|body| TileResponse { body })
        .map_err(|e| {
            if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                GetTileError::NotFound
            } else {
                GetTileError::Storage(e.to_string())
            }
        })
}
```

- [ ] **Step 2: Check the Storage API** (read the storage module to confirm `get_bytes` exists or adapt)

```bash
grep -r "get_bytes\|get_object\|download" crates/bdp-server/src/storage/ --include="*.rs" -l
```

Adapt the `handle` function to use whatever storage retrieval method exists. The key point is fetching raw bytes from MinIO by object key.

- [ ] **Step 3: Write routes.rs**

```rust
// crates/bdp-server/src/features/vectors/routes.rs
use crate::api::response::{ApiResponse, ErrorResponse};
use crate::features::FeatureState;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use super::queries::{
    GetNeighborsError, GetNeighborsQuery, GetTileError, GetTileQuery,
    GetVectorStatsQuery, SemanticSearchError, SemanticSearchQuery,
};

pub fn vectors_routes() -> Router<FeatureState> {
    Router::new()
        .route("/stats", get(get_stats))
        .route("/search", get(semantic_search))
        .route("/{entry_id}/neighbors", get(get_neighbors))
        .route("/tiles/{run_id}/{z}/{x}/{y}", get(get_tile))
}

async fn get_stats(State(state): State<FeatureState>) -> Response {
    let result = state.dispatch(GetVectorStatsQuery).await;
    match result {
        Ok(stats) => (StatusCode::OK, Json(ApiResponse::success(stats))).into_response(),
        Err(e) => {
            tracing::error!("get_stats error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to fetch stats"))).into_response()
        }
    }
}

async fn semantic_search(
    State(state): State<FeatureState>,
    Query(query): Query<SemanticSearchQuery>,
) -> Response {
    match state.dispatch(query).await {
        Ok(resp) => (StatusCode::OK, Json(ApiResponse::success(resp.items))).into_response(),
        Err(SemanticSearchError::QueryEmpty) | Err(SemanticSearchError::InvalidK) => {
            (StatusCode::BAD_REQUEST,
             Json(ErrorResponse::new("VALIDATION_ERROR", "Invalid query parameters"))).into_response()
        }
        Err(SemanticSearchError::EmbeddingUnavailable(msg)) => {
            tracing::warn!("Embedding service unavailable: {}", msg);
            (StatusCode::SERVICE_UNAVAILABLE,
             Json(ErrorResponse::new("SERVICE_UNAVAILABLE", "Embedding service unavailable"))).into_response()
        }
        Err(e) => {
            tracing::error!("semantic_search error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(ErrorResponse::new("INTERNAL_ERROR", "Search failed"))).into_response()
        }
    }
}

async fn get_neighbors(
    State(state): State<FeatureState>,
    Path(entry_id): Path<uuid::Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let k = params.get("k").and_then(|v| v.parse().ok()).unwrap_or(10);
    let query = GetNeighborsQuery { entry_id, k };
    match state.dispatch(query).await {
        Ok(resp) => (StatusCode::OK, Json(ApiResponse::success(resp.neighbors))).into_response(),
        Err(GetNeighborsError::NotFound) =>
            (StatusCode::NOT_FOUND,
             Json(ErrorResponse::new("NOT_FOUND", "Entry has no embedding"))).into_response(),
        Err(e) => {
            tracing::error!("get_neighbors error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(ErrorResponse::new("INTERNAL_ERROR", "Neighbor lookup failed"))).into_response()
        }
    }
}

async fn get_tile(
    State(state): State<FeatureState>,
    Path((run_id, z, x, y)): Path<(String, u32, u32, u32)>,
) -> Response {
    let query = GetTileQuery { run_id, z, x, y };
    match state.dispatch(query).await {
        Ok(tile) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json"),
             (header::CACHE_CONTROL, "public, max-age=86400, immutable")],
            Body::from(tile.body),
        ).into_response(),
        Err(GetTileError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_tile error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

- [ ] **Step 4: Register the module in features/mod.rs**

Add to `crates/bdp-server/src/features/mod.rs`:

```rust
pub mod vectors;
```

And inside the `router()` function add:

```rust
.nest("/vectors", vectors::vectors_routes().with_state(state.clone()))
```

- [ ] **Step 5: Register handlers in cqrs/mod.rs**

Add to the `build_mediator` function (after the search handlers section):

```rust
// ================================================================
// Vectors
// ================================================================
.add_handler({
    let pool = pool.clone();
    move |query| {
        let pool = pool.clone();
        async move { crate::features::vectors::queries::get_stats::handle(pool, query).await }
    }
})
.add_handler({
    let pool = pool.clone();
    move |query| {
        let pool = pool.clone();
        async move { crate::features::vectors::queries::semantic_search::handle(pool, query).await }
    }
})
.add_handler({
    let pool = pool.clone();
    move |query| {
        let pool = pool.clone();
        async move { crate::features::vectors::queries::get_neighbors::handle(pool, query).await }
    }
})
.add_handler({
    let storage = storage.clone();
    move |query| {
        let storage = storage.clone();
        async move { crate::features::vectors::queries::get_tile::handle(storage, query).await }
    }
})
```

- [ ] **Step 6: Build and verify**

```bash
cargo build -p bdp-server 2>&1 | grep -E "error|warning: unused"
```

Expected: builds cleanly (no errors). Fix any compilation errors before proceeding.

- [ ] **Step 7: Regenerate SQLx metadata**

```bash
cargo xtask sqlx prepare
```

- [ ] **Step 8: Run all server tests**

```bash
cargo test -p bdp-server 2>&1 | tail -20
```

Expected: existing tests still pass. New vector unit tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/bdp-server/src/features/vectors/ \
        crates/bdp-server/src/features/mod.rs \
        crates/bdp-server/src/cqrs/mod.rs
git commit -m "feat(vectors): add get_tile, routes, and register all vector handlers in mediator"
```

---

## Phase D: Frontend

### Task 11: Source type colors + tile loader

**Files:**
- Create: `web/lib/source-type-colors.ts`
- Create: `web/lib/vectors/tile-loader.ts`

- [ ] **Step 1: Write source-type-colors.ts**

```typescript
// web/lib/source-type-colors.ts

export const SOURCE_TYPE_COLORS: Record<string, string> = {
  protein:             '#3b82f6',
  genome:              '#22c55e',
  annotation:          '#f97316',
  structure:           '#06b6d4',
  predicted_structure: '#0891b2',
  taxonomy:            '#a855f7',
  transcript:          '#84cc16',
  domain:              '#f59e0b',
  ontology_term:       '#8b5cf6',
  pathway:             '#10b981',
  interaction:         '#ef4444',
  variant:             '#f43f5e',
  compound:            '#d946ef',
  expression:          '#14b8a6',
  metagenome:          '#78716c',
  literature:          '#e2e8f0',
  tool:                '#64748b',
};

export const DEFAULT_POINT_COLOR = '#94a3b8';

export function getSourceTypeColor(sourceType: string | null | undefined): string {
  if (!sourceType) return DEFAULT_POINT_COLOR;
  return SOURCE_TYPE_COLORS[sourceType] ?? DEFAULT_POINT_COLOR;
}
```

- [ ] **Step 2: Write tile-loader.ts**

```typescript
// web/lib/vectors/tile-loader.ts

const API_BASE = '/api/v1/vectors';

export interface TilePoint {
  id:   string;
  x:    number;
  y:    number;
  l:    string;   // label
  et:   string;   // entry_type
  st:   string;   // source_type ('' if null)
  org:  string;
  slug: string;
}

export interface VectorStats {
  current_run_id:  string | null;
  status:          string | null;
  entry_count:     number | null;
  embedded_count:  number | null;
  projected_count: number | null;
  projected_at:    string | null;
  tile_prefix:     string | null;
}

// In-session tile cache — avoids re-fetching on pan-back
const tileCache = new Map<string, TilePoint[]>();

export async function fetchStats(): Promise<VectorStats> {
  const res = await fetch(`${API_BASE}/stats`);
  if (!res.ok) throw new Error(`Stats fetch failed: ${res.status}`);
  const json = await res.json();
  return json.data as VectorStats;
}

export async function fetchTile(
  runId: string,
  z: number,
  tx: number,
  ty: number,
): Promise<TilePoint[]> {
  const key = `${runId}/${z}/${tx}/${ty}`;
  if (tileCache.has(key)) return tileCache.get(key)!;

  const res = await fetch(`${API_BASE}/tiles/${runId}/${z}/${tx}/${ty}`);
  if (res.status === 404) {
    tileCache.set(key, []);
    return [];
  }
  if (!res.ok) throw new Error(`Tile fetch failed: ${res.status}`);

  const points: TilePoint[] = await res.json();
  tileCache.set(key, points);
  return points;
}

/** Fetch all tiles for the current viewport at a given zoom level. */
export async function fetchViewportTiles(
  runId: string,
  zoom: number,
  xMin: number, xMax: number,
  yMin: number, yMax: number,
  totalBounds: { x: [number, number]; y: [number, number] },
): Promise<TilePoint[]> {
  const nCells = Math.pow(2, zoom);
  const cellW = (totalBounds.x[1] - totalBounds.x[0]) / nCells;
  const cellH = (totalBounds.y[1] - totalBounds.y[0]) / nCells;

  const txMin = Math.max(0, Math.floor((xMin - totalBounds.x[0]) / cellW));
  const txMax = Math.min(nCells - 1, Math.floor((xMax - totalBounds.x[0]) / cellW));
  const tyMin = Math.max(0, Math.floor((yMin - totalBounds.y[0]) / cellH));
  const tyMax = Math.min(nCells - 1, Math.floor((yMax - totalBounds.y[0]) / cellH));

  const fetches: Promise<TilePoint[]>[] = [];
  for (let tx = txMin; tx <= txMax; tx++) {
    for (let ty = tyMin; ty <= tyMax; ty++) {
      fetches.push(fetchTile(runId, zoom, tx, ty));
    }
  }

  const results = await Promise.all(fetches);
  return results.flat();
}

export async function fetchSemanticSearch(
  q: string,
  k = 20,
): Promise<Array<{ slug: string; name: string; org_slug: string; x?: number; y?: number; similarity: number }>> {
  const res = await fetch(`${API_BASE}/search?q=${encodeURIComponent(q)}&k=${k}`);
  if (!res.ok) throw new Error(`Search failed: ${res.status}`);
  const json = await res.json();
  return json.data ?? [];
}

export async function fetchNeighbors(entryId: string, k = 6) {
  const res = await fetch(`${API_BASE}/${entryId}/neighbors?k=${k}`);
  if (!res.ok) return [];
  const json = await res.json();
  return json.data ?? [];
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd web && yarn tsc --noEmit 2>&1 | head -20
```

Expected: no errors in the new files.

- [ ] **Step 4: Commit**

```bash
git add web/lib/source-type-colors.ts web/lib/vectors/
git commit -m "feat(web): add source-type colors constant and vector tile loader"
```

---

### Task 12: /vectors page — main canvas

**Files:**
- Create: `web/app/[locale]/vectors/page.tsx`
- Create: `web/app/[locale]/vectors/vectors-view.tsx`

- [ ] **Step 1: Install regl-scatterplot**

```bash
cd web && yarn add regl-scatterplot
```

- [ ] **Step 2: Write the page shell**

```typescript
// web/app/[locale]/vectors/page.tsx
import { Metadata } from 'next';
import VectorsView from './vectors-view';

export const metadata: Metadata = {
  title: 'Vector Space — BDP',
  description: 'Explore all bioinformatics datasets in semantic embedding space',
};

export default function VectorsPage() {
  return <VectorsView />;
}
```

- [ ] **Step 3: Write vectors-view.tsx**

```typescript
// web/app/[locale]/vectors/vectors-view.tsx
'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import createScatterplot from 'regl-scatterplot';
import {
  fetchStats, fetchViewportTiles, VectorStats, TilePoint
} from '@/lib/vectors/tile-loader';
import { getSourceTypeColor, SOURCE_TYPE_COLORS } from '@/lib/source-type-colors';
import VectorSidebar from './vector-sidebar';
import VectorSearchBar from './vector-search-bar';

const INITIAL_ZOOM = 3;
// Total projection space bounds (will be derived from first tile batch)
const DEFAULT_BOUNDS = { x: [-15, 15] as [number, number], y: [-15, 15] as [number, number] };

export default function VectorsView() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const scatterRef = useRef<ReturnType<typeof createScatterplot> | null>(null);
  const [stats, setStats] = useState<VectorStats | null>(null);
  const [points, setPoints] = useState<TilePoint[]>([]);
  const [selectedPoint, setSelectedPoint] = useState<TilePoint | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [enabledTypes, setEnabledTypes] = useState<Set<string>>(
    new Set(Object.keys(SOURCE_TYPE_COLORS))
  );

  // Load stats and initial tiles on mount
  useEffect(() => {
    (async () => {
      try {
        const s = await fetchStats();
        setStats(s);
        if (!s.current_run_id) { setLoading(false); return; }

        // Load initial viewport at zoom 3
        const initial = await fetchViewportTiles(
          s.current_run_id, INITIAL_ZOOM,
          DEFAULT_BOUNDS.x[0], DEFAULT_BOUNDS.x[1],
          DEFAULT_BOUNDS.y[0], DEFAULT_BOUNDS.y[1],
          DEFAULT_BOUNDS,
        );
        setPoints(initial);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  // Initialize regl-scatterplot once canvas is ready
  useEffect(() => {
    if (!canvasRef.current || points.length === 0) return;

    const scatter = createScatterplot({
      canvas: canvasRef.current,
      pointSize: 3,
      opacity: 0.8,
      colorBy: 'category',
    });

    const data = points
      .filter(p => enabledTypes.has(p.st || 'other'))
      .map(p => [p.x, p.y, getSourceTypeColor(p.st)]);

    scatter.draw({ x: data.map(d => d[0] as number), y: data.map(d => d[1] as number) });

    scatter.subscribe('select', ({ points: selected }: { points: number[] }) => {
      if (selected.length > 0) {
        setSelectedPoint(points[selected[0]] ?? null);
      }
    });

    scatterRef.current = scatter;
    return () => scatter.destroy();
  }, [points, enabledTypes]);

  const handleSearchResult = useCallback((x: number, y: number) => {
    scatterRef.current?.zoomToLocation([x, y], 0.5, { transition: true });
  }, []);

  if (loading) return (
    <div className="flex items-center justify-center h-screen text-muted-foreground">
      Loading vector space…
    </div>
  );

  if (error) return (
    <div className="flex items-center justify-center h-screen text-destructive">
      {error}
    </div>
  );

  if (!stats?.current_run_id) return (
    <div className="flex items-center justify-center h-screen text-muted-foreground">
      <div className="text-center">
        <p className="text-lg font-medium">No embeddings yet</p>
        <p className="text-sm mt-1">Run <code>bdp-embed embed</code> to get started.</p>
      </div>
    </div>
  );

  const embeddedPct = stats.embedded_count && stats.entry_count
    ? Math.round((stats.embedded_count / stats.entry_count) * 100)
    : 0;

  return (
    <div className="relative w-full h-screen overflow-hidden">
      {/* Stats bar */}
      <div className="absolute top-0 left-0 right-0 z-10 px-4 py-2 bg-background/80 backdrop-blur text-xs text-muted-foreground flex gap-4">
        <span>{stats.embedded_count?.toLocaleString()} of {stats.entry_count?.toLocaleString()} entries embedded ({embeddedPct}%)</span>
        {stats.projected_at && (
          <span>projected {new Date(stats.projected_at).toLocaleString()}</span>
        )}
        <span className="capitalize">{stats.status}</span>
      </div>

      {/* Search bar */}
      <VectorSearchBar onResult={handleSearchResult} />

      {/* Canvas */}
      <canvas ref={canvasRef} className="w-full h-full" />

      {/* Legend */}
      <div className="absolute bottom-4 left-4 z-10 flex flex-col gap-1">
        {Object.entries(SOURCE_TYPE_COLORS).map(([type, color]) => (
          <button
            key={type}
            onClick={() => setEnabledTypes(prev => {
              const next = new Set(prev);
              if (next.has(type)) next.delete(type); else next.add(type);
              return next;
            })}
            className={`flex items-center gap-1.5 text-xs px-2 py-0.5 rounded transition-opacity ${
              enabledTypes.has(type) ? 'opacity-100' : 'opacity-30'
            }`}
          >
            <span className="w-2 h-2 rounded-full" style={{ background: color }} />
            {type}
          </button>
        ))}
      </div>

      {/* Point count HUD */}
      <div className="absolute bottom-4 right-4 z-10 text-xs text-muted-foreground">
        {points.length.toLocaleString()} points visible
      </div>

      {/* Sidebar */}
      {selectedPoint && (
        <VectorSidebar
          point={selectedPoint}
          onClose={() => setSelectedPoint(null)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add web/app/\[locale\]/vectors/
git commit -m "feat(web): add /vectors page with regl-scatterplot and tile-based loading"
```

---

### Task 13: Sidebar + search bar components

**Files:**
- Create: `web/app/[locale]/vectors/vector-sidebar.tsx`
- Create: `web/app/[locale]/vectors/vector-search-bar.tsx`
- Modify: `web/components/layout/header.tsx`

- [ ] **Step 1: Write vector-sidebar.tsx**

```typescript
// web/app/[locale]/vectors/vector-sidebar.tsx
'use client';

import { useEffect, useState } from 'react';
import { TilePoint, fetchNeighbors } from '@/lib/vectors/tile-loader';
import { getSourceTypeColor } from '@/lib/source-type-colors';

interface Props {
  point: TilePoint;
  onClose: () => void;
}

export default function VectorSidebar({ point, onClose }: Props) {
  const [neighbors, setNeighbors] = useState<TilePoint[]>([]);

  useEffect(() => {
    fetchNeighbors(point.id, 6).then(setNeighbors).catch(() => {});
  }, [point.id]);

  const color = getSourceTypeColor(point.st);
  const detailUrl = `/sources/${point.org}/${point.slug}`;

  return (
    <div className="absolute right-0 top-0 h-full w-72 bg-background/95 backdrop-blur border-l z-20 flex flex-col p-4 gap-3 overflow-y-auto">
      <div className="flex items-center justify-between">
        <span className="text-xs font-mono px-1.5 py-0.5 rounded" style={{ background: color + '33', color }}>
          {point.st || point.et}
        </span>
        <button onClick={onClose} className="text-muted-foreground hover:text-foreground text-lg leading-none">×</button>
      </div>

      <div className="font-medium text-sm leading-snug">{point.l}</div>

      <div className="text-xs text-muted-foreground">
        <span>{point.org}</span>
        <span className="mx-1">·</span>
        <span className="font-mono">{point.slug}</span>
      </div>

      <div className="text-xs text-muted-foreground font-mono">
        x: {point.x.toFixed(3)} · y: {point.y.toFixed(3)}
      </div>

      {neighbors.length > 0 && (
        <div>
          <div className="text-xs font-medium text-muted-foreground mb-1.5">Nearest in embedding space</div>
          <div className="flex flex-col gap-1">
            {neighbors.map((n: TilePoint) => (
              <a
                key={n.id}
                href={`/sources/${n.org}/${n.slug}`}
                className="flex items-center gap-1.5 text-xs hover:bg-muted rounded px-1 py-0.5 transition-colors"
              >
                <span className="w-1.5 h-1.5 rounded-full shrink-0" style={{ background: getSourceTypeColor(n.st) }} />
                <span className="truncate">{n.l}</span>
              </a>
            ))}
          </div>
        </div>
      )}

      <div className="mt-auto flex gap-2">
        <a href={detailUrl} className="flex-1 text-center text-xs py-1.5 px-2 bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors">
          Open entry
        </a>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Write vector-search-bar.tsx**

```typescript
// web/app/[locale]/vectors/vector-search-bar.tsx
'use client';

import { useState, useRef } from 'react';
import { fetchSemanticSearch } from '@/lib/vectors/tile-loader';

interface Props {
  onResult: (x: number, y: number) => void;
}

export default function VectorSearchBar({ onResult }: Props) {
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleSearch = async (q: string) => {
    if (!q.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const results = await fetchSemanticSearch(q, 20);
      // Fly to centroid of top results that have coordinates
      const withCoords = results.filter(r => r.x != null && r.y != null);
      if (withCoords.length > 0) {
        const cx = withCoords.reduce((s, r) => s + (r.x ?? 0), 0) / withCoords.length;
        const cy = withCoords.reduce((s, r) => s + (r.y ?? 0), 0) / withCoords.length;
        onResult(cx, cy);
      } else {
        setError('No results with known coordinates.');
      }
    } catch (e) {
      setError('Search failed.');
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => handleSearch(val), 300);
  };

  return (
    <div className="absolute top-10 left-1/2 -translate-x-1/2 z-10 w-80">
      <input
        type="text"
        value={query}
        onChange={handleChange}
        placeholder="Search the embedding space…"
        className="w-full px-3 py-2 text-sm rounded-lg border bg-background/90 backdrop-blur shadow-sm focus:outline-none focus:ring-2 focus:ring-primary"
      />
      {loading && <div className="text-xs text-muted-foreground mt-1 text-center">Searching…</div>}
      {error && <div className="text-xs text-destructive mt-1 text-center">{error}</div>}
    </div>
  );
}
```

- [ ] **Step 3: Add /vectors link to header**

In `web/components/layout/header.tsx`, add a nav link to `/vectors` alongside the existing nav links. Find where other nav links like `/search` or `/sources` are defined and add:

```typescript
<Link href="/vectors">Vectors</Link>
```

(Exact placement and styling depends on the existing header structure — match the existing pattern.)

- [ ] **Step 4: Verify TypeScript**

```bash
cd web && yarn tsc --noEmit 2>&1 | grep -E "error TS"
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add web/app/\[locale\]/vectors/vector-sidebar.tsx \
        web/app/\[locale\]/vectors/vector-search-bar.tsx \
        web/components/layout/header.tsx
git commit -m "feat(web): add vector sidebar, search bar, and header nav link"
```

---

## Phase E: Integration Smoke Test

### Task 14: End-to-end smoke test

This task verifies the whole pipeline works together with a small dataset.

- [ ] **Step 1: Start the dev server**

```bash
cargo xtask dev server
```

- [ ] **Step 2: Verify /stats returns valid JSON**

```bash
curl -s http://localhost:3000/api/v1/vectors/stats | jq .
```

Expected:
```json
{
  "data": {
    "current_run_id": null,
    "status": null,
    "entry_count": <number>,
    "embedded_count": 0,
    ...
  }
}
```

- [ ] **Step 3: Verify /search returns 503 without OPENAI_API_KEY**

```bash
curl -s "http://localhost:3000/api/v1/vectors/search?q=insulin&k=5" | jq .status
```

Expected: `"SERVICE_UNAVAILABLE"` or similar (graceful failure without API key).

- [ ] **Step 4: Verify /vectors page loads in browser**

```bash
cd web && yarn dev
```

Open `http://localhost:3001/vectors` — expected: "No embeddings yet" message (since no bdp-embed run has completed).

- [ ] **Step 5: Run a tiny embed on test data (optional — requires OPENAI_API_KEY)**

```bash
cd tools/bdp-embed
DATABASE_URL=$DATABASE_URL OPENAI_API_KEY=$OPENAI_API_KEY \
  bdp-embed embed --batch-size 10
```

Expected: embeds the first 10 entries, writes to `entry_embeddings`.

- [ ] **Step 6: Commit (if any fixes were needed)**

```bash
git add -A && git commit -m "fix(vectors): smoke test fixes"
```

---

## Notes for Implementor

**Storage API:** Before implementing `get_tile.rs`, check `crates/bdp-server/src/storage/` for how other handlers fetch object bytes (see `features/files/queries/download.rs` for an existing S3 download example). Adapt `get_tile.rs` accordingly.

**pgvector Rust types:** The `pgvector` crate's `HalfVector` type must be used for SQLx parameter binding. See pgvector crate docs for the exact feature flags and type conversions.

**sqlx prepare:** Run `cargo xtask sqlx prepare` after every change to `.sql` query strings in Rust code. The project requires offline query metadata.

**bdp-embed in production:** Register `bdp-embed` as a system package in the deployment Dockerfile/docker-compose so it's available on `$PATH` when the Rust job system invokes it.

**MCP wiring (BDP-66):** When the MCP server is implemented, `search_sources` should call `GET /api/v1/vectors/search?q={query}&k=5` and merge results with the existing text search (`GET /api/v1/search?q={query}`), ranking by combined score.
