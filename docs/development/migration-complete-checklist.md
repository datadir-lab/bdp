# Migration Completion Checklist and Status Report

**Migration**: Just → xtask (Pure Rust Task Runner)
**Date Started**: 2026-02-09
**Current Status**: 85% Complete (Phases 1-7 Complete, Phase 8 Remaining)
**Last Updated**: 2026-02-09

---

## Executive Summary

The BDP project is migrating from `just` (external tool) to `xtask` (pure Rust, integrated with Cargo). This migration provides better type safety, IDE support, cross-platform compatibility, and eliminates external dependencies.

**Progress**: 7 of 8 phases complete (85%)

### What's Done
- ✅ All 103 commands converted from justfile to 16 xtask modules
- ✅ xtask compiles successfully with only minor warnings
- ✅ All modules integrated in main.rs with proper routing
- ✅ CI/CD workflows updated to use `cargo xtask`
- ✅ Critical documentation updated (CLAUDE.md, README.md)
- ✅ xtask guides created and documentation index updated
- ✅ Cargo-make configuration added as alternative (Makefile.toml)

### What Remains
- ⏳ **Phase 8: Testing & Verification** (commands need real-world testing)
- ⏳ Post-migration cleanup (delete justfile after full verification)
- ⏳ Update remaining 42 documentation files with just references

---

## Migration Status Overview

| Phase | Status | Progress | Notes |
|-------|--------|----------|-------|
| 1. Preparation & Design | ✅ Complete | 100% | Infrastructure designed, 16 modules identified |
| 2. Core Infrastructure | ✅ Complete | 100% | utils.rs, cross-platform support, colored output |
| 3. Convert Task Modules | ✅ Complete | 100% | All 103 commands converted across 16 modules |
| 4. Update Main Entry Point | ✅ Complete | 100% | All modules registered and routed in main.rs |
| 5. Update CI/CD | ✅ Complete | 100% | .github/workflows/ci.yml uses cargo xtask |
| 6. Update Documentation | ✅ Complete | 95% | Core docs updated, 42 files remain with just refs |
| 7. Root Cleanup | ✅ Complete | 100% | Makefile.toml added, justfile kept for rollback |
| 8. Testing & Verification | ⏳ **IN PROGRESS** | 0% | **NEEDS TESTING** |
| **TOTAL** | **85%** | **85%** | Ready for comprehensive testing |

---

## Detailed Verification Checklist

### Phase 1-7: Completed Work ✅

#### Module Conversion (103 Commands)
- [x] `db.rs` - 12 database commands (up, down, migrate, shell, etc.)
- [x] `build.rs` - 4 build commands (workspace, release, all, docker)
- [x] `test.rs` - 11 testing commands (all, unit, integration, coverage, CLI)
- [x] `dev.rs` - 10 development commands (server, web, fmt, lint, watch)
- [x] `docker.rs` - 7 Docker commands (up, down, logs, migrate)
- [x] `sqlx.rs` - 3 SQLx commands (prepare, check, clean)
- [x] `minio.rs` - 3 MinIO commands (up, down, logs)
- [x] `ingest.rs` - 3 ingestion commands (uniprot, ncbi, all)
- [x] `ci.rs` - 2 CI/CD commands (all, offline)
- [x] `clean.rs` - 4 cleanup commands (workspace, all, stop services)
- [x] `docs.rs` - 5 documentation commands (cargo, web, CLI docs)
- [x] `setup.rs` - 4 setup commands (all, deps, env, verify)
- [x] `infra.rs` - 8 infrastructure commands (init, plan, apply, ssh)
- [x] `util.rs` - 15 utility commands (info, health, audit logs)
- [x] `e2e.rs` - 6 E2E testing commands (ci, real, download, debug)
- [x] `release.rs` - 6 release commands (patch, minor, major, bump)

#### Infrastructure
- [x] xtask compiles successfully (`cargo build -p xtask`)
- [x] All 16 modules integrated in main.rs
- [x] Cross-platform support (Windows PowerShell + Unix Bash)
- [x] Colored output with progress indicators
- [x] Proper error handling with anyhow::Result
- [x] Legacy command compatibility (generate-cli-docs)

