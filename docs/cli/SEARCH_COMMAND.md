# BDP Search Command - Quick Reference

The `bdp search` command provides full-text search across organizations, data sources, and tools in the BDP registry.

## Overview

```bash
bdp search <query> [FLAGS]
```

## Basic Usage

### Simple Search

```bash
# Interactive search (default)
bdp search insulin

# Search with multiple words
bdp search "human genome"

# Non-interactive mode
bdp search protein --no-interactive
```

### Search Output

**Interactive Mode** (default for TTY):
```
🔍 Searching for 'insulin'...

Found 12 results

┌─────┬─────────────────────────┬─────────┬────────┬─────────────────────┐
│ #   │ Source                  │ Version │ Format │ Description         │
├─────┼─────────────────────────┼─────────┼────────┼─────────────────────┤
│ 1   │ uniprot:P01308-fasta    │ 1.0     │ fasta  │ Insulin precursor   │
│ 2   │ uniprot:P01308-xml      │ 1.0     │ xml    │ Insulin precursor   │
│ 3   │ ncbi:insulin-gene       │ 2.0     │ gbk    │ Human insulin gene  │
└─────┴─────────────────────────┴─────────┴────────┴─────────────────────┘

[↑/↓] Navigate  [Enter] View Details  [Space] Select  [C] Copy  [Q] Quit
```

**Non-Interactive Mode**:
```bash
bdp search insulin --no-interactive
```

Output:
```
uniprot:P01308-fasta@1.0 - Insulin precursor (fasta)
uniprot:P01308-xml@1.0 - Insulin precursor (xml)
ncbi:insulin-gene@2.0 - Human insulin gene (gbk)
```

## Output Formats

### Table Format

```bash
bdp search protein --format table --no-interactive
```

### JSON Format

```bash
bdp search insulin --format json --no-interactive
```

Output:
```json
{
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "organization": "uniprot",
      "name": "P01308-fasta",
      "version": "1.0",
      "description": "Insulin precursor",
      "format": "fasta",
      "entry_type": "data_source"
    }
  ],
  "total": 12,
  "page": 1,
  "page_size": 10
}
```

### Compact Format

```bash
bdp search genome --format compact --no-interactive
```

Output:
```
ncbi:NC_000001@11.10
ncbi:NC_000002@11.10
ncbi:NC_000003@11.10
```

## Filtering

### By Entry Type

```bash
# Only data sources
bdp search protein --type data_source

# Only tools
bdp search blast --type tool

# Multiple types
bdp search annotation --type data_source --type tool
```

### By Source Type

```bash
# Only proteins
bdp search human --source-type protein

# Only genomes
bdp search bacteria --source-type genome

# Multiple source types
bdp search "e. coli" --source-type protein --source-type genome
```

### Combined Filters

```bash
bdp search human \
  --type data_source \
  --source-type protein \
  --format json
```

## Pagination

### Manual Pagination (Non-Interactive)

```bash
# Page 1 (default)
bdp search protein --page 1 --limit 10

# Page 2
bdp search protein --page 2 --limit 10

# Large page size
bdp search genome --limit 100
```

### Interactive Pagination

In interactive mode, use arrow keys and built-in pagination:
- `↓` / `PageDown` - Next page
- `↑` / `PageUp` - Previous page

## Interactive Mode Features

### Navigation

- `↑` / `↓` - Move cursor up/down
- `PageUp` / `PageDown` - Scroll pages
- `Home` / `End` - Jump to first/last result

### Actions

- `Enter` - View detailed information
- `Space` - Select/deselect item
- `C` - Copy source specification to clipboard
- `A` - Add to manifest (`bdp.yml`)
- `Q` / `Esc` - Quit

### Clipboard Integration

Press `C` to copy the selected source specification:

```
uniprot:P01308-fasta@1.0
```

This can be pasted directly into your `bdp.yml` or used with `bdp source add`.

### Manifest Integration

Press `A` to automatically add the selected source to `bdp.yml`:

