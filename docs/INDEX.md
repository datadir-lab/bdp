# BDP Documentation Index

Complete documentation index for developers and AI agents working on the Bioinformatics Dependencies Platform.

## Quick Start

- **[Installation Guide](./INSTALL.md)** - How to install BDP CLI
- **[Quick Start](./QUICK_START.md)** - Get started in 5 minutes
- **[Setup Guide](./SETUP.md)** - Development environment setup
- **[Testing Guide](./TESTING.md)** - Running tests

## For AI Agents (Claude Code)

### Core Architecture & Patterns
- **[Architecture Overview](./agents/architecture.md)** - System design, database schema, API design
- **[Backend Architecture](./agents/backend-architecture.md)** - **MANDATORY CQRS pattern**
- **[CQRS Architecture](./agents/implementation/cqrs-architecture.md)** - Detailed CQRS implementation
- **[Mediator-CQRS Architecture](./agents/implementation/mediator-cqrs-architecture.md)** - Mediator pattern guide
- **[Technology Stack](./agents/stack.md)** - Technology choices and rationale

### Development Guidelines (MANDATORY)
- **[Logging Best Practices](./agents/logging.md)** - **NO `println!`/`eprintln!`/`dbg!`** - Use structured logging
- **[Error Handling Policy](./agents/error-handling.md)** - **NO `.unwrap()`** in production
- **[Best Practices](./agents/best-practices.md)** - Code style, security, performance
- **[Testing Strategy](./agents/testing.md)** - Unit tests, integration tests, E2E tests

### Backend Development
- **[Rust Backend Guide](./agents/rust-backend.md)** - axum server, database patterns, async
- **[SQLx Guide](./agents/implementation/sqlx-guide.md)** - Complete SQLx reference
- **[CLI Development](./agents/cli-development.md)** - CLI commands and patterns

### Frontend Development
- **[Next.js Frontend](./agents/nextjs-frontend.md)** - **Next.js 16** patterns (use `proxy.ts` NOT `middleware.ts`)

### Workflows (Step-by-Step)
- **[Adding Feature with CQRS](./agents/workflows/adding-feature-cqrs.md)** - **REQUIRED for all backend features**
- **[Adding New Query](./agents/workflows/adding-new-query.md)** - Step-by-step SQLx query workflow
- **[Adding Migration](./agents/workflows/adding-migration.md)** - Database migration workflow

### Design Specifications
- **[Database Schema](./agents/design/database-schema.md)** - PostgreSQL schema, tables, relationships
- **[API Design](./agents/design/api-design.md)** - REST endpoints, response formats
- **[File Formats](./agents/design/file-formats.md)** - bdp.yml, bdl.lock, dependency cache
- **[Cache Strategy](./agents/design/cache-strategy.md)** - Local caching, team sharing
- **[Dependency Resolution](./agents/design/dependency-resolution.md)** - How aggregate sources work
- **[Version Mapping](./agents/design/version-mapping.md)** - External to internal version translation
- **[UniProt Ingestion](./agents/design/uniprot-ingestion.md)** - Automated scraping and parsing
- **[CLI Audit & Provenance](./agents/design/cli-audit-provenance.md)** - Audit trail design

## For Developers

### Getting Started
- **[Docker Setup](./DOCKER_SETUP.md)** - Docker configuration
- **[Database Setup](./database-setup.md)** - PostgreSQL setup guide
- **[Quick Start - SQLx](./development/QUICK_START_SQLX.md)** - One-page SQLx reference
- **[Quick Start - Ingestion](./development/QUICK_START_INGESTION.md)** - Ingestion pipeline quick start

### Development Workflow
- **[Just Guide](./development/just-guide.md)** - Task runner commands
- **[Testing Infrastructure](./development/testing.md)** - Testing setup
- **[SQLx Setup](./development/sqlx-setup.md)** - SQLx configuration

### CI/CD & Releases
- **[CI/CD Guide](./development/CI_CD.md)** - Complete CI/CD documentation
- **[Release Process](./development/RELEASE_PROCESS.md)** - How to release new versions
- **[Release Testing](./development/RELEASE_TESTING.md)** - Testing installers
- **[Versioning](./development/VERSIONING.md)** - Version management
- **[CLI Documentation Generation](./development/cli-documentation-generation.md)** - Auto-generate CLI docs
- **[CLI Docs CI Integration](./development/cli-docs-ci-integration.md)** - CI/CD for CLI docs

## Additional Resources

### Research & Analysis
- **[Reproducibility Crisis in Bioinformatics](./research/reproducibility-crisis-bioinformatics.md)**
- **[Refgenie Paper Analysis](./research/refgenie-paper-analysis.md)**
- **[Bioconda Paper Analysis](./research/bioconda-paper-analysis.md)**

### Organization Metadata
- **[Organization Metadata](./ORGANIZATION_METADATA.md)** - Organization structure and metadata

## Archived Documentation

Historical implementation documentation and summaries are in **[docs/archive/](./archive/)** for reference.

## Project Status

**Current Version**: 0.1.0
**Last Updated**: 2026-01-28

### Status Overview
- ✅ Backend API (25+ endpoints, CQRS architecture)
- ✅ Database (67 migrations)
- ✅ CLI (10 commands with full audit system)
- ✅ Frontend (Next.js 16, all pages built)
- ✅ Ingestion Pipelines (4 complete: UniProt, NCBI Taxonomy, GenBank, Gene Ontology)
- ✅ Infrastructure (Terraform, OVH Cloud)
- 🔄 Production Data Ingestion (ready to run)
- 🔄 Deployment (pending credentials)

## Contact

For questions or contributions, see project repository: https://github.com/datadir-lab/bdp

**Contact**: sebastian.stupak@pm.me
