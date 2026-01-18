# BDP Development Roadmap

Comprehensive roadmap for building the Bioinformatics Dependencies Platform.

## Vision

BDP aims to be the **npm for bioinformatics**, starting with versioned data source management (proteins, genomes, annotations) before expanding to software tools. The platform enables reproducible research through lockfiles, semantic versioning, and dependency management.

## Quick Progress Overview

| Phase | Status | Key Deliverables |
|-------|--------|------------------|
| **Phase 1: Backend** | ✅ Complete | Database, 17 API endpoints, S3 storage, 110+ tests |
| **Phase 3: CLI** | ✅ Complete | 8 commands, multi-platform installers, CI/CD, 61 tests |
| **Phase 3.5: Release** | ✅ Complete | Automated releases, version management, documentation |
| **Phase 2: Ingestion** | 🔄 70% Complete | Infrastructure ready, parsers built, pipeline needs implementation |
| **Phase 4: Frontend** | 🔄 70% Complete | Next.js app with 75 files, all core pages, search UI, needs API testing |
| **Phase 5: Launch** | 🔄 60% Complete | CLI released, docs complete, frontend built, need data + deployment |

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
- ✅ SQLx migrations (15+ migration files in `migrations/`)
- ✅ Triggers for denormalization (dependency_count, size_bytes calculation)
- ✅ Full-text search indexes (GIN indexes on tsvector columns)
- ✅ Seed data for system organizations

**Deliverables**:
- ✅ `migrations/` directory with 15+ SQL migration files
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
- ✅ **Search feature implemented** (1 handler):
  - Unified search across organizations, data_sources, tools
  - PostgreSQL full-text search with relevance ranking
  - Filtering by type, organism, format
  - Pagination support
- ✅ **Resolve feature implemented** (1 handler):
  - Manifest resolution (bdp.yml → lockfile)
  - Recursive dependency resolution
  - Conflict detection
  - Spec parsing (org:name@version-format)
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
- ✅ **15 handlers registered in mediator** (5 orgs + 8 data_sources + 1 search + 1 resolve)

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
- ✅ **15 RESTful API endpoints** (5 organizations + 8 data sources + 1 search + 1 resolve)
- ✅ All endpoints following CQRS pattern
- ✅ Health check endpoint at `/health`
- ✅ Audit log endpoint at `/api/v1/audit`
- ✅ Comprehensive test coverage with `#[sqlx::test]`
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
- ✅ Test coverage >70% for all features (**>110 tests total**)
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
- ✅ Test coverage >70% for all features (>110 tests total)
- ✅ CI/CD pipeline operational (Phase 3.5)
- ✅ Developer documentation (SETUP.md, TESTING.md, multiple guides)

## Phase 2: UniProt Scraping & Ingestion

**Status**: 🔄 Infrastructure Complete (70% done) - Implementation needed for production use

**Note**: Core infrastructure built by 4 specialized agents (33 files, ~5,100 lines, 205+ tests). Production pipeline implementation still needed.

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

### 2.2 UniProt Parser 🔄 PARTIAL (Framework Complete)

**Completed** (Agent 3):
- ✅ Parser module structure in `crates/bdp-server/src/ingest/uniprot/parser.rs` (~450 lines)
- ✅ DAT file parser framework
  - Entry-level parsing (ID, AC, DE, GN, OS, OX, SQ sections)
  - Protein metadata extraction structure
  - Citations parsing structure
  - Sequence extraction
- ✅ FTP client for UniProt downloads (`ftp.rs` ~200 lines)
  - Release listing and file download
  - Error handling and retries
- ✅ Unit tests for parser framework (35+ tests)
- ✅ Compression handling (gzip detection)

**Pending** (Needs Production Implementation):
- [ ] Complete DAT parser implementation (currently placeholder)
- [ ] Implement FASTA parser
- [ ] Implement XML parser
- [ ] Test with real UniProt files (currently uses mock data)
- [ ] Handle large files with streaming (framework exists, needs testing)
- [ ] Generate output JSON format

