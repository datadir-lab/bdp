# Commit Conventions

This document defines commit message standards and Linear integration requirements for BDP development.

## Conventional Commits

BDP follows the [Conventional Commits](https://www.conventionalcommits.org/) specification for all commit messages.

### Format

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### Types

| Type | Description | Example |
|------|-------------|---------|
| `feat` | New feature | `feat(cli): add bdp query command` |
| `fix` | Bug fix | `fix(api): resolve database connection timeout` |
| `chore` | Maintenance, dependencies, tooling | `chore: update dependencies` |
| `docs` | Documentation only | `docs: update API integration guide` |
| `style` | Code formatting (no logic change) | `style: apply rustfmt to all files` |
| `refactor` | Code restructuring (no behavior change) | `refactor(db): simplify query builder` |
| `perf` | Performance improvement | `perf(search): optimize full-text search query` |
| `test` | Adding or fixing tests | `test(cli): add integration tests for bdp pull` |
| `build` | Build system, CI/CD changes | `build: update Cargo.toml dependencies` |
| `ci` | CI/CD configuration | `ci: add GitHub Actions workflow for releases` |
| `revert` | Revert a previous commit | `revert: revert "feat: add experimental feature"` |

### Scope

Scope is optional but recommended. Common scopes:

- `cli` - CLI commands and functionality
- `api` - Backend API endpoints
- `db` - Database schema, migrations, queries
- `ingest` - Data ingestion pipelines
- `web` - Frontend/Next.js application
- `infra` - Infrastructure, deployment, Docker
- `docs` - Documentation
- `deps` - Dependency updates

### Subject

- Use imperative mood: "add feature" not "added feature" or "adds feature"
- Don't capitalize first letter
- No period at the end
- Keep under 72 characters

### Examples

```bash
# Good commit messages
feat(cli): implement bdp query command with SQL-like syntax
fix(api): prevent race condition in concurrent uploads
chore(deps): update sqlx to 0.8.2
docs(agents): add commit conventions guide
refactor(ingest): extract common FTP logic to shared module
perf(db): add index on proteins.accession column
test(cli): add integration tests for bdp search
ci: add automated release workflow

# Bad commit messages
fix stuff                          # Too vague
Added new feature                  # Wrong tense, capitalized
fix: fixes the bug in the thing    # Redundant, vague
feat!: breaking change             # Missing scope and description
```

## Linear Integration

### Managing Linear Tasks

When working on Linear tasks:

1. **Start working**: Update task status to "In Progress" in Linear
2. **Commit regularly**: Use conventional commit format
3. **Complete work**: Update task status to "Completed" in Linear
4. **Link commits**: Use Linear's GitHub integration (automatic via branch names or PR descriptions)

### Branch Naming for Linear Integration

Name branches to automatically link to Linear issues:

```bash
<type>/<linear-id>-<short-description>

Examples:
feat/bdp-17-query-command
fix/bdp-23-search-timeout
chore/bdp-5-update-deps
```

Linear will automatically associate commits from these branches with the corresponding issues.

### Pull Request Integration

Reference Linear issues in PR descriptions (not individual commits):

```
feat(cli): implement bdp query and search commands

Implements SQL-like query syntax with JOIN, WHERE, and LIMIT support.
Adds remote search via API with filtering.

Resolves BDP-17, BDP-18
```

This keeps commit messages clean while maintaining Linear integration through PRs.

## Git Workflow

### Branch Naming

Follow this pattern:

```
<type>/<linear-id>-<short-description>

Examples:
feat/bdp-17-query-command
fix/bdp-23-search-timeout
chore/bdp-5-update-deps
```

### Commit Frequency

- Commit early and often
- Each commit should be a logical unit of work
- Keep commits focused on a single change
- Don't mix refactoring with feature changes

### Pull Requests

PR titles should follow conventional commit format:

```
feat(cli): implement bdp query and search commands

Resolves BDP-17, BDP-18
```

PR description should:
- Reference all related Linear issues
- Include test plan
- Describe changes and rationale
- Note any breaking changes

## Examples from BDP

### Feature Implementation

```bash
# Starting work on BDP-17 (branch name links to Linear)
git checkout -b feat/bdp-17-query-command

# Make changes, commit with clean messages
git commit -m "feat(cli): add query parser for bdp query

Implements SQL-like syntax parser with WHERE clause support."

git commit -m "feat(cli): add JOIN support to bdp query

Allows joining protein metadata with GO annotations."

git commit -m "test(cli): add integration tests for bdp query

Covers basic queries, JOINs, and error cases."

# Create PR with Linear reference
gh pr create --title "feat(cli): implement bdp query command" \
  --body "Resolves BDP-17"
```

### Bug Fix

```bash
git checkout -b fix/bdp-45-search-crash

git commit -m "fix(api): prevent null pointer in search endpoint

Adds validation for empty search queries before database access."

# PR description links to Linear
gh pr create --title "fix(api): prevent search endpoint crash" \
  --body "Fixes BDP-45"
```

### Chore Updates

```bash
git commit -m "chore(deps): update Rust dependencies

Updates sqlx, tokio, axum to latest patch versions."
```

## Enforcement

- GitHub Actions CI checks commit message format
- PRs with improperly formatted commits will fail CI
- Use `git commit --amend` to fix the last commit message
- Use interactive rebase to fix earlier commits

## Tools

### Commitizen

Optional but recommended for consistent commit messages:

```bash
# Install commitizen
cargo install git-cz

# Use for commits
git cz
```

### Pre-commit Hooks

Add commit message validation:

```bash
# .git/hooks/commit-msg
#!/bin/bash
commit_msg=$(cat "$1")
pattern="^(feat|fix|chore|docs|style|refactor|perf|test|build|ci|revert)(\(.+\))?: .{1,72}"

if ! echo "$commit_msg" | grep -qE "$pattern"; then
    echo "ERROR: Commit message doesn't follow Conventional Commits format"
    echo "Format: <type>(<scope>): <subject>"
    echo "Example: feat(cli): add bdp query command"
    exit 1
fi
```

## Resources

- [Conventional Commits Specification](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [Linear Git Integration](https://linear.app/docs/github)
- [Keep a Changelog](https://keepachangelog.com/)

---

**Generated with Claude Code** - Last updated: 2026-01-28
