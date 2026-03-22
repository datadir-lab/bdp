# tools/bdp-embed/bdp_embed/tiles.py
import asyncio
import json
import io
from typing import Annotated
from collections import defaultdict
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
        max_per_cell = max(1, len(points) // (4 ** (zoom_max - z))) if z < zoom_max else len(points)

        # Group point indices by (tx, ty) cell
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
