# BDP Development Roadmap

Comprehensive roadmap for building the Bioinformatics Dependencies Platform.

## Vision

BDP aims to be the **npm for bioinformatics**, starting with versioned data source management (proteins, genomes, annotations) before expanding to software tools. The platform enables reproducible research through lockfiles, semantic versioning, and dependency management.

## Quick Progress Overview

| Phase | Status | Key Deliverables |
|-------|--------|------------------|
| **Phase 1: Backend** | ✅ Complete | Database (67 migrations), 25+ API endpoints, S3 storage, search optimization, 750+ tests |
| **Phase 2: Ingestion** | ✅ 95% Complete | UniProt, NCBI Taxonomy, GenBank/RefSeq, Gene Ontology pipelines fully coded |
| **Phase 3: CLI** | ✅ Complete | 10 commands including audit system, multi-platform installers, CI/CD |
| **Phase 3.5: Release** | ✅ Complete | Automated releases, version management, documentation |
| **Phase 3.8: Audit** | ✅ Complete | SQLite audit trail, FDA/NIH/EMA exports, hash chain verification |
| **Phase 4: Frontend** | ✅ 80% Complete | Next.js app, all pages, jobs dashboard, docs, needs E2E testing |
| **Phase 5: Launch** | 🔄 80% Complete | CLI released, docs complete, frontend built, infrastructure ready, need data + credentials |

**Current Version**: 0.1.0 (ready for first release bump!)

## Architecture Decision: CQRS with Mediator Pattern

**Updated 2026-01-16**: The project has adopted a **mediator-based CQRS architecture** instead of the traditional layered approach:

- **Commands** (write operations) and **Queries** (read operations) are separate
- **No shared database layer** - each handler contains inline SQL queries
- **Function-based handlers** instead of handler structs
- **Mediator pattern** for command/query dispatch
- **Vertical slicing** - features are self-contained
- **Audit middleware** automatically logs all commands
- **Minimal boilerplate** with focused, concise code

See [Mediator-CQRS Architecture](./docs/agents/implementation/mediator-cqrs-architecture.md) for details.

## MVP Scope

**Core Focus**: Data source versioning and retrieval
- **Primary Use Case**: Version-controlled access to biological databases (UniProt proteins)
- **Not in MVP**: Software package management (tools come later)

**Key Features**:
1. Backend registry with PostgreSQL database
2. CLI for managing data sources locally
3. Web interface for browsing and discovery
4. UniProt protein scraping and ingestion
5. Local caching with team sharing support
6. Lockfiles for reproducibility

## Design Documents

Detailed technical specifications are in `docs/agents/design/`:

- **[Database Schema](./docs/agents/design/database-schema.md)** - PostgreSQL schema, tables, relationships
- **[File Formats](./docs/agents/design/file-formats.md)** - bdp.yml, bdl.lock, dependency cache
- **[API Design](./docs/agents/design/api-design.md)** - REST endpoints, response formats
- **[Cache Strategy](./docs/agents/design/cache-strategy.md)** - Local caching, team sharing, file locking
- **[Dependency Resolution](./docs/agents/design/dependency-resolution.md)** - How aggregate sources work
- **[Version Mapping](./docs/agents/design/version-mapping.md)** - External to internal version translation
- **[UniProt Ingestion](./docs/agents/design/uniprot-ingestion.md)** - Automated scraping and parsing

## Implementation Guides

Backend implementation patterns in `docs/agents/implementation/`:

- **[Mediator-CQRS Architecture](./docs/agents/implementation/mediator-cqrs-architecture.md)** - **MANDATORY** CQRS pattern guide
- **[CQRS Architecture](./docs/agents/implementation/cqrs-architecture.md)** - Detailed CQRS implementation
- **[SQLx Guide](./docs/agents/implementation/sqlx-guide.md)** - SQLx offline mode and best practices
- **[Backend Architecture](./docs/agents/backend-architecture.md)** - General backend architecture

## Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| **Backend** | Rust + axum | 0.7 |
| **Database** | PostgreSQL | 16+ |
| **ORM** | SQLx | 0.8 |
| **CQRS** | mediator crate | 0.2 |
| **CLI** | Rust + clap | 4.x |
| **Frontend** | Next.js | 16 |
| **Docs** | Nextra | 3.0 |
| **UI** | Tailwind CSS + Radix UI | Latest |
| **Storage** | MinIO / S3 | Latest |
| **Reverse Proxy** | Caddy | 2.x |
| **Task Runner** | Just | Latest |

See [Technology Stack](./docs/agents/implementation/stack.md) for detailed rationale.

## Phase 1: Foundation - Backend Registry ✅ COMPLETE

**Goal**: Build the core registry backend with database and API.

**Status**: All tasks complete. Backend is production-ready with full database, API, and storage integration.

### 1.1 Database Setup ✅ COMPLETE

**Completed**:
- ✅ PostgreSQL database created
- ✅ Full schema implemented from [Database Schema](./docs/agents/design/database-schema.md)
  - Organizations table (with website, logo_url, is_system)
  - Registry entries (base table)
  - Data sources (proteins, genomes)
  - Tools (schema ready)
  - Versions with dual versioning (internal + external)
  - Version files (multiple formats)
  - Dependencies (for aggregates with efficient pagination)
  - Organisms (taxonomy)
  - Protein metadata
  - Citations
  - Tags
  - Downloads tracking
  - Version mappings (external → internal version translation)
  - Audit log (comprehensive audit trail)
- ✅ Indexes for performance
- ✅ SQLx migrations (**67 migration files** in `migrations/`)
- ✅ Triggers for denormalization (dependency_count, size_bytes calculation)
- ✅ Full-text search indexes (GIN indexes on tsvector columns)
- ✅ Search materialized views with performance optimization
- ✅ Seed data for system organizations
- ✅ Gene Ontology metadata tables
- ✅ Protein sequences and deduplication tables
- ✅ RefSeq/nucleotide sequences tables
- ✅ Citation policies and licenses tables

**Deliverables**:
- ✅ `migrations/` directory with **67 SQL migration files**
- ✅ Database initialization via `just db-setup`
- ✅ Seed data ready

**References**:
- [Database Schema](./docs/agents/design/database-schema.md)
- [Version Mapping](./docs/agents/design/version-mapping.md)
- [SQLx Guide](./docs/agents/implementation/sqlx-guide.md)

### 1.2 Rust API Server ✅ COMPLETE

**Completed**:
- ✅ Axum project structure initialized
- ✅ SQLx connection pool configured
- ✅ **CQRS architecture with mediator pattern** implemented
- ✅ **Organizations feature fully implemented** (5 handlers):
  - Commands: `create`, `update`, `delete` (with inline SQL, no shared DB layer)
  - Queries: `list`, `get` (by slug or ID)
  - All handlers are standalone async functions
  - Comprehensive validation and error handling
  - Full test coverage with `#[sqlx::test]`
- ✅ **Data Sources feature fully implemented** (8 handlers):
  - Commands: `create`, `update`, `delete`, `publish_version`
  - Queries: `list`, `get`, `get_version`, `list_dependencies`
  - Full CRUD operations with complex relationships
  - Version publishing with files, checksums, citations
  - Comprehensive validation and error handling
- ✅ **Search feature implemented** (3 handlers):
  - Unified search across organizations, data_sources, tools
  - PostgreSQL full-text search with relevance ranking
  - **Search suggestions/autocomplete**
  - **Materialized views for performance** (pg_trgm GIN indexes)
  - Filtering by type, organism, format
  - Pagination support
  - **Refresh search index endpoint**
- ✅ **Resolve feature implemented** (1 handler):
  - Manifest resolution (bdp.yml → lockfile)
  - Recursive dependency resolution
  - Conflict detection
  - Spec parsing (org:name@version-format)
- ✅ **Jobs feature implemented** (3 handlers):
  - `GET /api/v1/jobs` - List all ingestion jobs
  - `GET /api/v1/jobs/:id` - Get job details
  - `GET /api/v1/sync-status` - Get sync status per organization
