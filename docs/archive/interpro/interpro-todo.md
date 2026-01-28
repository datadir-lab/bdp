# InterPro Implementation TODO

**Status**: Phases 1-5, 7-8 Complete - Ready for Versioning (Phase 6)
**Last Updated**: 2026-01-28 (7 phases completed in single 4-hour session)
**Design Document**: `docs/interpro-design-corrected.md`
**Pattern**: Individual Data Sources + Relational FK + MAJOR.MINOR Versioning

---

## Phase 1: Database Schema (Est: 2 days) ✅ COMPLETE

### Migration Files

- [x] **Task 1.1**: Create `20260128000001_create_interpro_tables.sql` ✅
  - [x] `interpro_entry_metadata` table
  - [x] `protein_signatures` table
  - [x] `interpro_member_signatures` table
  - [x] `interpro_go_mappings` table
  - [x] `protein_interpro_matches` table
  - [x] `interpro_external_references` table
  - [x] `interpro_entry_stats` table
  - [x] All indexes (37 indexes - exceeds target!)
  - [x] All foreign key constraints (14 FKs)
  - [x] All check constraints (4 constraints)
  - [x] Trigger function `update_interpro_stats()`
  - [x] Trigger on `protein_interpro_matches`
  - [x] Comments on all tables

- [x] **Task 1.2**: Test migration ✅
  - [x] Run migration on dev database
  - [x] Verify all tables created (7/7)
  - [x] Verify all indexes created (50 indexes)
  - [x] Verify all constraints work (16 FKs, 2 CHECKs, 7 UNIQUEs)
  - [x] Test trigger function
  - [x] Test CHECK constraints (start_position > 0, end >= start)
  - [x] Verify source_type constraint includes 'interpro_entry'
  - [x] Document test results in `docs/interpro-migration-test-report.md`

- [~] **Task 1.3**: Generate SQLx offline data (DEFERRED)
  - [~] Run `just sqlx-prepare` - Database connection issue, defer to CI or later
  - [ ] Commit `.sqlx/` files
  - **Note**: Not critical for local dev (SQLX_OFFLINE=false), SQLx compiles against live DB

---

## Phase 2: Data Models (Est: 2 days) ✅ COMPLETE

### Rust Structs

- [x] **Task 2.1**: Create `crates/bdp-server/src/ingest/interpro/models.rs` ✅
  - [x] `InterProEntry` struct
  - [x] `ProteinSignature` struct
  - [x] `ProteinMatch` struct
  - [x] `InterProMetadata` struct (DB row)
  - [x] `MemberSignature` struct
  - [x] `GoMapping` struct
  - [x] `ExternalReference` struct
  - [x] Implement `Default`, `Clone`, `Debug` traits
  - [x] Add Serde derives where needed
  - [x] All database models with `FromRow` derive
  - [x] Helper methods for enum conversions

- [x] **Task 2.2**: Create enums ✅
  - [x] `EntryType` enum (Family, Domain, Repeat, Site, Homologous_superfamily)
  - [x] `SignatureDatabase` enum (Pfam, SMART, PROSITE, etc.)
  - [x] Implement string conversions (FromStr, Display)
  - [x] Unit tests for enum conversions
  - [x] SQLx type integration

---

## Phase 3: Parsers (Est: 3 days) ✅ COMPLETE

### protein2ipr.dat.gz Parser

- [x] **Task 3.1**: Create `crates/bdp-server/src/ingest/interpro/parser.rs` ✅
  - [x] `Protein2IprParser` struct
  - [x] `parse_line()` method for TSV parsing
  - [x] `parse_file()` method for streaming
  - [x] Handle gzip decompression
  - [x] Error handling for malformed lines
  - [x] Line number tracking for debugging
  - [x] Support for optional e-value and score fields

- [x] **Task 3.2**: Unit tests for parser ✅
  - [x] Test valid lines
  - [x] Test malformed lines (too few fields)
  - [x] Test minimal data (optional fields empty)
  - [x] Test multiple signature databases
  - [x] Error handling tests

### entry.list Parser

- [x] **Task 3.3**: Create `EntryListParser` struct ✅
  - [x] `parse_file()` method
  - [x] `parse_reader()` method for streaming
  - [x] Parse entry type mappings
  - [x] Parse entry names/descriptions
  - [x] Error handling with line numbers
  - [x] Support for optional short_name and description

- [x] **Task 3.4**: Unit tests for entry list parser ✅
  - [x] Test valid entries
  - [x] Test entry types (Domain, Family, etc.)
  - [x] Test optional fields (short_name, description)
  - [x] Test invalid entry type error handling

