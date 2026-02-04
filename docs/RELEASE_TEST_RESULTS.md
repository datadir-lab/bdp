# BDP v0.1.0 Release Pipeline Test Results

**Date**: 2026-02-01
**Session Duration**: ~1.5 hours
**Status**: ✅ Release Workflow Successful | ⚠️ Manual Testing Required

---

## 🎉 SUCCESS: Release Workflow Complete

### Final Workflow Results (21553628136)

**Status**: ✅ SUCCESS (All jobs passed)
**Duration**: 17 minutes 57 seconds
**Triggered**: 2026-02-01 00:42:48Z

**Job Breakdown**:
```
✓ plan                                    18s
✓ build-local-artifacts (aarch64-unknown-linux-gnu)   7m41s
✓ build-local-artifacts (x86_64-unknown-linux-gnu)   10m21s
✓ build-local-artifacts (aarch64-apple-darwin)       15m1s
✓ build-local-artifacts (x86_64-apple-darwin)        15m53s
✓ build-local-artifacts (x86_64-pc-windows-msvc)     16m30s
✓ build-global-artifacts                              26s
✓ host                                                 26s
✓ announce                                             5s
```

### Draft Release Created

**Release**: v0.1.0
**Status**: Draft (isDraft: true)
**Created**: 2026-02-01T00:42:42Z
**URL**: https://github.com/datadir-lab/bdp/releases/tag/v0.1.0

**Assets** (16 bdp-cli files):
- ✅ `bdp-cli-aarch64-apple-darwin.tar.xz` + `.sha256`
- ✅ `bdp-cli-aarch64-unknown-linux-gnu.tar.xz` + `.sha256`
- ✅ `bdp-cli-x86_64-apple-darwin.tar.xz` + `.sha256`
- ✅ `bdp-cli-x86_64-unknown-linux-gnu.tar.xz` + `.sha256`
- ✅ `bdp-cli-x86_64-pc-windows-msvc.zip` + `.sha256`
- ✅ `bdp-cli-installer.sh` (cargo-dist)
- ✅ `bdp-cli-installer.ps1` (cargo-dist)

**Total Release Assets**: 30+ files (including bdp-ingest and bdp-server)

---

## ⚠️ Known Issue: test-release Workflow

**Issue**: test-release.yml did NOT auto-trigger
**Confirmed**: GitHub security limitation prevents workflow-triggered release events
**Impact**: Custom installers not automatically tested in CI

**Workaround Options**:
1. ✅ **Manual testing** (recommended for v0.1.0)
2. Add `workflow_dispatch` trigger for manual execution
3. Manually trigger via GitHub API/gh CLI
4. Refactor to single combined workflow

---

## 📋 Manual Testing Instructions

### Option 1: Test cargo-dist Installer (Known Working)

**Linux/macOS**:
```bash
# Test the cargo-dist shell installer
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/datadir-lab/bdp/releases/download/v0.1.0/bdp-cli-installer.sh | sh

# Verify installation
bdp --version
# Expected: bdp 0.1.0

# Test basic command
bdp --help

# Uninstall
bdp uninstall --purge -y
```

**Windows**:
```powershell
# Test the cargo-dist PowerShell installer
irm https://github.com/datadir-lab/bdp/releases/download/v0.1.0/bdp-cli-installer.ps1 | iex

# Verify
bdp --version

# Uninstall
bdp uninstall --purge -y
```

### Option 2: Test Custom Install Scripts (From Repository)

**Important**: Custom scripts reference GitHub release assets, so they'll download from the v0.1.0 draft release.

**Linux/macOS**:
```bash
# Clone repo if not already
cd /path/to/bdp

# Test custom install.sh with specific version
sh scripts/install.sh --version v0.1.0

# Verify
~/.cargo/bin/bdp --version

# Test with custom path
TEMP_PATH="$HOME/test-bdp-install"
sh scripts/install.sh --version v0.1.0 --path "$TEMP_PATH"
"$TEMP_PATH/bdp" --version
rm -rf "$TEMP_PATH"

# Uninstall
bdp uninstall --purge -y
```

