# Unified Version Management

This document explains how BDP manages versions across the entire monorepo with a single source of truth.

## Overview

BDP uses **cargo-release** with custom hooks to manage versions in ONE place and automatically sync to all files.

### The Problem

In a monorepo with both Rust and Node.js projects, versions can get out of sync:
- ❌ `Cargo.toml` (workspace + 4 crates)
- ❌ `web/package.json`
- ❌ Manual git tagging
- ❌ Error-prone manual updates

### The Solution

✅ **Single source of truth:** `Cargo.toml` workspace version
✅ **Automatic sync:** Syncs to all files before committing
✅ **Automatic git tags:** Creates and pushes tags automatically
✅ **CI/CD trigger:** Tag push triggers release workflow

---

## Architecture

```
┌──────────────────────────────────────────────────────┐
│  Cargo.toml (ROOT)                                   │
│  [workspace.package]                                 │
│  version = "0.1.0"   ← SINGLE SOURCE OF TRUTH       │
└──────────────────────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
         ↓               ↓               ↓
  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
  │ bdp-server  │ │   bdp-cli   │ │  bdp-ingest │
  │ inherits    │ │  inherits   │ │  inherits   │
  │ version     │ │  version    │ │  version    │
  └─────────────┘ └─────────────┘ └─────────────┘

                         │
                         ↓
         ┌───────────────────────────────┐
         │  cargo-release runs           │
         │  with pre-release-hook        │
         └───────────────────────────────┘
                         │
                         ↓
         ┌───────────────────────────────┐
         │  scripts/sync-version.js      │
         │  • Reads $NEW_VERSION         │
         │  • Updates web/package.json   │
         └───────────────────────────────┘
                         │
                         ↓
         ┌───────────────────────────────┐
         │  cargo-release commits        │
         │  • All version changes        │
         │  • Creates git tag v0.1.0     │
         │  • Pushes tag to origin       │
         └───────────────────────────────┘
                         │
                         ↓
         ┌───────────────────────────────┐
         │  GitHub Actions               │
         │  • Tag push triggers workflow │
         │  • Builds release artifacts   │
         │  • Tests installers           │
         │  • Publishes release          │
         └───────────────────────────────┘
```

---

## Quick Start

### 1. Install cargo-release

```bash
just install-cargo-release
# OR
cargo install cargo-release
```

### 2. Check Current Version

```bash
just version
# Output:
# 📦 BDP Version Information
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Rust:    v0.1.0
# Node:    v0.1.0
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 3. Bump Version

```bash
# Patch release (0.1.0 → 0.1.1)
just release-patch

# Minor release (0.1.0 → 0.2.0)
just release-minor

# Major release (0.1.0 → 1.0.0)
just release-major
```

**That's it!** This automatically:
1. ✅ Bumps version in `Cargo.toml`
2. ✅ Syncs to `web/package.json`
3. ✅ Commits changes
4. ✅ Creates git tag `v0.1.1`
5. ✅ Pushes tag to GitHub
6. ✅ Triggers CI/CD release workflow

---

## Detailed Usage

### Dry Run (Preview Changes)

Before making a release, see what will change:

```bash
# Preview patch release
just release-patch-dry

# Preview minor release
just release-minor-dry
```

**Output shows:**
- Current version
- New version
- Files that will be updated
- Git operations (commit, tag, push)

### Manual Version Bump (Testing)

For local testing without git operations:

```bash
just bump-version 0.2.0-beta.1
```

This updates files but **does NOT**:
- Commit changes
- Create git tags
- Push to remote

Useful for:
- Testing the sync script
- Creating pre-release versions locally
- Debugging version issues

### What Gets Updated

When you run `just release-*`, these files are updated:

1. **Root Cargo.toml**
   ```toml
   [workspace.package]
   version = "0.1.1"  # ← Updated
   ```

2. **All Crate Cargo.toml files** (via inheritance)
   ```toml
   [package]
   version.workspace = true  # ← Gets new version
   ```

3. **web/package.json**
   ```json
   {
     "version": "0.1.1"  # ← Updated by sync script
   }
   ```

4. **Git Tag**
   ```bash
   v0.1.1  # ← Created and pushed
   ```

---

## Configuration

### cargo-release Settings

Configuration in `Cargo.toml`:

```toml
[workspace.metadata.release]
# All crates share the same version
shared-version = true

# Git tagging
tag = true
tag-name = "v{{version}}"
tag-message = "chore: Release {{crate_name}} v{{version}}"

