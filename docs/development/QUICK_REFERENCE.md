# xtask Quick Reference Card

**🚀 Now works directly:** `cargo xtask <command>` (alias configured!)

---

## Most Common Commands

```bash
# Setup & Verification
cargo xtask setup verify      # Check environment

# Database
cargo xtask db up              # Start database
cargo xtask db migrate         # Run migrations
cargo xtask db shell           # Connect to database

# Development
cargo xtask dev server         # Start backend
cargo xtask dev web            # Start frontend
cargo xtask dev fmt            # Format code
cargo xtask dev lint           # Lint code

# Testing
cargo xtask test all           # Run all tests
cargo xtask test unit          # Unit tests only
cargo xtask test integration   # Integration tests

# Building
cargo xtask build workspace    # Build Rust code
cargo xtask build release      # Release build

# CI/CD
cargo xtask ci all             # Run all CI checks
cargo xtask sqlx check         # Verify SQLx metadata

# Docker
cargo xtask docker up          # Start all services
cargo xtask docker down        # Stop all services

# Utilities
cargo xtask util info          # Show environment info
cargo xtask util health        # Health check services
```

---

## Short Form (with aliases)

```bash
cargo dev                      # Start backend
cargo test-all                 # Run tests
cargo db-up                    # Start database
cargo db-migrate               # Run migrations
cargo lint                     # Lint code
cargo fmt                      # Format code
```

---

## Get Help

```bash
cargo xtask --help             # All modules
cargo xtask db --help          # Database commands
cargo xtask dev --help         # Development commands
cargo xtask test --help        # Testing commands
cargo xtask docker --help      # Docker commands
```

---

## Migration from Just

| Old | New |
|-----|-----|
| `just setup` | `cargo xtask setup all` |
| `just dev` | `cargo xtask dev server` OR `cargo dev` |
| `just test` | `cargo xtask test all` OR `cargo test-all` |
| `just db-up` | `cargo xtask db up` OR `cargo db-up` |
| `just db-migrate` | `cargo xtask db migrate` OR `cargo db-migrate` |
| `just lint` | `cargo xtask dev lint` OR `cargo lint` |
| `just fmt` | `cargo xtask dev fmt` OR `cargo fmt` |
| `just ci` | `cargo xtask ci all` |

---

## All Modules (17 total)

1. **db** - Database operations (12 commands)
2. **dev** - Development workflows (10 commands)
3. **test** - Testing operations (11 commands)
4. **build** - Build tasks (4 commands)
5. **docs** - Documentation (5 commands)
6. **docker** - Docker operations (7 commands)
7. **sqlx** - SQLx management (3 commands)
8. **ci** - CI/CD simulation (2 commands)
9. **clean** - Cleanup operations (4 commands)
10. **setup** - Setup & initialization (4 commands)
11. **minio** - MinIO operations (3 commands)
12. **ingest** - Data ingestion (3 commands)
13. **infra** - Infrastructure/Terraform (8 commands)
14. **e2e** - E2E testing (6 commands)
15. **release** - Version management (6 commands)
16. **util** - Utilities (14 commands)
17. **generate-cli-docs** - CLI docs (legacy)

---

## Documentation

- **Full Guide**: `docs/development/xtask-guide.md`
- **Command Reference**: `docs/development/xtask-command-reference.md`
- **Migration Status**: `FINAL_STATUS.md`
- **Test Report**: `TEST_REPORT.md`

---

**Quick Tip**: Use `cargo x` as an even shorter alias!

```bash
cargo x db up
cargo x test all
cargo x dev server
```
