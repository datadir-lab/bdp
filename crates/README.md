# BDP Crates

This directory contains all Rust workspace crates for the Biological Data Platform.

## Crates Overview

### bdp-server
The main API server providing REST endpoints for querying biological data. Handles request routing, database interactions, and response formatting.

**Key directories:**
- `src/api/` - REST API endpoint handlers and routing
- `src/models/` - Domain models and data structures
- `src/storage/` - Storage layer abstractions and implementations
- `src/db/` - Database connection and query management
- `tests/` - Integration and unit tests

### bdp-cli
Command-line interface for interacting with the BDP system. Provides tools for querying, data management, and administrative tasks.

**Key directories:**
- `src/commands/` - CLI command implementations
- `src/cache/` - Local caching mechanisms for CLI performance

### bdp-ingest
Data ingestion system for importing biological data from various sources.

**Key directories:**
- `src/uniprot/` - UniProt protein database ingestion
- `src/ncbi/` - NCBI (GenBank, RefSeq, PubMed) data ingestion
- `src/ensembl/` - Ensembl genome annotation ingestion

### bdp-common
Shared types, utilities, and common functionality used across all crates.

**Key directories:**
- `src/types/` - Common type definitions and data structures

## Building

Build all crates:
```bash
cargo build --workspace
```

Build a specific crate:
```bash
cargo build -p bdp-server
cargo build -p bdp-cli
cargo build -p bdp-ingest
```

## Testing

Run all tests:
```bash
cargo test --workspace
```

Test a specific crate:
```bash
cargo test -p bdp-server
```

## Development

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines and best practices.