#### CI/CD
- [x] `.github/workflows/ci.yml` updated
  - [x] `cargo xtask lint` (line 52)
  - [x] `cargo xtask test` (line 98)
  - [x] `cargo xtask sqlx check` (line 147)
  - [x] `cargo run --package xtask -- generate-cli-docs` (line 120)
- [x] No `extractions/setup-just@v3` dependencies
- [x] All CI jobs reference xtask

#### Documentation
- [x] `CLAUDE.md` - Updated with xtask commands (line 134-150)
- [x] `README.md` - Contains xtask references
- [x] `docs/INDEX.md` - Updated with xtask guide link (line 68)
- [x] `docs/development/xtask-guide.md` - Comprehensive guide created (303 lines)
- [x] `docs/development/xtask-command-reference.md` - Command reference (198 lines)
- [x] `docs/development/xtask-migration.md` - Migration guide created
- [x] `docs/development/cargo-make-migration.md` - This migration doc

#### Files
- [x] All xtask module files created (18 files in xtask/src/)
- [x] `Makefile.toml` - cargo-make alternative added
- [x] `justfile` - Kept for rollback safety (will delete after testing)

---

## Phase 8: Testing & Verification Tasks

This is the **CRITICAL PHASE** - all commands must be tested before considering the migration complete.

### Testing Strategy

Test commands in order of risk and dependency:
1. **Low-risk utility commands** (no side effects)
2. **Database commands** (with test database)
3. **Development commands** (may modify files)
4. **Build commands** (may take time)
5. **CI simulation** (comprehensive check)

### Test Environment Setup

```bash
# 1. Ensure xtask is built
cargo build -p xtask

# 2. Set up test database (for db commands)
docker compose up -d postgres-test

# 3. Set up CLI test directory (for CLI tests)
mkdir -p D:\dev\datadir\bdp-example
```

### Testing Checklist

#### A. Utility Commands (Low Risk) ⏳

Test these first - they have no side effects:

```bash
# Info commands
- [ ] cargo xtask util info
- [ ] cargo xtask util health
- [ ] cargo xtask util logs

# Verification
- [ ] cargo xtask setup verify

# Expected: Display system info, no errors
```

#### B. Database Commands (Medium Risk) ⏳

Test with test database to avoid affecting dev data:

```bash
# Start/stop database
- [ ] cargo xtask db up
- [ ] cargo xtask db down
- [ ] cargo xtask db test-up
- [ ] cargo xtask db test-down

# Migrations (use test DB)
- [ ] cargo xtask db migrate
- [ ] cargo xtask db migrate-revert
- [ ] cargo xtask db shell (verify connection)

# Logs and status
- [ ] cargo xtask db logs

# Expected: Database operations work, containers start/stop correctly
```

#### C. SQLx Commands ⏳

```bash
- [ ] cargo xtask sqlx check
- [ ] cargo xtask sqlx prepare
- [ ] cargo xtask sqlx clean

# Expected: SQLx metadata operations work
```

#### D. Development Commands (Medium Risk) ⏳

```bash
# Code quality
- [ ] cargo xtask dev fmt
- [ ] cargo xtask dev lint
- [ ] cargo xtask dev fix

# Development servers (run in separate terminals, then Ctrl+C)
- [ ] cargo xtask dev server (start backend, verify http://localhost:8000/health)
- [ ] cargo xtask dev web (start frontend, verify http://localhost:3000)

# Watch mode (test briefly then cancel)
- [ ] cargo xtask dev watch

# Expected: Code formatted, linting passes, servers start correctly
```

#### E. Build Commands (High Time Cost) ⏳

```bash
# Rust builds
- [ ] cargo xtask build workspace
- [ ] cargo xtask build release (SLOW - ~5-10 min)

# Web build
- [ ] cargo xtask dev web-build

# Docker builds (VERY SLOW - skip unless critical)
- [ ] cargo xtask build docker (OPTIONAL - takes 15-30 min)

# Expected: Builds complete without errors
```

#### F. Testing Commands ⏳

