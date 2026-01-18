# BDP Server Models

This directory contains domain models and data structures used throughout the BDP server.

## Purpose

Models define the core data structures for:

- Biological entities (proteins, genes, sequences)
- Database records and schemas
- Request/response DTOs (Data Transfer Objects)
- Internal business logic representations

## Structure

Models are organized by domain:

- **Core biological entities** - Protein, Gene, Sequence, etc.
- **Database models** - SQLx-compatible structures for database operations
- **API models** - Request and response structures for API endpoints
- **Common types** - Shared structures used across multiple domains

## Guidelines

- Use `serde` for serialization/deserialization
- Implement validation where appropriate
- Document fields with doc comments
- Use type-safe wrappers for identifiers
- Keep database models separate from API DTOs when they differ

## Example

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub accession: String,
    pub name: String,
    pub sequence: String,
    pub organism: String,
}
```