- ✅ **Stats feature implemented**:
  - `GET /stats` - Platform statistics (total sources, downloads, etc.)
- ✅ **Audit middleware** implemented and tested (14 integration tests)
  - Automatically logs all commands (POST/PUT/PATCH/DELETE)
  - Excludes queries (GET) to reduce noise
  - Captures user ID, IP, user agent, request body, metadata
  - Non-blocking async writes for performance
- ✅ **CORS middleware** configured
  - Configurable allowed origins
  - Custom header support (x-user-id)
  - Proper preflight handling
- ✅ **Rate limiting middleware** implemented
  - Token bucket algorithm (tower-governor)
  - Configurable (default: 100 req/min per IP)
  - Per-IP rate limiting
- ✅ Error handling with typed error enums per feature
- ✅ Request logging and tracing (tracing + tracing-subscriber)
- ✅ Health check endpoint
- ✅ Graceful shutdown handling
- ✅ **25+ handlers registered in mediator** (5 orgs + 10 data_sources + 3 search + 1 resolve + 3 jobs + stats + files)

**Pending**:
- [ ] API documentation (OpenAPI/Swagger)

**Architecture Notes**:
- Using **mediator crate** for command/query dispatch
- **No shared DB layer** - each handler contains inline SQL queries
- **Function-based handlers** instead of handler structs
- **Vertical slicing** - each feature is completely self-contained
- **Tower middleware** for cross-cutting concerns (audit, CORS, tracing)
- **Just command runner** instead of shell scripts (60+ commands)

**Deliverables**:
- ✅ `crates/bdp-server/` Rust project with CQRS architecture
- ✅ Running API server on port 8000
- ✅ **25+ RESTful API endpoints** (5 organizations + 10 data sources + 3 search + 1 resolve + 3 jobs + stats + files)
- ✅ All endpoints following CQRS pattern
- ✅ Health check endpoint at `/health`
- ✅ Stats endpoint at `/stats`
- ✅ Audit log endpoint at `/api/v1/audit`
- ✅ Jobs monitoring endpoints at `/api/v1/jobs`
- ✅ Comprehensive test coverage with `#[sqlx::test]` (**750+ tests**)
- ✅ CORS and rate limiting configured

**References**:
- **[Mediator-CQRS Architecture](./docs/agents/implementation/mediator-cqrs-architecture.md)** - **MANDATORY**
- [API Design](./docs/agents/design/api-design.md)
- [CQRS Architecture](./docs/agents/implementation/cqrs-architecture.md)
- [Audit Middleware Testing](./docs/audit-middleware-testing.md)

### 1.3 S3/MinIO Integration ✅ COMPLETE

**Completed**:
- ✅ MinIO set up in docker-compose with automatic bucket initialization
- ✅ S3 client wrapper using AWS SDK for Rust
- ✅ File upload with SHA256 checksums
- ✅ File download with presigned signed URLs (1 hour expiration)
- ✅ S3 key structure implemented:
  - Data sources: `data-sources/{org}/{name}/{version}/{file}`
  - Tools: `tools/{org}/{name}/{version}/{file}`
- ✅ Large file support with streaming uploads
- ✅ **Files feature** implemented (CQRS pattern):
  - `UploadFileCommand` - Upload files with checksum verification
  - `DownloadFileQuery` - Generate presigned download URLs
  - Routes: `POST /files/:org/:name/:version/:filename`, `GET /files/:org/:name/:version/:filename`
- ✅ **30 comprehensive integration tests** for storage operations
- ✅ **12 unit tests** for files feature validation

**Deliverables**:
- ✅ S3 storage backend configured (MinIO + AWS S3 support)
- ✅ Upload/download functions with streaming support
- ✅ Checksum verification (SHA256)
- ✅ Storage module (~477 lines)
- ✅ Files feature (~600 lines)
- ✅ Comprehensive test suite (~1,035 lines)

**References**:
- [Cache Strategy](./docs/agents/design/cache-strategy.md)
- [UniProt Ingestion](./docs/agents/design/uniprot-ingestion.md)

### 1.4 Testing & Documentation ✅ COMPLETE

**Completed**:
- ✅ Unit tests for all features (inline `#[cfg(test)]` modules)
- ✅ Integration tests using `#[sqlx::test]` and `#[tokio::test]` attributes
- ✅ **Comprehensive test coverage**:
  - Organizations: ~12 tests (4 validation + 8 integration)
  - Data Sources: ~32 tests (4 per command/query)
  - Search: ~8 tests (4 validation + 4 integration)
  - Resolve: ~12 tests (9 parsing + 3 integration)
  - Middleware: 6 integration tests (CORS + rate limiting)
  - **Storage**: 30 integration tests (upload, download, presigned URLs, etc.)
  - **Files**: 12 unit tests (upload/download validation)
- ✅ Audit middleware tests (14 comprehensive integration tests)
- ✅ Search performance tests (load tests, integration tests)
- ✅ Test coverage >70% for all features (**750+ tests total**)
- ✅ Development setup guide ([SETUP.md](./SETUP.md))
- ✅ Testing guide ([TESTING.md](./TESTING.md))
- ✅ Backend architecture documentation
- ✅ CQRS implementation guides
- ✅ Phase 1.2 completion summary ([docs/phase-1.2-completion-summary.md](./docs/phase-1.2-completion-summary.md))
- ✅ Phase 1.3 completion summary ([docs/phase-1.3-completion-summary.md](./docs/phase-1.3-completion-summary.md))
- ✅ CI/CD pipeline (GitHub Actions) - See Phase 3.5

**Pending** (Optional):
- [ ] API endpoint documentation (OpenAPI)
- [ ] Load testing

**Deliverables**:
- ✅ Test coverage >70% for all features (**750+ tests total**)
- ✅ CI/CD pipeline operational (Phase 3.5)
- ✅ Developer documentation (SETUP.md, TESTING.md, multiple guides)

## Phase 2: Data Ingestion Pipelines ✅ 95% COMPLETE

**Status**: ✅ All pipelines fully implemented and coded. Ready for production data population.

**Note**: Complete ETL pipelines built for 4 major data sources (~80+ files, ~18,000+ lines). All parsing, storage, and orchestration code is complete. Only needs integration testing with production data.

### 2.1 Version Mapping Implementation ✅ COMPLETE

**Completed** (Agent 3):
- ✅ Version mapping functions implemented in `crates/bdp-server/src/ingest/uniprot/version_mapping.rs`
  - `map_uniprot_version()` - Date-based (YYYY_MM) to semantic versioning
  - Auto-increment logic for new releases (1.0, 1.1, 1.2, etc.)
  - Database lookup with caching
- ✅ Unit tests for mapping logic (12+ tests)
- ✅ Integration with UniProt pipeline
- ✅ Error handling for invalid versions

**Pending**:
- [ ] Add version_mappings table population script for historical data
- [ ] Add API endpoints for version lookups (optional - not critical for MVP)

**Note**: Database schema for version_mappings already exists (migration complete).

**Deliverables**:
- ✅ Version mapping module in `crates/bdp-server/src/ingest/uniprot/version_mapping.rs` (~250 lines)
- ✅ Unit tests (12+ tests)
- ⬜ Populated version_mappings table (needs initial data load)

**References**:
- [Version Mapping](./docs/agents/design/version-mapping.md)

### 2.2 UniProt Ingestion Pipeline ✅ COMPLETE

**Fully Implemented**:
- ✅ `UniProtFtp` - FTP downloader with release discovery
- ✅ `DatParser` - Full UniProt flat file format parser
  - Entry-level parsing (ID, AC, DE, GN, OS, OX, SQ sections)
  - Protein metadata extraction
  - Citations parsing
  - Sequence extraction
  - Streaming support for large files
- ✅ `UniProtStorage` - Store to PostgreSQL + S3
- ✅ `UniProtPipeline` - End-to-end pipeline orchestration
- ✅ `VersionDiscovery` - Discover UniProt releases
- ✅ `UniProtParser`/`UniProtFormatter` - Format adapters
- ✅ Version mapping (external → internal)
- ✅ Deduplication logic
- ✅ Batch insert optimization (500-1000 record chunks)
- ✅ Configuration (FTP URLs, batch sizes, parse limits)