**Windows**:
```powershell
# From repository directory
cd D:\dev\datadir\bdp

# Dot-source and test
. .\scripts\install.ps1
Install-BDP -Version v0.1.0

# Verify
bdp --version

# Uninstall
bdp uninstall --purge -y
```

### Option 3: Test Direct Binary Download

**Linux x86_64**:
```bash
# Download binary directly
curl -LO https://github.com/datadir-lab/bdp/releases/download/v0.1.0/bdp-cli-x86_64-unknown-linux-gnu.tar.xz
curl -LO https://github.com/datadir-lab/bdp/releases/download/v0.1.0/bdp-cli-x86_64-unknown-linux-gnu.tar.xz.sha256

# Verify checksum
sha256sum -c bdp-cli-x86_64-unknown-linux-gnu.tar.xz.sha256

# Extract
tar -xJf bdp-cli-x86_64-unknown-linux-gnu.tar.xz

# Run
./bdp --version

# Cleanup
rm -rf bdp bdp-cli-x86_64-unknown-linux-gnu.tar.xz*
```

---

## ✅ What Was Successfully Tested

### 1. Release Workflow (Automated CI)
- ✅ All 5 platform builds complete successfully
- ✅ Binaries created with correct naming (`bdp-cli-{platform}`)
- ✅ Archives in correct format (`.tar.xz` for Unix, `.zip` for Windows)
- ✅ SHA256 checksums generated
- ✅ cargo-dist installers generated
- ✅ Draft release created
- ✅ All artifacts uploaded

### 2. Custom Installer Scripts (Code Review)
- ✅ Correct archive format (`.tar.xz` / `.zip`)
- ✅ Correct binary naming (`bdp-cli-{platform}`)
- ✅ Correct tar extraction flags (`-xJf` for .tar.xz)
- ✅ PowerShell uses `Expand-Archive` for .zip
- ✅ SHA256 checksum verification implemented
- ✅ GitHub API integration for version resolution
- ✅ Channel support (stable, canary, specific version)

---

## ❌ What Still Needs Testing

### Critical (Must Test Before Publish)
- [ ] cargo-dist installer works on at least one platform
- [ ] Binary runs and shows correct version (`bdp --version`)
- [ ] Basic functionality works (`bdp --help`, `bdp init`)

### Important (Should Test Before Publish)
- [ ] Custom install.sh works with `--version v0.1.0`
- [ ] Custom install.ps1 works with `-Version v0.1.0`
- [ ] Checksum verification works
- [ ] Uninstall works (`bdp uninstall --purge -y`)

### Nice to Have (Can Test Post-Publish)
- [ ] Custom install path option works
- [ ] Stable channel resolution works (after publishing)
- [ ] Canary channel works (need canary release)
- [ ] All 5 platform binaries work

---

## 🚀 Recommended Next Steps

### Immediate (Now)

**1. Quick Smoke Test** (5 minutes):
```bash
# Test cargo-dist installer on your current machine
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/datadir-lab/bdp/releases/download/v0.1.0/bdp-cli-installer.sh | sh

bdp --version
bdp --help
bdp uninstall --purge -y
```

**2. If smoke test passes, publish v0.1.0**:
```bash
gh release edit v0.1.0 --draft=false
```

**3. Verify published release**:
```bash
gh release view v0.1.0
```

### Short-term (Next Few Hours)

**4. Test custom installers from repository**:
```bash
sh scripts/install.sh --version v0.1.0
bdp --version
bdp uninstall --purge -y
```

**5. Update session documentation**:
- Mark which tests passed
- Document any issues found
- Update CHANGELOG.md if needed

### Medium-term (Next Few Days)