```bash
# Unit tests (fast)
- [ ] cargo xtask test unit

# Integration tests (requires test DB)
- [ ] cargo xtask test integration

# All tests
- [ ] cargo xtask test all

# CLI testing
- [ ] cargo xtask test cli-setup
- [ ] cargo xtask test cli "init"
- [ ] cargo xtask test cli-clean

# E2E tests (if applicable)
- [ ] cargo xtask e2e ci

# Expected: Tests pass (same results as with just)
```

#### G. Documentation Commands ⏳

```bash
- [ ] cargo xtask docs cargo
- [ ] cargo xtask docs cli
- [ ] cargo xtask docs web

# Expected: Documentation generated successfully
```

#### H. Docker Commands ⏳

```bash
- [ ] cargo xtask docker up
- [ ] cargo xtask docker down
- [ ] cargo xtask docker logs
- [ ] cargo xtask docker ps

# MinIO
- [ ] cargo xtask minio up
- [ ] cargo xtask minio down
- [ ] cargo xtask minio logs

# Expected: Containers start/stop, logs displayed
```

#### I. Cleanup Commands ⏳

```bash
- [ ] cargo xtask clean workspace
- [ ] cargo xtask clean stop
- [ ] cargo xtask clean all (WARNING: removes node_modules, target/)

# Expected: Build artifacts cleaned
```

#### J. Infrastructure Commands (If Applicable) ⏳

```bash
- [ ] cargo xtask infra init
- [ ] cargo xtask infra plan
# DO NOT RUN: cargo xtask infra apply (production impact)

# Expected: Terraform operations work
```

#### K. CI Simulation ⏳

```bash
# Full CI check
- [ ] cargo xtask ci all

# Offline mode
- [ ] cargo xtask ci offline

# Expected: All CI checks pass (same as GitHub Actions)
```

#### L. Cross-Platform Testing (Windows) ⏳

All commands should work on Windows with PowerShell:

```powershell
# Key commands to test on Windows
- [ ] cargo xtask db up (PowerShell script)
- [ ] cargo xtask dev server (PowerShell script)
- [ ] cargo xtask test all (PowerShell environment)

# Expected: Cross-platform conditionals work correctly
```

#### M. Command Compatibility Testing ⏳

Verify backward compatibility (if any scripts call just):

```bash
# Search for any remaining just calls in scripts
- [ ] grep -r "just " .github/ scripts/ docker/ (should find none in critical paths)

# Test that common workflows still work
- [ ] Complete new developer setup workflow
- [ ] Complete CI/CD workflow end-to-end

# Expected: No broken workflows
```

---

## Success Criteria

Before marking migration as **COMPLETE**, verify:

### Functionality
- [ ] All 103 commands execute successfully
- [ ] No regression in behavior (commands work identically to just versions)
- [ ] Cross-platform support verified (Windows PowerShell works)
- [ ] Error handling works (proper error messages, non-zero exits)

### CI/CD
- [ ] GitHub Actions CI pipeline passes completely
- [ ] All jobs using xtask complete successfully
- [ ] No timeout or performance issues

### Documentation
- [ ] All critical docs updated with xtask syntax
- [ ] Command reference accurate
- [ ] Examples tested and work

### Code Quality
- [ ] `cargo build -p xtask` completes without errors
- [ ] `cargo clippy -p xtask` passes (only minor warnings acceptable)
- [ ] `cargo fmt -p xtask --check` passes

### Performance
- [ ] No significant slowdown compared to just commands
- [ ] Acceptable startup time (< 1s for most commands)

---

## Known Issues

### Current Warnings (Non-Critical)

From `cargo build -p xtask`:

```
warning: unused import: `std::process::Command`
  --> xtask\src\test.rs:5:5

warning: function `command_exists` is never used
  --> xtask\src\utils.rs:99:8
```

**Action**: Clean up after testing (remove unused imports/functions).

### Documentation Gaps

42 documentation files still contain `just` command references:

**Files with just/cargo xtask references** (953 occurrences):
- `docs/development/just-references-summary.md` (63)
- `docs/development/just-guide.md` (63)
- `docs/SETUP.md` (72)
- `docs/development/QUICK_START_SQLX.md` (32)
- `docs/development/testing.md` (19)
- And 37 more files...

**Action**: Bulk update after full verification (Phase 9 post-migration task).

