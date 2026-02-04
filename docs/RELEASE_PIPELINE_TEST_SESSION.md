# Release Pipeline Test Session

**Date**: 2026-02-01
**Session Duration**: ~1.5 hours
**Status**: In Progress - Monitoring second release workflow

## Objective

Test the complete BDP CLI release pipeline including:
1. Custom installer scripts (install.sh, install.ps1)
2. cargo-dist release workflow
3. Test-release workflow for installer validation
4. Draft → Test → Publish flow

## Session Timeline

### Phase 1: Initial Push and Tag (00:08 - 00:12)

**Actions**:
1. Pushed custom installer implementation (commit 8fa21b7)
2. Created test tag `v0.1.0-test.1` → Failed
   - **Error**: cargo-dist didn't recognize tag format (expected v0.1.0, not v0.1.0-test.1)
3. Deleted bad tag, created `v0.1.0` tag
4. Release workflow started (21553164271) → Failed
   - **Error**: cargo-dist detected manually modified release.yml (missing `allow-dirty`)

### Phase 2: Fix cargo-dist Configuration (00:12 - 00:13)

**Actions**:
1. Added `allow-dirty = ["ci"]` to `dist-workspace.toml`
2. Committed fix (644f8e2)
3. Deleted and recreated v0.1.0 tag
4. Release workflow started (21553224753) → **SUCCESS!**

**Results**:
- ✅ All 5 platform builds completed successfully
- ✅ Draft release created with all artifacts
- ✅ Build times:
  - plan: 18s
  - aarch64-unknown-linux-gnu: 7m22s
  - x86_64-unknown-linux-gnu: 10m14s
  - aarch64-apple-darwin: 12m8s
  - x86_64-pc-windows-msvc: 17m11s
  - x86_64-apple-darwin: 20m21s (slowest)
  - build-global-artifacts: 25s
  - host: 33s
  - announce: 4s
- **Total workflow time**: ~25 minutes

### Phase 3: Discovery - Archive Format Mismatch (00:40)

**Discovery**:
- Checked release assets → Found `.tar.xz` for Unix, `.zip` for Windows
- Custom installers expected `.tar.gz`
- Binary naming: `bdp-cli-{platform}.tar.xz` (has `-cli` suffix)

**Root Cause**:
- cargo-dist uses `.tar.xz` compression (not `.tar.gz`)
- cargo-dist adds package name suffix (`-cli`, `-ingest`, `-server`)
- Custom installers were written based on assumption, not actual cargo-dist output

**Actions**:
1. Updated `install.sh`:
   - Changed archive from `.tar.gz` to `.tar.xz`
   - Added `-cli` suffix to binary name
   - Changed tar flags from `-xzf` to `-xJf` (xz compression)
2. Updated `install.ps1`:
   - Changed archive from `.tar.gz` to `.zip`
   - Added `-cli` suffix to binary name
   - Replaced tar extraction with `Expand-Archive` cmdlet
3. Committed fix (e0a1b57)

### Phase 4: Second Release Attempt (00:42 - In Progress)

**Actions**:
1. Deleted first draft release and tag
2. Recreated v0.1.0 tag with fixed installers
3. Release workflow started (21553628136) → **Building...**

**Current Status** (as of last check):
- 1 of 6 jobs completed (plan)
- Builds in progress for all 5 platforms
- ETA: ~20 minutes

### Phase 5: test-release Workflow Issue

**Discovery**:
- test-release.yml workflow did NOT auto-trigger when draft release was created
- **Root Cause**: GitHub security limitation
  - Workflows triggered by other workflows don't automatically trigger subsequent workflow events
  - Prevents infinite workflow loops
  - `release` event from release.yml doesn't propagate to test-release.yml

**Impact**:
- Cannot automatically test custom installers via CI
- Must either:
  1. Manually trigger test-release workflow (if workflow_dispatch added)
  2. Manually test installers from draft release
  3. Publish release without automated testing
  4. Refactor workflow strategy

**Current Workaround**:
- Manual testing of installers from draft release

## Artifacts and Assets

### Draft Release v0.1.0 (First Attempt)