**Note**: Parser framework and structure are complete. The actual parsing logic needs to be implemented for production use.

**Deliverables**:
- ✅ `crates/bdp-server/src/ingest/uniprot/` module (~1,200 lines)
- ✅ Parser framework with tests (35+ tests)
- ✅ FTP client implementation
- ⬜ Production-ready parsers (needs implementation)

**References**:
- [UniProt Ingestion](./docs/agents/design/uniprot-ingestion.md)

### 2.3 Job Queue & Pipeline ✅ INFRASTRUCTURE COMPLETE

**Completed** (Agents 1, 2, 4):
- ✅ **Job Queue Infrastructure** (Agent 1 - ~500 lines):
  - apalis job queue with PostgreSQL storage
  - JobScheduler with worker management
  - Monitor system for job processing
  - Database migration for apalis schema (jobs and workers tables)
  - Configuration management (worker threads, auto-ingest settings)
  - Unit tests (12+ tests)

- ✅ **CQRS Commands for Ingestion** (Agent 2 - ~1,800 lines):
  - `organisms` commands (create, batch_create) with 45+ tests
  - `version_files` commands (add_batch) with 40+ tests
  - `protein_metadata` commands (upsert_batch) with 50+ tests
  - All using mediator pattern with inline SQL
  - Comprehensive error handling and validation

- ✅ **Pipeline Orchestration** (Agent 4 - ~800 lines):
  - `UniProtPipeline` structure with configuration
  - `UniProtIngestJob` definition with stats tracking
  - `IngestStats` with progress monitoring
  - Integration with job scheduler
  - Configuration via environment variables
  - Unit tests (20+ tests)

**Pending** (Needs Implementation):
- [ ] Complete `process_uniprot_job()` implementation (currently placeholder)
- [ ] Implement full pipeline steps:
  1. Check for new releases
  2. Download release files (FTP client exists)
  3. Parse proteins (parser framework exists)
  4. Extract per-protein files
  5. Upload to S3 (storage exists)
  6. Insert into database (CQRS commands exist)
  7. Create aggregate source
  8. Update search indexes
- [ ] Add error recovery (resume partial ingestion)
- [ ] Set up cron schedule (daily at 2 AM)
- [ ] Production testing with real data

**Note**: All infrastructure is in place. The actual pipeline logic connecting the components needs to be implemented.

**Deliverables**:
- ✅ Job queue infrastructure (apalis integration)
- ✅ CQRS commands for all ingestion operations
- ✅ Pipeline orchestration framework
- ✅ Configuration system
- ✅ 167+ tests for infrastructure
- ⬜ Complete working pipeline (needs implementation)

**References**:
- [UniProt Ingestion](./docs/agents/design/uniprot-ingestion.md)
- [Version Mapping](./docs/agents/design/version-mapping.md)
- [Mediator-CQRS Architecture](./docs/agents/implementation/mediator-cqrs-architecture.md)

### 2.4 Initial Data Population ⬜ NOT STARTED

**Prerequisites**: Complete section 2.3 pipeline implementation first.

**Tasks**:
- [ ] Configure oldest_version (e.g., 2020_01 → 1.0)
- [ ] Run initial ingestion for historical releases
- [ ] Ingest SwissProt (~570k proteins)
- [ ] Create aggregate source `uniprot:all@1.0`
- [ ] Verify data integrity
- [ ] Build search indexes

**Deliverables**:
- Database populated with proteins
- Aggregate sources created
- Search indexes built

**References**:
- [UniProt Ingestion](./docs/agents/design/uniprot-ingestion.md)
- [Dependency Resolution](./docs/agents/design/dependency-resolution.md)

### Phase 2 Summary - Infrastructure Complete (2026-01-16)

**What's Actually Done** (70% of Phase 2):
- ✅ **33 files created** (~5,100 lines of code)
- ✅ **205+ tests written** (all passing)
- ✅ **4 specialized agents** completed their work
- ✅ **All compilation errors fixed** (125+ errors resolved)
- ✅ Job queue infrastructure (apalis + PostgreSQL)
- ✅ CQRS commands for organisms, version_files, protein_metadata
- ✅ Version mapping logic with auto-increment
- ✅ UniProt FTP client
- ✅ Parser framework (DAT file structure)
- ✅ Pipeline orchestration structure
- ✅ Configuration system