---

## Rollback Plan

If critical issues are discovered during testing:

### Immediate Rollback (Git)

```bash
# Restore justfile
git checkout main -- justfile

# Restore CI workflow
git checkout main -- .github/workflows/ci.yml

# Restore core documentation
git checkout main -- CLAUDE.md README.md docs/INDEX.md

# Restart development
cargo xtask dev server
```

### Full Reset

```bash
# Nuclear option: reset all changes
git reset --hard HEAD~10  # Adjust number based on commits
git clean -fdx
```

### Partial Rollback (Keep Both)

If xtask has issues but cargo-make works:

```bash
# Use cargo-make instead
cargo install cargo-make
cargo make dev

# Keep justfile as primary, xtask as experimental
# (No changes needed - justfile is still present)
```

---

## Post-Migration Tasks

After successful testing and verification:

### Phase 9: Cleanup ⏳

```bash
# 1. Remove justfile
- [ ] git rm justfile
- [ ] git commit -m "chore: remove justfile after xtask migration complete"

# 2. Clean up xtask warnings
- [ ] Remove unused imports in test.rs
- [ ] Remove unused command_exists function in utils.rs
- [ ] Run cargo clippy -p xtask --fix

# 3. Final formatting
- [ ] cargo fmt --all
```

### Phase 10: Bulk Documentation Update ⏳

Update the 42 remaining documentation files:

```bash
# Find all just command references
- [ ] grep -r "just " docs/ --include="*.md" | grep -v "adjustment" | grep -v "justfile"

# Systematic replacement strategy:
# - just <cmd> → cargo xtask <module> <cmd>
# - Update all examples
# - Update all command tables
# - Verify links still work

# Priority files:
- [ ] docs/SETUP.md (72 occurrences)
- [ ] docs/development/just-guide.md (63 occurrences)
- [ ] docs/development/just-references-summary.md (63 occurrences)
- [ ] docs/development/QUICK_START_SQLX.md (32 occurrences)
- [ ] docs/development/testing.md (19 occurrences)
- [ ] docs/TESTING.md (20 occurrences)
```

**Note**: Can use sed/awk for bulk replacement or manual updates for accuracy.

### Phase 11: Announcement ⏳

```bash
# 1. Create announcement
- [ ] Write CHANGELOG entry
- [ ] Update version if needed (minor bump for breaking change)

# 2. Notify team
- [ ] Send email/Slack message about migration
- [ ] Include command mapping guide
- [ ] Provide support channel for issues

# 3. Update external docs (if any)
- [ ] GitHub wiki
- [ ] External blog posts
- [ ] Tutorial videos
```

---

## Command Mapping Reference

Quick reference for common commands:

| Old (just) | New (xtask) | Module |
|------------|-------------|--------|
| `just dev` | `cargo xtask dev server` | dev |
| `just web` | `cargo xtask dev web` | dev |
| `just test` | `cargo xtask test all` | test |
| `just db-up` | `cargo xtask db up` | db |
| `just db-migrate` | `cargo xtask db migrate` | db |
| `just fmt` | `cargo xtask dev fmt` | dev |
| `just lint` | `cargo xtask dev lint` | dev |
| `just build` | `cargo xtask build workspace` | build |
| `just clean` | `cargo xtask clean workspace` | clean |
| `just ci` | `cargo xtask ci all` | ci |
| `just sqlx-prepare` | `cargo xtask sqlx prepare` | sqlx |
| `just docker-up` | `cargo xtask docker up` | docker |
| `just docs-cli` | `cargo xtask docs cli` | docs |

Full command reference: `docs/development/xtask-command-reference.md`

---

## Migration Benefits Achieved

### Advantages Over Just

1. **✅ No External Dependencies**
   - Just required separate installation (`cargo install just`)
   - xtask: Built-in with `cargo build -p xtask`

2. **✅ Type Safety**
   - Just: String-based arguments, runtime validation
   - xtask: Clap types validated at compile time

3. **✅ IDE Support**
   - Just: No autocomplete, no jump-to-definition
   - xtask: Full Rust tooling (autocomplete, refactoring, debugging)