**Examples/Tests**:
- `examples/run_uniprot_ingestion.rs` - Manual trigger
- `examples/run_historical_ingestion.rs` - Historical versions
- `examples/uniprot_pipeline_with_dedup.rs` - Deduplication
- `examples/test_storage_pipeline.rs` - Storage testing

**Deliverables**:
- ✅ `crates/bdp-server/src/ingest/uniprot/` module (~6,600+ lines)
- ✅ Complete DAT parser implementation
- ✅ FTP client with release discovery
- ✅ Storage integration (PostgreSQL + S3)

**References**:
- [UniProt Ingestion](./docs/agents/design/uniprot-ingestion.md)

### 2.3 NCBI Taxonomy Pipeline ✅ COMPLETE

**Fully Implemented**:
- ✅ `NcbiTaxonomyFtp` - FTP downloader for taxdump files
- ✅ `TaxdumpParser` - Parse taxdump files
  - `rankedlineage.dmp` - Taxonomic lineage
  - `merged.dmp` - Merged taxa tracking
  - `delnodes.dmp` - Deleted taxa tracking
- ✅ `NcbiTaxonomyStorage` - Store to PostgreSQL
- ✅ `NcbiTaxonomyPipeline` - End-to-end pipeline
- ✅ `TaxonomyVersionDiscovery` - Discover NCBI releases
- ✅ Tar.gz extraction and processing
- ✅ Batch operations (500 record chunks)

**Examples/Tests**:
- `bin/ncbi_taxonomy_test_small.rs` - Small dataset test
- `bin/ncbi_taxonomy_full_catchup.rs` - Full catchup ingestion

**Deliverables**:
- ✅ `crates/bdp-server/src/ingest/ncbi_taxonomy/` module (~3,100+ lines)
- ✅ Complete taxdump parser
- ✅ Merged/deleted taxa handling

### 2.4 GenBank/RefSeq Pipeline ✅ COMPLETE

**Fully Implemented**:
- ✅ `GenbankFtp` - FTP downloader for GenBank files
- ✅ `GenbankParser` - Parse GenBank flat file format
  - Feature parsing (CDS, source, organism, etc.)
  - Sequence extraction (FASTA generation)
  - Metadata extraction
- ✅ `GenbankStorage` - Store sequences + metadata
- ✅ `GenbankPipeline` - End-to-end pipeline
- ✅ `GenbankOrchestrator` - Job orchestration
- ✅ RefSeq sequences table
- ✅ Nucleotide sequences table
- ✅ Batch operations (500 record chunks)

**Examples/Tests**:
- `bin/genbank_test_phage.rs` - Phage GenBank test

**Deliverables**:
- ✅ `crates/bdp-server/src/ingest/genbank/` module (~2,500+ lines)
- ✅ Complete GenBank flat file parser
- ✅ FASTA sequence generation

### 2.5 Gene Ontology Pipeline ✅ COMPLETE

**Fully Implemented**:
- ✅ `GoDownloader` - HTTP downloader for GO files
- ✅ `OboParser` - Parse OBO ontology format
  - GO term extraction
  - Relationship parsing
  - Synonym handling
- ✅ `GafParser` - Parse GAF annotation files
  - Protein-GO annotations
  - Evidence codes
- ✅ `GoStorage` - Store to PostgreSQL
- ✅ `GoPipeline` - End-to-end pipeline
- ✅ GO term tables (terms, relationships, synonyms)
- ✅ Annotation tables (GAF data)
- ✅ Namespace support (BP, MF, CC)
- ✅ Batch operations (500-1000 record chunks)

**Examples/Tests**:
- `bin/go_test_sample.rs` - Sample GO data
- `bin/go_test_ftp.rs` - FTP download test
- `bin/go_test_human.rs` - Human proteins
- `bin/go_test_local_ontology.rs` - Local file parsing

**Deliverables**:
- ✅ `crates/bdp-server/src/ingest/gene_ontology/` module (~2,800+ lines)
- ✅ Complete OBO parser
- ✅ Complete GAF parser

### 2.6 Generic ETL Framework ✅ COMPLETE

**Fully Implemented**:
- ✅ `IngestionCoordinator` - Job orchestration
- ✅ `IngestionWorker` - Parallel processing
- ✅ `IngestionJob` - Job tracking with status
- ✅ `IngestionWorkUnit` - Unit of work abstraction
- ✅ `BatchConfig` - Batch size configuration
- ✅ Idempotent processing (resume on failure)
- ✅ PostgreSQL-backed state persistence
- ✅ Checksum verification (MD5, SHA-256)
- ✅ Metalink support
- ✅ Distributed coordinator pattern

**Deliverables**:
- ✅ `crates/bdp-server/src/ingest/framework/` module (~1,500+ lines)
- ✅ Reusable ETL infrastructure

### 2.7 Job Queue & Orchestration ✅ COMPLETE

**Fully Implemented**:
- ✅ **apalis job queue** with PostgreSQL backend
- ✅ `IngestOrchestrator` - Background job runner
- ✅ Version discovery for all sources
- ✅ Missing version detection
- ✅ Parallel pipeline execution
- ✅ Auto-start on server boot (`INGEST_ENABLED=true`)
- ✅ Job API endpoints for monitoring
- ✅ Cron scheduling capability

**Deliverables**:
- ✅ Job queue infrastructure
- ✅ API endpoints for job monitoring
- ✅ Background orchestrator

### 2.8 Initial Data Population 🔄 READY TO RUN

**Prerequisites**: ✅ All pipelines complete. Ready for production data ingestion.

**Tasks**:
- [ ] Run UniProt ingestion (SwissProt ~570k proteins)
- [ ] Run NCBI Taxonomy ingestion (~2.4M taxa)
- [ ] Run GenBank/RefSeq ingestion (selected genomes)
- [ ] Run Gene Ontology ingestion (~45k terms)
- [ ] Verify data integrity
- [ ] Build/refresh search indexes

**Note**: All code is written. This task is about running the pipelines and populating production data.

**Deliverables**:
- Database populated with real data
- Search indexes built
- Production data available

### Phase 2 Summary - Pipelines Complete (2026-01-26)

**What's Done** (95% of Phase 2):
- ✅ **80+ files created** (~18,000+ lines of code)
- ✅ **4 complete data source pipelines** (UniProt, NCBI Taxonomy, GenBank, Gene Ontology)
- ✅ **Generic ETL framework** (reusable for new sources)
- ✅ **Job queue infrastructure** (apalis + PostgreSQL)
- ✅ **All parsers fully implemented**:
  - UniProt DAT parser
  - NCBI taxdump parser
  - GenBank flat file parser
  - OBO ontology parser
  - GAF annotation parser
- ✅ **Storage integration** (PostgreSQL + S3)
- ✅ **Job monitoring API** (list jobs, get status)
- ✅ **Auto-start orchestrator** (background ingestion)
- ✅ **Version discovery** for all sources
- ✅ **Deduplication logic**
- ✅ **Batch processing** (optimized inserts)

**What Remains** (5%):
- ⬜ Run production data ingestion
- ⬜ Verify data integrity post-ingestion
- ⬜ Performance tuning for very large datasets

**Assessment**:
Phase 2 is **essentially complete**. All pipelines are fully coded and tested. The remaining work is operational: running the pipelines to populate production data and verifying the results.

## Phase 3: CLI Tool Development ✅ COMPLETE

**Goal**: Build command-line tool for researchers to manage data sources locally.

**Status**: All tasks complete. CLI is production-ready with full test coverage.

### 3.1 CLI Core ✅ COMPLETE

**Completed**:
- ✅ CLI project initialized with clap
- ✅ All commands implemented:
  - `bdp init` - Initialize project with bdp.yml
  - `bdp source add/remove/list` - Manage sources in manifest
  - `bdp pull` - Download and cache sources
  - `bdp status` - Show cache status
  - `bdp audit` - Verify integrity
  - `bdp clean` - Remove cached files
  - `bdp config` - Configuration management
  - `bdp uninstall` - Self-uninstall command
