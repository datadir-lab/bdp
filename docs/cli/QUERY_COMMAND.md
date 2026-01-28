# BDP Query Command - Quick Reference

The `bdp query` command provides SQL-like querying capabilities for BDP data sources and metadata.

## Overview

```bash
bdp query [ENTITY] [FLAGS]
bdp query --sql "SELECT ..." [FLAGS]
```

## Entity Aliases

Query predefined data types with automatic metadata joins:

| Entity | Table | Description |
|--------|-------|-------------|
| `protein` | `data_sources` | Protein data sources with protein_metadata |
| `gene` | `data_sources` | Gene data sources with gene_metadata |
| `genome` | `data_sources` | Genome data sources with genome_metadata |
| `tool` | `tools` | Analysis tools and software |
| `organism` | `organisms` | Taxonomy and organism data |
| `org` | `organizations` | Data publishers (UniProt, NCBI, etc.) |

## Basic Usage

### Simple Queries

```bash
# List all proteins (default limit: 1000)
bdp query protein

# List first 20 proteins
bdp query protein --limit 20

# List specific fields
bdp query protein --select id,name,version
```

### Filtering

```bash
# Simple filter
bdp query protein --where organism=human

# Multiple filters (AND combined)
bdp query protein --where organism=human --where status=published

# Complex filter expressions
bdp query protein --where "organism='human' AND downloads>1000"
```

### Sorting

```bash
# Sort ascending (default)
bdp query protein --order-by name

# Sort descending
bdp query protein --order-by "downloads:desc"

# Multiple sorts
bdp query protein --order-by "organism:asc,name:asc"
```

### Pagination

```bash
# Skip first 20 results
bdp query protein --offset 20 --limit 10

# Page through results
bdp query protein --limit 50 --offset 0   # Page 1
bdp query protein --limit 50 --offset 50  # Page 2
bdp query protein --limit 50 --offset 100 # Page 3
```

## Output Formats

### Table Format (Default for TTY)

```bash
bdp query protein --format table --limit 5
```

Output:
```
┌──────────────────────────────────────┬─────────────────────┬─────────┐
│ id                                   │ name                │ version │
├──────────────────────────────────────┼─────────────────────┼─────────┤
│ 550e8400-e29b-41d4-a716-446655440000 │ UniProt Human       │ 2024.1  │
│ 6ba7b810-9dad-11d1-80b4-00c04fd430c8 │ E. coli Proteome    │ 1.0     │
└──────────────────────────────────────┴─────────────────────┴─────────┘
```

### JSON Format

```bash
bdp query protein --format json --limit 2
```

Output:
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "UniProt Human",
    "version": "2024.1"
  },
  {
    "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "name": "E. coli Proteome",
    "version": "1.0"
  }
]
```

### CSV Format

```bash
bdp query protein --format csv --limit 3
```

Output:
```csv
id,name,version
550e8400-e29b-41d4-a716-446655440000,UniProt Human,2024.1
6ba7b810-9dad-11d1-80b4-00c04fd430c8,E. coli Proteome,1.0
```

Without headers:
```bash
bdp query protein --format csv --no-header --limit 2
```

### TSV Format (Default for Pipes)

```bash
bdp query protein --format tsv --limit 2
```

Output:
```
id	name	version
550e8400-e29b-41d4-a716-446655440000	UniProt Human	2024.1
6ba7b810-9dad-11d1-80b4-00c04fd430c8	E. coli Proteome	1.0
```

### Compact Format

```bash
bdp query protein --format compact --limit 3
```

Output:
```
550e8400-e29b-41d4-a716-446655440000  UniProt Human  2024.1
6ba7b810-9dad-11d1-80b4-00c04fd430c8  E. coli Proteome  1.0
```

## Raw SQL Queries

Execute arbitrary SQL (read-only):

```bash
# Simple SELECT
bdp query --sql "SELECT id, name FROM data_sources LIMIT 10"