4. **✅ Cross-Platform**
   - Just: Limited Windows support
   - xtask: Native PowerShell + Bash with conditional compilation

5. **✅ Maintainable**
   - Just: 811-line justfile, hard to test
   - xtask: 18 modular Rust files, testable, versioned

6. **✅ Error Handling**
   - Just: Basic error messages
   - xtask: Rich error context with anyhow, colored output

7. **✅ Integrated**
   - Just: External tool, separate ecosystem
   - xtask: Part of Cargo workspace, shares dependencies

---

## Alternative: cargo-make

As a bonus, `Makefile.toml` provides a cargo-make configuration with identical commands. This gives developers three options:

1. **xtask** (recommended): `cargo xtask <module> <command>`
2. **cargo-make**: `cargo make <command>`
3. **just** (legacy): `just <command>` (until deleted)

All three execute the same operations, so choose based on preference.

---

## Timeline

| Phase | Estimated Time | Actual Time | Status |
|-------|----------------|-------------|--------|
| 1. Preparation | 1 hour | 1 hour | ✅ Complete |
| 2. Infrastructure | 1 hour | 1.5 hours | ✅ Complete |
| 3. Modules | 3 hours | 4 hours | ✅ Complete |
| 4. Main Entry | 1 hour | 0.5 hours | ✅ Complete |
| 5. CI/CD | 2 hours | 1 hour | ✅ Complete |
| 6. Documentation | 3 hours | 2 hours | ✅ Complete |
| 7. Cleanup | 1 hour | 0.5 hours | ✅ Complete |
| **8. Testing** | **2 hours** | **TBD** | ⏳ **IN PROGRESS** |
| 9. Final Cleanup | 1 hour | TBD | ⏳ Pending |
| 10. Bulk Docs Update | 2 hours | TBD | ⏳ Pending |
| 11. Announcement | 1 hour | TBD | ⏳ Pending |
| **TOTAL** | **18 hours** | **~10.5h + testing** | **85%** |

---

## Next Steps

1. **Start Phase 8 Testing** (THIS IS CRITICAL)
   - Begin with utility commands (low risk)
   - Progress through database, dev, build commands
   - Run full CI simulation
   - Test on Windows

2. **Document Test Results**
   - Check off items in testing checklist above
   - Note any issues or regressions
   - Record performance observations

3. **Decision Point**
   - If all tests pass → Proceed to Phase 9 (cleanup)
   - If critical issues → Execute rollback plan
   - If minor issues → Fix and re-test

4. **Final Cleanup**
   - Remove justfile
   - Clean up warnings
   - Update remaining docs

5. **Announce Migration**
   - Notify team
   - Update external resources

---

## Support & Troubleshooting

### Getting Help

If you encounter issues during testing:

1. **Check xtask help**: `cargo xtask --help`
2. **Check module help**: `cargo xtask <module> --help`
3. **Review guides**: `docs/development/xtask-guide.md`
4. **Compare with justfile**: Side-by-side comparison available

### Common Issues

**Issue**: `cargo: 'xtask' is not a cargo command`
**Fix**: Run `cargo build -p xtask` first

**Issue**: Command fails with "No such file or directory"
**Fix**: Check that paths are absolute (xtask uses absolute paths)

**Issue**: Windows PowerShell script fails
**Fix**: Verify conditional compilation `#[cfg(target_os = "windows")]`

**Issue**: Database commands fail
**Fix**: Ensure Docker services are running: `docker compose ps`

---

## Conclusion

The migration from just to xtask is **85% complete** with all code, infrastructure, and documentation in place. The final critical phase is **comprehensive testing** to ensure no regressions and full compatibility.

**Status**: Ready for Phase 8 Testing ⏳

**Recommended Action**: Begin systematic testing following the checklist above, starting with low-risk utility commands and progressing to more complex operations.

---

**Document Version**: 1.0
**Last Updated**: 2026-02-09
**Maintained By**: BDP Development Team
**Related Docs**:
- [xtask Guide](./xtask-guide.md)
- [xtask Command Reference](./xtask-command-reference.md)
- [xtask Migration Guide](./xtask-migration.md)
- [cargo-make Migration Doc](./cargo-make-migration.md)