**What Still Needs Work** (30% remaining):
- ⬜ Complete DAT parser implementation (parse real UniProt files)
- ⬜ Implement FASTA and XML parsers
- ⬜ Connect all components in `process_uniprot_job()` function
- ⬜ Test with real UniProt data
- ⬜ Error recovery and resume logic
- ⬜ Cron scheduling setup
- ⬜ Initial data population

**Assessment**:
The Phase 2 work is a **solid foundation** - all the infrastructure, database operations, job queue, and testing framework are in place. What's missing is the "glue code" that connects these components into a working end-to-end pipeline. The hard architectural work is done; what remains is implementation of the core parsing and pipeline logic.

**Estimated Time to Complete**: 1-2 weeks for a skilled developer familiar with the codebase.

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
- ✅ `crates/bdp-cli/` Rust project (4,500+ lines)
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

### 3.8 Audit & Provenance System 🔄 PLANNED

**Goal**: Local audit trail for regulatory compliance and research documentation.

**Status**: Design complete, ready for implementation

**Design**: [CLI Audit & Provenance](./docs/agents/design/cli-audit-provenance.md)

#### 3.8.1 Core Audit System (MVP)

**Tasks**:
- [ ] SQLite schema for audit trail
  - `audit_events` table (editable, for reports)
  - `files` table (cache tracking)
  - `generated_files` table (post-pull outputs)
  - `audit_snapshots` table (export tracking)
- [ ] Machine ID generation (hostname-based, stable)
- [ ] CQRS audit middleware pattern
  - Trait-based dependency injection
  - LocalAuditLogger implementation
  - Future: BackendAuditLogger (post-MVP)
- [ ] Event logging for all commands
  - Download, verify, post-pull, etc.
  - Automatic middleware injection
- [ ] Hash chain for tamper detection

**Commands**:
- [ ] `bdp audit list` - View recent audit events
- [ ] `bdp audit verify` - Verify chain integrity

**IMPORTANT**:
- Audit trail is **editable** and intended for **research documentation**, not legal evidence
- All commands clearly document this in help text
- Primary purpose: Generate reports for FDA, NIH, EMA compliance

**Deliverables**:
- Audit database schema
- CQRS middleware implementation
- Basic audit commands
- ~800 lines of code
- 20+ tests

**Estimated**: 3-5 days

**References**:
- [CLI Audit & Provenance Design](./docs/agents/design/cli-audit-provenance.md)
- [Backend Architecture](./docs/agents/backend-architecture.md) - CQRS pattern reference

#### 3.8.2 Export Formats (MVP)

**Tasks**:
- [ ] FDA 21 CFR Part 11 export (JSON)
- [ ] NIH DMS export (Markdown)
- [ ] EMA ALCOA++ export (YAML)
- [ ] Data Availability Statement generator
- [ ] Export snapshot tracking

**Commands**:
- [ ] `bdp audit export --format <fda|nih|ema|das|json>`

**Export Formats**:
1. **FDA**: JSON report with all events, verification status
2. **NIH**: Markdown Data Availability Statement for publications
3. **EMA**: YAML report demonstrating ALCOA++ compliance
4. **DAS**: Publication-ready data availability text
5. **JSON**: Raw export of all events

**Deliverables**:
- Export templates for each format
- Snapshot tracking
- ~600 lines of code
- 15+ tests

**Estimated**: 2-3 days

