# Agent Development Guide

This document serves as the entry point for AI agents (like Claude) working on the BDP project. Each section links to detailed documentation modules.

## Documentation Structure Rules

**Directory Organization:**

- `docs/agents/*.md` - Agent reference documentation (architecture, stack, best practices, testing guides)
- `docs/agents/implementation/` - Documentation created DURING implementation (summaries, session notes, implementation logs, feature documentation)

**When to place files:**
- Permanent guides and references → `docs/agents/*.md`
- Temporary/session-specific implementation docs → `docs/agents/implementation/`
- Summary docs created during work → `docs/agents/implementation/`

## Documentation Modules

### Core Architecture
- **[Architecture Overview](./docs/agents/architecture.md)** - System design, database schema, API design, package format
- **[Technology Stack](./docs/agents/stack.md)** - Detailed breakdown of all technologies and why they were chosen
- **[Logging Best Practices](./docs/agents/logging.md)** - **MANDATORY structured logging, no console logs**

### Backend Development

**⚠️ CRITICAL: All backend features MUST follow the CQRS architecture defined below.**

**🚨 CRITICAL: NEVER use `println!`, `eprintln!`, or `dbg!` - Use structured logging ONLY**

- **[Backend Architecture](./docs/agents/backend-architecture.md)** - **MANDATORY CQRS pattern, vertical slices, audit logging**
- **[CQRS Architecture](./docs/agents/implementation/cqrs-architecture.md)** - Detailed CQRS implementation guide
- **[Rust Backend](./docs/agents/rust-backend.md)** - axum server, database patterns, async best practices
- **[SQLx Guide](./docs/agents/implementation/sqlx-guide.md)** - Complete SQLx reference (compile-time queries, migrations, offline mode)
- **[CLI Development](./docs/agents/cli-development.md)** - CLI commands (`bdp source add`, `bdp tool add`), dependency resolution
- **[Logging Best Practices](./docs/agents/logging.md)** - **MANDATORY structured logging, NO console logs**

### CLI Development

**⚠️ CRITICAL: CLI Testing Location**

**NEVER test CLI commands in the main repository directory!** Commands like `bdp init` create files that would pollute the repository.

- **Test Directory**: `D:\dev\datadir\bdp-example\`
- **Reason**: CLI commands create `bdp.yml`, `.gitignore`, `.bdp/` directories

**Testing Workflow:**
```bash
# Set up test directory
just test-cli-setup

# Test individual commands
just test-cli "init --name test-project"
just test-cli "source add uniprot:P01308-fasta@1.0"
just test-cli "source list"

# Clean up after testing
just test-cli-clean

