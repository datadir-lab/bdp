# Claude Code Instructions for BDP

This document provides comprehensive instructions for AI agents (Claude Code) working on the BDP project.

## Quick Start

1. **Read the Agent Guide**: Start with [AGENTS.md](./AGENTS.md) for complete development guidelines
2. **Check the Roadmap**: Review [ROADMAP.md](./ROADMAP.md) for project status and priorities
3. **Follow Commit Conventions**: Use conventional commits and link to Linear issues - see [Commit Conventions](./docs/development/COMMIT_CONVENTIONS.md)

## Critical Rules

### Logging
- **NEVER** use `println!`, `eprintln!`, or `dbg!` in Rust code
- **ALWAYS** use structured logging: `info!`, `warn!`, `error!`
- See [Logging Best Practices](./docs/agents/logging.md)

### Error Handling
- **NEVER** use `.unwrap()` or `.expect()` in production code
- **ALWAYS** use `?` operator or proper error handling
- See [Error Handling Policy](./docs/agents/error-handling.md)

### Backend Architecture
- **MUST** follow CQRS pattern for all backend features
- Commands (write) → Use transactions, add audit logging
- Queries (read) → No transactions, no audit logging
- See [Backend Architecture](./docs/agents/backend-architecture.md)

### Frontend (Next.js 16)
- **ONLY** use `web/proxy.ts` for routing middleware
- **NEVER** create `web/middleware.ts` (deprecated in Next.js 16)
- **ALWAYS** use `yarn` (NOT npm) in web/ directory

### CLI Testing
- **NEVER** test CLI commands in main repository directory
- **ALWAYS** use `D:\dev\datadir\bdp-example\` for CLI testing
- Use `just test-cli-*` commands for testing

## Commit Conventions (MANDATORY)

BDP uses **Conventional Commits** with **Linear integration**:

### Format
```
<type>(<scope>): <subject>

[optional body]
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `chore`: Maintenance, dependencies, tooling
- `docs`: Documentation only
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `test`: Tests
- `ci`/`build`: CI/CD changes

### Examples
```bash
feat(cli): implement bdp query command

Adds SQL-like query syntax with JOIN and WHERE support.

---

fix(api): prevent database connection timeout

Adds retry logic and connection pooling.

---

chore(deps): update Rust dependencies

Updates sqlx, tokio, axum to latest versions.
```

### Linear Integration
- Use branch naming: `<type>/bdp-<id>-<description>` to auto-link commits
- Update Linear task status when starting/completing work
- Reference Linear issues in PR descriptions (e.g., "Resolves BDP-17")
- Keep commit messages clean (no issue IDs in commits)

**Full documentation**: [docs/development/COMMIT_CONVENTIONS.md](./docs/development/COMMIT_CONVENTIONS.md)

## Documentation Structure

```
docs/
├── agents/                          # Agent reference documentation
│   ├── architecture.md              # System design, database schema
│   ├── backend-architecture.md      # CQRS pattern (MANDATORY)
│   ├── cli-development.md           # CLI patterns
│   ├── error-handling.md            # Error handling policy
│   ├── logging.md                   # Logging best practices
│   ├── nextjs-frontend.md           # Next.js 16 patterns
│   ├── stack.md                     # Technology choices
│   ├── testing.md                   # Testing strategy
│   ├── design/                      # Design specifications
│   ├── implementation/              # Implementation guides (CQRS, SQLx)
│   └── workflows/                   # Step-by-step workflows
├── development/                     # Development guides
│   ├── COMMIT_CONVENTIONS.md        # Git commit standards
│   ├── CI_CD.md                     # CI/CD setup
│   ├── RELEASE_PROCESS.md           # Release workflow
│   ├── testing.md                   # Testing infrastructure
│   └── VERSIONING.md                # Version management
├── research/                        # Research papers and analysis
├── archive/                         # Archived implementation docs
│   ├── implementation/              # Old summaries and reports
│   └── interpro/                    # InterPro implementation sessions
├── database-setup.md                # PostgreSQL setup guide
├── DOCKER_SETUP.md                  # Docker configuration
├── INSTALL.md                       # Installation instructions
├── QUICK_START.md                   # Quick start guide
├── SETUP.md                         # Setup instructions
└── TESTING.md                       # Testing overview
```

## Quick Reference by Task Type

### Backend Development
1. Read [Backend Architecture](./docs/agents/backend-architecture.md) (MANDATORY)
2. Read [CQRS Architecture](./docs/agents/implementation/cqrs-architecture.md)
3. Read [Error Handling](./docs/agents/error-handling.md)
4. Read [Logging](./docs/agents/logging.md)
5. Follow [Adding Feature with CQRS](./docs/agents/workflows/adding-feature-cqrs.md)