# Push to GitHub
push = true
push-remote = "origin"

# Pre-release hook to sync version to package.json
pre-release-hook = ["node", "scripts/sync-version.js"]

# Don't publish to crates.io (we're using GitHub releases)
publish = false
```

### Sync Script

`scripts/sync-version.js` reads environment variables from cargo-release:

- `$NEW_VERSION` - The new version being released
- `$PREV_VERSION` - The previous version

The script:
1. Automatically finds workspace root (works when called from any crate directory)
2. Reads `web/package.json` relative to workspace root
3. Updates `version` field
4. Writes file back with proper formatting

**Note**: The script is smart enough to find the workspace root by looking for `Cargo.toml` with `[workspace]`, so it works correctly regardless of which crate directory cargo-release runs it from.

---

## Semantic Versioning

BDP follows [Semantic Versioning](https://semver.org/) (SemVer):

```
MAJOR.MINOR.PATCH

Examples:
  0.1.0  → Initial development
  0.2.0  → New features (backward compatible)
  0.2.1  → Bug fixes
  1.0.0  → First stable release
  1.1.0  → New features
  1.1.1  → Bug fixes
  2.0.0  → Breaking changes
```

### When to Bump What

**PATCH (0.1.0 → 0.1.1)**
- Bug fixes
- Documentation updates
- Performance improvements (no API changes)
- Internal refactoring

```bash
just release-patch
```

**MINOR (0.1.0 → 0.2.0)**
- New features (backward compatible)
- New CLI commands
- New API endpoints
- Deprecations (with backward compatibility)

```bash
just release-minor
```

**MAJOR (0.1.0 → 1.0.0)**
- Breaking changes
- API redesign
- Removed features
- Changed behavior of existing features

```bash
just release-major
```

---

## Workflow

### Standard Release

```bash
# 1. Check current version
just version

# 2. Make sure you're on main and up to date
git checkout main
git pull

# 3. Make sure tests pass
just ci

# 4. Preview what will change (optional)
just release-patch-dry

# 5. Bump version and release
just release-patch

# 6. Monitor CI/CD
# Go to https://github.com/datadir-lab/bdp/actions
# Watch the release workflow build and test
```

**What happens automatically:**
1. Version bumped in all files
2. Changes committed
3. Git tag created and pushed
4. GitHub Actions triggered
5. Artifacts built
6. Installers tested
7. Release published

**Total time:** ~15-20 minutes from `just release-patch` to public release

### Pre-release

For alpha/beta releases:

```bash
# Manually set pre-release version
just bump-version 0.2.0-beta.1

# Commit and tag manually
git add .
git commit -m "chore: Prepare v0.2.0-beta.1"
git tag v0.2.0-beta.1
git push origin main
git push origin v0.2.0-beta.1
```

Pre-release versions will be marked as "Pre-release" on GitHub.

---

## Integration with CI/CD

### Trigger Flow

```bash
just release-patch
   ↓
Version bumped to 0.1.1
   ↓
Git tag v0.1.1 created
   ↓
Tag pushed to origin
   ↓
GitHub Actions: release.yml
   ↓
Builds artifacts
   ↓
Creates draft release
   ↓
GitHub Actions: test-release.yml
   ↓
Tests installers
   ↓
Publishes release (if tests pass)
```

### Preventing Duplicate Releases

The system prevents duplicate releases because:
1. Git tags are unique (can't push same tag twice)
2. cargo-release checks if tag already exists
3. GitHub Actions only triggers on new tags

If you need to re-release a version:
1. Delete the tag locally and remotely
2. Delete the GitHub release
3. Re-run the release command

---

## Troubleshooting

### "Tag already exists"

**Problem:** You tried to release but the tag already exists.

**Solution:**
```bash
# Delete tag locally and remotely
git tag -d v0.1.0
git push origin :refs/tags/v0.1.0

# Re-run release
just release-patch
```

### "sync-version.js failed"

**Problem:** The pre-release-hook script failed.

**Possible causes:**
1. Node.js not installed
2. package.json has syntax errors
3. File permissions

**Solution:**
```bash
# Test the script manually
NEW_VERSION=0.1.1 node scripts/sync-version.js

# Check Node.js version
node --version  # Should be v18+
```

### "cargo-release not found"

**Problem:** cargo-release is not installed.

**Solution:**
```bash
just install-cargo-release
# OR
cargo install cargo-release
```

### Versions out of sync

**Problem:** Rust and Node versions don't match.

**Solution:**
```bash
# Check versions
just version

