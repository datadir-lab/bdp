# xtask Command Reference

Complete reference for all `cargo xtask` commands in the BDP project.

## Quick Start

```bash
# Show all available modules
cargo xtask --help

# Show commands in a specific module
cargo xtask db --help
cargo xtask dev --help

# Run a command
cargo xtask db up
cargo xtask dev server
cargo xtask test all
```

## Module Overview

| Module | Commands | Purpose |
|--------|----------|---------|
| db | 12 | Database operations |
| build | 4 | Build tasks |
| test | 11 | Testing operations |
| dev | 10 | Development tasks |
| docker | 7 | Docker Compose operations |
| sqlx | 3 | SQLx management |
| minio | 3 | MinIO/S3 operations |
| ingest | 3 | Data ingestion |
| ci | 2 | CI/CD simulation |
| clean | 4 | Cleanup operations |
| docs | 5 | Documentation |
| setup | 4 | Setup & installation |
| infra | 8 | Infrastructure (Terraform) |
| util | 15 | Utilities & audit logs |
| e2e | 6 | E2E testing |
| release | 6 | Version management |

**Total: 103 commands across 16 modules**

---

## Database Operations (db)

```bash
cargo xtask db <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `up` | Start development database | `just db-up` |
| `down` | Stop database | `just db-down` |
| `test-up` | Start test database | `just db-test-up` |
| `test-down` | Stop test database | `just db-test-down` |
| `setup` | Complete database setup (start + migrate) | `just db-setup` |
| `migrate` | Run database migrations | `just db-migrate` |
| `migrate-revert` | Revert last migration | `just db-migrate-revert` |
| `migrate-add <NAME>` | Create new migration | `just db-migrate-add NAME` |
| `reset` | Reset database (dangerous - drops all data) | `just db-reset` |
| `seed` | Seed development data | `just db-seed` |
| `shell` | Connect to database with psql | `just db-shell` |
| `logs` | Database logs | `just db-logs` |

**Examples:**
```bash
cargo xtask db up
cargo xtask db migrate
cargo xtask db migrate-add add_user_roles
cargo xtask db shell
```

---

## Build Tasks (build)

```bash
cargo xtask build <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `workspace` | Build all Rust crates | `just build` |
| `release` | Build release version | `just build-release` |
| `all` | Build all (backend + frontend) | `just build-all` |
| `docker` | Build Docker images | `just docker-build` |

**Examples:**
```bash
cargo xtask build workspace
cargo xtask build release
cargo xtask build all
cargo xtask build docker
```

---

## Testing (test)

```bash
cargo xtask test <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `all` | Run all tests | `just test` |
| `verbose` | Run tests with output | `just test-verbose` |
| `integration` | Run integration tests only | `just test-integration` |
| `unit` | Run unit tests only | `just test-unit` |
| `one <TEST>` | Run specific test | `just test-one TEST` |
| `coverage` | Test with coverage | `just test-coverage` |
| `fresh` | Reset and run tests | `just test-fresh` |
| `cli-setup` | Set up test directory for CLI testing | `just test-cli-setup` |
| `cli-clean` | Clean CLI test directory | `just test-cli-clean` |
| `cli <CMD>` | Run CLI command in test directory | `just test-cli CMD` |
| `cli-full` | Full CLI test workflow | `just test-cli-full` |

**Examples:**
```bash
cargo xtask test all
cargo xtask test unit
cargo xtask test one my_test_name
cargo xtask test cli-setup
cargo xtask test cli "init --name test"
```

---

## Development (dev)

```bash
cargo xtask dev <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `server` | Start development (database + backend server) | `just dev` |
| `web` | Start frontend development server with hot reload | `just web` |
| `web-build` | Build frontend (with Pagefind indexing) | `just web-build` |
| `web-prod` | Build frontend and start production server | `just web-prod` |
| `all` | Start all services (backend + frontend + database) | `just dev-all` |
| `watch` | Watch and rebuild on changes | `just watch` |
| `fmt` | Format code | `just fmt` |
| `lint` | Lint code | `just lint` |
| `fix` | Fix linting issues | `just fix` |
| `security-audit` | Run security audit | `just security-audit` |

**Examples:**
```bash
cargo xtask dev server
cargo xtask dev web
cargo xtask dev fmt
cargo xtask dev lint
```

---

## Docker Operations (docker)