```yaml
sources:
  - uniprot:P01308-fasta@1.0  # ← Added via search
```

## Search Caching

Search results are cached for **5 minutes** to improve performance:

```bash
# First search - queries the server
bdp search insulin

# Subsequent searches within 5 minutes - uses cache
bdp search insulin --format json
```

Clear the search cache:

```bash
bdp clean --search-cache
```

## Common Examples

### Find All Protein Data Sources

```bash
bdp search protein \
  --type data_source \
  --source-type protein \
  --limit 50
```

### Search for Specific Organism

```bash
bdp search "homo sapiens" --source-type genome
bdp search "e. coli" --source-type protein
bdp search yeast --format json
```

### Find Tools

```bash
bdp search blast --type tool
bdp search "sequence alignment" --type tool
```

### Export Search Results

```bash
bdp search protein \
  --format json \
  --limit 100 \
  --no-interactive \
  > protein_sources.json
```

### Pipe to Other Tools

```bash
# Extract just the names
bdp search insulin --format json --no-interactive \
  | jq -r '.results[].name'

# Filter with grep
bdp search protein --format compact --no-interactive \
  | grep fasta

# Count results
bdp search genome --format compact --no-interactive \
  | wc -l
```

## Search Tips

### Search Syntax

- **Multiple words**: Treated as AND (all words must match)
  ```bash
  bdp search human genome  # Matches "human genome" or "genome human"
  ```

- **Quoted phrases**: Exact phrase match
  ```bash
  bdp search "insulin precursor"
  ```

- **Wildcards**: Not supported (use broader terms)
  ```bash
  # ❌ bdp search P0130*
  # ✅ bdp search P01308
  ```

### Finding Data

1. **Start broad, then filter**:
   ```bash
   bdp search protein
   bdp search protein --source-type protein
   bdp search protein --source-type protein --limit 20
   ```

2. **Use specific identifiers**:
   ```bash
   bdp search P01308      # UniProt accession
   bdp search NC_000001   # RefSeq accession
   bdp search GO:0008150  # Gene Ontology ID
   ```

3. **Search by organism**:
   ```bash
   bdp search human
   bdp search "mus musculus"
   bdp search bacteria
   ```

## Smart Defaults

- **Default Format**: Interactive for TTY, table for non-TTY
- **Default Limit**: 10 results per page
- **Cache TTL**: 5 minutes
- **Auto-retry**: 3 attempts with exponential backoff

## Differences from Query Command

| Feature | `bdp search` | `bdp query` |
|---------|--------------|-------------|
| Purpose | Full-text search | SQL querying |
| Input | Keywords | SQL or flags |
| Scope | Organizations, sources, tools | Any database table |
| Interactive | Yes | No |
| Caching | Yes (5 min) | No |
| Output | Search results | Raw query results |

Use **search** to discover data, use **query** to analyze it.

## Error Handling

### Empty Results

```bash
$ bdp search nonexistent
No results found for 'nonexistent'

Try:
  - Using broader search terms
  - Checking spelling
  - Searching for partial names
```

### Server Unavailable

```bash
$ bdp search protein
Error: Server unavailable at http://localhost:8000
Check that the BDP server is running.
```

### Invalid Filters

```bash
$ bdp search protein --type invalid
Error: Invalid entry type 'invalid'
Valid types: data_source, tool, organization
```

## Configuration

Set default server URL:

```bash
export BDP_SERVER_URL=https://api.bdp.example.com
bdp search insulin
```

Or use the flag:

```bash
bdp search insulin --server-url https://api.bdp.example.com
```

## Related Commands

- `bdp query` - SQL-like querying of data
- `bdp source add` - Add sources to manifest
- `bdp source list` - List sources in manifest
- `bdp pull` - Download sources

## See Also

- [Query Command](./QUERY_COMMAND.md)
- [CLI Commands Overview](../INDEX.md#cli-commands)
- [Full Documentation](../INDEX.md)