- ✅ Manifest parsing and writing (bdp.yml)
- ✅ Lockfile generation (bdl.lock)
- ✅ User-friendly output with colors and progress bars (indicatif)
- ✅ .gitignore management (automatic, idempotent)

**Deliverables**:
- ✅ `crates/bdp-cli/` Rust project (6,000+ lines)
- ✅ All CLI commands working
- ✅ User documentation (INSTALL.md, QUICK_START.md)

**References**:
- [Installation Guide](./INSTALL.md)
- [File Formats](./docs/agents/design/file-formats.md)

### 3.2 API Client ✅ COMPLETE

**Completed**:
- ✅ HTTP client implemented (reqwest)
- ✅ API client wrapper created
- ✅ All endpoints implemented:
  - Resolve manifest (POST /api/v1/resolve)
  - Download files with presigned URLs
  - Error handling and retries
- ✅ Structured error types
- ✅ Environment variable configuration (BDP_SERVER_URL)

**Deliverables**:
- ✅ API client module (~400 lines)
- ✅ Integration tests with API

**References**:
- [API Design](./docs/agents/design/api-design.md)

### 3.3 Local Cache Management ✅ COMPLETE

**Completed**:
- ✅ Cache directory structure implemented
  - `.bdp/cache/sources/{org}/{name}@{version}/`
  - Platform-specific cache location (XDG on Linux, AppData on Windows)
- ✅ SQLite database (bdp.db) with WAL mode
  - cache_entries table with full metadata
  - Automatic migrations via sqlx
- ✅ File download with:
  - SHA-256 checksum verification
  - Progress bars (indicatif)
  - Atomic writes (temp file → rename)
  - Efficient streaming
- ✅ Cache configuration via environment variables
- ✅ Cache cleanup strategies (all, unused, by age)

**Deliverables**:
- ✅ Cache management module (~600 lines)
- ✅ SQLite schema with migrations
- ✅ Download pipeline with progress tracking

**References**:
- [Cache Strategy](./docs/agents/design/cache-strategy.md)
- [File Formats](./docs/agents/design/file-formats.md)

### 3.4 Dependency Resolution ✅ COMPLETE

**Completed**:
- ✅ Manifest resolution via API (POST /api/v1/resolve)
- ✅ Lockfile generation (bdl.lock)
  - JSON format with lockfile_version
  - Source entries with checksums, sizes, versions
  - Tool entries (schema ready)
  - Generation timestamp
- ✅ Spec parsing (org:name@version-format)
- ✅ Version validation and error handling

**Deliverables**:
- ✅ Lockfile generator module (~300 lines)
- ✅ Spec parser with validation
- ✅ Comprehensive tests

**References**:
- [Dependency Resolution](./docs/agents/design/dependency-resolution.md)
- [File Formats](./docs/agents/design/file-formats.md)

### 3.5 Integrity & Auditing ✅ COMPLETE

**Completed**:
- ✅ `bdp audit` command implemented
- ✅ Checksum verification (SHA-256)
- ✅ Lockfile comparison
- ✅ Detailed error reporting (missing files, checksum mismatches)
- ✅ Colored output for status (✓ green, ✗ red)

**Deliverables**:
- ✅ Audit command (~200 lines)
- ✅ Integrity verification
- ✅ Detailed reporting

**References**:
- [Cache Strategy](./docs/agents/design/cache-strategy.md)

### 3.6 Team Cache Support

**Status**: Deferred to post-MVP

**Tasks**:
- [ ] Implement `bdp config cache set` for shared paths
- [ ] Add file locking mechanism (SQLite locks)
- [ ] Handle concurrent access
- [ ] Implement lock timeout and cleanup
- [ ] Test with multiple concurrent users

**Note**: Basic single-user cache is complete. Multi-user support will be added based on demand.

### 3.7 Testing ✅ COMPLETE

**Completed**:
- ✅ **61 comprehensive tests** (100% pass rate)
  - 20 unit tests (validation logic)
  - 24 integration tests (cache operations)
  - 17 command tests (CLI workflow)
- ✅ Test coverage for all modules:
  - Manifest parsing/writing
  - Lockfile generation
  - Cache operations (store, retrieve, clean)
  - Checksum verification
  - .gitignore management
  - API client
  - All CLI commands
- ✅ Windows-specific fixes (in-memory SQLite, path handling)
- ✅ Cross-platform compatibility verified

**Deliverables**:
- ✅ 61 passing tests
- ✅ Test coverage >80%
- ✅ CI-ready test suite

### 3.8 Audit & Provenance System ✅ COMPLETE

**Goal**: Local audit trail for regulatory compliance and research documentation.

**Status**: ✅ Fully implemented

**Completed**:
- ✅ **SQLite schema for audit trail**
  - `audit_events` table (editable, for reports)
  - `files` table (cache tracking)
  - `generated_files` table (post-pull outputs)
  - `audit_snapshots` table (export tracking)
- ✅ **Machine ID generation** (hostname-based, stable)
- ✅ **Event logging for all commands**
  - Download, verify, post-pull, etc.
  - Automatic middleware injection
- ✅ **Hash chain for tamper detection**

**Commands Implemented**:
- ✅ `bdp audit list` - View recent audit events
- ✅ `bdp audit verify` - Verify chain integrity
- ✅ `bdp audit export --format <fda|nih|ema|das|json>` - Export reports

**Export Formats**:
1. ✅ **FDA**: JSON report with all events, verification status (21 CFR Part 11)
2. ✅ **NIH**: Markdown Data Availability Statement for publications
3. ✅ **EMA**: YAML report demonstrating ALCOA++ compliance
4. ✅ **DAS**: Publication-ready data availability text
5. ✅ **JSON**: Raw export of all events

**Deliverables**:
- ✅ Audit database schema in `.bdp/bdp.db`
- ✅ All audit commands working
- ✅ Export templates for all formats
- ✅ Hash chain verification

**References**:
- [CLI Audit & Provenance Design](./docs/agents/design/cli-audit-provenance.md)

### 3.9 Post-Pull Hooks System (Post-MVP)

**Status**: Deferred to post-MVP

**Goal**: Automatic processing of downloaded files (indexing, database creation, etc.)

**Tasks**:
- [ ] Built-in tool registry (samtools, BLAST, BWA)
- [ ] Post-pull execution with audit logging
- [ ] Output file tracking in database
- [ ] Wildcard pattern matching
- [ ] Custom hooks via `.bdp/hooks/` directory

**Note**: Core audit system is complete. Post-pull hooks will be added based on user demand.

### 3.10 Backend Audit Integration (Post-MVP)

**Status**: Deferred to post-MVP

**Tasks**:
- [ ] BackendAuditLogger implementation
- [ ] API client for audit endpoints
- [ ] Offline fallback to local
- [ ] Sync local → backend (`bdp audit sync --backend`)

**Benefits**:
- Central audit trail for teams
- Immutable server-side logs
- Better for legal/compliance needs

## Phase 3.5: CI/CD & Release Infrastructure ✅ COMPLETE

**Goal**: Automated release process with multi-platform builds, testing, and distribution.

**Status**: All tasks complete. Release pipeline is production-ready.

### 3.5.1 Build & Distribution ✅ COMPLETE

**Completed**:
- ✅ **cargo-dist** integration (v0.30.3)
  - Multi-platform binary builds
  - Automated installer generation
  - GitHub Releases integration
- ✅ **Multi-platform support**:
  - Linux (x86_64-unknown-linux-gnu)
  - macOS Intel (x86_64-apple-darwin)
  - macOS ARM (aarch64-apple-darwin)
  - Windows (x86_64-pc-windows-msvc)
- ✅ **Install scripts**:
  - Shell installer (Linux/macOS): `bdp-installer.sh`
  - PowerShell installer (Windows): `bdp-installer.ps1`
  - Homebrew support (via tarball)

**Deliverables**:
- ✅ `.github/workflows/release.yml` - Main release workflow
- ✅ `dist-workspace.toml` - cargo-dist configuration
- ✅ Multi-platform binaries automatically built

