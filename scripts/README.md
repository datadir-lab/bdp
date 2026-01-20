# Scripts

Development, testing, deployment, and ingestion utilities.

**Structure:** `dev/` `test/` `deploy/` `ingest/` `output/`

## DB Diagram

```bash
./scripts/generate-db-diagram.sh
# Outputs: schema.sql, tables.txt, schema.dot, schema.png
# View: open scripts/output/schema_latest.png
# Online: https://dreampuf.github.io/GraphvizOnline/
```

**Requirements:** Docker, optional Graphviz

## Guidelines

Use bash strict mode, add usage docs, validate prereqs, handle errors, export env vars