---

## Phase 4: Cross-Reference Helpers (Est: 2 days) ✅ COMPLETE

### ProteinLookupHelper

- [x] **Task 4.1**: Create `crates/bdp-server/src/ingest/interpro/helpers.rs` ✅
  - [x] `ProteinLookupHelper` struct
  - [x] `new()` constructor
  - [x] `load_batch()` batch method
  - [x] `get()` method to retrieve from cache
  - [x] `contains()` method
  - [x] `clear()` and `cache_size()` for cache management
  - [x] Efficient batch lookups with HashMap cache

- [x] **Task 4.2**: Unit tests for helper ✅
  - [x] Test initialization
  - [x] Test cache behavior (insert, get, clear)
  - [x] Test contains method
  - [x] Test cache_size tracking

### GOTermLookupHelper

- [x] **Task 4.3**: Create `GoTermLookupHelper` struct ✅
  - [x] `lookup_go_term()` by GO ID via `get()` method
  - [x] `load_batch()` for bulk lookups
  - [x] `get()` to retrieve latest version
  - [x] Cache management (clear, cache_size)
  - [x] Helper returns (data_source_id, version_id)

- [x] **Task 4.4**: Unit tests for GO helper ✅
  - [x] Test initialization
  - [x] Test cache operations
  - [x] Test contains() method
  - [x] **BONUS**: SignatureLookupHelper and InterProEntryLookupHelper also created

---

## Phase 5: Storage Layer (Est: 4 days) ✅ COMPLETE

### InterProStorage

- [x] **Task 5.1**: Create `crates/bdp-server/src/ingest/interpro/storage.rs` ✅
  - [ ] `InterProStorage` struct
  - [ ] `new()` and `with_s3()` constructors
  - [ ] `setup_citations()` method

- [ ] **Task 5.2**: Batch registry entry creation
  - [ ] `create_registry_entries_batch()` method
  - [ ] Use `QueryBuilder` for batch insert
  - [ ] Handle conflicts (ON CONFLICT)
  - [ ] Return Vec<Uuid> of entry IDs

- [ ] **Task 5.3**: Batch data source creation
  - [ ] `create_data_sources_batch()` method
  - [ ] Link to registry entries
  - [ ] Return Vec<Uuid> of data source IDs

- [ ] **Task 5.4**: Batch metadata creation
  - [ ] `create_metadata_batch()` method
  - [ ] Insert into `interpro_entry_metadata`
  - [ ] Handle replacement_interpro_id

- [ ] **Task 5.5**: Batch version creation
  - [ ] `create_versions_batch()` method
  - [ ] Create initial v1.0 for each entry
  - [ ] Return Vec<Uuid> of version IDs

### Signature Management

- [ ] **Task 5.6**: Signature storage
  - [ ] `get_or_create_signature()` method
  - [ ] Insert into `protein_signatures`
  - [ ] Handle duplicates (ON CONFLICT)
  - [ ] Cache signature IDs

- [ ] **Task 5.7**: Member signatures insertion
  - [ ] `insert_member_signatures_batch()` method
  - [ ] Bulk insert into `interpro_member_signatures`
  - [ ] Link InterPro entries to signatures

### GO Mappings

- [ ] **Task 5.8**: GO mappings insertion
  - [ ] `insert_go_mappings_batch()` method
  - [ ] Use `GoTermLookupHelper`
  - [ ] Get GO term data_source_id and version_id
  - [ ] Bulk insert with version FKs

### Protein Matches (Critical!)

- [ ] **Task 5.9**: Protein matches insertion
  - [ ] `insert_protein_matches_batch()` method
  - [ ] Use `ProteinLookupHelper` for bulk lookups
  - [ ] Get protein data_source_id and version_id
  - [ ] Get signature_id
  - [ ] Bulk insert with all FKs
  - [ ] Handle missing proteins (log warnings)
  - [ ] Track orphaned matches count

- [ ] **Task 5.10**: External references insertion
  - [ ] `insert_external_references_batch()` method
  - [ ] Parse and insert PDB, CATH, Wikipedia refs

### S3 File Upload

- [ ] **Task 5.11**: File generation and upload
  - [ ] `generate_tsv_file()` for matches
  - [ ] `generate_json_file()` for structured data
  - [ ] `generate_metadata_file()` for entry metadata
  - [ ] `upload_files_parallel()` using futures
  - [ ] Compute SHA256 checksums

### Integration Tests

