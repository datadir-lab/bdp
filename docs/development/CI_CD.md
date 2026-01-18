# CI/CD Pipeline Documentation

This document explains the CI/CD setup for BDP, including the release process, testing, and deployment.

## Overview

BDP uses **cargo-dist** for cross-platform binary distribution and **GitHub Actions** for CI/CD. The release pipeline follows a **draft → test → publish** workflow to ensure reliability.

## Release Workflow

### 1. Trigger

The release pipeline is triggered when you push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

**Tag Format:**
- `v0.1.0` - Standard release
- `v0.1.0-beta.1` - Pre-release (creates draft release)

### 2. Build Phase

The workflow automatically:
1. **Plans the release** - Determines what to build
2. **Builds artifacts** for all platforms:
   - Linux: x86_64, ARM64
   - macOS: x86_64 (Intel), ARM64 (Apple Silicon)
   - Windows: x86_64
3. **Generates installers**:
   - Shell script for Linux/macOS
   - PowerShell script for Windows
4. **Creates checksums** and generates release notes

### 3. Draft Release Phase

After building, the workflow:
1. Creates a **draft GitHub release**
2. Uploads all artifacts to the draft release
3. Generates release notes from changelogs

**Important:** The release remains in draft mode during testing!

### 4. Testing Phase

Before publishing, the workflow tests the installers on all platforms:

| Platform | OS Version | Tests |
|----------|-----------|-------|
| Linux | Ubuntu 22.04 | Install, Verify, Upgrade, Uninstall |
| macOS Intel | macOS 13 | Install, Verify, Upgrade, Uninstall |
| macOS ARM | macOS 14 | Install, Verify, Upgrade, Uninstall |
| Windows | Windows 2022 | Install, Verify, Upgrade, Uninstall |

**Test Sequence:**
1. **Fresh Install**: Download and run the installer script
2. **Verification**: Run `bdp --version` to confirm installation
3. **Upgrade Test**: Re-run installer (tests upgrade path)
4. **Uninstall**: Run `bdp uninstall --purge -y`
5. **Verify Uninstall**: Confirm binary is removed

### 5. Publish Phase

**Only after all tests pass**, the workflow:
1. Edits the draft release to make it public
2. Announces the release (if configured)

If any test fails, the release remains in draft mode and is NOT published.

## Architecture

### Workflow Jobs

```
plan
  ↓
build-local-artifacts (matrix: all platforms in parallel)
  ↓
build-global-artifacts (checksums, etc.)
  ↓
host (create draft release)
  ↓
test-installers (matrix: all platforms in parallel)
  ↓
publish-release (only if tests pass)
  ↓
announce
```

### Files

- **`.github/workflows/release.yml`** - Main release workflow
- **`dist-workspace.toml`** - cargo-dist configuration
- **`scripts/uninstall.sh`** - Standalone Unix uninstall script
- **`scripts/uninstall.ps1`** - Standalone Windows uninstall script

## Making a Release

### 1. Update Version

Update version in `Cargo.toml`:

```toml
[workspace.package]
version = "0.2.0"
```

### 2. Update Changelog

Add release notes to `CHANGELOG.md`:

```markdown
## [0.2.0] - 2026-01-16

### Added
- New feature X
- New feature Y

### Fixed
- Bug A
- Bug B
```

### 3. Commit and Tag

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.2.0"
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

### 4. Monitor Workflow

1. Go to https://github.com/datadir-lab/bdp/actions
2. Watch the "Release" workflow
3. Check that all tests pass
4. Verify the release is published

### 5. Verify Installation

Test the installation on your local machine:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

## Configuration

### cargo-dist Settings

Edit `dist-workspace.toml`:

```toml
[dist]
# cargo-dist version to use
cargo-dist-version = "0.30.3"

# CI backend
ci = "github"

# Installer types
installers = ["shell", "powershell"]

# Target platforms
targets = [
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc"
]

# Where to install binaries
install-path = "CARGO_HOME"
```

After changing configuration, regenerate the workflow:

```bash
dist generate
```

### Adding New Platforms

1. Add target to `targets` in `dist-workspace.toml`
2. Regenerate workflow: `dist generate`
3. Add test matrix entry in `.github/workflows/release.yml`

## Troubleshooting

### Test Failures

If install tests fail:

1. Check the GitHub Actions logs
2. The release stays in draft mode
3. Fix the issue and delete the tag
4. Retag and push again

### Build Failures

If builds fail:

1. Check Cargo.toml dependencies
2. Ensure all targets can compile
3. Test locally:
   ```bash
   cargo build --target x86_64-unknown-linux-gnu
   ```

### Draft Release Not Publishing

Check:
1. All tests passed (green checkmarks)
2. The `publish-release` job ran
3. GitHub Actions has write permissions

## Self-Uninstall Feature

BDP includes a `bdp uninstall` command that removes itself:

```bash
# Interactive uninstall
bdp uninstall

# Non-interactive uninstall
bdp uninstall -y

# Uninstall with cache removal
bdp uninstall --purge -y
```

**Implementation:**

- **Unix/Linux/macOS**: Spawns a background shell that waits 1s and removes the binary
  - Works reliably because Unix allows unlinking running executables

- **Windows**: More complex due to file locking
  1. First attempts to **rename** the executable (works even while running!)
  2. Creates a batch script to delete the renamed file after 2 seconds
  3. If rename fails (rare), provides manual instructions

**Why this approach?**

Windows locks files that are in use, so you can't directly delete a running executable. The rename trick works because:
- Renaming a file doesn't require it to be closed
- Once renamed, the original path is free
- The batch script then deletes the renamed file after the process exits

This is the same technique used by professional installers like rustup.

This provides a better user experience than requiring manual deletion or external scripts.

## Security Considerations

### Install Script Security

**The curl | sh pattern has known security concerns:**

1. **Mitigation:** Scripts served over HTTPS
2. **Mitigation:** cargo-dist generates scripts with integrity checks
3. **Mitigation:** Users can download and inspect before running

**Recommendations:**
```bash
# Safer: Download first, inspect, then run
curl -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh > installer.sh
less installer.sh
sh installer.sh
```

### Release Artifacts

- All artifacts have SHA-256 checksums
- Releases are signed by GitHub Actions
- Artifacts are immutable once published

## Best Practices

### 1. Always Test Locally First

Before releasing:
```bash
cargo build --release
cargo test --all-features
cargo clippy
```

### 2. Use Semantic Versioning

- **Major** (1.0.0): Breaking changes
- **Minor** (0.1.0): New features (backward compatible)
- **Patch** (0.0.1): Bug fixes

### 3. Write Detailed Release Notes

Include:
- What's new
- What's fixed
- Breaking changes (if any)
- Migration guide (if needed)

### 4. Monitor First Release

For the first few releases:
1. Watch the entire workflow
2. Test installation manually on multiple platforms
3. Check for user-reported issues

## References

### cargo-dist Documentation
- Homepage: https://axodotdev.github.io/cargo-dist/
- GitHub: https://github.com/axodotdev/cargo-dist
- Book: https://axodotdev.github.io/cargo-dist/book/

### GitHub Actions
- Documentation: https://docs.github.com/en/actions
- Release management: https://docs.github.com/en/repositories/releasing-projects-on-github

### Related Articles
- [Setting up effective CI/CD for Rust projects](https://www.shuttle.dev/blog/2025/01/23/setup-rust-ci-cd)
- [Optimizing CI/CD pipelines in Rust projects](https://blog.logrocket.com/optimizing-ci-cd-pipelines-rust-projects/)
- [Automating Multi-Platform Releases with GitHub Actions](https://itsfuad.medium.com/automating-multi-platform-releases-with-github-actions-f74de82c76e2)

## Changelog Integration

cargo-dist can automatically generate release notes from:
- CHANGELOG.md
- Git commits
- GitHub PRs

Configure in `dist-workspace.toml`:

```toml
[dist]
# Release note generation
changelog = "CHANGELOG.md"
```

## Future Enhancements

Potential improvements:
1. **Homebrew formula** generation
2. **Debian/RPM packages**
3. **Docker images**
4. **Artifact signing** with GPG
5. **Auto-update** functionality in CLI
6. **Metrics collection** on install success rates

---

## Quick Reference

### Release Checklist

- [ ] Update version in Cargo.toml
- [ ] Update CHANGELOG.md
- [ ] Commit changes
- [ ] Tag release: `git tag v0.x.y`
- [ ] Push tag: `git push origin v0.x.y`
- [ ] Monitor GitHub Actions workflow
- [ ] Verify release published
- [ ] Test installation script

### Commands

```bash
# Local testing
cargo dist build
cargo dist plan

# Regenerate workflow
dist generate

# Create release
git tag v0.1.0 && git push origin v0.1.0

# Test install (after release)
curl -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```