**References**:
- [CI/CD Guide](./CI_CD.md)
- [Release Process](./RELEASE_PROCESS.md)

### 3.5.2 Release Testing ✅ COMPLETE

**Completed**:
- ✅ **Two-workflow system**:
  - `release.yml` - Builds artifacts and creates draft release
  - `test-release.yml` - Tests installers before publishing
- ✅ **Comprehensive installer testing**:
  - Fresh install on all platforms
  - Verify binary works (`bdp --version`)
  - Test upgrade (re-install)
  - Test uninstall command
  - Verify complete removal
- ✅ **Automated publish**:
  - Only publishes if all tests pass
  - Draft → Test → Public workflow
- ✅ **Platform matrix testing**:
  - Ubuntu 22.04
  - macOS 12 (Intel)
  - macOS 14 (ARM)
  - Windows Server 2022

**Deliverables**:
- ✅ `.github/workflows/test-release.yml` - Test workflow
- ✅ Automated installer verification
- ✅ Safe release process (no bad releases reach users)

**References**:
- [Release Testing](./RELEASE_TESTING.md)
- [CI/CD Summary](./CI_CD_SUMMARY.md)

### 3.5.3 Self-Uninstall ✅ COMPLETE

**Completed**:
- ✅ **`bdp uninstall` command**
  - Platform-specific implementations
  - Graceful self-removal (works while running)
  - Optional purge mode (removes cache and config)
  - Confirmation prompt (can skip with `-y`)
- ✅ **Cross-platform strategies**:
  - Unix: Background process with sleep + rm
  - Windows: Rename trick + batch script
  - Fallback: Manual instructions if automated removal fails
- ✅ **Integration with installers**:
  - All installers add uninstall capability
  - Documented in INSTALL.md

**Deliverables**:
- ✅ `crates/bdp-cli/src/commands/uninstall.rs` (~350 lines)
- ✅ Cross-platform self-removal
- ✅ Comprehensive tests

**References**:
- [Installation Guide](./INSTALL.md)

### 3.5.4 Version Management ✅ COMPLETE

**Completed**:
- ✅ **Unified version management**:
  - Single source of truth: `Cargo.toml` workspace version
  - All crates inherit version automatically
  - Auto-sync to `web/package.json` via pre-release hook
- ✅ **cargo-release integration**:
  - `just release-patch` - Bump patch version (0.1.0 → 0.1.1)
  - `just release-minor` - Bump minor version (0.1.0 → 0.2.0)
  - `just release-major` - Bump major version (0.1.0 → 1.0.0)
  - `just release-*-dry` - Preview changes
- ✅ **Automated workflow**:
  1. Bump version in Cargo.toml
  2. Sync to package.json (via Node.js script)
  3. Commit changes
  4. Create git tag (e.g., v0.1.1)
  5. Push tag to GitHub
  6. Trigger CI/CD pipeline
  7. Build artifacts
  8. Create draft release
  9. Test installers
  10. Publish release

**Deliverables**:
- ✅ `scripts/sync-version.js` - Version sync script
- ✅ Cargo.toml configuration with cargo-release metadata
- ✅ Justfile commands for version management
- ✅ Complete documentation (VERSIONING.md - 658 lines)

**References**:
- [Versioning Guide](./VERSIONING.md)
- [Release Process](./RELEASE_PROCESS.md)

### 3.5.5 Documentation ✅ COMPLETE

**Completed**:
- ✅ **User guides**:
  - [INSTALL.md](./INSTALL.md) - All installation methods
  - [QUICK_START.md](./QUICK_START.md) - Getting started
  - [VERSIONING.md](./VERSIONING.md) - Release management
- ✅ **Developer guides**:
  - [CI_CD.md](./CI_CD.md) - Complete CI/CD documentation (300+ lines)
  - [RELEASE_PROCESS.md](./RELEASE_PROCESS.md) - Quick reference
  - [RELEASE_TESTING.md](./RELEASE_TESTING.md) - Testing architecture
  - [CI_CD_SUMMARY.md](./CI_CD_SUMMARY.md) - High-level overview
- ✅ **Contributing guide**: [CONTRIBUTING.md](./CONTRIBUTING.md)
- ✅ **Changelog**: [CHANGELOG.md](./CHANGELOG.md)

**Deliverables**:
- ✅ 8 comprehensive documentation files
- ✅ ~2,000 lines of documentation
- ✅ User and developer guides complete

## Phase 4: Web Frontend ✅ 80% COMPLETE

**Status**: ✅ 80% Complete - All pages built including jobs dashboard, documentation content written, needs E2E testing

**Note**: All UI components and pages are built. Includes jobs dashboard for monitoring ingestion. Documentation content complete.

### 4.1 Next.js Setup ✅ COMPLETE

**Completed**:
- ✅ Next.js 16 project initialized with App Router
- ✅ Nextra documentation framework configured
- ✅ Tailwind CSS + shadcn/ui (new-york style)
- ✅ Radix UI components (15+ components)
- ✅ TypeScript configuration
- ✅ API client wrapper (fetch-based)
- ✅ Internationalization (next-intl) with en/de locales
- ✅ Theme system (dark/light mode with next-themes)
- ✅ Development environment running

**Deliverables**:
- ✅ `web/` Next.js project (~96 TypeScript files)
- ✅ Development server runs on http://localhost:3000
- ✅ Full component library (15+ shadcn/ui components)
- ✅ Locale switcher (dropdown) + theme toggle
- ✅ Grainy gradient effects and modern design

**References**:
- [Next.js Frontend](./docs/agents/implementation/nextjs-frontend.md)
- `web/IMPLEMENTATION_SUMMARY.md` - Complete feature list

### 4.2 Core Pages ✅ COMPLETE

**Completed**:
- ✅ Homepage with hero section, search bar, stats, getting started, features
- ✅ Browse pages fully implemented:
  - ✅ `/sources` - Grid list with filtering, sorting, pagination
  - ✅ `/sources/:org/:name` - Data source detail with version selector
  - ✅ `/sources/:org/:name/:version` - Version detail with files, citations, dependencies
  - ✅ `/organizations` - Organization listing
  - ✅ `/organizations/:slug` - Organization detail page
- ✅ Search page with filters and results (`/search`)
- ✅ **Jobs dashboard** (`/jobs`) - Ingestion job monitoring
  - Job cards with status badges
  - Organization-grouped job sections
  - Timeline view for job progress
  - Real-time status updates
- ✅ 404 and error pages (localized)
- ✅ Navigation header with logo, locale switcher, theme toggle
- ✅ Footer (standalone and integrated versions)

**Deliverables**:
- ✅ 30+ page components (.tsx files in app/)
- ✅ Fully responsive layout (mobile-first)
- ✅ Complete navigation system
- ✅ Locale-aware routing ([locale] directory structure)

**References**:
- [Next.js Frontend](./docs/agents/implementation/nextjs-frontend.md)
- [API Design](./docs/agents/design/api-design.md)

### 4.3 Data Source UI ✅ COMPLETE

**Completed**:
- ✅ Data source cards with grid layout
- ✅ Version selector component (dropdown)
- ✅ Download buttons for all file formats
- ✅ Dependencies section with pagination (~240 lines)
- ✅ CLI command snippets component with copy-to-clipboard (~80 lines)
- ✅ Citations section with BibTeX display (~153 lines)
- ✅ Download statistics display
- ✅ Tags and badges (type, organism, version)
- ✅ Filtering by type, organization, sort order
- ✅ Pagination controls (previous/next)
- ✅ Loading states with spinner
- ✅ Empty states with helpful messages
- ✅ Error handling and display

**Deliverables**:
- ✅ 51+ component files in components/
- ✅ Interactive features (filters, sorting, pagination)
- ✅ Copy-paste install commands ready
- ✅ Complete data source detail page (~861 lines total)

**References**:
- [Next.js Frontend](./docs/agents/implementation/nextjs-frontend.md)
- `web/app/[locale]/sources/` - Source pages implementation

