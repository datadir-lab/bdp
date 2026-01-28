# BDP Query Command - Specification

## Overview

`bdp query` is an advanced SQL-like CLI interface for querying BDP data sources, metadata, and related entities with powerful filtering, aggregation, and output capabilities.

## Design Philosophy

1. **SQL-first, flags for convenience** - Pure SQL for power users, flags for common cases
2. **Pipe-friendly** - Clean output for Unix workflows
3. **Smart defaults** - Interactive vs batch mode differences
4. **Backend optimization** - Server-side query execution
5. **Aliased entities** - User-friendly names (protein, gene, etc.)
6. **Auto-join metadata** - Automatic metadata table joins based on source type

## Command Structure

```bash
bdp query [ENTITY|--sql <query>] [FLAGS]
```

## Entity Aliases

| Alias | Maps To | Description |
|-------|---------|-------------|
| `protein` | `data_sources WHERE type='protein'` | Protein data sources |
| `gene` | `data_sources WHERE type='gene'` | Gene data sources |
| `genome` | `data_sources WHERE type='genome'` | Genome assemblies |
| `transcriptome` | `data_sources WHERE type='transcriptome'` | RNA-seq data |
| `proteome` | `data_sources WHERE type='proteome'` | Proteome datasets |
| `tools` | `tools` | Bioinformatics tools |
| `orgs` | `organizations` | Organizations |

## Metadata Tables (Direct Access)

- `protein_metadata` - Protein functional annotations
- `gene_metadata` - Gene annotations and coordinates
- `organism_taxonomy` - NCBI taxonomy data
- `publication_refs` - Publication references

## Automatic Metadata Joins

When querying by entity alias, relevant metadata is automatically joined:

```bash
# This:
bdp query protein --where organism=human

# Automatically becomes:
SELECT ds.*, pm.*
FROM data_sources ds
LEFT JOIN protein_metadata pm ON ds.metadata_id = pm.id
WHERE ds.type='protein' AND ds.organism='human'
```

## Flags

### Selection
- `--select <fields>` - Select specific fields (default: *)
- `--where <key>=<value>` or `--where <expression>` - Filter results (repeatable)

### Sorting & Pagination
- `--order-by <field>[:asc|desc]` - Sort results (default: asc)
- `--limit <n>` - Limit results (default: 1000)
- `--offset <n>` - Skip first N results

### Aggregation
- `--group-by <field>` - Group results by field
- `--aggregate <expr>` - Aggregation expression (COUNT, SUM, AVG, etc.)
- `--having <expression>` - Filter grouped results

### Joins (Advanced)
- `--join <entity>` - Join with another entity/table
- `--on <condition>` - Join condition

### Direct SQL
- `--sql <query>` - Execute raw SQL query

### Output
- `--format <fmt>` - Output format: table|json|csv|tsv|compact
- `--output <file>` - Write to file
- `--no-header` - Omit header row (CSV/TSV)

### Debugging
- `--explain` - Show query execution plan
- `--dry-run` - Show generated SQL without executing

### Query Management (Roadmap)
- `--save <name>` - Save query as template
- `--load <name>` - Load saved query
- `--history` - Show query history
- `--history-run <n>` - Re-run query from history

## Smart Defaults

**TTY (Interactive):**
- Format: `table` (pretty-printed)
- Colors: enabled
- Progress: shown for long queries

**Piped (Batch):**
- Format: `tsv`
- Colors: disabled
- Progress: disabled

## Examples

### Simple Flag-Based Queries

```bash
# Basic query
bdp query protein --where organism=human --limit 10

# Multiple filters (AND combined)
bdp query protein --where organism=human --where format=fasta --order-by downloads:desc

# Field selection
bdp query protein --select name,version,file_size,downloads --where organism=human

# Aggregation
bdp query protein --group-by organism --aggregate "COUNT(*) as total" --order-by total:desc

# Access metadata directly
bdp query protein_metadata --where taxonomy_id=9606 --select protein_name,function
```

### Complex Expression Queries