**Binaries** (5 platforms):
- `bdp-cli-aarch64-apple-darwin.tar.xz` + `.sha256`
- `bdp-cli-aarch64-unknown-linux-gnu.tar.xz` + `.sha256`
- `bdp-cli-x86_64-apple-darwin.tar.xz` + `.sha256`
- `bdp-cli-x86_64-unknown-linux-gnu.tar.xz` + `.sha256`
- `bdp-cli-x86_64-pc-windows-msvc.zip` + `.sha256`

**Installers** (cargo-dist):
- `bdp-cli-installer.sh`
- `bdp-cli-installer.ps1`

**Additional Packages**:
- bdp-ingest binaries and installers (3 packages)
- bdp-server binaries and installers (3 packages)

**Total Assets**: ~30 files

## Commits Created

1. **8fa21b7** - `feat(installer): add custom install scripts with channel support`
   - Added install.sh (384 lines)
   - Added install.ps1 (340 lines)
   - Enhanced test-release.yml with custom installer testing
   - Updated documentation (README, INSTALL.md, CHANGELOG, etc.)
   - **Files changed**: 8 files, +1,396 insertions, -11 deletions

2. **644f8e2** - `fix(dist): allow manual modifications to CI workflow files`
   - Added `allow-dirty = ["ci"]` to dist-workspace.toml
   - **Files changed**: 1 file, +2 insertions

3. **e0a1b57** - `fix(installer): correct archive format to match cargo-dist output`
   - Fixed install.sh to use `.tar.xz` and `-xJf`
   - Fixed install.ps1 to use `.zip` and `Expand-Archive`
   - Added `-cli` suffix to match cargo-dist naming
   - **Files changed**: 2 files, +11 insertions, -12 deletions

## Issues Discovered

### 1. cargo-dist Tag Version Matching (Resolved)
- **Issue**: cargo-dist requires exact version match between tag and Cargo.toml
- **Error**: `v0.1.0-test.1` doesn't match `0.1.0` in Cargo.toml
- **Fix**: Use `v0.1.0` tag
- **Status**: ✅ Resolved

### 2. cargo-dist Workflow Modification (Resolved)
- **Issue**: cargo-dist validates workflow files and rejects manual modifications
- **Error**: `.github/workflows/release.yml has out of date contents`
- **Fix**: Add `allow-dirty = ["ci"]` to dist-workspace.toml
- **Status**: ✅ Resolved

### 3. Archive Format Mismatch (Resolved)
- **Issue**: Custom installers assumed `.tar.gz`, cargo-dist produces `.tar.xz`/`.zip`
- **Impact**: Installers would fail to download/extract binaries
- **Fix**: Update installers to use correct formats and compression flags
- **Status**: ✅ Resolved

### 4. Binary Naming Convention (Resolved)
- **Issue**: Custom installers assumed `bdp-{platform}`, cargo-dist produces `bdp-cli-{platform}`
- **Impact**: Installers would fail to download binaries (404)
- **Fix**: Add `-cli` suffix to archive names
- **Status**: ✅ Resolved

