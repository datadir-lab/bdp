# BDP Documentation Index

Comprehensive index of all BDP documentation organized by purpose and audience.

**Last Updated**: 2026-01-28

## Quick Start

| Document | Purpose | Audience |
|----------|---------|----------|
| [CLAUDE.md](../CLAUDE.md) | AI agent instructions | Claude Code |
| [AGENTS.md](../AGENTS.md) | Complete agent guide with links | Claude Code |
| [README.md](../README.md) | Project overview | All users |
| [ROADMAP.md](../ROADMAP.md) | Development roadmap and status | Developers |
| [QUICK_START.md](./QUICK_START.md) | Quick start guide | New users |
| [INSTALL.md](./INSTALL.md) | Installation instructions | New users |

## For AI Agents (Claude Code)

Start here for comprehensive development guidelines:

1. **[CLAUDE.md](../CLAUDE.md)** - Entry point with critical rules and commit conventions
2. **[AGENTS.md](../AGENTS.md)** - Complete modular documentation index
3. **[ROADMAP.md](../ROADMAP.md)** - Project status and priorities

## Agent Reference Documentation

Located in `docs/agents/` - permanent reference guides:

### Core Architecture
- **[architecture.md](./agents/architecture.md)** - System design, database schema, API design, package format
- **[backend-architecture.md](./agents/backend-architecture.md)** - **MANDATORY** CQRS pattern, vertical slices, audit logging
- **[stack.md](./agents/stack.md)** - Technology choices and rationale

### Development Guides
- **[rust-backend.md](./agents/rust-backend.md)** - axum server, database patterns, async best practices
- **[cli-development.md](./agents/cli-development.md)** - CLI commands, dependency resolution
- **[nextjs-frontend.md](./agents/nextjs-frontend.md)** - Next.js 16 patterns, Nextra docs, UI components
- **[testing.md](./agents/testing.md)** - Testing strategy for Rust and Next.js
- **[best-practices.md](./agents/best-practices.md)** - Code style, error handling, security, performance

### Critical Policies
- **[logging.md](./agents/logging.md)** - **MANDATORY** structured logging, NO console logs
- **[error-handling.md](./agents/error-handling.md)** - **MANDATORY** error handling patterns, NO `.unwrap()`
- **[database-design-philosophy.md](./agents/database-design-philosophy.md)** - Database design principles

### Design Specifications

Located in `docs/agents/design/` - detailed technical specifications:

- **[database-schema.md](./agents/design/database-schema.md)** - PostgreSQL schema, tables, relationships
- **[file-formats.md](./agents/design/file-formats.md)** - bdp.yml, bdl.lock, dependency cache
- **[api-design.md](./agents/design/api-design.md)** - REST endpoints, response formats
- **[cache-strategy.md](./agents/design/cache-strategy.md)** - Local caching, team sharing, file locking
- **[dependency-resolution.md](./agents/design/dependency-resolution.md)** - How aggregate sources work
- **[version-mapping.md](./agents/design/version-mapping.md)** - External to internal version translation
- **[uniprot-ingestion.md](./agents/design/uniprot-ingestion.md)** - Automated scraping and parsing
- **[cli-audit-provenance.md](./agents/design/cli-audit-provenance.md)** - Audit trail and provenance tracking

### Implementation Guides

Located in `docs/agents/implementation/` - detailed implementation patterns:

- **[cqrs-architecture.md](./agents/implementation/cqrs-architecture.md)** - Detailed CQRS implementation
- **[mediator-cqrs-architecture.md](./agents/implementation/mediator-cqrs-architecture.md)** - Mediator pattern guide
- **[sqlx-guide.md](./agents/implementation/sqlx-guide.md)** - Complete SQLx reference (compile-time queries, migrations, offline mode)
- **[mode-based-ingestion.md](./agents/implementation/mode-based-ingestion.md)** - Ingestion pipeline patterns
- **[INGESTION_PIPELINE_IMPLEMENTATION.md](./agents/implementation/INGESTION_PIPELINE_IMPLEMENTATION.md)** - Pipeline implementation details

### Workflows

Located in `docs/agents/workflows/` - step-by-step guides:

- **[adding-feature-cqrs.md](./agents/workflows/adding-feature-cqrs.md)** - **REQUIRED** workflow for all backend features
- **[adding-new-query.md](./agents/workflows/adding-new-query.md)** - Step-by-step workflow for adding SQLx queries
- **[adding-migration.md](./agents/workflows/adding-migration.md)** - Step-by-step workflow for database migrations

## Development Documentation

Located in `docs/development/` - development process and infrastructure:

- **[COMMIT_CONVENTIONS.md](./development/COMMIT_CONVENTIONS.md)** - **MANDATORY** conventional commits + Linear integration
- **[CI_CD.md](./development/CI_CD.md)** - GitHub Actions workflows, automated testing
- **[CI_CD_SUMMARY.md](./development/CI_CD_SUMMARY.md)** - CI/CD implementation summary
- **[RELEASE_PROCESS.md](./development/RELEASE_PROCESS.md)** - Release workflow and version management
- **[RELEASE_TESTING.md](./development/RELEASE_TESTING.md)** - Release testing checklist
- **[VERSIONING.md](./development/VERSIONING.md)** - Semantic versioning strategy
- **[testing.md](./development/testing.md)** - Testing infrastructure setup
- **[testing-quick-reference.md](./development/testing-quick-reference.md)** - Quick testing commands
- **[TESTING_INFRASTRUCTURE_SUMMARY.md](./development/TESTING_INFRASTRUCTURE_SUMMARY.md)** - Testing setup summary
- **[just-guide.md](./development/just-guide.md)** - Just task runner commands
- **[sqlx-setup.md](./development/sqlx-setup.md)** - SQLx setup and configuration
- **[QUICK_START_SQLX.md](./development/QUICK_START_SQLX.md)** - One-page SQLx reference
- **[QUICK_START_INGESTION.md](./development/QUICK_START_INGESTION.md)** - Ingestion pipeline quick start
- **[cli-docs-ci-integration.md](./development/cli-docs-ci-integration.md)** - CLI documentation CI integration
- **[cli-documentation-generation.md](./development/cli-documentation-generation.md)** - Automated CLI docs

