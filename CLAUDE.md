# Claude Code Instructions for BDP

Instructions for AI agents (Claude Code) working on the Bioinformatics Dependencies Platform.

## Documentation Index

**📖 Start here**: [docs/INDEX.md](./docs/INDEX.md)

The documentation index contains all guides organized by topic. **ALWAYS refer to this index** when working on the project.

## Critical Rules

### Logging (MANDATORY)

**Server Code (bdp-server)**:
- **NEVER** use `println!`, `eprintln!`, or `dbg!` in production code
- **ALWAYS** use structured logging: `info!`, `warn!`, `error!`, `debug!`
- Exception: Test modules can use println! for debugging

**CLI Code (bdp-cli)**:
- Use `println!` ONLY for direct user-facing output:
  - Command results (tables, status, formatted output)
  - Success/error messages shown to users
  - Interactive prompts
- Use `tracing` for internal operations:
  - `info!()` for progress and status updates
  - `warn!()` for warnings
  - `error!()` for internal errors
  - `debug!()` for debugging information
- Exception: Test modules can use println! for debugging

**Test Code**:
- `println!`, `eprintln!`, and `dbg!` are acceptable for debugging
- Prefer structured logging for integration tests

**Key Distinction**:
- User output (what the user sees) → `println!`
- Internal logging (diagnostics, debugging, monitoring) → `tracing`

See [Logging Best Practices](./docs/agents/logging.md) for details.

### Error Handling (MANDATORY)
- **NEVER** use `.unwrap()` or `.expect()` in production code
- **ALWAYS** use `?` operator or proper error handling
- See [Error Handling Policy](./docs/agents/error-handling.md)

### Backend Architecture (MANDATORY)
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

## Workflow Checklist

### Before Starting Work
- [ ] Check [docs/INDEX.md](./docs/INDEX.md) for relevant documentation
- [ ] Read relevant architecture/pattern docs
- [ ] Understand the task requirements

### During Development
- [ ] Follow architectural patterns (especially CQRS for backend)
- [ ] Use proper logging (structured logging in server, println! for CLI user output)
- [ ] Handle errors properly (NO `.unwrap()`)
- [ ] Write tests
- [ ] Commit with conventional format

### Before Committing
- [ ] Run `cargo clippy` (Rust)
- [ ] Run `cargo fmt` (Rust)
- [ ] Run tests
- [ ] Verify commit message format

## Project Status (2026-01-28)

| Component | Status | Notes |
|-----------|--------|-------|
| Backend | ✅ 100% | Production-ready, 67 migrations, 750+ tests |
| Ingestion | ✅ 95% | All pipelines coded, needs production data |
| CLI | ✅ 100% | 10 commands, installers, CI/CD complete |
| Frontend | ✅ 80% | All pages built, needs E2E testing |
| Infrastructure | ✅ Ready | Terraform IaC ready for deployment |

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

## Contact

**Email**: sebastian.stupak@pm.me  
**Issues**: https://github.com/datadir-lab/bdp/issues

---

**Last Updated**: 2026-01-28  
**Documentation Index**: [docs/INDEX.md](./docs/INDEX.md)
