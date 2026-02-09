# Summary: Remaining "just" References in BDP Codebase

**Migration Status**: Completed - Migrated from `just` to `cargo-make` to `cargo xtask`
**Date**: 2026-02-09
**Migration Documentation**: [xtask-migration.md](./xtask-migration.md)

## Overview

This document catalogs all remaining references to the `just` command runner in the BDP codebase after the migration to `cargo xtask`. These references are primarily in documentation files that serve as historical records or examples.

## Migration Summary

BDP has successfully migrated from:
1. **Just** (justfile-based task runner) →
2. **cargo-make** (Makefile.toml-based) →
3. **cargo xtask** (Rust-based task runner)

The current task runner is **cargo xtask**, which provides:
- Type-safe task definitions in Rust
- Full IDE support with code completion
- Better error messages
- Cross-platform compatibility
- No external dependencies

## Files With "just" References

### 1. Documentation Files (464 total occurrences across 36 files)

#### Active Documentation Files That Need Updating

These are actively used documentation files that should be updated to use `cargo xtask` commands:

1. **D:\dev\datadir\bdp\docs\QUICK_START.md** (6 occurrences)
   - Status: Needs update
   - Uses: `just dev`, `just test`, `just docker-up`, `just db-migrate`
   - Action: Replace with cargo xtask equivalents

2. **D:\dev\datadir\bdp\docs\TESTING.md** (20 occurrences)
   - Status: Needs update
   - Uses: Various just test commands
   - Action: Replace with cargo xtask equivalents

3. **D:\dev\datadir\bdp\docs\development\testing.md** (19 occurrences)
   - Status: Needs update
   - Uses: Various just test commands
   - Action: Replace with cargo xtask equivalents

4. **D:\dev\datadir\bdp\docs\development\sqlx-setup.md** (20 occurrences)
   - Status: Needs update
   - Uses: `just db-migrate-add`, `just db-migrate`, `just sqlx-prepare`, etc.
   - Action: Replace with cargo xtask equivalents

5. **D:\dev\datadir\bdp\docs\development\QUICK_START_SQLX.md** (31 occurrences)
   - Status: Needs update
   - Uses: Various just db and sqlx commands
   - Action: Replace with cargo xtask equivalents

6. **D:\dev\datadir\bdp\docs\DEPLOYMENT_CHECKLIST.md** (3 occurrences)
   - Status: Needs update
   - Uses: `just test`
   - Action: Replace with `cargo xtask test all`

7. **D:\dev\datadir\bdp\docs\NEXT_STEPS.md** (4 occurrences)
   - Status: Needs update
   - Uses: `just test`, `just test-integration`, `just dev`
   - Action: Replace with cargo xtask equivalents

8. **D:\dev\datadir\bdp\docs\agents\backend-architecture.md** (7 occurrences)
   - Status: Needs update
   - Contains just command examples
   - Action: Replace with cargo xtask equivalents

9. **D:\dev\datadir\bdp\docs\agents\error-handling.md** (6 occurrences)
   - Status: Needs update
   - Contains just command examples
   - Action: Replace with cargo xtask equivalents

10. **D:\dev\datadir\bdp\docs\development\VERSIONING.md** (41 occurrences)
    - Status: Needs update
    - Heavy use of just commands in version management
    - Action: Replace with cargo xtask equivalents

#### Archive Documentation Files (Historical Reference)

These files are in the archive directory and serve as historical records. They can remain as-is or have a note added:

- **D:\dev\datadir\bdp\docs\archive\implementation\e2e-testing-setup.md** (14 occurrences)
- **D:\dev\datadir\bdp\docs\archive\implementation\pagefind-integration.md** (2 occurrences)
- **D:\dev\datadir\bdp\docs\archive\implementation\uniprot-ingestion-testing.md** (1 occurrence)
- **D:\dev\datadir\bdp\docs\archive\implementation\migration-safety-tests.md** (1 occurrence)
- **D:\dev\datadir\bdp\docs\archive\implementation\genbank-version-discovery.md** (2 occurrences)
- **D:\dev\datadir\bdp\docs\archive\interpro\interpro-todo.md** (1 occurrence)
- **D:\dev\datadir\bdp\docs\archive\interpro\interpro-migration-test-report.md** (2 occurrences)
- **D:\dev\datadir\bdp\docs\archive\interpro\interpro-progress-summary.md** (1 occurrence)
- **D:\dev\datadir\bdp\docs\archive\ingestion-pipeline-completion.md** (1 occurrence)

