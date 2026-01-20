# Batch Operations Implementation for UniProt Storage

This document describes the batch operations implementation to eliminate N+1 queries.

## Changes Made

### 1. Added Organism Cache

Added `OrganismCache` struct with:
- HashMap cache for taxonomy_id → organism_id mapping
- 5-minute refresh interval
- Prepopulate method for batch processing
- get_or_create method with cache-first lookup

### 2. Modified UniProtStorage

Added `organism_cache: RefCell<OrganismCache>` field to UniProtStorage.

### 3. Replaced store_entries() method

Instead of looping through entries one by one (N+1 pattern), the new implementation:

1. Pre-populates organism cache for all unique taxonomy IDs in batch
2. Gets or creates all organisms (using cache)
3. Processes entries in chunks of 500 (PostgreSQL parameter limit)
4. For each chunk:
   - Batch inserts registry_entries (1 query)
   - Batch inserts data_sources (1 query)
   - Batch deduplicates and inserts sequences (2-3 queries)
   - Batch inserts protein_metadata (1 query)
   - Batch inserts versions (1 query)
   - Batch inserts version_files (1-2 queries for 3 files per protein)

## Query Reduction

**Before (N+1 pattern):**
- Per protein: 7-10 queries
- Per 1000 proteins: ~10,000 queries

**After (batch operations):**
- Organism cache prepopulation: 1 query
- Per 500-entry chunk: ~8-10 queries
- Per 1000 proteins: ~20-30 queries

**Improvement:** 300-500x reduction in database queries!

## Implementation Status

The code changes are ready to be applied to `storage.rs`. The key modifications are:

1. Add imports for HashMap, RefCell, Duration, SystemTime, QueryBuilder
2. Add OrganismCache struct implementation
3. Add organism_cache field to UniProtStorage
4. Replace store_entries() with batch version
5. Add helper methods for batch operations

## Files to Modify

- `crates/bdp-server/src/ingest/uniprot/storage.rs` - Main implementation