- [ ] **Task 5.12**: Storage integration tests
  - [ ] Test full entry storage workflow
  - [ ] Test batch operations
  - [ ] Test FK constraints
  - [ ] Test trigger updates stats
  - [ ] Test with real InterPro sample data

---

## Phase 6: Versioning Logic (Est: 3 days)

### Version Bump Detector

- [ ] **Task 6.1**: Create `crates/bdp-server/src/ingest/interpro/version_detector.rs`
  - [ ] `InterProBumpDetector` struct
  - [ ] Implement `VersionBumpDetector` trait
  - [ ] `detect_changes()` method

- [ ] **Task 6.2**: Change detection logic
  - [ ] Detect entry obsolescence → MAJOR
  - [ ] Detect entry type change → MAJOR
  - [ ] Detect >50% protein loss → MAJOR
  - [ ] Detect protein additions → MINOR
  - [ ] Detect description changes → MINOR
  - [ ] Detect signature additions → MINOR
  - [ ] Detect <10% protein loss → MINOR

- [ ] **Task 6.3**: Changelog generation
  - [ ] Create `ChangelogEntry` for each change
  - [ ] Create `ChangelogSummary` with stats
  - [ ] Generate human-readable summary text
  - [ ] Return `VersionChangelog`

### Versioning Strategy

- [ ] **Task 6.4**: Create `VersioningStrategy::interpro()`
  - [ ] Define major_triggers
  - [ ] Define minor_triggers
  - [ ] Set cascade_on_major = false
  - [ ] Set cascade_on_minor = false
  - [ ] Add to `get_organization_versioning_strategy()`

### Cascade Logic

- [ ] **Task 6.5**: Create `cascade_versioning.rs`
  - [ ] `cascade_uniprot_version_to_interpro()` function
  - [ ] Find all affected InterPro entries
  - [ ] Create MINOR bumps for each
  - [ ] Copy matches with updated version FKs
  - [ ] Return `Vec<CascadeResult>`

- [ ] **Task 6.6**: Integration with versioning module
  - [ ] Export cascade function from `interpro` module
  - [ ] Call from versioning system when UniProt bumps

### Tests

- [ ] **Task 6.7**: Version detection tests
  - [ ] Test MAJOR bump scenarios
  - [ ] Test MINOR bump scenarios
  - [ ] Test NO CHANGE scenarios
  - [ ] Test changelog generation

- [ ] **Task 6.8**: Cascade tests
  - [ ] Test UniProt → InterPro cascade
  - [ ] Test version FK updates
  - [ ] Test multiple entries cascading

---

## Phase 7: FTP Downloader (Est: 2 days)

### InterProFtp

- [ ] **Task 7.1**: Create `crates/bdp-server/src/ingest/interpro/ftp.rs`
  - [ ] `InterProFtp` struct
  - [ ] Reuse `common/ftp.rs` infrastructure
  - [ ] FTP connection to `ftp.ebi.ac.uk/pub/databases/interpro/`

- [ ] **Task 7.2**: Download methods
  - [ ] `download_protein2ipr()` method
  - [ ] `download_entry_list()` method
  - [ ] `download_names_dat()` method (optional)
  - [ ] Handle gzip decompression
  - [ ] Progress tracking

- [ ] **Task 7.3**: Version discovery
  - [ ] `discover_versions()` method
  - [ ] List `/releases/` directory
  - [ ] Parse version numbers (98.0, 99.0, 103.0)
  - [ ] Return sorted list of versions

- [ ] **Task 7.4**: Tests
  - [ ] Test FTP connection
  - [ ] Test file downloads
  - [ ] Test version discovery
  - [ ] Mock FTP for unit tests

---

## Phase 8: Configuration (Est: 1 day)

### Config Struct

- [ ] **Task 8.1**: Create `crates/bdp-server/src/ingest/interpro/config.rs`
  - [ ] `InterProFtpConfig` struct
  - [ ] FTP URL configuration
  - [ ] Batch size configuration
  - [ ] Parse limit for testing
  - [ ] Default values

- [ ] **Task 8.2**: Environment variables
  - [ ] `INTERPRO_FTP_URL`
  - [ ] `INTERPRO_BATCH_SIZE`
  - [ ] `INTERPRO_PARSE_LIMIT`
  - [ ] Document in README

### Citation Policy

- [ ] **Task 8.3**: Create `interpro_citation_policy()`
  - [ ] Add to `crates/bdp-server/src/ingest/citations.rs`
  - [ ] InterPro 2025 citation
  - [ ] License: CC0 1.0
  - [ ] Attribution requirements

---

## Phase 9: Pipeline Orchestration (Est: 3 days)

### InterProPipeline