#### Migration Documentation (Should Remain)

These files document the migration process and should keep their references:

- **D:\dev\datadir\bdp\docs\development\just-guide.md** (110 occurrences) - Historical guide
- **D:\dev\datadir\bdp\docs\development\cargo-make-migration.md** (18 occurrences) - Migration doc
- **D:\dev\datadir\bdp\docs\development\xtask-migration.md** (4 occurrences) - Current migration doc

#### Other Documentation

- **D:\dev\datadir\bdp\docs\cli\SEARCH_COMMAND.md** (1 occurrence)
- **D:\dev\datadir\bdp\docs\development\cli-docs-ci-integration.md** (19 occurrences)
- **D:\dev\datadir\bdp\docs\development\cli-documentation-generation.md** (11 occurrences)
- **D:\dev\datadir\bdp\docs\agents\design\cli-audit-provenance.md** (5 occurrences)
- **D:\dev\datadir\bdp\docs\agents\design\version-mapping.md** (1 occurrence)
- **D:\dev\datadir\bdp\docs\agents\database-design-philosophy.md** (2 occurrences)
- **D:\dev\datadir\bdp\docs\agents\implementation\mediator-cqrs-architecture.md** (3 occurrences)
- **D:\dev\datadir\bdp\docs\architecture\genbank-streaming.md** (1 occurrence)

### 2. Source Code Files

#### Disabled Test Files (Can Ignore)

- **crates\bdp-server\tests\uniprot_integration_test.rs.disabled**
- **crates\bdp-server\tests\resolve_tests.rs.disabled**
- **crates\bdp-server\tests\e2e.rs.disabled**

#### Example Files

- **crates\bdp-server\examples\test_storage_pipeline.rs**
- **crates\bdp-server\examples\go_historical_ingestion.rs.disabled**

#### Active Code Files

- **crates\bdp-cli\src\commands\search.rs** - Contains comment about "just"
  - Line references "just" in context, not as a command

### 3. Root Directory Files

#### Justfile (810 lines)

- **D:\dev\datadir\bdp\justfile** - Original justfile still exists
  - Status: Can be removed or archived
  - Contains all the original just task definitions
  - Replaced by: `xtask/src/main.rs` and task modules

### 4. Web/Frontend Files

#### No just references in package.json

The web/package.json file was checked and contains NO references to just in scripts.

### 5. Docker & Infrastructure Files

#### No just references found in:

- docker-compose.yml
- docker/docker-compose.test.yml
- infrastructure/deploy/docker-compose.prod.yml
- All Dockerfile.* files

### 6. Shell Scripts

#### Only one benign reference:

- **scripts/install.sh** (line 266) - Comment about checksum format ("just checksum")
  - This is NOT a reference to the just command
  - It's describing checksum file format

### 7. Web Frontend Files

Multiple web frontend files contain the word "just" in natural language contexts (e.g., "just released", "just a moment"). These are NOT command references and should remain unchanged.

## Recommended Actions

### High Priority (Active Documentation)

1. **Update D:\dev\datadir\bdp\docs\INDEX.md**
   - ✅ COMPLETED - Updated to reference xtask-command-reference.md and xtask-migration.md

2. **Update D:\dev\datadir\bdp\docs\QUICK_START.md**
   - Replace `just dev` → `cargo xtask dev server`
   - Replace `just test` → `cargo xtask test all`
   - Replace `just db-migrate` → `cargo xtask db migrate`

3. **Update D:\dev\datadir\bdp\docs\TESTING.md**
   - Replace all just test commands with cargo xtask equivalents

4. **Update D:\dev\datadir\bdp\docs\development\testing.md**
   - Replace all just test commands with cargo xtask equivalents

5. **Update D:\dev\datadir\bdp\docs\development\sqlx-setup.md**
   - Replace all just db/sqlx commands with cargo xtask equivalents

