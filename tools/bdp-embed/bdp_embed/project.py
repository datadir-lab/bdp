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
