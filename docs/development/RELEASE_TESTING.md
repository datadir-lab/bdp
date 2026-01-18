# Release Testing & Self-Uninstall - Implementation Summary

This document explains the complete release testing pipeline and self-uninstall implementation for BDP.

## Overview

BDP uses a **two-workflow system** to ensure installers are fully tested before releases are published:

1. **`release.yml`** (cargo-dist generated) - Builds artifacts and creates **draft** release
2. **`test-release.yml`** (custom) - Tests installers on all platforms, then publishes

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    WORKFLOW: release.yml                    │
│                  (cargo-dist generated)                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ↓
                    Tag pushed (v0.1.0)
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │         plan                             │
        │  • Determine what to build               │
        │  • Check configuration                   │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │    build-local-artifacts (matrix)        │
        │  • Linux x86_64, ARM64                   │
        │  • macOS x86_64, ARM64                   │
        │  • Windows x86_64                        │
        │  ✅ Builds all binaries in parallel      │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │    build-global-artifacts                │
        │  • Generate checksums                    │
        │  • Create install scripts                │
        │    - bdp-installer.sh (Unix)             │
        │    - bdp-installer.ps1 (Windows)         │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │            host                          │
        │  • Upload artifacts to GitHub            │
        │  • Create DRAFT release                  │
        │    (--draft flag added)                  │
        └─────────────────────────────────────────┘
                              │
                              ↓
              🚨 RELEASE CREATED (DRAFT) 🚨
                    Triggers test-release.yml
                              │
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                 WORKFLOW: test-release.yml                  │
│                    (custom testing)                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │    test-installers (matrix)              │
        │  Runs on 4 platforms in parallel:        │
        │  • Ubuntu 22.04                          │
        │  • macOS 13 (Intel)                      │
        │  • macOS 14 (ARM)                        │
        │  • Windows 2022                          │
        └─────────────────────────────────────────┘
                              │
                              ↓
           For EACH platform, test sequence:
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │  1. Fresh Install                        │
        │     curl ... | sh  OR  irm ... | iex     │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │  2. Verify Installation                  │
        │     bdp --version                        │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │  3. Test Upgrade (re-install)            │
        │     curl ... | sh  OR  irm ... | iex     │
        │     Tests idempotency                    │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │  4. Test Uninstall                       │
        │     bdp uninstall --purge -y             │
        └─────────────────────────────────────────┘
                              │
                              ↓
        ┌─────────────────────────────────────────┐
        │  5. Verify Uninstall                     │
        │     Ensure `bdp` command removed         │
        └─────────────────────────────────────────┘
                              │
                              ↓
                    ✅ All 4 platforms passed?
                              │
                      ┌───────┴───────┐
                      │ YES           │ NO
                      ↓               ↓
        ┌──────────────────┐  ┌─────────────────┐
        │ publish-release  │  │ Workflow fails  │
        │ • Undraft        │  │ Release stays   │
        │   release        │  │ in DRAFT        │
        │ • Announce       │  └─────────────────┘
        └──────────────────┘
                 │
                 ↓
        ✅ PUBLIC RELEASE PUBLISHED!