6. **Update D:\dev\datadir\bdp\docs\development\QUICK_START_SQLX.md**
   - Replace all just commands with cargo xtask equivalents

7. **Update D:\dev\datadir\bdp\docs\development\VERSIONING.md**
   - Replace all just commands with cargo xtask equivalents

8. **Update D:\dev\datadir\bdp\docs\DEPLOYMENT_CHECKLIST.md**
   - Replace `just test` with `cargo xtask test all`

9. **Update D:\dev\datadir\bdp\docs\NEXT_STEPS.md**
   - Replace just commands with cargo xtask equivalents

10. **Update D:\dev\datadir\bdp\docs\agents\backend-architecture.md**
    - Replace just command examples with cargo xtask equivalents

11. **Update D:\dev\datadir\bdp\docs\agents\error-handling.md**
    - Replace just command examples with cargo xtask equivalents

### Medium Priority (Historical/Archive)

1. **Add migration notice to archived docs**
   - Add a note at the top of archive docs referencing the migration
   - Example: "Note: This document uses `just` commands. See xtask-migration.md for current commands."

2. **Keep migration documentation as-is**
   - just-guide.md - Keep as historical reference
   - cargo-make-migration.md - Keep as migration record
   - xtask-migration.md - Keep as current migration guide

### Low Priority (Optional)

1. **Remove or archive justfile**
   - Option A: Delete D:\dev\datadir\bdp\justfile
   - Option B: Move to docs/archive/justfile.deprecated
   - Option C: Keep with deprecation notice at top

2. **Add migration banner to CLAUDE.md**
   - Already updated with xtask commands in "Common Commands" section

## Command Mapping Reference

For updating documentation, use these mappings:

| Old (just) | New (cargo xtask) |
|------------|-------------------|
| `just dev` | `cargo xtask dev server` |
| `just test` | `cargo xtask test all` |
| `just test-unit` | `cargo xtask test unit` |
| `just test-integration` | `cargo xtask test integration` |
| `just fmt` | `cargo xtask dev fmt` |
| `just lint` | `cargo xtask dev lint` |
| `just db-migrate` | `cargo xtask db migrate` |
| `just db-migrate-add NAME` | `cargo xtask db add-migration NAME` |
| `just sqlx-prepare` | `cargo xtask sqlx prepare` |
| `just build` | `cargo build` (no xtask wrapper needed) |
| `just test-cli-setup` | `cargo xtask test cli-setup` |
| `just test-cli "CMD"` | `cargo xtask test cli "CMD"` |
| `just test-cli-clean` | `cargo xtask test cli-clean` |

## Statistics

- **Total "just" word occurrences**: 464 across 36 documentation files
- **Files with just commands**: ~25 active documentation files
- **Archive/historical files**: ~10 files
- **Migration documentation**: 3 files (should keep references)
- **Justfile size**: 810 lines (can be removed)
- **Docker/infrastructure**: 0 references (clean)
- **Web package.json**: 0 references (clean)
- **Shell scripts**: 1 benign reference (not a command)

## Verification

To verify all references:

```bash
# Search for "just" in documentation
cargo xtask dev grep "\bjust\b" docs/

# Search for justfile references
cargo xtask dev grep "justfile"

# Search for specific just commands
cargo xtask dev grep "just (dev|test|migrate|build|fmt)" docs/
```

## Next Steps

1. ✅ Update docs/INDEX.md (COMPLETED)
2. Update high-priority active documentation files
3. Add migration notices to archive documentation
4. Decide on justfile disposition (keep, archive, or delete)
5. Update CLAUDE.md with complete xtask reference (already done)
6. Run verification grep to ensure all updates are complete

## References

- [xtask Migration Guide](./xtask-migration.md) - Complete migration documentation
- [xtask Command Reference](./xtask-command-reference.md) - All available xtask commands
- [Just Guide](./just-guide.md) - Historical reference (deprecated)
- [cargo-make Migration](./cargo-make-migration.md) - First migration step (historical)

---

**Last Updated**: 2026-02-09
**Status**: Documentation update in progress
