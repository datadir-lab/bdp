# Migration from Just to xtask

**Date Started**: 2026-02-09
**Status**: In Progress

## Overview

This document tracks the migration from Just (task runner) to xtask (Rust-based task automation pattern).

## Progress

- [x] Phase 1: Preparation & Design - ✅ In Progress
- [ ] Phase 2: Core Infrastructure
- [ ] Phase 3: Convert Task Modules
- [ ] Phase 4: Update Main Entry Point
- [ ] Phase 5: Update CI/CD Workflows
- [ ] Phase 6: Update Documentation
- [ ] Phase 7: Root Directory Cleanup
- [ ] Phase 8: Testing & Verification

## Command Mapping

| Justfile Command | xtask Command | Status |
|------------------|---------------|--------|
| `just setup` | `cargo xtask setup all` | Pending |
| `just dev` | `cargo xtask dev` | Pending |
| `just test` | `cargo xtask test` | Pending |
| `just db-up` | `cargo xtask db up` | Pending |
| ... (95 total) | ... | ... |

## Breaking Changes

None planned - maintaining 100% compatibility with flat command aliases.

## Rollback Instructions

```bash
git checkout main -- justfile xtask/
git checkout main -- .github/workflows/ci.yml
git reset --hard HEAD
```

## Notes

- Using xtask pattern (community standard for Rust build automation)
- All commands remain type-safe and compiled
- No external dependencies required (uses Cargo)
- Better IDE support and refactoring capabilities