# Manually sync
NEW_VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="bdp-cli") | .version')
echo $NEW_VERSION
NEW_VERSION=$NEW_VERSION node scripts/sync-version.js
```

---

## Best Practices

### 1. Always Dry Run First

```bash
# Preview changes
just release-patch-dry

# Review output
# If looks good, run actual release
just release-patch
```

### 2. Keep main Up to Date

```bash
git checkout main
git pull
just release-patch
```

### 3. Test Before Releasing

```bash
# Run all tests
just ci

# Then release
just release-patch
```

### 4. Write Changelog

Update `CHANGELOG.md` before releasing:

```markdown
## [0.1.1] - 2026-01-16

### Fixed
- Fixed bug in data source resolution
- Improved error messages

### Added
- New `bdp uninstall` command
```

### 5. Monitor CI/CD

After releasing, watch the GitHub Actions workflow to ensure:
- Build succeeds
- Tests pass
- Release is published

---

## Comparison with Manual Process

### Before (Manual)

```bash
# 1. Update Cargo.toml
vim Cargo.toml  # Change version in 5 places

# 2. Update package.json
vim web/package.json  # Change version

# 3. Commit
git add .
git commit -m "Bump version to 0.1.1"

# 4. Tag
git tag v0.1.1

# 5. Push
git push origin main
git push origin v0.1.1

# ❌ Error-prone (easy to miss files)
# ❌ 5+ manual steps
# ❌ Easy to forget tags
# ❌ Easy to make typos
```

### Now (Automated)

```bash
just release-patch

# ✅ One command
# ✅ No manual editing
# ✅ Consistent formatting
# ✅ Automatic sync
# ✅ Automatic tagging
# ✅ Automatic push
```

---

## Advanced Usage

### Custom Version Bump

If you need a specific version (not patch/minor/major):

```bash
# Using cargo-release (with git operations)
cargo release --execute --no-publish 0.2.0-rc.1

# Using manual bump (no git operations)
just bump-version 0.2.0-rc.1
git add .
git commit -m "chore: Release v0.2.0-rc.1"
git tag v0.2.0-rc.1
git push origin main
git push origin v0.2.0-rc.1
```

### Skip Sync Hook (for testing)

If you want to test without running the sync script:

```bash
# Edit Cargo.toml, temporarily comment out pre-release-hook
cargo release patch --execute --no-publish
```

### Sync Only (without release)

To sync versions without creating a release:

```bash
# Get current version
VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="bdp-cli") | .version')

# Sync to package.json
NEW_VERSION=$VERSION node scripts/sync-version.js
```

---

## FAQ

### Q: Can I release from a branch other than main?

**A:** Yes, but not recommended. cargo-release will push the tag from any branch. However, our CI/CD is configured to expect releases from main.

### Q: What if I need to rollback a release?

**A:** Delete the tag and GitHub release, then fix the issue and re-release:

```bash
git tag -d v0.1.1
git push origin :refs/tags/v0.1.1
gh release delete v0.1.1 --yes
```

### Q: Can I have different versions for different crates?

**A:** Not with the current configuration (`shared-version = true`). If you need independent versioning, set `shared-version = false` in Cargo.toml and handle package.json separately.

### Q: Does this work offline?

**A:** The version bump and sync work offline, but pushing the tag requires internet. You can use `--no-push` flag:

```bash
cargo release patch --execute --no-publish --no-push
```

Then push manually later:
```bash
git push origin main
git push origin v0.1.1
```

---

## Documentation Links

- **[cargo-release documentation](https://github.com/crate-ci/cargo-release)**
- **[Semantic Versioning](https://semver.org/)**
- **[CI_CD.md](CI_CD.md)** - Complete CI/CD pipeline docs
- **[RELEASE_PROCESS.md](RELEASE_PROCESS.md)** - Release process guide

---

## Quick Reference

```bash
# Check version
just version

# Dry run
just release-patch-dry
just release-minor-dry

# Release
just release-patch   # 0.1.0 → 0.1.1
just release-minor   # 0.1.0 → 0.2.0
just release-major   # 0.1.0 → 1.0.0

# Manual bump (no git)
just bump-version 0.2.0-beta.1

# Install cargo-release
just install-cargo-release
```

---

**Next Steps:**

1. Install cargo-release: `just install-cargo-release`
2. Check current version: `just version`
3. Make a test release: `just release-patch-dry`
4. Actually release: `just release-patch`
5. Monitor: https://github.com/datadir-lab/bdp/actions
