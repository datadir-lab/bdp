# BDP Release Process - Quick Start

This is a quick reference for maintainers releasing new versions of BDP.

## TL;DR - Making a Release

```bash
# 1. Update version
vim Cargo.toml  # Change workspace.package.version

# 2. Update changelog
vim CHANGELOG.md  # Add release notes

# 3. Commit, tag, and push
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.1.0"
git tag v0.1.0
git push origin main
git push origin v0.1.0  # ← This triggers everything!
```

**What happens next (automatically):**
1. ✅ Builds binaries for 5 platforms (Linux, macOS, Windows - various architectures)
2. ✅ Creates **DRAFT** GitHub release (not public yet!)
3. ✅ Tests install scripts on 4 different OS configurations
4. ✅ Tests upgrade path (re-install over existing)
5. ✅ Tests uninstall functionality
6. ✅ **Publishes release** only if ALL tests pass

**Duration:** ~10-15 minutes from tag push to public release

---

## Architecture Overview

### The Problem We Solved

**Challenge:** How to safely release cross-platform binaries with tested install scripts?

**Solution:** A 3-phase pipeline:

```
Phase 1: BUILD
├─ Build artifacts for all platforms (parallel)
├─ Generate install scripts (shell + PowerShell)
└─ Create DRAFT GitHub release

Phase 2: TEST (only if build succeeds)
├─ Test on Ubuntu 22.04: Install → Verify → Upgrade → Uninstall
├─ Test on macOS 13 Intel: Install → Verify → Upgrade → Uninstall
├─ Test on macOS 14 ARM: Install → Verify → Upgrade → Uninstall
└─ Test on Windows 2022: Install → Verify → Upgrade → Uninstall

Phase 3: PUBLISH (only if tests pass)
└─ Make draft release public
```

If ANY test fails, the release stays in draft mode and is NOT published.

---

## Key Innovation: Self-Uninstall

**Problem:** How do users uninstall the CLI tool?

**Bad Solutions:**
- ❌ Require users to manually find and delete the binary
- ❌ Provide external scripts (but they need to be hosted somewhere)
- ❌ Hope users remember where they installed it

**Our Solution:** `bdp uninstall` command

```bash
bdp uninstall           # Interactive
bdp uninstall -y        # Auto-confirm
bdp uninstall --purge   # Remove everything
```

**Technical Challenge:** How does a program delete itself while running?

### Unix/Linux/macOS Implementation

✅ **Easy** - Unix allows unlinking open files:

```rust
// Spawn background shell that waits then deletes
sh -c "(sleep 1 && rm -f /path/to/bdp) &"
```

Works because:
1. File is "unlinked" from filesystem immediately
2. Inode/data remains until process exits
3. Background process waits for exit, then cleans up

### Windows Implementation

❌ **Hard** - Windows locks files in use!

**Solution:** The Rename Trick™

```rust
// 1. Rename running executable (this works!)
mv bdp.exe → bdp.exe.old

// 2. Create batch script to delete renamed file
@echo off
timeout /t 2 /nobreak >nul
del /f /q "bdp.exe.old"
exit

// 3. Spawn batch script in background
cmd /C start /B uninstall.bat
```

**Why this works:**
- Renaming a file doesn't require closing it
- Original path is now free
- Batch script waits for process to exit, then deletes renamed file

**Fallback:** If rename fails (rare), provide manual instructions.

This is the same technique used by `rustup self uninstall`!

---

## Installation Flow for End Users

### Install

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh

# Windows
irm https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1 | iex
```

### Verify

```bash
bdp --version
bdp --help
```

### Upgrade

Just run the installer again:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

### Uninstall

```bash
bdp uninstall --purge -y
```

---

## Testing Strategy

### Why We Test Installers

Many projects build binaries but don't test the actual installation experience. We test:

1. **Fresh Install** - Does the installer work on a clean system?
2. **Verification** - Can users actually run the installed binary?
3. **Upgrade** - Does reinstalling work (idempotent)?
4. **Uninstall** - Can users cleanly remove the tool?

### Test Matrix

| OS | Architecture | Installer Type |
|----|--------------|----------------|
| Ubuntu 22.04 | x86_64 | Shell |
| macOS 13 | x86_64 (Intel) | Shell |
| macOS 14 | ARM64 (Apple Silicon) | Shell |
| Windows 2022 | x86_64 | PowerShell |

### What Each Test Does

```yaml
- name: Test Fresh Install
  run: ${{ install-script }}  # Downloads and installs

- name: Verify Installation
  run: bdp --version          # Confirms it works

- name: Test Upgrade
  run: ${{ install-script }}  # Re-installs (tests idempotency)

- name: Test Uninstall
  run: bdp uninstall --purge -y

- name: Verify Uninstall
  run: |
    if command -v bdp; then
      echo "ERROR: Still installed!"
      exit 1
    fi
