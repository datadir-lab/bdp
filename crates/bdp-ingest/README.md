# BDP Ingest

Data ingestion system for importing biological data from various public databases.

## Overview

The ingestion system handles:

- Downloading data from public biological databases
- Parsing various file formats (XML, JSON, FASTA, etc.)
- Data transformation and normalization
- Batch processing and incremental updates
- Error handling and retry logic

## Supported Data Sources

### UniProt
Swiss-Prot and TrEMBL protein sequences and annotations.

**Files:** `src/uniprot/`

**Data formats:**
- UniProt XML
- FASTA sequences
- DAT flat files

### NCBI
GenBank, RefSeq, and related NCBI databases.

**Files:** `src/ncbi/`

**Data formats:**
- GenBank flat files
- RefSeq records
- FASTA sequences
- Taxonomy data

### Ensembl
Genome annotations and comparative genomics data.

**Files:** `src/ensembl/`

**Data formats:**
- GFF3 annotations
- FASTA sequences
- Comparative genomics data

## Usage

```bash
# Ingest UniProt data
bdp-ingest uniprot --release 2024_01

# Ingest NCBI data
bdp-ingest ncbi --source genbank --taxon 9606

# Ingest Ensembl data
bdp-ingest ensembl --release 110 --species human
```

## Architecture

Each data source module implements:

1. **Downloader** - Fetches data from source APIs/FTP
2. **Parser** - Parses source-specific file formats
3. **Transformer** - Converts to internal data model
4. **Loader** - Bulk inserts into database

## Configuration

Configure ingestion in `config/ingestion.toml`:

```toml
[uniprot]
ftp_url = "ftp://ftp.uniprot.org/pub/databases/uniprot"
batch_size = 1000

[ncbi]
api_key = "your-api-key"
rate_limit = 3
```

## Adding New Data Sources

1. Create a new module under `src/`
2. Implement the `DataSource` trait
3. Add parser for the source's file format
4. Implement incremental update logic
5. Add tests and documentation

## Guidelines

- Use streaming parsers for large files
- Implement checkpointing for long-running ingestions
- Add comprehensive error handling
- Log progress and statistics
- Support incremental updates
- Validate data before insertion