### 5. test-release Workflow Not Triggering (Open)
- **Issue**: test-release.yml doesn't auto-trigger when release.yml creates draft release
- **Root Cause**: GitHub security limitation (workflow-triggered events don't propagate)
- **Impact**: Cannot automatically test custom installers in CI
- **Potential Solutions**:
  1. Add `workflow_dispatch` trigger for manual execution
  2. Combine workflows into single workflow file
  3. Use repository_dispatch event with PAT
  4. Manual testing before publish
- **Status**: ⚠️ Open - Requires decision

## Lessons Learned

### 1. Always Check Actual cargo-dist Output
- **Mistake**: Assumed `.tar.gz` format based on common practice
- **Reality**: cargo-dist uses `.tar.xz` for better compression
- **Lesson**: Check actual release assets before writing custom installers

### 2. cargo-dist Binary Naming
- **Mistake**: Assumed `{binary}-{platform}` format
- **Reality**: `{package-name}-{platform}` format (bdp-cli, not bdp)
- **Lesson**: cargo-dist uses package name from Cargo.toml, not binary name

### 3. GitHub Workflow Event Propagation
- **Mistake**: Assumed release event from one workflow triggers another workflow
- **Reality**: GitHub blocks workflow-triggered events for security
- **Lesson**: Test workflow triggers thoroughly; consider workflow_dispatch for manual fallback

### 4. cargo-dist Validation
- **Mistake**: Manually modified auto-generated workflow file
- **Reality**: cargo-dist validates workflow file integrity
- **Lesson**: Use `allow-dirty` config when manual modifications are needed

## Next Steps

### Immediate (Once Current Workflow Completes)

1. **Verify Draft Release Assets**:
   - Check all binaries are present
   - Verify SHA256 checksums
   - Confirm installers are included

2. **Manual Installer Testing**:
   - Test install.sh on Linux (via WSL or Docker)
   - Test install.ps1 on Windows
   - Test cargo-dist installers for comparison
   - Verify version output matches v0.1.0

3. **Decide on test-release Strategy**:
   - Option A: Add workflow_dispatch and trigger manually
   - Option B: Manually test and publish
   - Option C: Refactor to single workflow
   - Option D: Accept manual testing for first release

4. **Publish Release**:
   - If tests pass, publish v0.1.0 release
   - Update release notes if needed
   - Announce first release

### Short-term (Next Few Days)

1. **Add workflow_dispatch to test-release.yml**:
   ```yaml
   on:
     release:
       types: [created]
     workflow_dispatch:
       inputs:
         release_tag:
           description: 'Release tag to test'
           required: true
   ```

2. **Test Custom Installers from install.bdp.dev**:
   - Deploy scripts to OVH instance
   - Configure Caddy
   - Set up DNS
   - Test from public URL

3. **Create v0.1.1 or v0.2.0-canary.1**:
   - Test canary channel functionality
   - Validate pre-release detection

### Long-term (Post-Launch)

1. **Enhanced Workflow Integration**:
   - Investigate repository_dispatch with PAT
   - Consider consolidating workflows
   - Add workflow_run trigger if supported

2. **Installer Improvements**:
   - Add GPG signature verification
   - Implement auto-update check
   - Add telemetry (opt-in)

3. **Package Manager Distribution**:
   - Homebrew formula
   - Scoop manifest
   - apt/yum repositories

## Monitoring Commands

```bash
# Watch current release workflow
gh run watch 21553628136

# Check workflow status
gh run view 21553628136

# List recent workflow runs
gh run list --limit 10

# View release
gh release view v0.1.0

# Check release assets
gh release view v0.1.0 --json assets --jq '.assets[].name'

# Monitor in real-time (background task b171ea6)
# Output file: C:\Users\sebas\AppData\Local\Temp\claude\D--dev-datadir-bdp\tasks\b171ea6.output
```

## Files for Reference

**Implementation**:
- `scripts/install.sh` - Unix custom installer
- `scripts/install.ps1` - Windows custom installer
- `.github/workflows/release.yml` - cargo-dist release workflow
- `.github/workflows/test-release.yml` - Installer testing workflow
- `dist-workspace.toml` - cargo-dist configuration

**Documentation**:
- `docs/INSTALLER_IMPLEMENTATION.md` - Complete implementation guide
- `docs/INSTALL.md` - User installation guide
- `CHANGELOG.md` - Feature changelog
- `README.md` - Quick start
- `scripts/README.md` - Scripts documentation

**Session Logs**:
- Background monitor: `C:\Users\sebas\AppData\Local\Temp\claude\D--dev-datadir-bdp\tasks\b171ea6.output`
- Previous workflow watch: `C:\Users\sebas\AppData\Local\Temp\claude\D--dev-datadir-bdp\tasks\b94d812.output`

## Summary

### Successes ✅
- Custom installer scripts implemented and committed
- First release workflow completed successfully (25 minutes)
- All 5 platform builds working
- Draft release created with all artifacts
- Discovered and fixed 4 critical issues before user testing
- Comprehensive documentation created

### Challenges ⚠️
- test-release workflow auto-trigger limitation
- Multiple tag recreations needed (3 attempts)
- Archive format mismatch required code fixes

### Remaining Work 🔄
- Current release workflow building (ETA ~15 minutes)
- Manual installer testing needed
- test-release workflow strategy decision
- Release publication

---

**Last Updated**: 2026-02-01 00:50 UTC
**Next Update**: When workflow completes or issues arise
