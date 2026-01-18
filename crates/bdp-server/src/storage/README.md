# BDP Server Storage

This directory contains storage layer abstractions and implementations.

## Purpose

The storage layer provides:

- Abstract interfaces for data persistence
- Concrete implementations for different storage backends
- Caching strategies and optimizations
- Connection pooling and management

## Architecture

The storage layer follows the repository pattern:

1. **Traits** - Define storage interfaces (`ProteinRepository`, `GeneRepository`, etc.)
2. **Implementations** - Concrete storage backends (PostgreSQL, file-based, etc.)
3. **Caching** - Optional caching layer for performance
4. **Migrations** - Database schema management

## Supported Backends

- **PostgreSQL** - Primary relational database
- **File storage** - For large sequence files and bulk data
- **Cache** - Redis or in-memory caching

## Usage

```rust
use storage::ProteinRepository;

async fn get_protein(repo: &dyn ProteinRepository, id: &str) -> Result<Protein> {
    repo.find_by_accession(id).await
}
```

## Guidelines

- Use async traits for I/O operations
- Implement proper error handling
- Add connection pooling for database backends
- Write unit tests with mock implementations
- Document transaction boundaries