### Database Work
1. Read [SQLx Guide](./docs/agents/implementation/sqlx-guide.md)
2. Read [Adding New Query](./docs/agents/workflows/adding-new-query.md)
3. Read [Adding Migration](./docs/agents/workflows/adding-migration.md)

### CLI Development
1. Read [CLI Development](./docs/agents/cli-development.md)
2. Read [Error Handling](./docs/agents/error-handling.md)
3. Read [Logging](./docs/agents/logging.md)
4. Use `D:\dev\datadir\bdp-example\` for testing

### Frontend Development
1. Read [Next.js Frontend](./docs/agents/nextjs-frontend.md)
2. **CRITICAL**: Use `yarn` (NOT npm) in web/ directory
3. **CRITICAL**: Use `proxy.ts` NOT `middleware.ts`

### Testing
1. Read [Testing Strategy](./docs/agents/testing.md)
2. Read [Development Testing](./docs/development/testing.md)

### Releases & Deployment
1. Read [Release Process](./docs/development/RELEASE_PROCESS.md)
2. Read [CI/CD](./docs/development/CI_CD.md)
3. Read [Versioning](./docs/development/VERSIONING.md)

## Workflow Checklist

### Before Starting Work
- [ ] Read relevant documentation from AGENTS.md
- [ ] Check ROADMAP.md for context
- [ ] Create/assign Linear task
- [ ] Create branch: `<type>/<linear-id>-<description>`

### During Development
- [ ] Follow architectural patterns (especially CQRS for backend)
- [ ] Use structured logging (NO `println!`)
- [ ] Handle errors properly (NO `.unwrap()`)
- [ ] Write tests
- [ ] Commit with conventional format + Linear ID

### Before Committing
- [ ] Run `cargo clippy` (Rust)
- [ ] Run `cargo fmt` (Rust)
- [ ] Run tests
- [ ] Verify commit message format
- [ ] Include Linear issue ID

### Before PR
- [ ] All tests pass
- [ ] CI passes
- [ ] Linear tasks updated
- [ ] PR description includes test plan
- [ ] Breaking changes documented

## Project Status (2026-01-28)

| Component | Status | Notes |
|-----------|--------|-------|
| Backend | ✅ 100% | Production-ready, 67 migrations, 750+ tests |
| Ingestion | ✅ 95% | All pipelines coded, needs production data |
| CLI | ✅ 100% | 10 commands, installers, CI/CD complete |
| Frontend | 🔄 80% | Needs E2E testing |
| Infrastructure | ✅ Ready | Terraform IaC ready for deployment |
| Documentation | ✅ 90% | Comprehensive agent guides |

**Current Version**: 0.1.0
**Target Launch**: March 15, 2026

## Technology Stack

- **Backend**: Rust + axum + SQLx (CQRS with mediator pattern)
- **Database**: PostgreSQL 16+
- **CLI**: Rust + clap
- **Frontend**: Next.js 16 + Tailwind + Radix UI
- **Storage**: MinIO/S3
- **Infrastructure**: Terraform + OVH Cloud
- **CI/CD**: GitHub Actions
- **Task Runner**: Just

## Common Commands

```bash
# Development
just dev              # Start development servers
just test             # Run all tests
just fmt              # Format code
just lint             # Run linters

# CLI Testing (use bdp-example directory!)
just test-cli-setup   # Set up test directory
just test-cli "init"  # Test CLI command
just test-cli-clean   # Clean test directory

# Database
just migrate          # Run migrations
just sqlx-prepare     # Regenerate SQLx metadata

# Frontend (use yarn!)
cd web && yarn install
cd web && yarn dev
cd web && yarn build
```

## Getting Help

- **Documentation Issues**: Update relevant docs in `docs/agents/`
- **Architectural Questions**: Consult AGENTS.md and design docs
- **Project Status**: Check ROADMAP.md
- **Commit Format**: See docs/development/COMMIT_CONVENTIONS.md

## Contributing

1. Read AGENTS.md and relevant documentation
2. Create Linear task (or get assigned)
3. Create feature branch with Linear ID
4. Follow architectural patterns
5. Write tests
6. Use conventional commits with Linear ID
7. Create PR with proper description
8. Update Linear task status

---

**Last Updated**: 2026-01-28
**Project**: BDP (Bioinformatics Dependencies Platform)
**Documentation Version**: 1.0
