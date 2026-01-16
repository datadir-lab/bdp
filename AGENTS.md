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

### Backend Development
- **[Rust Backend](./docs/agents/rust-backend.md)** - axum server, database patterns, async best practices
- **[CLI Development](./docs/agents/cli-development.md)** - CLI commands (`bdp source add`, `bdp tool add`), dependency resolution

### Frontend Development
- **[Next.js Frontend](./docs/agents/nextjs-frontend.md)** - **Next.js 16** patterns, Nextra documentation, UI components
- **[API Integration](./docs/agents/api-integration.md)** - Frontend-backend communication patterns

### Quality & Testing
- **[Testing Strategy](./docs/agents/testing.md)** - Unit tests, integration tests, E2E tests for Rust and Next.js
- **[Best Practices](./docs/agents/best-practices.md)** - Code style, error handling, security, performance

### Operations
- **[Deployment](./docs/agents/deployment.md)** - Single server deployment, database migrations, CI/CD

## Development Workflow

1. **Read** relevant agent docs before implementing features
2. **Follow** the architectural patterns and best practices
3. **Test** according to the testing strategy
4. **Document** new patterns or decisions in the appropriate agent doc

## Quick Reference

### For Rust Backend Work
→ Read: [Architecture](./docs/agents/architecture.md), [Rust Backend](./docs/agents/rust-backend.md), [Testing](./docs/agents/testing.md)

### For CLI Implementation
→ Read: [CLI Development](./docs/agents/cli-development.md), [Architecture](./docs/agents/architecture.md), [Best Practices](./docs/agents/best-practices.md)

### For Frontend Work
→ Read: [Next.js Frontend](./docs/agents/nextjs-frontend.md), [API Integration](./docs/agents/api-integration.md)

### For Testing
→ Read: [Testing Strategy](./docs/agents/testing.md), [Best Practices](./docs/agents/best-practices.md)

## Contributing

When working on BDP:
1. Consult the relevant agent documentation first
2. Make incremental, testable changes
3. Update documentation when architectural decisions change
4. Follow the single-server deployment philosophy

---

**Note**: These documents are living guides that evolve with the project. Update them as patterns emerge or decisions change.