```bash
# Complex WHERE
bdp query protein --where "organism='human' AND (downloads>1000 OR verified=true)"

# Range queries
bdp query protein --where "file_size > 1000000 AND file_size < 10000000"

# Pattern matching
bdp query protein --where "name LIKE 'INS%'" --limit 50
```

### Full SQL Queries

```bash
# Pure SQL
bdp query --sql "
  SELECT
    d.name,
    d.version,
    o.display_name,
    d.downloads
  FROM data_sources d
  JOIN organizations o ON d.organization = o.name
  WHERE d.type = 'protein' AND d.organism = 'human'
  ORDER BY d.downloads DESC
  LIMIT 20
"

# Metadata joins
bdp query --sql "
  SELECT
    ds.name,
    pm.protein_name,
    pm.function,
    ot.scientific_name
  FROM data_sources ds
  JOIN protein_metadata pm ON ds.metadata_id = pm.id
  JOIN organism_taxonomy ot ON ds.organism_taxonomy_id = ot.id
  WHERE pm.subcellular_location LIKE '%nucleus%'
"

# Aggregation
bdp query --sql "
  SELECT
    o.display_name,
    COUNT(*) as datasets,
    SUM(d.downloads) as total_downloads
  FROM organizations o
  JOIN data_sources d ON o.name = d.organization
  WHERE d.type = 'protein'
  GROUP BY o.display_name
  HAVING datasets > 5
  ORDER BY total_downloads DESC
"
```

### Output Formats

```bash
# JSON for piping to jq
bdp query protein --where organism=human --format json | jq '.[].name'

# CSV for spreadsheets
bdp query protein --where organism=human --format csv > proteins.csv

# TSV for awk
bdp query protein --where organism=human --format tsv | awk -F'\t' '{print $1}'

# Table for humans
bdp query protein --where organism=human --format table

# Write to file
bdp query protein --where organism=human --format json --output proteins.json
```

## Architecture

### CLI → Backend Flow

1. **CLI parses command** - Convert flags to SQL
2. **Entity alias resolution** - protein → data_sources WHERE type='protein'
3. **Auto-join metadata** - Add LEFT JOIN based on entity type
4. **Send to backend** - POST /api/v1/query with SQL
5. **Backend validates** - Check permissions, parse SQL
6. **Backend executes** - Run against PostgreSQL
7. **Backend caches** - Cache results (5min TTL)
8. **Backend returns** - Paginated results
9. **CLI formats** - Convert to requested output format

### SQL Parser

Use `sqlparser-rs` for:
- Parsing flag-based queries into AST
- Validating user-provided SQL
- Extracting table names for permission checks
- Building query from flags

### Security

- **SQL injection prevention** - Parameterized queries on backend
- **Permission checks** - User can only query tables they have access to
- **Query limits** - Max 1000 rows by default
- **Query timeout** - 30 second timeout on backend

## Error Messages

### Simplified (Default)
```
Error: No data sources found matching 'organism=martian'
Hint: Check available organisms with: bdp query protein --group-by organism
```

### Detailed (--verbose)
```
Error: Query execution failed

Query:
  SELECT * FROM data_sources WHERE type='protein' AND organism='martian'

SQL Error:
  No rows returned

Available values:
  Run: bdp query protein --select DISTINCT organism
```

## Future Features (Roadmap)

### Query History
```bash
# Show last 20 queries
bdp query --history

# Re-run query #5
bdp query --history-run 5

# Search history
bdp query --history --search "protein human"

# Clear history
bdp query --history-clear
```

### Saved Queries
```bash
# Save current query
bdp query protein --where organism=human --save frequent_proteins

# Load saved query
bdp query --load frequent_proteins

# List saved queries
bdp query --list-saved

# Delete saved query
bdp query --delete-saved frequent_proteins

# Export saved queries
bdp query --export-saved queries.yaml
```