## User Documentation

End-user facing documentation:

- **[INSTALL.md](./INSTALL.md)** - Installation instructions
- **[QUICK_START.md](./QUICK_START.md)** - Quick start guide
- **[SETUP.md](./SETUP.md)** - Detailed setup instructions
- **[database-setup.md](./database-setup.md)** - PostgreSQL database setup
- **[DOCKER_SETUP.md](./DOCKER_SETUP.md)** - Docker deployment guide
- **[TESTING.md](./TESTING.md)** - Testing overview

## Feature Documentation

Documentation for specific features:

- **[ORGANIZATION_METADATA.md](./ORGANIZATION_METADATA.md)** - Organization metadata feature

## Research Documentation

Located in `docs/research/` - research papers and analysis:

- **[bioconda-paper-analysis.md](./research/bioconda-paper-analysis.md)** - Analysis of Bioconda paper
- **[refgenie-paper-analysis.md](./research/refgenie-paper-analysis.md)** - Analysis of RefGenie paper
- **[reproducibility-crisis-bioinformatics.md](./research/reproducibility-crisis-bioinformatics.md)** - Reproducibility crisis analysis

## Archived Documentation

Located in `docs/archive/` - historical implementation docs:

### Implementation Archive

Located in `docs/archive/implementation/` - completed implementation summaries:

- GenBank implementation docs
- Gene Ontology implementation docs
- UniProt implementation docs
- Search optimization docs
- CLI implementation summaries
- Ingestion framework completion docs
- Schema migration docs
- Testing setup docs

### InterPro Archive

Located in `docs/archive/interpro/` - InterPro implementation sessions:

- Design documents
- Feasibility analysis
- Progress summaries
- Session notes

### Other Archives

Located in `docs/archive/` - older documentation:

- Phase completion summaries
- Legacy implementation reports
- Logging setup docs

## Documentation Structure

```
docs/
├── agents/                          # Agent reference documentation
│   ├── *.md                         # Core architecture, development guides
│   ├── design/                      # Design specifications
│   ├── implementation/              # Implementation patterns
│   │   └── archive/                 # Old implementation sessions
│   └── workflows/                   # Step-by-step workflows
├── development/                     # Development process docs
│   ├── COMMIT_CONVENTIONS.md        # Git standards (MANDATORY)
│   ├── CI_CD.md                     # CI/CD setup
│   ├── RELEASE_PROCESS.md           # Release workflow
│   └── *.md                         # Other dev docs
├── research/                        # Research papers
├── archive/                         # Archived docs
│   ├── implementation/              # Old summaries
│   └── interpro/                    # InterPro sessions
├── database-setup.md                # Database setup
├── DOCKER_SETUP.md                  # Docker guide
├── INSTALL.md                       # Installation
├── QUICK_START.md                   # Quick start
├── SETUP.md                         # Setup guide
└── TESTING.md                       # Testing overview
```

## Documentation Maintenance

### Adding New Documentation

1. **Agent Reference Docs**: Add to `docs/agents/` if it's a permanent guide
2. **Implementation Docs**: Add to `docs/agents/implementation/` if it's implementation-specific
3. **Development Docs**: Add to `docs/development/` if it's about process/infrastructure
4. **Research**: Add to `docs/research/` if it's paper analysis
5. **Archive**: Move to `docs/archive/` when docs become outdated

### Updating This Index

When adding new documentation:

1. Add entry to appropriate section above
2. Update the structure tree
3. Update last updated date
4. Commit with: `docs: update documentation index`

## Quick Links by Role

### For New Contributors
1. [README.md](../README.md) - Project overview
2. [INSTALL.md](./INSTALL.md) - Get set up
3. [QUICK_START.md](./QUICK_START.md) - Start developing
4. [AGENTS.md](../AGENTS.md) - Development guidelines

### For Backend Developers
1. [backend-architecture.md](./agents/backend-architecture.md) - CQRS pattern (MANDATORY)
2. [cqrs-architecture.md](./agents/implementation/cqrs-architecture.md) - Detailed CQRS
3. [sqlx-guide.md](./agents/implementation/sqlx-guide.md) - Database queries
4. [logging.md](./agents/logging.md) - Logging requirements
5. [error-handling.md](./agents/error-handling.md) - Error handling policy

### For Frontend Developers
1. [nextjs-frontend.md](./agents/nextjs-frontend.md) - Next.js patterns
2. [architecture.md](./agents/architecture.md) - System design

### For DevOps/Infrastructure
1. [DOCKER_SETUP.md](./DOCKER_SETUP.md) - Docker deployment
2. [CI_CD.md](./development/CI_CD.md) - CI/CD setup
3. [RELEASE_PROCESS.md](./development/RELEASE_PROCESS.md) - Release workflow

### For Technical Writers
1. [cli-documentation-generation.md](./development/cli-documentation-generation.md) - CLI docs
2. All docs in `web/app/[locale]/docs/content/` - User-facing documentation

---

**Maintained by**: BDP Development Team
**Documentation Version**: 1.0
**Project**: BDP (Bioinformatics Dependencies Platform)
