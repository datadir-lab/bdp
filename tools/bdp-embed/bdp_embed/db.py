import psycopg
from typing import AsyncGenerator
from contextlib import asynccontextmanager


@asynccontextmanager
async def get_conn(db_url: str) -> AsyncGenerator[psycopg.AsyncConnection, None]:
    async with await psycopg.AsyncConnection.connect(db_url) as conn:
        yield conn