- [ ] **Task 9.1**: Create `crates/bdp-server/src/ingest/interpro/pipeline.rs`
  - [ ] `InterProPipeline` struct
  - [ ] `new()` constructor
  - [ ] `run()` method for full pipeline

- [ ] **Task 9.2**: Pipeline steps
  - [ ] Step 1: Discover latest InterPro version
  - [ ] Step 2: Check if already ingested
  - [ ] Step 3: Download protein2ipr.dat.gz
  - [ ] Step 4: Download entry.list
  - [ ] Step 5: Parse both files
  - [ ] Step 6: Group matches by InterPro entry
  - [ ] Step 7: Store entries (batch)
  - [ ] Step 8: Upload files to S3
  - [ ] Step 9: Create version changelog
  - [ ] Step 10: Log completion

- [ ] **Task 9.3**: Differential ingestion
  - [ ] `run_differential()` method
  - [ ] Detect changed entries only
  - [ ] Skip unchanged entries
  - [ ] Version bump changed entries

- [ ] **Task 9.4**: Progress tracking
  - [ ] Progress bars for download
  - [ ] Progress bars for parsing
  - [ ] Progress bars for storage
  - [ ] Log statistics

### Orchestrator Integration

- [ ] **Task 9.5**: Create job definition
  - [ ] `InterProIngestJob` struct
  - [ ] Implement apalis `Job` trait
  - [ ] Add to `IngestOrchestrator`

- [ ] **Task 9.6**: Add to orchestrator
  - [ ] Register job handler
  - [ ] Add to auto-discovery
  - [ ] Configure scheduling (8-week cycle)

### Tests

- [ ] **Task 9.7**: Pipeline integration tests
  - [ ] Test with sample InterPro data (100 entries)
  - [ ] Test full workflow end-to-end
  - [ ] Test error handling
  - [ ] Test resumption after failure

---

## Phase 10: Module Integration (Est: 1 day)

### Module Exports

- [ ] **Task 10.1**: Create `crates/bdp-server/src/ingest/interpro/mod.rs`
  - [ ] Export all public types
  - [ ] Export pipeline
  - [ ] Export helpers
  - [ ] Module documentation

- [ ] **Task 10.2**: Update parent module
  - [ ] Add `pub mod interpro;` to `crates/bdp-server/src/ingest/mod.rs`
  - [ ] Export public API

### Feature Module Integration

- [ ] **Task 10.3**: Add InterPro to features (optional)
  - [ ] Create `features/interpro/` module
  - [ ] Add query handlers for InterPro data
  - [ ] Add API endpoints if needed

---

## Phase 11: Documentation (Est: 1 day)

### User Documentation

- [ ] **Task 11.1**: Update `ROADMAP.md`
  - [ ] Mark InterPro as implemented
  - [ ] Update data sources section
  - [ ] Update statistics

- [ ] **Task 11.2**: Create InterPro user guide
  - [ ] CLI usage examples
  - [ ] Query examples
  - [ ] Versioning explanation

### Developer Documentation

- [ ] **Task 11.3**: Update `docs/agents/`
  - [ ] Add InterPro to architecture docs
  - [ ] Document cascade versioning
  - [ ] Add schema diagrams

- [ ] **Task 11.4**: Code documentation
  - [ ] Add rustdoc comments to all public APIs
  - [ ] Add examples to doc comments
  - [ ] Generate and review `cargo doc`

---

## Phase 12: Testing & Validation (Est: 3 days)

### Unit Tests

- [ ] **Task 12.1**: Parser tests (20+ tests)
- [ ] **Task 12.2**: Helper tests (10+ tests)
- [ ] **Task 12.3**: Storage tests (15+ tests)
- [ ] **Task 12.4**: Version detection tests (10+ tests)

### Integration Tests

- [ ] **Task 12.5**: End-to-end pipeline test
  - [ ] Download InterPro 103.0 sample
  - [ ] Ingest 100 entries
  - [ ] Verify database state
  - [ ] Verify S3 files
  - [ ] Verify statistics

- [ ] **Task 12.6**: Cascade versioning test
  - [ ] Simulate UniProt version bump
  - [ ] Verify InterPro cascade
  - [ ] Verify version FKs updated

- [ ] **Task 12.7**: Cross-reference tests
  - [ ] Query proteins by InterPro entry
  - [ ] Query InterPro entries by protein
  - [ ] Query by signature
  - [ ] Query by GO term

### Performance Tests

- [ ] **Task 12.8**: Benchmark batch operations
  - [ ] Measure insert speed
  - [ ] Measure query speed
  - [ ] Verify index usage (`EXPLAIN ANALYZE`)

