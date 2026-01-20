# Search Query Syntax

## Overview

The BDP search functionality supports special query syntax that allows users to search for specific data sources using a structured format. This syntax enables precise filtering by organization, version, and format without requiring manual filter selection.

## Syntax Format

The special query syntax follows this pattern:

```
organization:identifier[@version][-format]
```

### Components

- **organization** (required): The organization slug (e.g., `uniprot`, `genbank`, `ensembl`)
- **identifier** (required): The data source slug or bundle name
- **@version** (optional): Version specifier prefixed with `@`
- **-format** (optional): File format specifier prefixed with `-`

### Rules

- Organization names can contain alphanumeric characters, hyphens, and underscores
- Identifiers (slugs) can contain alphanumeric characters and underscores (no hyphens)
- Versions can contain alphanumeric characters, dots, and underscores (no hyphens)
- Formats can contain alphanumeric characters and underscores

## Examples

### Basic Organization and Identifier

```
uniprot:P01308
```
Searches for UniProt entry P01308

### Bundle Searches

```
uniprot:swissprot
```
Searches for UniProt SwissProt bundle

```
genbank:refseq
```
Searches for GenBank RefSeq bundle

```
ensembl:vertebrates
```
Searches for Ensembl Vertebrates bundle

### With Version

```
uniprot:P01308@1.0
```
Searches for UniProt entry P01308 version 1.0

### With Format

```
uniprot:P01308-fasta
```
Searches for UniProt entry P01308 in FASTA format

```
ensembl:vertebrates-gtf
```
Searches for Ensembl Vertebrates bundle in GTF format

### Full Syntax (All Components)

```
uniprot:P01308@1.0-fasta
```
Searches for UniProt entry P01308 version 1.0 in FASTA format

## Known Bundles

The parser recognizes the following bundle identifiers:

### UniProt
- `swissprot` - SwissProt (reviewed entries)
- `trembl` - TrEMBL (unreviewed entries)

### GenBank
- `refseq` - RefSeq database
- `genbank` - GenBank database

### Ensembl
- `vertebrates` - Vertebrate genomes
- `plants` - Plant genomes
- `fungi` - Fungal genomes
- `metazoa` - Metazoan genomes
- `protists` - Protist genomes
- `bacteria` - Bacterial genomes

## Visual Feedback

When you type a query using the special syntax, the search bar will display a visual indicator showing how your query has been parsed:

Example:
```
Input: uniprot:P01308@1.0-fasta
Display: Organization: uniprot • Source: P01308 • Version: 1.0 • Format: fasta
```

## How It Works

1. **Type your query** using the special syntax in the search bar
2. **See the parsed result** displayed as a badge below the input
3. **Press Enter or click Search** to execute the search
4. **Filters are applied automatically** based on the parsed query components:
   - Organization filter is set to the specified organization
   - Search query is set to the identifier (or bundle name)
   - Format filter is applied if specified
   - Version filter is applied if specified

## Plain Text Searches

If your query doesn't match the special syntax format, it will be treated as a regular plain text search. For example:

```
protein kinase
```
This will perform a standard full-text search for "protein kinase" without any special parsing.

## Implementation Details

### Files Modified

- `web/lib/utils/query-parser.ts` - Core parser logic
- `web/components/search/search-bar.tsx` - Search bar integration
- `web/app/[locale]/search/search-results.tsx` - Results page integration
- `web/lib/types/search.ts` - Type definitions (added `formats` field)
- `web/lib/api/search.ts` - API client (added `version` parameter)

### Testing

Run the demonstration script to see the parser in action:

```bash
node web/lib/utils/query-parser-demo.js
```

## API Changes

The search API now accepts two additional parameters:

- `format` (string, optional) - Filter by file format
- `version` (string, optional) - Filter by version

Example API request:
```
GET /api/v1/search?query=P01308&organizations=uniprot&formats=fasta&version=1.0
```

## Future Enhancements

Potential improvements to consider:

1. **Auto-completion for special syntax** - Suggest completions when typing `org:`
2. **Format validation** - Validate that the specified format is available
3. **Version suggestions** - Show available versions when typing `@`
4. **Multiple formats** - Support syntax like `uniprot:P01308-fasta,json`
5. **Negation** - Support excluding formats with `!format`
6. **Wildcards** - Support pattern matching with `*` and `?`

## Troubleshooting

### Query not recognized as special format

Make sure your query follows the exact syntax:
- Use `:` to separate organization and identifier
- Use `@` for version (not `v` or other prefixes)
- Use `-` for format (not `_` or `.`)
- Identifiers cannot contain hyphens (use underscores instead)

### Format not being applied

Check that:
1. The format is spelled correctly
2. The format is supported by the data source
3. There's no space before or after the `-` separator

### Version not being recognized

Ensure:
1. Version uses dots or underscores as separators (not hyphens)
2. Version follows immediately after `@` with no spaces
3. If using format, the format comes after the version