### Query Templates
```bash
# Use built-in template
bdp query --template popular_proteins --param organism=human

# Create custom template
bdp query --template-create my_template --sql "SELECT * FROM data_sources WHERE type='{{type}}' AND organism='{{organism}}'"

# List templates
bdp query --list-templates

# Built-in templates:
# - popular_datasets: Most downloaded datasets by type
# - recent_updates: Recently updated data sources
# - large_files: Largest files by type
# - org_summary: Dataset summary by organization
```

## Implementation Phases

### Phase 1: Core Functionality ✅
- [ ] Basic flag parsing
- [ ] Entity alias resolution
- [ ] Simple WHERE conditions
- [ ] --sql direct query
- [ ] Table, JSON, CSV output
- [ ] Backend /api/v1/query endpoint
- [ ] SQL parser integration
- [ ] Permission validation
- [ ] Basic error handling

### Phase 2: Advanced Features
- [ ] Auto-join metadata tables
- [ ] Complex WHERE expressions
- [ ] Aggregation (GROUP BY, HAVING)
- [ ] JOIN support
- [ ] --explain, --dry-run
- [ ] Smart default detection (TTY vs pipe)
- [ ] Progress indicators
- [ ] Enhanced error messages

### Phase 3: Query Management
- [ ] Query history
- [ ] Save queries
- [ ] Load queries
- [ ] Query templates
- [ ] Built-in templates
- [ ] Template parameters
- [ ] Export/import queries

### Phase 4: Optimization & Polish
- [ ] Query result caching
- [ ] Query optimization hints
- [ ] Parallel query execution
- [ ] Streaming results for large queries
- [ ] Query analytics
- [ ] Performance profiling

## Database Schema

### Backend Tables

```sql
-- Main data sources table
CREATE TABLE data_sources (
    id UUID PRIMARY KEY,
    organization VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    version VARCHAR NOT NULL,
    type VARCHAR NOT NULL, -- protein, gene, genome, etc.
    organism VARCHAR,
    format VARCHAR,
    file_size BIGINT,
    downloads BIGINT DEFAULT 0,
    verified BOOLEAN DEFAULT false,
    metadata_id UUID,
    organism_taxonomy_id UUID,
    publication_id UUID,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Protein metadata
CREATE TABLE protein_metadata (
    id UUID PRIMARY KEY,
    protein_name VARCHAR,
    function TEXT,
    pathway VARCHAR,
    subcellular_location VARCHAR,
    taxonomy_id INTEGER,
    uniprot_id VARCHAR,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Gene metadata
CREATE TABLE gene_metadata (
    id UUID PRIMARY KEY,
    gene_symbol VARCHAR,
    gene_name VARCHAR,
    chromosome VARCHAR,
    start_position BIGINT,
    end_position BIGINT,
    strand VARCHAR(1),
    gene_type VARCHAR,
    ncbi_gene_id INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Organism taxonomy
CREATE TABLE organism_taxonomy (
    id UUID PRIMARY KEY,
    taxonomy_id INTEGER UNIQUE,
    scientific_name VARCHAR,
    common_name VARCHAR,
    taxonomy_rank VARCHAR,
    lineage TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Publication references
CREATE TABLE publication_refs (
    id UUID PRIMARY KEY,
    doi VARCHAR UNIQUE,
    title TEXT,
    authors TEXT,
    journal VARCHAR,
    year INTEGER,
    pmid INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);
```

## Testing Strategy

### Unit Tests
- SQL parser (flag → SQL conversion)
- Entity alias resolution
- Auto-join logic
- Output formatters

### Integration Tests
- Full query execution (mock backend)
- All output formats
- Complex WHERE expressions
- Aggregations and JOINs

### E2E Tests
- Real backend queries
- Permission checks
- Error handling
- Performance benchmarks

## Performance Targets

- Simple queries: < 100ms
- Complex aggregations: < 1s
- Large result sets (1000 rows): < 2s
- Query history lookup: < 50ms
- Saved query load: < 50ms

## Success Metrics

- Query success rate > 99%
- Average query time < 500ms
- User satisfaction score > 4.5/5
- Documentation completeness > 95%
- Test coverage > 90%