```

---

## cargo-dist Configuration

**Why cargo-dist?**
- Industry standard for Rust CLI distribution
- Handles cross-compilation automatically
- Generates install scripts
- Integrates with GitHub Actions
- Used by: ripgrep, bat, fd, and many others

**Configuration:** `dist-workspace.toml`

```toml
[dist]
cargo-dist-version = "0.30.3"
ci = "github"
installers = ["shell", "powershell"]
targets = [
    "aarch64-apple-darwin",      # macOS ARM
    "aarch64-unknown-linux-gnu", # Linux ARM
    "x86_64-apple-darwin",       # macOS Intel
    "x86_64-unknown-linux-gnu",  # Linux x86
    "x86_64-pc-windows-msvc"     # Windows
]
install-path = "CARGO_HOME"  # ~/.cargo/bin
```

**Regenerate workflow after config changes:**
```bash
dist generate
```

---

## Security Considerations

### The `curl | sh` Debate

**The Problem:**
```bash
curl https://example.com/install.sh | sh
```

Is considered dangerous because:
1. Pipes data directly to shell
2. Server could detect `curl` and serve malicious script
3. Interrupted downloads = partial/broken commands
4. Users typically don't review before running

**Our Mitigations:**
1. ✅ HTTPS only (prevents MITM)
2. ✅ cargo-dist generates scripts (not hand-written)
3. ✅ Scripts include integrity checks
4. ✅ All tests must pass before release is public
5. ✅ Users can download and review first:
   ```bash
   curl -LsSf https://...install.sh > installer.sh
   less installer.sh  # Review
   sh installer.sh    # Run after review
   ```

**Reference:** [GitLab moved away from curl|bash in 2026](https://gitlab.com/gitlab-org/gitlab-runner/-/merge_requests/6036)

### Artifact Integrity

- All artifacts have SHA-256 checksums
- Released artifacts are immutable
- GitHub Actions signs all builds
- cargo-dist validates downloads

---

## Troubleshooting

### Release stays in draft mode

**Check:**
```bash
# View workflow runs
gh run list --workflow=release.yml

# View specific run
gh run view <run-id>

# Check test results
gh run view <run-id> --log
```

**Common causes:**
1. Test failed on a platform
2. Build error on a target
3. Installer script issue

**Fix:**
1. Address the issue
2. Delete the tag: `git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0`
3. Fix the code
4. Re-tag and push

### Build fails for a platform

**Test locally:**
```bash
# Add target
rustup target add aarch64-unknown-linux-gnu

# Build for target
cargo build --target aarch64-unknown-linux-gnu

# Test with cargo-dist
cargo dist build --target aarch64-unknown-linux-gnu
```

### Installer test fails

**Reproduce locally:**
```bash
# Build locally
cargo dist build

# Test installer
sh target/distrib/bdp-installer.sh
```

---

## Comparison with Other Tools

### rustup
- ✅ Has `rustup self uninstall`
- ✅ Uses similar rename trick on Windows
- ✅ Tests installers before release

### deno
- ✅ Has `deno upgrade` built-in
- ❌ No self-uninstall (manual deletion)

### npm/node
- ✅ Can be uninstalled via package manager
- ❌ No self-uninstall command

### Our Approach
- ✅ Self-uninstall command
- ✅ Tested installers
- ✅ Works on all platforms
- ✅ Draft → Test → Publish flow

---

## Future Enhancements

Potential improvements:

1. **Auto-update:** `bdp upgrade` command
2. **Package managers:**
   - Homebrew formula (macOS)
   - Scoop/Chocolatey (Windows)
   - APT/YUM packages (Linux)
3. **Signature verification:** GPG-signed artifacts
4. **Telemetry:** Track install success rates (opt-in)
5. **Delta updates:** Only download changed files

---

## Files Reference

```
.github/workflows/
  └─ release.yml              # Main release workflow

dist-workspace.toml           # cargo-dist configuration

scripts/
  ├─ uninstall.sh            # Standalone Unix uninstall
  └─ uninstall.ps1           # Standalone Windows uninstall

crates/bdp-cli/src/commands/
  └─ uninstall.rs            # Self-uninstall implementation

INSTALL.md                    # User installation guide
CI_CD.md                      # Detailed CI/CD documentation
RELEASE_PROCESS.md            # This file
```

---

## Release Checklist

Before tagging:
- [ ] Version updated in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] All tests pass locally: `cargo test --all-features`
- [ ] CLI builds: `cargo build --release`
- [ ] No clippy warnings: `cargo clippy`

After tagging:
- [ ] Monitor GitHub Actions workflow
- [ ] Verify all 4 platform tests pass
- [ ] Confirm release published
- [ ] Test installation manually on at least one platform
- [ ] Announce release (if applicable)

---

## Quick Commands

```bash
# Check current version
cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="bdp-cli") | .version'

# List releases
gh release list

# View latest release
gh release view --web

# Local cargo-dist test
cargo dist build
cargo dist plan

# Regenerate workflow
dist generate
```

---

**For more details, see [CI_CD.md](CI_CD.md)**