**References**:
- [CLI Audit & Provenance Design](./docs/agents/design/cli-audit-provenance.md#export-formats)

### 3.9 Post-Pull Hooks System 🔄 PLANNED

**Goal**: Automatic processing of downloaded files (indexing, database creation, etc.)

**Status**: Design complete, ready for implementation

**Design**: [CLI Audit & Provenance](./docs/agents/design/cli-audit-provenance.md#post-pull-hooks-system)

#### 3.9.1 Pre-Defined Tools (MVP)

**Tasks**:
- [ ] Built-in tool registry
  - samtools (FASTA indexing)
  - BLAST (makeblastdb)
  - BWA (genome indexing)
- [ ] Post-pull execution with audit logging
- [ ] Output file tracking in database
- [ ] Wildcard pattern matching

**Wildcard Support**:
```yaml
post_pull:
  uniprot:*-fasta@1.0:  # Matches any UniProt protein
    - "samtools"

  ncbi:*@2.0:  # Matches any NCBI source at v2.0
    - "bwa"
```

**Deliverables**:
- Tool registry implementation
- Pattern matching engine
- Post-pull executor
- ~500 lines of code
- 25+ tests

**Estimated**: 2-3 days

**References**:
- [CLI Audit & Provenance Design](./docs/agents/design/cli-audit-provenance.md#post-pull-hooks-system)

#### 3.9.2 Enhanced Verification (MVP)

**Tasks**:
- [ ] Verify post-pull generated files exist
- [ ] Checksum verification for deterministic outputs
- [ ] Report missing generated files
- [ ] Integration with `bdp verify` command

**Deliverables**:
- Generated file verification
- Updated `bdp verify` command
- ~200 lines of code
- 10+ tests

**Estimated**: 1-2 days

#### 3.9.3 Custom Hooks (Post-MVP)

**Status**: Deferred to post-MVP

**Tasks**:
- [ ] `.bdp/hooks/` directory support
- [ ] Custom script discovery (.sh, .py, .R)
- [ ] Hook execution with audit logging
- [ ] Hook validation and security

**Example**:
```bash
# .bdp/hooks/custom-analysis.sh
#!/bin/bash
INPUT=$1
python /path/to/analysis.py "$INPUT"
```

```yaml
# bdp.yml
post_pull:
  uniprot:*-fasta@1.0:
    - "samtools"
    - "custom-analysis"  # Looks for .bdp/hooks/custom-analysis.sh
```

**Deliverables**:
- Hook discovery system
- Security validation
- ~300 lines of code
- 15+ tests

**Estimated**: 2-3 days

**Note**: Custom hooks are committed to git (in `.bdp/hooks/`), unlike machine-specific config.

### 3.10 Audit Archive System (Post-MVP)

**Status**: Deferred to post-MVP

**Tasks**:
- [ ] Archive old events to JSON
- [ ] `audit_events_archived` table
- [ ] Auto-archive configuration
- [ ] Verification with archived events

**Commands**:
- [ ] `bdp audit archive --before <date>`
- [ ] `bdp audit restore --from <archive-file>`

**Use Case**: Archive events older than 6 months to keep database size manageable

**Deliverables**:
- Archive system
- Restoration logic
- ~400 lines of code
- 10+ tests

**Estimated**: 1-2 days

### 3.11 Backend Audit Integration (Post-MVP)

**Status**: Deferred to post-MVP

**Tasks**:
- [ ] BackendAuditLogger implementation
- [ ] API client for audit endpoints
- [ ] Offline fallback to local
- [ ] Sync local → backend

**Commands**:
- [ ] `bdp audit sync --backend`

**Benefits**:
- Central audit trail for teams
- Immutable server-side logs
- Better for legal/compliance needs

**Deliverables**:
- Backend integration
- Sync logic
- ~600 lines of code
- 20+ tests

**Estimated**: 3-5 days

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

## Phase 4: Web Frontend

**Status**: 🔄 70% Complete - Core pages and UI implemented, needs API integration testing and documentation polish

**Note**: Most UI components and pages are built. Ready for backend integration.

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
- ✅ `web/` Next.js project (~75 TypeScript files)
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
- ✅ 404 and error pages (localized)
- ✅ Navigation header with logo, locale switcher, theme toggle
- ✅ Footer (standalone and integrated versions)

**Deliverables**:
- ✅ 25 page components (.tsx files in app/)
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
- ✅ 27+ component files in components/
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

### 4.5 Nextra Documentation 🔄 PARTIAL (30% Complete)

**Completed**:
- ✅ Documentation structure in `app/[locale]/docs/`
- ✅ Docs layout with sidebar navigation
- ✅ Getting started page (`quick-start/page.tsx`)
- ✅ Installation page (`installation/page.tsx`)
- ✅ Documentation index page
- ✅ Docs search component
- ✅ MDX support configured
- ✅ Pagefind integration for search
- ✅ Sidebar component with navigation

**Pending**:
- [ ] CLI reference documentation (content needed)
- [ ] API documentation (OpenAPI integration)
- [ ] FAQ page
- [ ] Examples and tutorials (content needed)
- [ ] Expand existing doc pages with complete content

**Note**: Structure is complete, mostly needs content writing.

**Deliverables**:
- ✅ Documentation framework ready (7 doc-related files)
- ✅ Searchable docs (via Pagefind)
- ⬜ Complete content for all sections

**References**:
- [Next.js Frontend](./docs/agents/implementation/nextjs-frontend.md)
- `web/app/[locale]/docs/` - Documentation pages

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

### 5.4 Deployment

**Tasks**:
- [ ] Set up production server
- [ ] Configure Caddy reverse proxy (automatic HTTPS)
- [ ] Set up systemd services
- [ ] Configure PostgreSQL
- [ ] Set up S3/MinIO
- [ ] Configure backups
- [ ] Set up monitoring (Prometheus + Grafana - optional)
- [ ] Configure log aggregation
- [ ] Set up DNS
- [ ] SSL certificates (via Caddy)

**Deliverables**:
- Production deployment
- Monitoring dashboard
- Backup strategy
- Deployment scripts

**References**:
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
- Phase 1.1 (Database) ✅ COMPLETE
- Phase 1.2 (API) ✅ COMPLETE (17 endpoints implemented)
- Phase 1.3 (S3/MinIO) ✅ COMPLETE (Storage + Files feature)
- Phase 1.4 (Testing) ✅ COMPLETE (>110 tests)

**Stream 2: CLI Tools** 🔄 MOSTLY COMPLETE
- Phase 3.1 (Core) ✅ COMPLETE (All commands implemented)
- Phase 3.2 (API Client) ✅ COMPLETE
- Phase 3.3 (Cache) ✅ COMPLETE
- Phase 3.4 (Resolution) ✅ COMPLETE
- Phase 3.5 (CI/CD & Release) ✅ COMPLETE
- Phase 3.7 (Testing) ✅ COMPLETE (61 tests)
- Phase 3.8 (Audit & Provenance) 🔄 PLANNED - Design complete
- Phase 3.9 (Post-Pull Hooks) 🔄 PLANNED - Design complete
- **Status**: Core CLI complete, audit & hooks system designed and ready for implementation

**Stream 3: Data Ingestion** ⬜ READY TO START
- Phase 2.1 (Version Mapping) → Phase 2.2 (Parsers) → Phase 2.3 (Cron) → Phase 2.4 (Population)
- Depends on: Phase 1.2 ✅ COMPLETE
- **Status**: All dependencies met, can begin immediately

**Stream 4: Frontend** 🔄 MOSTLY COMPLETE
- Phase 4.1 (Setup) ✅ COMPLETE → Phase 4.2-4.4 (Pages/Features) ✅ COMPLETE → Phase 4.5-4.6 (Docs/Auth) 🔄 PARTIAL
- Depends on: Phase 1.2 ✅ COMPLETE
- **Status**: 70% complete - Core UI done, needs API integration testing and doc content

**Stream 5: Launch Preparation**
- Phase 5.1-5.5 (Testing, Docs, Deploy)
- Depends on: Streams 1-4
- **Status**: Backend and CLI complete, awaiting data ingestion and frontend

### Current Status Summary

| Phase | Status | Progress | LOC |
|-------|--------|----------|-----|
| **1.1 Database** | ✅ Complete | 100% | 15+ migrations |
| **1.2 API Server** | ✅ Complete | 100% | ~8,000 lines |
| **1.3 S3/MinIO** | ✅ Complete | 100% | ~1,500 lines |
| **1.4 Testing** | ✅ Complete | 100% | >110 tests |
| **2.x Ingestion** | 🔄 Infrastructure | 70% | ~5,100 lines, 205+ tests |
| **3.x CLI Core** | ✅ Complete | 100% | ~4,500 lines, 61 tests |
| **3.5 CI/CD** | ✅ Complete | 100% | ~2,000 lines docs |
| **3.8 Audit** | 🔄 Designed | 0% | Design complete, ready to implement |
| **3.9 Post-Pull Hooks** | 🔄 Designed | 0% | Design complete, ready to implement |
| **4.x Frontend** | 🔄 Core Pages Done | 70% | ~75 files, 52 components |
| **5.x Launch** | 🔄 In Progress | 60% | Partial |

### Next Immediate Steps

**🎉 Phases 1, 3, 4 (Core Pages), & 2 (Infrastructure) COMPLETE! 🎉**

Backend infrastructure, CLI tool, frontend UI, and data ingestion framework are ready!

**What's Working Now**:
- ✅ Backend API with 17 endpoints
- ✅ Full database schema with PostgreSQL (19 migrations)
- ✅ S3/MinIO storage integration
- ✅ CLI tool with 8 commands
- ✅ Multi-platform installers (Linux, macOS, Windows)
- ✅ Automated CI/CD with cargo-dist
- ✅ Unified version management
- ✅ **Frontend web app** (Next.js 16 with 75 TypeScript files, 52 components)
  - Homepage with search, stats, getting started
  - Data sources browse/detail pages
  - Organizations pages
  - Search functionality
  - Full API client integration
  - Internationalization (en/de)
  - Dark/light theme
- ✅ **Data ingestion infrastructure** (apalis job queue, CQRS commands, parsers framework)
- ✅ 376+ tests passing (>110 backend + 61 CLI + 205+ ingestion)

**Choose Next Development Path**:

1. **Phase 3.8-3.9 - Audit & Post-Pull Hooks** (Quick Win - CLI Enhancement):
   - ✅ Design complete (100% done)
   - [ ] Implement audit trail system (3-5 days)
   - [ ] Implement post-pull hooks (2-3 days)
   - [ ] Export formats (FDA, NIH, EMA) (2-3 days)
   - [ ] Enhanced verification (1-2 days)
   - **Priority**: HIGH - Regulatory compliance, needed for scientific publications
   - **Estimated**: 1-2 weeks total
   - **Benefits**:
     - Research documentation ready
     - Data Availability Statements
     - Regulatory compliance (FDA, NIH, EMA)
     - Automatic file processing (samtools, BLAST, BWA)

2. **Phase 2 - Complete Ingestion Pipeline** (Recommended - Get Real Data):
   - ✅ Infrastructure complete (70% done)
   - [ ] Implement DAT parser (parse real UniProt files)
   - [ ] Connect components in `process_uniprot_job()` function
   - [ ] Test with real UniProt data
   - [ ] Set up cron scheduling
   - [ ] Initial data population
   - **Priority**: HIGH - needed for real-world usage
   - **Estimated**: 1-2 weeks (infrastructure saves significant time!)

3. **Phase 4 - Frontend Polish** (Recommended - User Discovery):
   - ✅ Core pages complete (70% done)
   - [ ] Test API integration with real backend data
   - [ ] Write documentation content (structure exists)
   - [ ] Add remaining doc pages (CLI reference, API docs, examples)
   - [ ] Implement authentication (optional for MVP)
   - **Priority**: MEDIUM - core UI is done, needs polish
   - **Estimated**: 3-5 days (testing + docs content)

4. **Phase 5 - Launch Preparation** (Optional - Polish):
   - API documentation (OpenAPI/Swagger)
   - Performance optimization
   - Production deployment setup
   - Beta testing
   - **Priority**: MEDIUM - can be done incrementally
   - **Estimated**: 1-2 weeks

**Recommendation**:
- **Option 1 (Quick Win)**: Start with Phase 3.8-3.9 (Audit & Hooks) - 1-2 weeks
  - Delivers immediate value for researchers
  - CLI becomes publication-ready
  - Regulatory compliance documentation
  - Then move to Phase 2 (Ingestion)

- **Option 2 (Data First)**: Start Phase 2 (Data Ingestion) - 1-2 weeks
  - Get real UniProt data flowing
  - Critical blocker for testing frontend with real data
  - Then add audit features

- **Option 3 (Parallel - if resources available)**:
  - Developer 1: Phase 3.8-3.9 (Audit & Hooks)
  - Developer 2: Phase 2 (Data Ingestion)
  - **Result**: Both done in 1-2 weeks

**Our Suggestion**: Start with **Option 1** (Audit & Hooks) for quick wins, then Phase 2

### Milestones

**M1: Backend Alpha** (End of Phase 1) - ✅ COMPLETE (2026-01-16)
- ✅ Database operational (100%)
- ✅ API endpoints fully functional (17 endpoints, 100%)
- ✅ S3 storage working (MinIO + AWS S3 support)
- ✅ Comprehensive tests passing (>110 tests, all features)
- **Duration**: ~2 weeks

**M2: CLI Release** (End of Phase 3) - ✅ COMPLETE (2026-01-16)
- ✅ All CLI commands working (8 commands)
- ✅ Lockfile generation
- ✅ Dependency resolution
- ✅ Cache management (single-user)
- ✅ Multi-platform installers (Linux, macOS, Windows)
- ✅ Automated CI/CD pipeline
- ✅ Unified version management
- ✅ Self-uninstall support
- ✅ 61 CLI tests passing
- **Duration**: ~1 week

**M3: Data Available** (End of Phase 2) - ⬜ NOT STARTED
- UniProt proteins ingested
- Search indexes built
- Aggregate sources created
- Cron job running
- **Estimated Duration**: 1-2 weeks

**M4: Web Beta** (End of Phase 4) - ⬜ NOT STARTED
- Web interface live
- Search and browse functional
- Documentation published
- Publishing workflow operational
- **Estimated Duration**: 2-3 weeks

**M5: Public Launch** (End of Phase 5) - 🔄 PARTIAL
- ✅ CLI tool released and installable
- ✅ Documentation complete (user + developer guides)
- ✅ CI/CD operational
- ⬜ Production deployment (backend + frontend)
- ⬜ Real data ingested
- ⬜ Monitoring active
- ⬜ Public announcement
- **Estimated Duration**: 1-2 weeks (remaining work)

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

**Last Updated**: 2026-01-16
**Version**: 2.1.0
**Status**: **Phases 1 & 3 ✅ COMPLETE | Phase 2 🔄 70% COMPLETE** - Backend + CLI + Ingestion Infrastructure Ready

**Major Achievements**:
- ✅ Backend API with 17 endpoints (CQRS architecture)
- ✅ Full PostgreSQL database schema (19 migrations including apalis)
- ✅ S3/MinIO storage integration
- ✅ CLI tool with 8 commands
- ✅ Multi-platform installers (4 platforms)
- ✅ Automated CI/CD with cargo-dist
- ✅ Unified version management with cargo-release
- ✅ Self-uninstall support
- ✅ **Data ingestion infrastructure** (33 files, ~5,100 lines, 205+ tests)
  - apalis job queue with PostgreSQL backend
  - CQRS commands for organisms, version_files, protein_metadata
  - Version mapping with auto-increment logic
  - UniProt FTP client and parser framework
  - Pipeline orchestration structure
- ✅ 376+ tests passing (>110 backend + 61 CLI + 205+ ingestion)
- ✅ Comprehensive documentation (~10,000+ lines)
- ✅ All 125+ compilation errors fixed

**Next Focus**: Complete Ingestion Pipeline (30% remaining) + Frontend (Phase 4)