```bash
cargo xtask docker <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `up` | Start all services with Docker Compose | `just docker-up` |
| `down` | Stop all Docker Compose services | `just docker-down` |
| `migrate` | Run migrations in Docker container | `just docker-migrate` |
| `logs` | View logs from all services | `just docker-logs` |
| `logs-backend` | View backend logs | `just docker-logs-backend` |
| `restart-backend` | Restart backend service | `just docker-restart-backend` |
| `setup` | Full stack with migrations (recommended for first time) | `just docker-setup` |

**Examples:**
```bash
cargo xtask docker up
cargo xtask docker migrate
cargo xtask docker logs-backend
```

---

## SQLx Management (sqlx)

```bash
cargo xtask sqlx <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `prepare` | Generate SQLx offline metadata | `just sqlx-prepare` |
| `check` | Verify SQLx metadata is up to date | `just sqlx-check` |
| `clean` | Clean SQLx metadata | `just sqlx-clean` |

**Examples:**
```bash
cargo xtask sqlx prepare
cargo xtask sqlx check
```

---

## MinIO Operations (minio)

```bash
cargo xtask minio <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `up` | Start MinIO | `just minio-up` |
| `down` | Stop MinIO | `just minio-down` |
| `logs` | MinIO logs | `just minio-logs` |

**Examples:**
```bash
cargo xtask minio up
cargo xtask minio logs
```

---

## Data Ingestion (ingest)

```bash
cargo xtask ingest <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `uniprot` | Run UniProt ingestion | `just ingest-uniprot` |
| `ncbi` | Run NCBI ingestion (future) | `just ingest-ncbi` |
| `all` | Run all ingestion | `just ingest-all` |

**Examples:**
```bash
cargo xtask ingest uniprot
cargo xtask ingest all
```

---

## CI/CD Simulation (ci)

```bash
cargo xtask ci <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `all` | Run all CI checks locally | `just ci` |
| `offline` | Run CI checks in offline mode (like GitHub Actions) | `just ci-offline` |

**Examples:**
```bash
cargo xtask ci all
cargo xtask ci offline
```

---

## Cleanup (clean)

```bash
cargo xtask clean <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `workspace` | Clean build artifacts | `just clean` |
| `all` | Deep clean (including dependencies) | `just clean-all` |
| `stop` | Stop all Docker services | `just stop` |
| `stop-all` | Stop all and remove volumes (deletes data) | `just stop-all` |

**Examples:**
```bash
cargo xtask clean workspace
cargo xtask clean all
cargo xtask clean stop
```

---

## Documentation (docs)

```bash
cargo xtask docs <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `cargo` | Build Cargo documentation | `just docs` |
| `web` | Serve frontend docs | `just docs-web` |
| `cli` | Generate CLI reference documentation (MDX format) | `just docs-cli` |
| `cli-raw` | Generate CLI documentation using hidden flag | `just docs-cli-raw` |
| `cli-check` | Check if CLI docs are up to date (for CI) | `just docs-cli-check` |

**Examples:**
```bash
cargo xtask docs cargo
cargo xtask docs cli
cargo xtask docs cli-check
```

---

## Setup & Installation (setup)

```bash
cargo xtask setup <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `all` | Complete first-time setup (quick start) | `just setup` |
| `install-deps` | Install all dependencies | `just install-deps` |
| `env-setup` | Setup environment file | `just env-setup` |
| `verify` | Verify setup is correct | `just verify` |

**Examples:**
```bash
cargo xtask setup all
cargo xtask setup verify
```

---

## Infrastructure (infra)