```

## Self-Uninstall Implementation

### Research & Best Practices

Based on [rustup's implementation](https://github.com/rust-lang/rustup/) and industry best practices:

**Key Requirements:**
1. ✅ Confirmation prompt (unless `--yes`)
2. ✅ Pre-uninstall checks (no locked files)
3. ✅ Clean cache removal (optional with `--purge`)
4. ✅ Verification step
5. ✅ Graceful fallback on Windows

**References:**
- [How to Uninstall Rust via rustup](https://medium.com/@trivajay259/how-to-uninstall-rust-installed-via-rustup-cleanly-safely-completely-66fff19ab90d)
- [rustup PR #2864: Improved uninstall process](https://github.com/rust-lang/rustup/pull/2864)
- [rustup Issue #3330: Windows uninstall challenges](https://github.com/rust-lang/rustup/issues/3330)

### Unix/Linux/macOS Implementation

**Problem:** How to delete an executable while it's running?

**Solution:** Unix allows unlinking open files!

```rust
// Spawn background shell that waits then deletes
let script = format!(r#"(sleep 1 && rm -f '{}') &"#, exe_path);
Command::new("sh").arg("-c").arg(&script).spawn()?;
```

**How it works:**
1. File is "unlinked" from filesystem directory
2. Inode and data remain until process exits
3. Background process waits 1 second
4. Process exits, releases file handle
5. Background script deletes the inode

**Result:** ✅ Clean, reliable removal

### Windows Implementation

**Problem:** Windows locks files in use - can't delete running executable!

**Solution:** The Rename Trick™ (same as rustup)

```rust
// 1. Rename the executable (this works even while running!)
fs::rename(exe_path, temp_path)?;

// 2. Create batch script to delete renamed file
let batch_script = r#"@echo off
timeout /t 2 /nobreak >nul
del /f /q "bdp.exe.old"
exit"#;

// 3. Spawn batch script in background
Command::new("cmd").arg("/C").arg("start").arg("/B").arg(script).spawn()?;
```

**Why this works:**
- **Renaming** a file doesn't require closing it (Windows quirk!)
- Original path (`bdp.exe`) is now free
- Batch script waits 2 seconds for process to exit
- Then deletes the renamed file (`bdp.exe.old`)

**Fallback:** If rename fails (rare), provide manual instructions:
```
⚠ Unable to remove the executable while it's running.

To complete uninstallation, either:
  1. Restart your computer (recommended)
  2. Manually delete: C:\Users\name\.cargo\bin\bdp.exe
```

**Result:** ✅ Robust, user-friendly removal

### Testing the Uninstall

The CI tests verify:

```bash
# 1. Install works
curl ... | sh

# 2. Command is available
bdp --version  # ✅ Should succeed

# 3. Uninstall works
bdp uninstall --purge -y

# 4. Verify removal
sleep 3  # Wait for background processes
command -v bdp  # ❌ Should fail (not found)
```

**Why sleep 3?**
- Unix: Background process needs 1s to execute
- Windows: Batch script needs 2s timeout
- Extra time for any async cleanup

## Why Two Workflows?

### Option 1: Single Workflow (what we rejected)
```
Tag → Build → Test → Create Release
```

**Problem:** cargo-dist generates `release.yml` and overwrites custom changes

**Attempted:** Add test jobs to `release.yml`

**Result:** Running `dist generate` removes our changes

### Option 2: Two Workflows (our solution)
```
Tag → release.yml (build, create draft)
    → test-release.yml (test, publish)
```

**Benefits:**
- ✅ Keep cargo-dist workflow pristine
- ✅ Can run `dist generate` without losing tests
- ✅ Separation of concerns (build vs test)
- ✅ Easier to maintain

**Workflow Triggers:**
- `release.yml`: Triggered by git tag push
- `test-release.yml`: Triggered by `release: [created]` event

## Configuration Files

### dist-workspace.toml
```toml
[dist]
cargo-dist-version = "0.30.3"
ci = "github"
installers = ["shell", "powershell"]
targets = [
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc"
]
install-path = "CARGO_HOME"
```

**Note:** Only edit this file, then run `dist generate` to update `release.yml`

### release.yml (Modified)

**Single change made:**
```yaml
# Before (cargo-dist default):
gh release create "$TAG" ... artifacts/*

# After (our modification):
gh release create "$TAG" --draft ... artifacts/*
#                        ^^^^^^^^ Added --draft flag
```

**Comment added:**
```yaml
# Create as DRAFT - test-release.yml workflow will test installers and publish
```

### test-release.yml (Custom)

Completely custom workflow that:
1. Waits for draft release creation
2. Tests all platforms
3. Publishes if tests pass

## Testing Strategy

### Why Test Installers?

Many projects build binaries but don't test the **actual user experience**:

❌ **Common approach:**
- Build for platforms
- Publish release
- Hope it works
- Users report issues

✅ **Our approach:**
- Build for platforms
- Create draft release
- **Test actual install scripts**
- **Test upgrade path**
- **Test uninstall**
- **Only then** publish

### Test Matrix

| Platform | OS | Architecture | Installer |
|----------|----------|--------------|-----------|
| Linux | Ubuntu 22.04 | x86_64 | Shell script |
| macOS Intel | macOS 13 | x86_64 | Shell script |
| macOS ARM | macOS 14 | ARM64 | Shell script |
| Windows | Windows 2022 | x86_64 | PowerShell |

**Why these versions?**
- Ubuntu 22.04: LTS, widely used
- macOS 13: Latest Intel Mac support
- macOS 14: Latest ARM Mac (Apple Silicon)
- Windows 2022: Latest stable Windows Server

### What Gets Tested

For EACH platform:

1. **Fresh Install Test**
   - Clean system
   - Run installer script
   - Verify binary installed
   - Verify PATH updated

2. **Upgrade Test**
   - Re-run installer
   - Tests idempotency
   - Ensures no conflicts

3. **Uninstall Test**
   - Run `bdp uninstall --purge -y`
   - Tests self-uninstall
   - Verifies cache removal

4. **Verification Test**
   - Confirm binary removed
   - Confirm cache removed
   - Confirm no leftover files

### Failure Handling

**If ANY test fails:**
- ❌ Release stays in DRAFT mode
- ❌ Users cannot install
- ✅ Maintainers can investigate
- ✅ Fix and re-release

**If ALL tests pass:**
- ✅ Release published automatically
- ✅ Users can install immediately
- ✅ Confidence in quality

## End-User Experience

### Installation

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh

# Windows
irm https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1 | iex
```

**What happens:**
1. Downloads installer script from latest release
2. Installer downloads appropriate binary for platform
3. Verifies checksum
4. Installs to `~/.cargo/bin/`
5. Updates PATH (if needed)

### Uninstallation

```bash
# Interactive
bdp uninstall

# Non-interactive
bdp uninstall -y

# With cache cleanup
bdp uninstall --purge -y
```

**What happens:**
1. Confirmation prompt (unless `-y`)
2. Removes cache if `--purge`
3. On Unix: Spawns background process to delete binary
4. On Windows: Renames then schedules deletion
5. Success message displayed

## Maintenance

### Making a Release

```bash
# 1. Update version
vim Cargo.toml  # workspace.package.version

# 2. Tag and push
git tag v0.1.0
git push origin v0.1.0
```

**Then monitor:**
1. `release.yml` builds artifacts (~5-10 min)
2. Draft release created
3. `test-release.yml` runs tests (~5-10 min)
4. Release published (if tests pass)

**Total time:** ~15-20 minutes

### Updating cargo-dist

```bash
# Update version in dist-workspace.toml
vim dist-workspace.toml

# Regenerate workflow
dist generate

# Re-add --draft flag
vim .github/workflows/release.yml
# Find: gh release create
# Add: --draft flag
```

## Security Considerations

### Tested Before Public

- All releases tested before users can access them
- Failed releases stay in draft mode
- Only successful builds are published

### Installer Script Security

See [CI_CD.md Security Considerations](CI_CD.md#security-considerations) for details on:
- HTTPS-only downloads
- Checksum verification
- Signed artifacts
- Review-before-run recommendations

### Self-Uninstall Safety

- Confirmation prompt (default)
- Graceful fallback on Windows
- No sudo/admin required
- Cache removal optional

## Troubleshooting

### Release Stays in Draft

**Check test-release.yml logs:**
```bash
gh run list --workflow=test-release.yml
gh run view <run-id> --log
```

**Common issues:**
- Installer script 404 (cargo-dist didn't upload)
- Binary not found in PATH
- Permission issues
- Uninstall failed to remove binary

### Fix and Re-Release

```bash
# 1. Delete tag locally and remotely
git tag -d v0.1.0
git push origin :refs/tags/v0.1.0

# 2. Delete draft release
gh release delete v0.1.0 --yes

# 3. Fix the issue

# 4. Re-tag and push
git tag v0.1.0
git push origin v0.1.0
```

## Future Enhancements

1. **Auto-update:** `bdp upgrade` command
2. **Rollback:** `bdp downgrade v0.1.0`
3. **Platform detection:** Smarter install script
4. **Package managers:** Homebrew, Scoop, APT
5. **Delta updates:** Only download changed files
6. **Telemetry:** Track install success rates (opt-in)

---

## Quick Reference

### Commands
```bash
# Build and test locally
cargo build --release
cargo test --all-features
cargo dist plan

# Make release
git tag v0.1.0 && git push origin v0.1.0

# Check workflow status
gh run list
gh run view <run-id>

# Test install locally
curl -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

### Files
- `.github/workflows/release.yml` - Build and create draft
- `.github/workflows/test-release.yml` - Test and publish
- `dist-workspace.toml` - cargo-dist configuration
- `crates/bdp-cli/src/commands/uninstall.rs` - Self-uninstall implementation

---

**For more details:**
- [CI_CD.md](CI_CD.md) - Complete CI/CD documentation
- [RELEASE_PROCESS.md](RELEASE_PROCESS.md) - Quick start guide
- [INSTALL.md](INSTALL.md) - User installation instructions