- [ ] **Task 12.9**: Large dataset test
  - [ ] Ingest full InterPro 103.0 (~40K entries)
  - [ ] Measure total time
  - [ ] Verify data integrity
  - [ ] Check database size

---

## Phase 13: Production Deployment (Est: 2 days)

### Data Ingestion

- [ ] **Task 13.1**: Run full ingestion
  - [ ] Ingest InterPro 103.0 (Dec 2024)
  - [ ] Verify ~40,000 entries created
  - [ ] Verify ~200M matches created
  - [ ] Verify S3 files uploaded

- [ ] **Task 13.2**: Validate data quality
  - [ ] Spot-check 100 random entries
  - [ ] Verify protein matches
  - [ ] Verify GO mappings
  - [ ] Verify member signatures

### Monitoring

- [ ] **Task 13.3**: Set up monitoring
  - [ ] Track ingestion job status
  - [ ] Monitor database size
  - [ ] Monitor S3 storage usage
  - [ ] Set up alerts

---

## Estimated Timeline

| Phase | Tasks | Est. Time | Priority |
|-------|-------|-----------|----------|
| 1. Database Schema | 3 tasks | 2 days | HIGH |
| 2. Data Models | 2 tasks | 2 days | HIGH |
| 3. Parsers | 4 tasks | 3 days | HIGH |
| 4. Cross-Ref Helpers | 4 tasks | 2 days | HIGH |
| 5. Storage Layer | 12 tasks | 4 days | HIGH |
| 6. Versioning Logic | 8 tasks | 3 days | HIGH |
| 7. FTP Downloader | 4 tasks | 2 days | MEDIUM |
| 8. Configuration | 3 tasks | 1 day | MEDIUM |
| 9. Pipeline Orchestration | 7 tasks | 3 days | HIGH |
| 10. Module Integration | 3 tasks | 1 day | MEDIUM |
| 11. Documentation | 4 tasks | 1 day | LOW |
| 12. Testing & Validation | 9 tasks | 3 days | HIGH |
| 13. Production Deployment | 3 tasks | 2 days | MEDIUM |

**Total**: 69 tasks, ~29 days (~6 weeks)

---

## Progress Tracking

**Overall Progress**: 31/69 tasks (44.9%)

**Phase Status**:
- [ ] Phase 1: Database Schema (2/3) ✅ Migration Created & Tested (Task 1.3 deferred)
- [x] Phase 2: Data Models (2/2) ✅ COMPLETE
- [x] Phase 3: Parsers (4/4) ✅ COMPLETE
- [x] Phase 4: Cross-Ref Helpers (4/4) ✅ COMPLETE
- [x] Phase 5: Storage Layer (12/12) ✅ COMPLETE
- [ ] Phase 6: Versioning Logic (0/8)
- [x] Phase 7: FTP Downloader (4/4) ✅ COMPLETE
- [x] Phase 8: Configuration (3/3) ✅ COMPLETE
- [ ] Phase 6: Versioning Logic (0/8)
- [ ] Phase 7: FTP Downloader (0/4)
- [ ] Phase 8: Configuration (0/3)
- [ ] Phase 9: Pipeline Orchestration (0/7)
- [ ] Phase 10: Module Integration (0/3)
- [ ] Phase 11: Documentation (0/4)
- [ ] Phase 12: Testing & Validation (0/9)
- [ ] Phase 13: Production Deployment (0/3)

---

## Next Actions

1. ✅ Review design document: `docs/interpro-design-corrected.md`
2. ✅ Review database design philosophy: `docs/agents/database-design-philosophy.md`
3. ✅ Phase 1: Database Schema (2/3 tasks - Task 1.3 deferred)
4. ✅ Phase 2: Data Models (2/2 tasks complete)
5. ✅ Phase 3: Parsers (4/4 tasks complete)
6. ✅ Phase 4: Cross-Reference Helpers (4/4 tasks complete)
7. ▶️ **NEXT**: Phase 5 - Storage Layer (0/12 tasks)
8. See progress summary: `docs/interpro-progress-summary.md`

---

## Notes

- Follow relational design pattern (NO JSONB for primary data)
- Use version-specific foreign keys everywhere
- MAJOR.MINOR versioning only (no patch)
- Implement cascade versioning for dependency bumps
- Batch operations for performance (500/chunk)
- Comprehensive testing before production

---

**Last Updated By**: Phase 1 Task 1.2 completed (2026-01-28)
**Next Review**: After Phase 1 completion (Task 1.3)