# Run full test workflow
just test-cli-full
```

**Manual Testing:**
```bash
cd D:\dev\datadir\bdp-example
cargo run --bin bdp -- init --name my-project
cargo run --bin bdp -- source add "uniprot:P01308-fasta@1.0"
cargo run --bin bdp -- source list
cargo run --bin bdp -- pull  # Requires backend running
cargo run --bin bdp -- status
cargo run --bin bdp -- audit
```

### Frontend Development

**⚠️ CRITICAL: Next.js 16 - Use `proxy.ts` NOT `middleware.ts`**

**IMPORTANT:** Next.js 16 has deprecated `middleware.ts` in favor of `proxy.ts`.

- ✅ **ONLY use** `web/proxy.ts` for routing middleware (i18n, redirects, etc.)
- ❌ **NEVER create** `web/middleware.ts` - it causes conflicts and build errors
- The `proxy.ts` file handles i18n routing using `next-intl/middleware`
- **Do not mix both files** - use `proxy.ts` exclusively

**Correct structure:**
```
web/
├── proxy.ts          # ✅ Use this for routing/middleware
└── middleware.ts     # ❌ NEVER create this in Next.js 16
```

- **[Next.js Frontend](./docs/agents/nextjs-frontend.md)** - **Next.js 16** patterns, Nextra documentation, UI components
- **[API Integration](./docs/agents/api-integration.md)** - Frontend-backend communication patterns

### Quality & Testing
- **[Testing Strategy](./docs/agents/testing.md)** - Unit tests, integration tests, E2E tests for Rust and Next.js
- **[Best Practices](./docs/agents/best-practices.md)** - Code style, error handling, security, performance

### Operations
- **[Deployment](./docs/agents/deployment.md)** - Single server deployment, database migrations, CI/CD

### Workflow Guides
- **[Adding Feature with CQRS](./docs/agents/workflows/adding-feature-cqrs.md)** - **REQUIRED workflow for all backend features**
- **[Adding New Query](./docs/agents/workflows/adding-new-query.md)** - Step-by-step workflow for adding SQLx queries
- **[Adding Migration](./docs/agents/workflows/adding-migration.md)** - Step-by-step workflow for database migrations
- **[SQLx Quick Start](./docs/QUICK_START_SQLX.md)** - One-page SQLx reference

## Development Workflow

### Backend Feature Development (MANDATORY PROCESS)

1. **Read Backend Architecture** - [backend-architecture.md](./docs/agents/backend-architecture.md) - **NON-NEGOTIABLE**
2. **Follow CQRS Pattern**:
   - Commands (write) → Use transactions, add audit logging
   - Queries (read) → No transactions, no audit logging
3. **Use Vertical Slices** - All code for a feature in `features/feature_name/`
4. **Use Structured Logging** - **NEVER** `println!`/`eprintln!`/`dbg!`, **ALWAYS** `info!`/`warn!`/`error!`
5. **Follow the Workflow** - [Adding Feature with CQRS](./docs/agents/workflows/adding-feature-cqrs.md)
6. **Test** - Verify commands create audit logs, queries don't
7. **Review Checklist** - [Implementation Checklist](./docs/agents/backend-architecture.md#implementation-checklist)

### General Workflow

1. **Read** relevant agent docs before implementing features
2. **Follow** the architectural patterns and best practices (especially CQRS for backend)
3. **Test** according to the testing strategy
4. **Document** new patterns or decisions in the appropriate agent doc

## Quick Reference

### For Rust Backend Work
→ **MUST READ FIRST**: [Backend Architecture](./docs/agents/backend-architecture.md) (CQRS pattern - MANDATORY)
→ **MUST READ**: [Logging Best Practices](./docs/agents/logging.md) - **NO `println!`/`eprintln!`/`dbg!`**
→ Then read: [CQRS Architecture](./docs/agents/implementation/cqrs-architecture.md), [Adding Feature with CQRS](./docs/agents/workflows/adding-feature-cqrs.md)
→ Additional: [Rust Backend](./docs/agents/rust-backend.md), [SQLx Guide](./docs/agents/implementation/sqlx-guide.md), [Testing](./docs/agents/testing.md)

### For Database Work
→ Read: [SQLx Quick Start](./QUICK_START_SQLX.md), [Adding New Query](./docs/agents/workflows/adding-new-query.md), [Adding Migration](./docs/agents/workflows/adding-migration.md)

### For CLI Implementation
→ **MUST READ**: [Logging Best Practices](./docs/agents/logging.md) - **NO `println!`/`eprintln!`/`dbg!`**
→ Read: [CLI Development](./docs/agents/cli-development.md), [Architecture](./docs/agents/architecture.md), [Best Practices](./docs/agents/best-practices.md)

### For Frontend Work
→ Read: [Next.js Frontend](./docs/agents/nextjs-frontend.md), [API Integration](./docs/agents/api-integration.md)
→ **⚠️ CRITICAL - Package Manager**:
  - **ALWAYS use `yarn`** for web/ directory (NOT npm)
  - Run `cd web && yarn install` to install dependencies
  - Run `cd web && yarn add <package>` to add new packages
  - Run `cd web && yarn build` to build
  - **NEVER use npm** - it causes lock file conflicts
→ **⚠️ CRITICAL**: Use `proxy.ts` NOT `middleware.ts` in Next.js 16

### For Testing
→ Read: [Testing Strategy](./docs/agents/testing.md), [Best Practices](./docs/agents/best-practices.md)

### For Logging
→ **MUST READ**: [Logging Best Practices](./docs/agents/logging.md)
→ **NEVER use `println!`, `eprintln!`, or `dbg!`** - Use structured logging: `info!`, `warn!`, `error!`

## Contributing

When working on BDP:
1. Consult the relevant agent documentation first
2. **NEVER use `println!`, `eprintln!`, or `dbg!`** - Use structured logging (`info!`, `warn!`, `error!`)
3. Make incremental, testable changes
4. Update documentation when architectural decisions change
5. Follow the single-server deployment philosophy

---

**Note**: These documents are living guides that evolve with the project. Update them as patterns emerge or decisions change.
