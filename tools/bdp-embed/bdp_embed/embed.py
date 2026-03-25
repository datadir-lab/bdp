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