```bash
cargo xtask infra <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `init` | Initialize Terraform | `just infra-init` |
| `plan` | Preview infrastructure changes | `just infra-plan` |
| `apply` | Apply infrastructure changes | `just infra-apply` |
| `destroy` | Destroy infrastructure (careful!) | `just infra-destroy` |
| `output` | Show infrastructure outputs | `just infra-output` |
| `env` | Generate production .env file from Terraform | `just infra-env` |
| `ssh` | SSH into production server | `just infra-ssh` |
| `status` | Show infrastructure status | `just infra-status` |

**Examples:**
```bash
cargo xtask infra init
cargo xtask infra plan
cargo xtask infra apply
cargo xtask infra status
```

---

## Utilities (util)

```bash
cargo xtask util <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `info` | Show environment info | `just info` |
| `check-db` | Check database connection | `just check-db` |
| `logs` | Show logs for all services | `just logs` |
| `logs-backend` | Follow backend logs | `just logs-backend` |
| `logs-frontend` | Follow frontend logs | `just logs-frontend` |
| `health` | Health check all services | `just health` |
| `version` | Show current version | `just version` |
| `audit-logs [LIMIT]` | View recent audit logs | `just audit-logs LIMIT` |
| `audit-search <TERM>` | Search audit logs by action | `just audit-search TERM` |
| `audit-by-resource <TYPE>` | View audit logs for a specific resource type | `just audit-by-resource TYPE` |
| `audit-by-user <USER_ID>` | View audit logs for a specific user | `just audit-by-user USER_ID` |
| `audit-trail <TYPE> <ID>` | View audit trail for a specific resource | `just audit-trail TYPE ID` |
| `audit-export [OUTPUT]` | Export audit logs to JSON | `just audit-export OUTPUT` |
| `audit-stats` | Show audit statistics | `just audit-stats` |

**Examples:**
```bash
cargo xtask util info
cargo xtask util version
cargo xtask util audit-logs 100
cargo xtask util audit-search "user_created"
```

---

## E2E Testing (e2e)

```bash
cargo xtask e2e <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `ci` | Run E2E tests in CI mode (fast, uses committed fixtures) | `just e2e-ci` |
| `real` | Run E2E tests in Real mode (uses downloaded data) | `just e2e-real` |
| `download-data` | Download real UniProt test data (idempotent, cached) | `just e2e-download-data` |
| `debug` | Run E2E tests with full observability output | `just e2e-debug` |
| `clean` | Clean E2E test data (removes downloaded data, keeps CI fixtures) | `just e2e-clean` |
| `info` | Show E2E test data info | `just e2e-info` |

**Examples:**
```bash
cargo xtask e2e ci
cargo xtask e2e download-data
cargo xtask e2e debug
```

---

## Version Management (release)

```bash
cargo xtask release <command>
```

| Command | Description | Just Equivalent |
|---------|-------------|-----------------|
| `patch` | Bump patch version (0.1.0 → 0.1.1) and create git tag | `just release-patch` |
| `minor` | Bump minor version (0.1.0 → 0.2.0) and create git tag | `just release-minor` |
| `major` | Bump major version (0.1.0 → 1.0.0) and create git tag | `just release-major` |
| `patch-dry` | Dry run of patch release (preview changes) | `just release-patch-dry` |
| `minor-dry` | Dry run of minor release (preview changes) | `just release-minor-dry` |
| `bump <VERSION>` | Manual version bump without git operations (for testing) | `just bump-version VERSION` |

**Examples:**
```bash
cargo xtask release patch
cargo xtask release minor-dry
cargo xtask release bump 0.2.0
```

---

## Common Workflows

### First Time Setup
```bash
# Complete setup with database
cargo xtask setup all

# Or step by step
cargo xtask setup install-deps
cargo xtask setup env-setup
cargo xtask db up
cargo xtask db migrate
```

### Daily Development
```bash
# Start backend (in terminal 1)
cargo xtask dev server

# Start frontend (in terminal 2)
cargo xtask dev web

# Run tests
cargo xtask test all

# Format and lint
cargo xtask dev fmt
cargo xtask dev lint
```

### Before Committing
```bash
# Run CI checks locally
cargo xtask ci all

# Or individual checks
cargo xtask dev fmt
cargo xtask dev lint
cargo xtask sqlx check
cargo xtask docs cli-check
cargo xtask test all
```

### Docker Development
```bash
# Start full stack with Docker
cargo xtask docker setup

# View logs
cargo xtask docker logs-backend

# Stop everything
cargo xtask clean stop-all
```

### CLI Testing
```bash
# Set up test directory
cargo xtask test cli-setup

# Test CLI commands
cargo xtask test cli "init --name test"
cargo xtask test cli "source add uniprot:P01308"

# Clean up
cargo xtask test cli-clean
```

---

## Cross-Platform Notes

All commands work on both Windows and Unix-like systems:
- **Windows**: Uses PowerShell for shell scripts
- **Linux/macOS**: Uses Bash for shell scripts

The xtask tool automatically detects the platform and uses the appropriate shell.

---

**Last Updated**: 2026-02-09
**Total Commands**: 103 across 16 modules
**Status**: Phase 3 Complete - All modules implemented
