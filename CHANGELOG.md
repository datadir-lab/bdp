# Changelog

All notable changes to the Bioinformatics Dependencies Platform (BDP) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Audit & Provenance System** (Phase 3.8.1 - MVP Complete)
  - Local SQLite audit trail (`.bdp/bdp.db`) for regulatory compliance
  - Hash-chain integrity verification for tamper detection
  - CQRS middleware pattern with dependency injection
  - Machine ID generation (hostname-based, privacy-conscious)
  - Automatic audit logging for all CLI commands
  - `bdp audit verify` command for chain integrity checking
  - Support for FDA, NIH, EMA compliance (export formats coming soon)
  - 20 comprehensive tests covering audit functionality
  - Multi-workspace independence validation
  - Editable-by-design for research documentation
- Initial project setup with Rust workspace structure
- CLI tool (`bdp-cli`) for local dependency management (78 tests passing)
- API server (`bdp-server`) with Axum framework
- Data ingestion pipeline (`bdp-ingest`) for bioinformatics databases
- Common utilities crate (`bdp-common`) shared across workspace
- Next.js frontend with documentation site using Nextra
- PostgreSQL database schema with SQLx migrations
- Docker Compose setup for local development
- MinIO S3-compatible object storage integration
- Just command runner replacing shell scripts
- Comprehensive GitHub Actions CI/CD workflows
  - Multi-version Rust testing (stable, beta, nightly)
  - Multi-platform builds (Linux, macOS, Windows)
  - Frontend linting, type checking, and builds
  - SQLx offline query verification
  - Security auditing with cargo-audit
- GitHub issue templates for bug reports and feature requests
- Contributing guidelines with Just command documentation
- Development environment verification tools

### Changed
- Migrated all shell scripts to Just recipes for cross-platform compatibility
- Updated CI workflows to use Just commands throughout
- Enhanced caching strategy for faster CI builds

### Infrastructure
- PostgreSQL 16 for primary data storage
- MinIO for object storage (S3-compatible)
- Docker and Docker Compose for development environment
- GitHub Actions for CI/CD automation
- Just as unified command runner

### Documentation
- Comprehensive README with quick start guide and audit system overview
- **Audit & Provenance Design Document** - Complete CQRS architecture specification
- Architecture and design documents
- API documentation structure
- Contributing guidelines
- Code of conduct
- Development roadmap (updated with audit system phases)
- Internationalized documentation (English and German) with audit features

## [0.1.0] - 2024-01-16

### Added
- Initial release (in development)
- Project scaffolding and infrastructure setup

---

## Version History

### Planned Releases

#### v0.2.0 - Core Functionality (Planned)
- Basic dependency resolution
- UniProt data ingestion
- CLI commands for project initialization
- Web UI for browsing dependencies

#### v0.3.0 - Enhanced Features (Planned)
- NCBI database integration
- Team collaboration features
- Caching and optimization
- Enhanced search capabilities

#### v1.0.0 - Production Release (Planned)
- Complete feature set
- Production-ready stability
- Comprehensive documentation
- Deployment guides

---

## Change Categories

This changelog uses the following categories:

- **Added** for new features
- **Changed** for changes in existing functionality
- **Deprecated** for soon-to-be removed features
- **Removed** for now removed features
- **Fixed** for any bug fixes
- **Security** for vulnerability fixes
- **Infrastructure** for infrastructure and tooling changes
- **Documentation** for documentation changes

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for information on how to contribute to BDP.

## Links

- [Repository](https://github.com/datadir-lab/bdp)
- [Issue Tracker](https://github.com/datadir-lab/bdp/issues)
- [Documentation](https://github.com/datadir-lab/bdp/tree/main/docs)
- [Roadmap](ROADMAP.md)

---

[Unreleased]: https://github.com/datadir-lab/bdp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/datadir-lab/bdp/releases/tag/v0.1.0
