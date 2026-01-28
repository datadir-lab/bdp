# CLI Documentation CI/CD Integration Summary

This document summarizes the CLI documentation generation integration into build and CI/CD workflows.

## Overview

CLI documentation is now automatically generated and validated throughout the development and deployment pipeline, ensuring docs are always in sync with the code.

## Integration Points

### 1. Local Development

#### Just Commands

```bash
# Generate CLI docs manually
just docs-cli

# Verify docs are up-to-date
just docs-cli-check

# Run all CI checks (includes doc check)
just ci
```

#### Web Builds

```bash
# Development build (includes doc generation)
just web-build

# Production build with Pagefind (includes doc generation)
just web-prod
```

#### Production Builds

```bash
# Full production build (includes doc generation)
just prod-build
```

### 2. GitHub Actions CI/CD

#### Main CI Workflow (`.github/workflows/ci.yml`)

**New Job: `cli-docs-check`**

- Runs on every push and PR
- Generates fresh CLI documentation
- Compares with committed version
- Fails if docs are out of date
- Provides helpful error message

**Flow:**
1. Checkout code
2. Setup Rust toolchain
3. Cache dependencies
4. Generate CLI docs: `cargo run --package xtask -- generate-cli-docs`
5. Check for differences: `git diff --exit-code cli-reference.mdx`
6. Exit with error if outdated

**Error Message:**
```
❌ CLI documentation is out of date!
Please run 'just docs-cli' and commit the changes.
```

#### Web CI Workflow (`.github/workflows/ci-web.yml`)

**Modified Build Job**

- Added Rust toolchain setup
- Added cargo caching
- Generates CLI docs before building frontend
- Ensures web build has latest CLI reference

**Flow:**
1. Checkout code
2. Setup Rust toolchain
3. Cache Rust dependencies
4. **Generate CLI documentation**
5. Setup Node.js
6. Install web dependencies
7. Build Next.js site
8. Index with Pagefind

### 3. Justfile Integration

#### Updated Recipes

**`web-build`**
```just
web-build:
    @Write-Host "📚 Generating CLI documentation..."
    @cargo run --package xtask -- generate-cli-docs
    @Write-Host "🌐 Building frontend..."
    @cd web; yarn build
    # ... rest of build
```

**`web-prod`**
```just
web-prod:
    @Write-Host "📚 Generating CLI documentation..."
    @cargo run --package xtask -- generate-cli-docs
    @Write-Host "🌐 Building frontend..."
    @cd web; yarn build
    # ... Pagefind indexing and server start
```

**`ci`**
```just
ci: docs-cli-check sqlx-check lint test
    @echo "✓ All CI checks passed!"
```

**`prod-build`**
```just
prod-build: docs-cli build-release web-build docker-build
    @echo "✓ Production build complete"
```

## Benefits

### For Developers

✅ **No manual steps** - Docs regenerate automatically on build
✅ **Immediate feedback** - CI catches outdated docs before merge
✅ **Clear instructions** - Error messages tell you exactly what to do
✅ **Local validation** - Run `just ci` to check before pushing

### For Users

✅ **Always accurate** - Docs match the actual CLI implementation
✅ **Auto-updated** - Every deployment has fresh documentation
✅ **Comprehensive** - All commands, flags, and options documented
✅ **Searchable** - Pagefind indexes CLI docs for easy discovery

### For Maintainers

✅ **Version controlled** - Docs are tracked in git
✅ **PR reviewable** - Doc changes visible in diffs
✅ **Enforced by CI** - Can't merge outdated docs
✅ **Zero maintenance** - Automated end-to-end

## Developer Workflow

### Making CLI Changes

1. **Modify CLI code**
   ```bash
   code crates/bdp-cli/src/lib.rs
   ```

2. **Test locally**
   ```bash
   cargo run --bin bdp -- --help
   ```

3. **Regenerate docs**
   ```bash
   just docs-cli
   ```

4. **Verify changes**
   ```bash
   git diff web/app/[locale]/docs/content/en/cli-reference.mdx
   ```

5. **Run CI checks locally**
   ```bash
   just ci
   ```

6. **Commit everything**
   ```bash
   git add crates/bdp-cli/src/lib.rs
   git add web/app/[locale]/docs/content/en/cli-reference.mdx
   git commit -m "feat: add --format flag to export command"
   ```