# With WHERE clause
bdp query --sql "SELECT * FROM data_sources WHERE type='protein'"

# JOIN multiple tables
bdp query --sql "
  SELECT ds.name, pm.organism
  FROM data_sources ds
  JOIN protein_metadata pm ON ds.metadata_id = pm.id
  WHERE pm.organism = 'human'
"

# EXPLAIN query plan
bdp query --sql "EXPLAIN SELECT * FROM data_sources"
```

## File Output

Save results to a file:

```bash
# CSV export
bdp query protein --format csv --output proteins.csv

# JSON export
bdp query protein --format json --output proteins.json

# TSV for spreadsheets
bdp query protein --format tsv --output proteins.tsv
```

## Dry Run

Preview the generated SQL without executing:

```bash
bdp query protein --where organism=human --dry-run
```

Output:
```sql
Generated SQL:
SELECT * FROM data_sources
LEFT JOIN protein_metadata pm ON data_sources.metadata_id = pm.id
WHERE data_sources.type='protein' AND organism='human'
LIMIT 1000
```

## Combining Flags

```bash
# Complex query with multiple features
bdp query protein \
  --select "id,name,organism,downloads" \
  --where "organism='human'" \
  --where "status='published'" \
  --order-by "downloads:desc" \
  --limit 50 \
  --format json \
  --output top_human_proteins.json
```

## Common Examples

### Find High-Impact Proteins

```bash
bdp query protein \
  --where "downloads > 1000" \
  --order-by "downloads:desc" \
  --limit 20
```

### List All Tools

```bash
bdp query tool --select name,version,description
```

### Find Data by Organization

```bash
bdp query --sql "
  SELECT ds.name, o.name as organization
  FROM data_sources ds
  JOIN organizations o ON ds.organization_id = o.id
  WHERE o.slug = 'uniprot'
"
```

### Export All Gene Metadata

```bash
bdp query gene \
  --select "id,name,gene_symbol,chromosome,organism" \
  --format csv \
  --output genes.csv
```

### Count Data Sources by Type

```bash
bdp query --sql "
  SELECT type, COUNT(*) as count
  FROM data_sources
  GROUP BY type
  ORDER BY count DESC
"
```

## Smart Defaults

- **Default Format**: Table for interactive terminal, TSV for pipes
- **Default Limit**: 1000 rows
- **Timeout**: 30 seconds per query
- **Auto-join**: Metadata tables are automatically joined based on source type

## Security

All queries are **read-only**. The following operations are blocked:

- DROP, DELETE, UPDATE, INSERT
- TRUNCATE, ALTER, CREATE
- GRANT, REVOKE
- EXECUTE, CALL
- COPY

Only `SELECT` and `EXPLAIN` queries are allowed.

## Error Handling

### Simplified Errors

```bash
$ bdp query protein --where "bad syntax
Error: Invalid SQL syntax
```

### Detailed Errors (with --verbose)

```bash
$ bdp query --verbose protein --where "bad syntax"
[ERROR] SQL parsing failed: Expected closing quote
[DEBUG] Generated SQL: SELECT * FROM data_sources WHERE bad syntax
```

## Tips & Tricks

1. **Use Dry Run First**: Preview complex queries with `--dry-run`
2. **Pipe to jq**: `bdp query protein --format json | jq '.[] | .name'`
3. **Chain with grep**: `bdp query protein --format tsv | grep insulin`
4. **Sort in Shell**: `bdp query protein --format tsv | sort -k2`
5. **Count Lines**: `bdp query protein --format tsv --no-header | wc -l`

## Related Commands

- `bdp search` - Full-text search across all data
- `bdp status` - View cached data sources
- `bdp source list` - List sources in manifest

## See Also

- [Full Query Specification](../features/bdp-query-specification.md)
- [Query Implementation Details](../features/bdp-query-implementation-summary.md)
- [Search Command](./SEARCH_COMMAND.md)