### 4.4 Search & Discovery ✅ COMPLETE

**Completed**:
- ✅ Search bar component (used in hero and header)
- ✅ Search results page with grid layout
- ✅ Filter components (type, organism, format)
- ✅ Sort options (downloads, name, date - ascending/descending)
- ✅ Pagination with page controls
- ✅ Dedicated search page at `/search`
- ✅ Search filters component in `components/search/`
- ✅ Search pagination component
- ✅ Empty states for no results
- ✅ Loading states during search

**Pending** (Optional):
- [ ] Search suggestions/autocomplete (can be added later)
- [ ] Real-time search as you type (can be added later)

**Deliverables**:
- ✅ Search interface fully functional
- ✅ 3 search components (search-bar, search-filters, search-pagination)
- ✅ Pagination working
- ✅ Filter and sort working

**References**:
- [API Design](./docs/agents/design/api-design.md)

### 4.5 Nextra Documentation ✅ 80% COMPLETE

**Completed**:
- ✅ Documentation structure in `app/[locale]/docs/`
- ✅ Docs layout with sidebar navigation
- ✅ **MDX content files** (English and German):
  - `introduction.mdx` - Project overview
  - `quick-start.mdx` - Getting started guide
  - `installation.mdx` - Installation instructions
  - `best-practices.mdx` - Usage best practices
  - `audit.mdx` - Audit trail documentation
  - `cli-reference.mdx` - CLI command reference
- ✅ Documentation index page
- ✅ Docs search component (Pagefind integration)
- ✅ Sidebar component with navigation
- ✅ Code block component with syntax highlighting
- ✅ Workflow tabs for multi-step guides
- ✅ CTA cards for navigation

**Pending** (Optional):
- [ ] API documentation (OpenAPI integration)
- [ ] FAQ page
- [ ] Additional examples and tutorials

**Note**: Core documentation content is complete in both English and German.

**Deliverables**:
- ✅ Documentation framework with MDX support
- ✅ Searchable docs (via Pagefind)
- ✅ 6+ MDX content pages per locale
- ✅ Bilingual documentation (en/de)

**References**:
- [Next.js Frontend](./docs/agents/implementation/nextjs-frontend.md)
- `web/app/[locale]/docs/content/` - Documentation MDX files

### 4.6 Publishing Interface (Auth Required) ⬜ NOT STARTED

**Status**: Deferred to post-MVP

**Tasks**:
- [ ] User authentication (JWT)
- [ ] Login/register pages
- [ ] Publish form:
  - Upload file
  - Add metadata
  - Compute checksum
  - Submit to API
- [ ] User dashboard (published sources)
- [ ] API token management

**Note**: Will use CQRS commands for publishing operations. This is post-MVP functionality.

**Deliverables**:
- Authentication system
- Publish workflow
- User dashboard

**References**:
- [API Design](./docs/agents/design/api-design.md)

## Phase 5: Polish & Launch Preparation

**Goal**: Production readiness, testing, documentation.

### 5.1 Testing

**Tasks**:
- [ ] Backend integration tests (expand coverage)
- [ ] CLI end-to-end tests
- [ ] Frontend component tests
- [ ] API load testing
- [ ] User acceptance testing
- [ ] Cross-platform CLI testing (Linux, macOS, Windows)

**Deliverables**:
- Comprehensive test suite
- Test coverage reports
- Load test results

### 5.2 Performance Optimization

**Tasks**:
- [ ] Database query optimization
- [ ] Add additional database indexes (based on query patterns)
- [ ] Implement caching (Redis - optional)
- [ ] Frontend code splitting
- [ ] Image optimization
- [ ] CDN setup for downloads

**Deliverables**:
- Performance benchmarks
- Optimized queries
- Faster load times

### 5.3 Documentation

**Tasks**:
- [ ] User guide (getting started, common workflows)
- [ ] CLI reference (all commands)
- [ ] API documentation (OpenAPI)
- [ ] Architecture documentation (update with final patterns)
- [ ] Deployment guide
- [ ] Contributing guide
- [ ] Troubleshooting guide

**Deliverables**:
- Complete documentation website
- README files
- Code comments

### 5.4 Deployment ✅ INFRASTRUCTURE READY

**Infrastructure as Code (Terraform)** - ✅ COMPLETE:
- ✅ OVH Cloud Terraform configuration (`infrastructure/`)
- ✅ Single instance MVP setup (d2-2, 2 vCPU, 4GB RAM)
- ✅ Managed PostgreSQL (Essential plan)
- ✅ S3-compatible Object Storage
- ✅ Security groups (SSH, HTTP, HTTPS)
- ✅ Terraform Cloud backend for secure state storage
- ✅ CI/CD workflow with manual approval gates
- ✅ Fork PR protection for open source security
- ✅ Comprehensive setup documentation

**Estimated MVP Cost**: ~36 EUR/month

**Deployment Scripts** - ✅ COMPLETE:
- ✅ `infrastructure/deploy/setup.sh` - Server provisioning (Docker, Caddy, tools)
- ✅ `infrastructure/deploy/docker-compose.prod.yml` - Production compose
- ✅ `infrastructure/deploy/Caddyfile.example` - Reverse proxy config
- ✅ Justfile commands (`just infra-*`)

**CI/CD Pipeline** (`.github/workflows/infrastructure.yml`) - ✅ COMPLETE:
- ✅ `plan` - Runs automatically on PRs
- ✅ `apply` - Manual trigger, requires maintainer approval
- ✅ `destroy` - Manual trigger, requires approval + confirmation
- ✅ GitHub Environment secrets (not repo secrets)
- ✅ Fork PR protection

**Remaining Tasks**:
- [ ] Configure Terraform Cloud account and workspace
- [ ] Add GitHub Environment secrets (OVH credentials)
- [ ] Run `terraform apply` to provision infrastructure
- [ ] Configure DNS and SSL
- [ ] Set up monitoring (Prometheus + Grafana - optional)
- [ ] Configure backups

**Deliverables**:
- ✅ Infrastructure as Code (Terraform)
- ✅ Deployment scripts
- ✅ CI/CD pipeline for infrastructure
- ⬜ Production deployment (pending credentials)
- ⬜ Monitoring dashboard (optional)

**References**:
- [Infrastructure Setup Guide](./infrastructure/setup.md)
- [Infrastructure Security Guide](./infrastructure/SECURITY.md)
- [Deployment](./docs/agents/implementation/deployment.md)

### 5.5 Launch

**Tasks**:
- [ ] Beta testing with select users
- [ ] Bug fixes from beta
- [ ] Create demo video/screenshots
- [ ] Write announcement blog post
- [ ] Announce on relevant communities (r/bioinformatics, Twitter, etc.)
- [ ] Create example projects
- [ ] Monitor initial usage

**Deliverables**:
- Public launch
- Marketing materials
- Example projects

## Phase 6: Future Enhancements

**Post-MVP features** (prioritize based on user feedback):

### 6.1 Tool Management

**Goal**: Extend from data sources to bioinformatics tools (like npm/conda)

**Features**:
- Tool registry (BLAST, BWA, SAMtools, etc.)
- Build recipes
- Binary distribution
- Tool dependencies on data sources
- Version constraints

**Effort**: Large (3-4 months)

### 6.2 Advanced Search

**Features**:
- Elasticsearch/MeiliSearch integration
- Protein sequence search (BLAST API)
- Advanced filters (GO terms, pathways, domains)
- Semantic search

**Effort**: Medium (1-2 months)

### 6.3 Citation Generation

**Features**:
- `bdp cite` command
- Generate BibTeX from bdp.yml
- Generate LaTeX citations
- Support multiple citation styles
- Track provenance

**Effort**: Small (2-3 weeks)

### 6.4 Research Publishing

**Goal**: Share entire research environments

**Features**:
- `bdp research publish` - Publish bdp.yml as citable object
- Others can download: `bdp research install {doi}`
- DOI assignment (Zenodo integration)
- Environment snapshots

**Effort**: Medium (1 month)

### 6.5 Version Ranges