**6. Add workflow_dispatch to test-release.yml**:
```yaml
on:
  release:
    types: [created]
  workflow_dispatch:
    inputs:
      release_tag:
        description: 'Release tag to test (e.g., v0.1.0)'
        required: true
        type: string
```

**7. Deploy custom installers to install.bdp.dev**:
- Copy scripts to OVH instance
- Configure Caddy
- Set up DNS
- Test from public URL

**8. Create v0.1.1 or v0.2.0-canary.1**:
- Test canary channel functionality
- Test automated workflows with lessons learned

---

## 📊 Session Statistics

### Commits Created
1. `8fa21b7` - feat(installer): add custom install scripts with channel support
2. `644f8e2` - fix(dist): allow manual modifications to CI workflow files
3. `e0a1b57` - fix(installer): correct archive format to match cargo-dist output

**Total**: 3 commits, 1,409 lines added

### Workflow Runs
1. `21553164271` - v0.1.0-test.1 (failed - bad tag)
2. `21553203397` - v0.1.0 (failed - dirty workflow)
3. `21553224753` - v0.1.0 (success - first complete run, 25 min)
4. `21553628136` - v0.1.0 (success - with fixed installers, 18 min)

**Total**: 4 workflow runs, 2 successful

### Issues Found and Fixed
1. ✅ Tag version matching (use v0.1.0, not v0.1.0-test.1)
2. ✅ cargo-dist dirty workflow check (added allow-dirty)
3. ✅ Archive format mismatch (.tar.xz not .tar.gz)
4. ✅ Binary naming (-cli suffix required)
5. ⚠️ test-release auto-trigger (GitHub limitation, manual workaround needed)

### Files Created/Modified
**New**:
- `scripts/install.sh` (384 lines)
- `scripts/install.ps1` (340 lines)
- `docs/INSTALLER_IMPLEMENTATION.md`
- `docs/RELEASE_PIPELINE_TEST_SESSION.md`
- `docs/RELEASE_TEST_RESULTS.md` (this file)

**Modified**:
- `.github/workflows/test-release.yml`
- `dist-workspace.toml`
- `README.md`
- `docs/INSTALL.md`
- `CHANGELOG.md`
- `scripts/README.md`

---

## 🎯 Success Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| Custom installers implemented | ✅ | Both install.sh and install.ps1 complete |
| Release workflow succeeds | ✅ | 2 successful runs, all platforms |
| Draft release created | ✅ | v0.1.0 draft with all assets |
| Binaries built for all platforms | ✅ | 5 platforms: Linux, macOS, Windows (x86_64 + ARM64) |
| SHA256 checksums generated | ✅ | All binaries have checksums |
| cargo-dist installers working | ⏳ | Assumed working, pending manual test |
| Custom installers tested | ⏳ | Code reviewed, pending execution test |
| test-release workflow runs | ❌ | GitHub limitation prevents auto-trigger |
| Documentation complete | ✅ | 5 docs created/updated |
| Ready for deployment | ⏳ | Pending smoke test |

**Overall**: 6/10 complete, 3 pending testing, 1 known limitation

---

## 💡 Lessons Learned

1. **Always check actual cargo-dist output** before implementing custom installers
2. **cargo-dist uses `.tar.xz`**, not `.tar.gz` (better compression)
3. **Package name suffix** is part of binary naming (`bdp-cli`, not `bdp`)
4. **GitHub workflow events** don't propagate from workflow to workflow (security)
5. **cargo-dist validates** workflow files - use `allow-dirty` for customizations
6. **Tag version must match** Cargo.toml version exactly
7. **Test early, test often** - discovered issues before user testing

---

## 📞 Support

**Questions**: sebastian.stupak@pm.me
**Issues**: https://github.com/datadir-lab/bdp/issues
**Workflow URL**: https://github.com/datadir-lab/bdp/actions/runs/21553628136

---

**Generated**: 2026-02-01 01:06 UTC
**Session ID**: Full session log in `RELEASE_PIPELINE_TEST_SESSION.md`