7. **Push and create PR**
   ```bash
   git push origin feature-branch
   ```

### If CI Fails on Doc Check

```bash
# Regenerate docs
just docs-cli

# Commit the updated docs
git add web/app/[locale]/docs/content/en/cli-reference.mdx
git commit --amend --no-edit

# Force push to update PR
git push --force-with-lease
```

## Technical Details

### File Locations

- **Generator**: `xtask/src/main.rs`
- **CLI definitions**: `crates/bdp-cli/src/lib.rs`
- **Generated docs**: `web/app/[locale]/docs/content/en/cli-reference.mdx`
- **CI workflows**:
  - `.github/workflows/ci.yml`
  - `.github/workflows/ci-web.yml`

### Dependencies

- `clap = "4.5"` - CLI framework with derive macros
- `clap-markdown = "0.1"` - Markdown generation from Clap
- `chrono = "0.4"` - Timestamps in generated docs

### Generation Command

```bash
cargo run --package xtask -- generate-cli-docs [--output-dir DIR]
```

### Validation Command

```bash
just docs-cli-check
```

Or manually:
```bash
cargo run --package xtask -- generate-cli-docs
git diff --exit-code web/app/[locale]/docs/content/en/cli-reference.mdx
```

## CI/CD Pipeline Flow

### On Every Push/PR

```
┌─────────────────────────────────────────────┐
│  Developer pushes code                      │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│  GitHub Actions CI Triggered                │
└────────────────┬────────────────────────────┘
                 │
                 ├─────────────────┐
                 │                 │
                 ▼                 ▼
    ┌─────────────────────┐  ┌──────────────────┐
    │ cli-docs-check      │  │ Other CI jobs    │
    │                     │  │ (lint, test, etc)│
    │ 1. Generate docs    │  └──────────────────┘
    │ 2. Compare with git │
    │ 3. Fail if outdated │
    └──────────┬──────────┘
               │
               ├─ ✅ Docs up-to-date → Continue
               │
               └─ ❌ Docs outdated → Fail with message
```

### On Web Build

```
┌─────────────────────────────────────────────┐
│  just web-build / just web-prod             │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│  1. Generate CLI docs                       │
│     cargo run --package xtask --            │
│       generate-cli-docs                     │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│  2. Build Next.js                           │
│     - Includes fresh CLI reference          │
│     - Embedded in documentation site        │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│  3. Index with Pagefind                     │
│     - CLI docs searchable                   │
│     - Full-text search enabled              │
└────────────────┬────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│  ✅ Production-ready build with docs        │
└─────────────────────────────────────────────┘
```

## Troubleshooting

### CI fails with "CLI documentation is out of date"

**Cause**: You modified CLI code but didn't regenerate docs

**Solution**:
```bash
just docs-cli
git add web/app/[locale]/docs/content/en/cli-reference.mdx
git commit --amend --no-edit
git push --force-with-lease
```

### Web build fails on "generate-cli-docs"

**Cause**: Rust compilation error in bdp-cli or xtask

**Solution**:
```bash
# Fix Rust compilation errors first
cargo check --package bdp-cli
cargo check --package xtask

# Then try web build again
just web-build
```

### Docs generated but look wrong

**Cause**: Template or formatting issue in xtask

**Solution**:
```bash
# Check the xtask generator
code xtask/src/main.rs

# Regenerate with verbose output
cargo run --package xtask -- generate-cli-docs
```

## Future Enhancements

- [ ] Auto-commit docs in pre-commit hook
- [ ] Generate per-command documentation pages
- [ ] Add interactive command builder to web UI
- [ ] Generate shell completion scripts (bash, zsh, fish)
- [ ] Generate man pages for Unix systems
- [ ] Support multiple output formats (HTML, PDF, etc.)

## Related Documentation

- [CLI Documentation Generation](./cli-documentation-generation.md) - Detailed usage guide
- [CLI Development](./agents/cli-development.md) - CLI implementation guide
- [Testing Strategy](./agents/testing.md) - Testing best practices
- [Next.js Frontend](./agents/nextjs-frontend.md) - Web documentation setup

## References

- [clap-markdown](https://crates.io/crates/clap-markdown)
- [cargo-xtask pattern](https://github.com/matklad/cargo-xtask)
- [GitHub Actions - Caching](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows)