**Features**:
- Support `^1.0`, `~1.5`, `>=1.0` in bdp.yml
- Dependency resolver with version constraints
- Conflict resolution algorithm

**Effort**: Medium (3-4 weeks)

### 6.6 More Data Providers

**Providers to add**:
- NCBI (genomes, RefSeq)
- Ensembl (genomes, annotations)
- PDB (protein structures)
- KEGG (pathways)
- GO (ontologies)

**Effort**: Medium per provider (2-3 weeks each)

### 6.7 GUI Application

**Goal**: Desktop app for non-CLI users

**Features**:
- Electron or Tauri app
- Visual cache management
- Drag-and-drop file management
- Project templates

**Effort**: Large (2-3 months)

### 6.8 Bioconda Integration

**Goal**: Interoperability with existing ecosystem

**Features**:
- Import Bioconda recipes
- Resolve Bioconda dependencies
- Convert bdp.yml ↔ environment.yml
- Mixed dependencies (BDP + Bioconda)

**Effort**: Large (2-3 months)

## Development Approach

### Parallel Streams

Use multiple development streams with clear dependencies:

**Stream 1: Backend Core** ✅ COMPLETE
- Phase 1.1 (Database) ✅ COMPLETE (67 migrations)
- Phase 1.2 (API) ✅ COMPLETE (25+ endpoints implemented)
- Phase 1.3 (S3/MinIO) ✅ COMPLETE (Storage + Files feature)
- Phase 1.4 (Testing) ✅ COMPLETE (750+ tests)

**Stream 2: CLI Tools** ✅ COMPLETE
- Phase 3.1 (Core) ✅ COMPLETE (10 commands implemented)
- Phase 3.2 (API Client) ✅ COMPLETE
- Phase 3.3 (Cache) ✅ COMPLETE
- Phase 3.4 (Resolution) ✅ COMPLETE
- Phase 3.5 (CI/CD & Release) ✅ COMPLETE
- Phase 3.7 (Testing) ✅ COMPLETE (61 tests)
- Phase 3.8 (Audit & Provenance) ✅ COMPLETE (audit list/verify/export)
- **Status**: CLI fully complete with audit system and regulatory export formats

**Stream 3: Data Ingestion** ✅ 95% COMPLETE
- Phase 2.1 (Version Mapping) ✅ COMPLETE
- Phase 2.2-2.5 (Parsers) ✅ COMPLETE (UniProt, NCBI Taxonomy, GenBank, Gene Ontology)
- Phase 2.6 (ETL Framework) ✅ COMPLETE
- Phase 2.7 (Job Queue) ✅ COMPLETE
- Phase 2.8 (Data Population) 🔄 READY TO RUN
- **Status**: All pipelines coded, needs production data ingestion

**Stream 4: Frontend** ✅ 80% COMPLETE
- Phase 4.1 (Setup) ✅ COMPLETE → Phase 4.2-4.4 (Pages/Features) ✅ COMPLETE
- Phase 4.5 (Documentation) ✅ 80% COMPLETE → Phase 4.6 (Auth) ⬜ Deferred
- **Status**: All pages built including jobs dashboard, needs E2E testing

**Stream 5: Launch Preparation** 🔄 70% COMPLETE
- Phase 5.1-5.5 (Testing, Docs, Deploy)
- **Status**: CLI released, docs complete, frontend ready, need data + production deployment

### Current Status Summary

| Phase | Status | Progress | LOC |
|-------|--------|----------|-----|
| **1.1 Database** | ✅ Complete | 100% | 67 migrations |
| **1.2 API Server** | ✅ Complete | 100% | ~40,000 lines, 25+ endpoints |
| **1.3 S3/MinIO** | ✅ Complete | 100% | ~1,500 lines |
| **1.4 Testing** | ✅ Complete | 100% | 750+ tests |
| **2.x Ingestion** | ✅ Pipelines Complete | 95% | ~18,000 lines (4 pipelines) |
| **3.x CLI Core** | ✅ Complete | 100% | ~6,000 lines, 10 commands |
| **3.5 CI/CD** | ✅ Complete | 100% | ~2,000 lines docs |
| **3.8 Audit** | ✅ Complete | 100% | Full audit system with exports |
| **4.x Frontend** | ✅ All Pages Done | 80% | 31 pages, 51+ components |
| **5.4 Infrastructure** | ✅ IaC Complete | 100% | Terraform, CI/CD, ~36 EUR/mo |
| **5.x Launch** | 🔄 In Progress | 80% | Need data + credentials |

### Next Immediate Steps

**🎉 ALL CORE DEVELOPMENT COMPLETE! 🎉**

Backend, CLI (with audit), ingestion pipelines, and frontend are fully implemented!

**What's Working Now**:
- ✅ Backend API with **25+ endpoints** (search, jobs, data sources, organizations, resolve)
- ✅ Full database schema with PostgreSQL (**67 migrations**)
- ✅ S3/MinIO storage integration
- ✅ CLI tool with **10 commands** including full audit system
- ✅ **Audit & Provenance System** - `bdp audit list/verify/export`
  - FDA 21 CFR Part 11 export
  - NIH DMS export (Data Availability Statements)
  - EMA ALCOA++ export
  - Hash chain verification
- ✅ Multi-platform installers (Linux, macOS, Windows)
- ✅ Automated CI/CD with cargo-dist
- ✅ **Frontend web app** (Next.js 16)
  - All browse/detail pages
  - **Jobs dashboard** for ingestion monitoring
  - Search with suggestions
  - Full documentation (en/de)
  - Internationalization + dark/light theme
- ✅ **4 Complete Ingestion Pipelines**:
  - UniProt (proteins)
  - NCBI Taxonomy (~2.4M taxa)
  - GenBank/RefSeq (genomes)
  - Gene Ontology (annotations)
- ✅ Generic ETL framework (reusable for new sources)
- ✅ Job queue with apalis (background processing)
- ✅ **750+ backend tests + 61 CLI tests**

**What Remains (Operational Tasks)**:

1. **Run Production Data Ingestion** (Priority: HIGH):
   - [ ] Run UniProt pipeline (SwissProt ~570k proteins)
   - [ ] Run NCBI Taxonomy pipeline (~2.4M taxa)
   - [ ] Run GenBank/RefSeq pipeline (selected genomes)
   - [ ] Run Gene Ontology pipeline (~45k terms)
   - [ ] Verify data integrity
   - [ ] Build/refresh search indexes
   - **Note**: All code is written. This is about running the pipelines.

2. **Production Deployment** (Priority: HIGH):
   - ✅ Infrastructure as Code ready (Terraform + OVH Cloud)
   - ✅ CI/CD pipeline for infrastructure
   - ✅ Deployment scripts ready
   - [ ] Configure Terraform Cloud account
   - [ ] Add GitHub Environment secrets (OVH credentials)
   - [ ] Run `terraform apply` to provision
   - [ ] Configure DNS and SSL
   - [ ] Configure monitoring (optional)

3. **E2E Testing** (Priority: MEDIUM):
   - [ ] Test frontend with real backend data
   - [ ] Verify all API integrations
   - [ ] Load testing (optional)

4. **Optional Enhancements** (Post-MVP):
   - [ ] API documentation (OpenAPI/Swagger)
   - [ ] Post-pull hooks (samtools, BLAST, BWA)
   - [ ] User authentication for publishing
   - [ ] Team cache support

**Recommendation**:
Start with **data ingestion** - run the pipelines to populate production data. All code is complete; this is purely operational work.

### Milestones

**M1: Backend Alpha** (End of Phase 1) - ✅ COMPLETE (2026-01-16)
- ✅ Database operational (67 migrations)
- ✅ API endpoints fully functional (25+ endpoints)
- ✅ S3 storage working (MinIO + AWS S3 support)
- ✅ Comprehensive tests passing (750+ tests)
- **Duration**: ~2 weeks

**M2: CLI Release** (End of Phase 3) - ✅ COMPLETE (2026-01-26)
- ✅ All CLI commands working (10 commands)
- ✅ **Full audit system** (list, verify, export)
- ✅ **Regulatory exports** (FDA, NIH, EMA, DAS)
- ✅ Lockfile generation
- ✅ Dependency resolution
- ✅ Cache management (single-user)
- ✅ Multi-platform installers (Linux, macOS, Windows)
- ✅ Automated CI/CD pipeline
- ✅ 61 CLI tests passing
- **Duration**: ~2 weeks

**M3: Ingestion Pipelines** (End of Phase 2) - ✅ 95% COMPLETE (2026-01-26)
- ✅ UniProt pipeline fully coded
- ✅ NCBI Taxonomy pipeline fully coded
- ✅ GenBank/RefSeq pipeline fully coded
- ✅ Gene Ontology pipeline fully coded
- ✅ Generic ETL framework
- ✅ Job queue (apalis)
- 🔄 Production data ingestion (ready to run)
- **Duration**: ~2 weeks (code complete)

**M4: Web Beta** (End of Phase 4) - ✅ 80% COMPLETE (2026-01-26)
- ✅ Web interface built (all pages)
- ✅ **Jobs dashboard** for ingestion monitoring
- ✅ Search and browse functional
- ✅ Documentation published (en/de)
- 🔄 E2E testing with real data
- ⬜ Publishing workflow (deferred)
- **Duration**: ~2 weeks (UI complete)

**M5: Public Launch** (End of Phase 5) - 🔄 80% COMPLETE
- ✅ CLI tool released and installable
- ✅ Full audit & provenance system
- ✅ Documentation complete (user + developer guides)
- ✅ CI/CD operational
- ✅ All code written
- ✅ Infrastructure as Code (Terraform + OVH Cloud)
- ✅ Infrastructure CI/CD with manual approval
- 🔄 Production data ingestion (ready to run)
- ⬜ Configure credentials (Terraform Cloud, OVH, GitHub Environment)
- ⬜ Provision infrastructure (`terraform apply`)
- ⬜ Public announcement
- **Remaining**: Configure credentials + run pipelines + provision infrastructure

## Success Metrics

### Technical Metrics
- API response time: <200ms (p95)
- Database queries: <50ms (p95)
- CLI command execution: <2s (cold start)
- Download speed: Limited by network
- Search results: <500ms
- Uptime: >99.5%

### Usage Metrics (Post-Launch)
- Active users (monthly)
- Total downloads
- Popular data sources
- CLI installs
- Web visitors
- API requests

### Quality Metrics
- Test coverage: >70% (currently ~80% for implemented features)
- Bug reports: Tracked and resolved
- Documentation coverage: Complete
- User satisfaction: Survey feedback

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| UniProt API changes | High | Version scraping logic, test with multiple releases |
| Large file handling | Medium | Streaming uploads/downloads, chunking |
| Database performance | High | Proper indexing, connection pooling, query optimization ✅ |
| Storage costs | Medium | Compression, deduplication, usage monitoring |
| Team cache conflicts | Medium | File locking, WAL mode, conflict detection |
| Version mapping errors | High | Comprehensive tests, manual validation |
| CQRS complexity | Medium | Clear documentation ✅, code examples ✅, minimal boilerplate ✅ |

## Key Architecture Decisions Made

### ✅ CQRS with Mediator Pattern (2026-01-16)

**Decision**: Use mediator-based CQRS architecture instead of traditional layered approach.

**Rationale**:
- Better separation of concerns (commands vs queries)
- No shared database layer - vertical slicing by feature
- Easier to test - function handlers with inline SQL
- Automatic audit logging via middleware
- Type-safe command/query dispatch via mediator
- Simpler codebase - minimal boilerplate

**Impact**:
- All new features must follow CQRS pattern
- See [Mediator-CQRS Architecture](./docs/agents/implementation/mediator-cqrs-architecture.md) guide
- Existing code will be migrated incrementally

### ✅ Just Command Runner (2026-01-16)

**Decision**: Use Just instead of shell scripts.

**Rationale**:
- Cross-platform (Windows, Linux, macOS)
- Self-documenting with `just --list`
- Better error handling
- Community standard (used by tokio, ripgrep, axum)

**Impact**:
- All development tasks use `just` commands
- No `.sh` scripts in the repository
- See `justfile` for 60+ available commands

### ✅ SQLx with Offline Mode (2026-01-16)

**Decision**: Use SQLx with compile-time checked queries and offline mode.

**Rationale**:
- Type safety at compile time
- No runtime ORM overhead
- Offline mode for CI/CD
- Better IDE support

**Impact**:
- Use `just sqlx-prepare` to generate query metadata
- All SQL queries are compile-time verified
- See [SQLx Guide](./docs/agents/implementation/sqlx-guide.md)

### ✅ Inline Tests (2026-01-16)

**Decision**: Place tests inline with `#[cfg(test)]` modules instead of separate files.

**Rationale**:
- Standard Rust practice
- Tests close to implementation
- Better discoverability
- Easier to maintain

**Impact**:
- Unit tests in same file as implementation
- Integration tests in separate `tests/` directory
- Use `#[sqlx::test]` for database tests

## Resources

### Team Requirements
- Backend developer (Rust, PostgreSQL) - **Active**
- Frontend developer (Next.js, TypeScript)
- CLI developer (Rust)
- DevOps engineer (deployment, monitoring)
- Documentation writer

### Infrastructure
- Development server (16GB RAM, 4 cores)
- Production server (32GB RAM, 8 cores, 2TB SSD)
- PostgreSQL database (16GB RAM, SSD)
- S3/MinIO storage (5TB initial, expandable)
- Domain and SSL
- Monitoring tools

### External Dependencies
- UniProt FTP access
- S3/MinIO service
- Domain registrar
- Email service (notifications)
- GitHub (version control, CI/CD)

## Getting Started

For developers joining the project:

1. **Read Design Documents**: Review all docs in `docs/agents/design/`
2. **Read Implementation Guides**: Especially [Mediator-CQRS Architecture](./docs/agents/implementation/mediator-cqrs-architecture.md)
3. **Set Up Environment**: Follow [SETUP.md](./SETUP.md)
4. **Run Tests**: Follow [TESTING.md](./TESTING.md)
5. **Choose a Task**: Pick an unchecked task from the roadmap
6. **Follow CQRS Pattern**: All new backend features use mediator-based CQRS

## Questions & Discussion

For design discussions or clarifications:
- Open an issue on GitHub
- Refer to design documents
- Update roadmap as decisions are made

---

**Last Updated**: 2026-01-27
**Version**: 3.0.0
**Status**: **ALL CORE DEVELOPMENT COMPLETE** - Ready for Production Data Ingestion & Deployment

**Major Achievements**:
- ✅ Backend API with **25+ endpoints** (CQRS architecture)
- ✅ Full PostgreSQL database schema (**67 migrations**)
- ✅ S3/MinIO storage integration
- ✅ CLI tool with **10 commands** including full audit system
- ✅ **Audit & Provenance System** with regulatory exports (FDA, NIH, EMA)
- ✅ Multi-platform installers (4 platforms)
- ✅ Automated CI/CD with cargo-dist
- ✅ **4 Complete Data Ingestion Pipelines**:
  - UniProt (proteins, DAT parser)
  - NCBI Taxonomy (taxdump parser)
  - GenBank/RefSeq (flat file parser)
  - Gene Ontology (OBO + GAF parsers)
- ✅ Generic ETL framework (reusable for new sources)
- ✅ Job queue with apalis (background processing)
- ✅ **Frontend web app** (Next.js 16)
  - All pages including jobs dashboard
  - Full documentation (en/de)
  - Search with suggestions
- ✅ **Infrastructure as Code** (Terraform + OVH Cloud)
  - Single instance MVP (~36 EUR/month)
  - Managed PostgreSQL + S3 storage
  - CI/CD with manual approval gates
  - Fork PR protection for open source
- ✅ **810+ tests passing** (750+ backend + 61 CLI)
- ✅ Comprehensive documentation (~44,000+ lines)

**Next Focus**: Configure credentials (Terraform Cloud, OVH, GitHub) → Run data ingestion → Provision infrastructure
