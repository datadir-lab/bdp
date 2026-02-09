# xtask Guide

This guide explains how BDP uses the xtask pattern for build automation, providing a comprehensive reference for all available commands and how to extend the system.

## Table of Contents

- [What is xtask?](#what-is-xtask)
- [Why We Use xtask](#why-we-use-xtask)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Command Reference](#command-reference)
  - [Database Commands](#database-commands-cargo-xtask-db)
  - [Development Commands](#development-commands-cargo-xtask-dev)
  - [Testing Commands](#testing-commands-cargo-xtask-test)
  - [Build Commands](#build-commands-cargo-xtask-build)
  - [Documentation Commands](#documentation-commands-cargo-xtask-docs)
  - [CI/CD Commands](#cicd-commands-cargo-xtask-ci)
  - [Cleanup Commands](#cleanup-commands-cargo-xtask-clean)
  - [Docker Commands](#docker-commands-cargo-xtask-docker)
  - [SQLx Commands](#sqlx-commands-cargo-xtask-sqlx)
  - [MinIO Commands](#minio-commands-cargo-xtask-minio)
  - [Ingestion Commands](#ingestion-commands-cargo-xtask-ingest)
  - [Setup Commands](#setup-commands-cargo-xtask-setup)
  - [Infrastructure Commands](#infrastructure-commands-cargo-xtask-infra)
  - [E2E Testing Commands](#e2e-testing-commands-cargo-xtask-e2e)
  - [Release Commands](#release-commands-cargo-xtask-release)
  - [Utility Commands](#utility-commands-cargo-xtask-util)
- [Migration from Just](#migration-from-just)
- [Adding New Tasks](#adding-new-tasks)
- [Troubleshooting](#troubleshooting)

## What is xtask?

The **xtask pattern** is a Rust community standard for build automation. Instead of external tools like `make` or `just`, you write build tasks as Rust code in a separate crate within your workspace.

### Key Features

- **Type-safe**: Tasks are Rust code, fully compiled and type-checked
- **No external dependencies**: Uses the Cargo toolchain you already have
- **IDE support**: Full autocomplete, refactoring, go-to-definition, and debugging
- **Code reuse**: Can import project utilities, types, and shared logic
- **Community standard**: Used by rust-analyzer, cargo, ripgrep, and many major Rust projects
- **Cross-platform**: Works identically on Linux, macOS, and Windows

### How It Works

xtask is implemented as a workspace member at `D:\dev\datadir\bdp\xtask` with its own `Cargo.toml`. When you run `cargo xtask <command>`, Cargo:

1. Builds the xtask binary
2. Runs it with your command arguments
3. The xtask binary executes the requested task

This approach leverages Rust's build system, ensuring all tasks are properly compiled before execution.

## Why We Use xtask

BDP migrated from Just to xtask to leverage Rust's type system and tooling for build automation.

### Benefits Over Just

| Feature | xtask | Just |
|---------|-------|------|
| **Type safety** | Yes (compile-time checked) | No |
| **IDE support** | Full (autocomplete, refactoring) | Limited |
| **Installation** | None (part of Cargo) | Requires separate tool |
| **Code reuse** | Can import project code | Limited to shell |
| **Debugging** | Full Rust debugging | Shell debugging |
| **Error handling** | Result types, proper errors | Shell exit codes |
| **Cross-platform** | Rust handles platform differences | Manual shell variants |
| **Documentation** | Rustdoc + clap help | Comments only |

### Example Comparison

**Before (Just)**:
```just
# Start development server
dev: db-up
    @echo "🚀 Starting backend server..."
    cargo run --bin bdp-server
```

**After (xtask)**:
```rust
fn server() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::Up)?;
    info("🚀 Starting backend server...");
    run_streaming("cargo", &["run", "--bin", "bdp-server"], "Starting backend server")
}
```

Benefits:
- Type-checked at compile time
- Proper error propagation with `Result`
- Can reuse utility functions
- Full IDE support

## Installation

**Nothing to install!** xtask is part of the BDP workspace. If you have the repository, you have xtask.

The xtask binary is automatically built and cached by Cargo on first use.

### Verify Installation

```bash
# View all available commands
cargo xtask --help

# Run a simple command
cargo xtask setup verify
```

### Optional: Shell Alias

For convenience, you can create a shorter alias:

**Bash/Zsh** (add to `~/.bashrc` or `~/.zshrc`):
```bash
alias x='cargo xtask'
```

**PowerShell** (add to your profile):
```powershell
Set-Alias x 'cargo xtask'
```

Then you can use:
```bash
x dev server        # Instead of: cargo xtask dev server
x test all          # Instead of: cargo xtask test all
```

## Getting Started

### View All Commands

List all available commands with descriptions:

```bash
cargo xtask --help
```

List commands for a specific module:

```bash
cargo xtask db --help
cargo xtask dev --help
cargo xtask test --help
```

### Running a Command

To run a command, use this pattern:

```bash
cargo xtask <module> <command> [arguments]
```

Examples:

```bash
cargo xtask db up                    # Start database
cargo xtask dev server               # Start development server
cargo xtask test all                 # Run all tests
cargo xtask db migrate-add add_users # Create migration with name
```

### First-Time Setup

For new developers, run the complete setup:

```bash
# Complete first-time setup
cargo xtask setup all

# Or run steps individually
cargo xtask setup install-deps    # Install dependencies
cargo xtask setup env-setup        # Create .env file
cargo xtask db setup               # Start database
cargo xtask db migrate             # Run migrations
cargo xtask setup verify           # Verify everything works
```

## Command Reference

### Database Commands (`cargo xtask db`)

Commands for managing PostgreSQL databases.

#### `cargo xtask db up`
Start the development database container.

```bash
cargo xtask db up
```

**Output**: PostgreSQL available at `localhost:5432`

#### `cargo xtask db down`
Stop the development database.

```bash
cargo xtask db down
```

#### `cargo xtask db test-up`
Start the test database container (port 5433).

```bash
cargo xtask db test-up
```

**Use case**: Run before integration tests

#### `cargo xtask db test-down`
Stop the test database.

```bash
cargo xtask db test-down
```

#### `cargo xtask db setup`
Complete database setup (starts container + waits for ready).

```bash
cargo xtask db setup
```

**Use case**: First-time setup or after `db down -v`

#### `cargo xtask db migrate`
Run all pending database migrations.

```bash
cargo xtask db migrate
```

**Use case**: After pulling new migrations from git

#### `cargo xtask db migrate-revert`
Revert the last applied migration.

```bash
cargo xtask db migrate-revert
```

**Warning**: Use with caution, may cause data loss

#### `cargo xtask db migrate-add <name>`
Create a new database migration file.

```bash
cargo xtask db migrate-add create_users_table
cargo xtask db migrate-add add_email_to_users
```

**Output**: New files in `migrations/` directory

#### `cargo xtask db reset`
**DESTRUCTIVE**: Drop all data and recreate database.

```bash
cargo xtask db reset
```

**Warning**: Requires confirmation, deletes all data

#### `cargo xtask db seed`
Seed the database with development data.

```bash
cargo xtask db seed
```

**Use case**: Populate database with sample data for testing

#### `cargo xtask db shell`
Connect to the database with psql.

```bash
cargo xtask db shell
```

**Use case**: Interactive SQL queries and inspection

#### `cargo xtask db logs`
View database logs in real-time.

```bash
cargo xtask db logs
```

**Use case**: Debug connection issues or query problems

---

### Development Commands (`cargo xtask dev`)

Commands for development workflows.

#### `cargo xtask dev server`
Start the backend development server.

```bash
cargo xtask dev server
```

**Output**: API available at `http://localhost:8000`

**Includes**: Automatically starts database if not running

#### `cargo xtask dev web`
Start the frontend development server with hot reload.

```bash
cargo xtask dev web
```

**Output**: Frontend available at `http://localhost:3000`

**Features**: Hot reload, fast refresh

#### `cargo xtask dev web-build`
Build the frontend for production with Pagefind indexing.

```bash
cargo xtask dev web-build
```

**Output**: Build artifacts in `web/.next/standalone/`

**Use case**: Before deployment or Docker builds

#### `cargo xtask dev web-prod`
Build frontend and start production server.

```bash
cargo xtask dev web-prod
```

**Use case**: Test production build locally

#### `cargo xtask dev all`
Display instructions for starting all services.

```bash
cargo xtask dev all
```

**Note**: Run backend and frontend in separate terminals

#### `cargo xtask dev watch`
Watch Rust files and rebuild on changes.

```bash
cargo xtask dev watch
```

**Use case**: Continuous compilation during development

#### `cargo xtask dev fmt`
Format all code (Rust + frontend).

```bash
cargo xtask dev fmt
```

**Formats**: Rust code with `cargo fmt`, web code with Prettier

#### `cargo xtask dev lint`
Lint all code (Rust + frontend).

```bash
cargo xtask dev lint
```

**Checks**: Rust with clippy, TypeScript with ESLint

#### `cargo xtask dev fix`
Auto-fix linting issues.

```bash
cargo xtask dev fix
```

**Applies**: Clippy fixes and formatting

#### `cargo xtask dev security-audit`
Run security audit on dependencies.

```bash
cargo xtask dev security-audit
```

**Use case**: Check for known vulnerabilities

---

### Testing Commands (`cargo xtask test`)

Commands for running tests.

#### `cargo xtask test all`
Run all tests (unit + integration).

```bash
cargo xtask test all
```

**Includes**: Automatically starts test database

#### `cargo xtask test verbose`
Run tests with output visible.

```bash
cargo xtask test verbose
```

**Use case**: Debug failing tests

#### `cargo xtask test integration`
Run integration tests only.

```bash
cargo xtask test integration
```

**Requires**: Test database running

#### `cargo xtask test unit`
Run unit tests only.

```bash
cargo xtask test unit
```

**Fast**: No database required

#### `cargo xtask test one <test>`
Run a specific test by name.

```bash
cargo xtask test one test_create_organization
cargo xtask test one test_user_login
```

**Output**: Shows test output with `--nocapture`

#### `cargo xtask test coverage`
Generate test coverage report.

```bash
cargo xtask test coverage
```

**Output**: HTML report in `coverage/` directory

**Requires**: `cargo-tarpaulin` installed

#### `cargo xtask test fresh`
Reset test database and run all tests.

```bash
cargo xtask test fresh
```

**Use case**: Ensure clean slate for tests

#### `cargo xtask test cli-setup`
Create CLI test directory at `D:/dev/datadir/bdp-example`.

```bash
cargo xtask test cli-setup
```

**Use case**: Prepare for CLI testing

#### `cargo xtask test cli-clean`
Clean CLI test directory.

```bash
cargo xtask test cli-clean
```

**Use case**: Reset CLI test environment

#### `cargo xtask test cli <command>`
Run CLI command in test directory.

```bash
cargo xtask test cli "init --name test-project"
cargo xtask test cli "source add uniprot:P01308-fasta@1.0"
cargo xtask test cli "status"
```

**Use case**: Manual CLI testing

#### `cargo xtask test cli-full`
Run complete CLI test workflow.

```bash
cargo xtask test cli-full
```

**Workflow**: Setup → init → add sources → list

---

### Build Commands (`cargo xtask build`)

Commands for building the project.

#### `cargo xtask build workspace`
Build all Rust crates in debug mode.

```bash
cargo xtask build workspace
```

**Output**: Debug binaries in `target/debug/`

#### `cargo xtask build release`
Build all Rust crates in release mode.

```bash
cargo xtask build release
```

**Output**: Optimized binaries in `target/release/`

**Use case**: Before deployment

#### `cargo xtask build all`
Build everything (backend + frontend).

```bash
cargo xtask build all
```

**Includes**: Rust workspace + Next.js production build

#### `cargo xtask build docker`
Build all Docker images.

```bash
cargo xtask build docker
```

**Creates**:
- `bdp-server:latest` - Backend API
- `bdp-cli:latest` - CLI tool
- `bdp-ingest:latest` - Data ingestion
- `bdp-web:latest` - Frontend

---

### Documentation Commands (`cargo xtask docs`)

Commands for generating and viewing documentation.

#### `cargo xtask docs cargo`
Build and open Cargo documentation.

```bash
cargo xtask docs cargo
```

**Opens**: Documentation in browser

#### `cargo xtask docs web`
Start the documentation server (frontend docs).

```bash
cargo xtask docs web
```

**Output**: Docs at `http://localhost:3000/docs`

#### `cargo xtask docs cli`
Generate CLI reference documentation (MDX format).

```bash
cargo xtask docs cli
```

**Output**: `web/app/[locale]/docs/content/en/cli-reference.mdx`

**Use case**: After changing CLI commands

#### `cargo xtask docs cli-raw`
Generate raw markdown from CLI.

```bash
cargo xtask docs cli-raw
```

**Output**: `web/app/[locale]/docs/content/en/cli-reference-raw.md`

#### `cargo xtask docs cli-check`
Check if CLI docs are up to date (for CI).

```bash
cargo xtask docs cli-check
```

**Exit code**: 0 if up to date, 1 if outdated

**Use case**: CI pipeline verification

---

### CI/CD Commands (`cargo xtask ci`)

Commands for simulating CI/CD checks locally.

#### `cargo xtask ci all`
Run all CI checks locally.

```bash
cargo xtask ci all
```

**Checks**:
1. CLI docs up to date
2. SQLx metadata up to date
3. Linting passes
4. All tests pass

**Use case**: Before pushing to GitHub

#### `cargo xtask ci offline`
Run CI checks in offline mode (like GitHub Actions).

```bash
cargo xtask ci offline
```

**Environment**: Sets `SQLX_OFFLINE=true`

**Use case**: Test CI without database

---

### Cleanup Commands (`cargo xtask clean`)

Commands for cleaning build artifacts and stopping services.

#### `cargo xtask clean workspace`
Clean build artifacts.

```bash
cargo xtask clean workspace
```

**Removes**:
- `target/` directory
- `web/.next/` directory
- `web/node_modules/.cache/`

#### `cargo xtask clean all`
Deep clean (including dependencies).

```bash
cargo xtask clean all
```

**Removes**:
- All workspace artifacts
- `web/node_modules/`
- Cargo build cache

**Use case**: Fix weird build issues

#### `cargo xtask clean stop`
Stop all Docker services.

```bash
cargo xtask clean stop
```

**Stops**: All services defined in `docker-compose.yml`

#### `cargo xtask clean stop-all`
Stop all services and remove volumes (deletes data).

```bash
cargo xtask clean stop-all
```

**Warning**: Removes database data

---

### Docker Commands (`cargo xtask docker`)

Commands for Docker Compose operations.

#### `cargo xtask docker up`
Start all services with Docker Compose.

```bash
cargo xtask docker up
```

**Starts**:
- PostgreSQL at `localhost:5432`
- Backend API at `http://localhost:8000`
- MinIO Console at `http://localhost:9001`

#### `cargo xtask docker down`
Stop all Docker Compose services.

```bash
cargo xtask docker down
```

#### `cargo xtask docker migrate`
Run migrations in Docker container.

```bash
cargo xtask docker migrate
```

**Use case**: When running full stack in Docker

#### `cargo xtask docker logs`
View logs from all services.

```bash
cargo xtask docker logs
```

**Output**: Real-time logs with colors

#### `cargo xtask docker logs-backend`
View backend logs only.

```bash
cargo xtask docker logs-backend
```

#### `cargo xtask docker restart-backend`
Restart the backend service.

```bash
cargo xtask docker restart-backend
```

**Use case**: Apply configuration changes

#### `cargo xtask docker setup`
Complete Docker setup (up + migrations).

```bash
cargo xtask docker setup
```

**Use case**: First-time Docker setup

---

### SQLx Commands (`cargo xtask sqlx`)

Commands for managing SQLx offline mode.

#### `cargo xtask sqlx prepare`
Generate SQLx offline metadata.

```bash
cargo xtask sqlx prepare
```

**Output**: JSON files in `.sqlx/` directory

**Use case**: Before pushing commits (for CI)

#### `cargo xtask sqlx check`
Verify SQLx metadata is up to date.

```bash
cargo xtask sqlx check
```

**Exit code**: 0 if current, 1 if outdated

**Use case**: CI pipeline verification

#### `cargo xtask sqlx clean`
Remove SQLx metadata files.

```bash
cargo xtask sqlx clean
```

**Use case**: Force regeneration

---

### MinIO Commands (`cargo xtask minio`)

Commands for managing MinIO object storage.

#### `cargo xtask minio up`
Start MinIO service.

```bash
cargo xtask minio up
```

**Output**: Console at `http://localhost:9001`

**Credentials**: `minioadmin` / `minioadmin`

#### `cargo xtask minio down`
Stop MinIO service.

```bash
cargo xtask minio down
```

#### `cargo xtask minio logs`
View MinIO logs.

```bash
cargo xtask minio logs
```

---

### Ingestion Commands (`cargo xtask ingest`)

Commands for data ingestion pipelines.

#### `cargo xtask ingest uniprot`
Run UniProt data ingestion.

```bash
cargo xtask ingest uniprot
```

**Use case**: Populate database with UniProt data

#### `cargo xtask ingest ncbi`
Run NCBI data ingestion (future).

```bash
cargo xtask ingest ncbi
```

#### `cargo xtask ingest all`
Run all ingestion pipelines.

```bash
cargo xtask ingest all
```

---

### Setup Commands (`cargo xtask setup`)

Commands for first-time setup and installation.

#### `cargo xtask setup all`
Complete first-time setup.

```bash
cargo xtask setup all
```

**Runs**:
1. Install dependencies
2. Create .env file
3. Start database
4. Run migrations

**Use case**: New developer onboarding

#### `cargo xtask setup install-deps`
Install all dependencies.

```bash
cargo xtask setup install-deps
```

**Installs**:
- `sqlx-cli` (Cargo)
- Node dependencies (yarn)

#### `cargo xtask setup env-setup`
Create .env file from .env.example.

```bash
cargo xtask setup env-setup
```

**Safe**: Skips if .env already exists

#### `cargo xtask setup verify`
Verify setup is correct.

```bash
cargo xtask setup verify
```

**Checks**:
- Required files exist
- Docker installed
- Rust toolchain
- SQLx CLI
- Node.js

---

### Infrastructure Commands (`cargo xtask infra`)

Commands for Terraform infrastructure management.

#### `cargo xtask infra init`
Initialize Terraform.

```bash
cargo xtask infra init
```

**Use case**: First-time infrastructure setup

#### `cargo xtask infra plan`
Preview infrastructure changes.

```bash
cargo xtask infra plan
```

**Output**: Shows what would change

#### `cargo xtask infra apply`
Apply infrastructure changes.

```bash
cargo xtask infra apply
```

**Warning**: Modifies cloud resources

#### `cargo xtask infra destroy`
Destroy infrastructure.

```bash
cargo xtask infra destroy
```

**Warning**: Deletes all cloud resources

#### `cargo xtask infra output`
Show infrastructure outputs.

```bash
cargo xtask infra output
```

**Shows**: IP addresses, endpoints, etc.

#### `cargo xtask infra env`
Generate production .env from Terraform.

```bash
cargo xtask infra env
```

**Output**: `production.env` file

#### `cargo xtask infra ssh`
SSH into production server.

```bash
cargo xtask infra ssh
```

#### `cargo xtask infra status`
Show infrastructure status.

```bash
cargo xtask infra status
```

**Shows**: Instance IP, database host, S3 endpoint

---

### E2E Testing Commands (`cargo xtask e2e`)

Commands for end-to-end testing.

#### `cargo xtask e2e ci`
Run E2E tests in CI mode (fast, uses fixtures).

```bash
cargo xtask e2e ci
```

**Environment**: `BDP_E2E_MODE=ci`

**Use case**: CI pipeline, fast feedback

#### `cargo xtask e2e real`
Run E2E tests with real downloaded data.

```bash
cargo xtask e2e real
```

**Environment**: `BDP_E2E_MODE=real`

**Requires**: Downloaded test data

#### `cargo xtask e2e download-data`
Download real UniProt test data.

```bash
cargo xtask e2e download-data
```

**Output**: Data in `tests/fixtures/real/`

**Idempotent**: Safe to run multiple times

#### `cargo xtask e2e debug`
Run E2E tests with full debug output.

```bash
cargo xtask e2e debug
```

**Logging**: `RUST_LOG=debug,bdp_server=trace`

#### `cargo xtask e2e clean`
Clean downloaded test data.

```bash
cargo xtask e2e clean
```

**Removes**: `tests/fixtures/real/*` (keeps `.gitkeep`)

#### `cargo xtask e2e info`
Show E2E test data information.

```bash
cargo xtask e2e info
```

**Shows**: File sizes, availability status

---

### Release Commands (`cargo xtask release`)

Commands for version management and releases.

#### `cargo xtask release patch`
Bump patch version (0.1.0 → 0.1.1).

```bash
cargo xtask release patch
```

**Actions**:
1. Updates version in Cargo.toml
2. Syncs to package.json
3. Creates git commit
4. Creates git tag

#### `cargo xtask release minor`
Bump minor version (0.1.0 → 0.2.0).

```bash
cargo xtask release minor
```

#### `cargo xtask release major`
Bump major version (0.1.0 → 1.0.0).

```bash
cargo xtask release major
```

#### `cargo xtask release patch-dry`
Preview patch release changes.

```bash
cargo xtask release patch-dry
```

**Safe**: No changes made

#### `cargo xtask release minor-dry`
Preview minor release changes.

```bash
cargo xtask release minor-dry
```

#### `cargo xtask release bump <version>`
Manually set version number.

```bash
cargo xtask release bump 0.2.5
```

**Note**: No git operations, manual commit required

---

### Utility Commands (`cargo xtask util`)

Miscellaneous utility commands.

#### `cargo xtask util info`
Show environment information.

```bash
cargo xtask util info
```

**Shows**: Versions, URLs, endpoints

#### `cargo xtask util check-db`
Check database connection.

```bash
cargo xtask util check-db
```

**Exit code**: 0 if connected, 1 if failed

#### `cargo xtask util logs`
Show logs from all services.

```bash
cargo xtask util logs
```

#### `cargo xtask util logs-backend`
Follow backend logs.

```bash
cargo xtask util logs-backend
```

#### `cargo xtask util logs-frontend`
Follow frontend logs.

```bash
cargo xtask util logs-frontend
```

#### `cargo xtask util health`
Health check all services.

```bash
cargo xtask util health
```

**Checks**: Backend, frontend, MinIO

#### `cargo xtask util version`
Show current version.

```bash
cargo xtask util version
```

**Shows**: Rust and Node versions

#### `cargo xtask util audit-logs [limit]`
View recent audit logs.

```bash
cargo xtask util audit-logs
cargo xtask util audit-logs 100
```

**Default**: 50 most recent entries

#### `cargo xtask util audit-search <term>`
Search audit logs.

```bash
cargo xtask util audit-search "user_created"
cargo xtask util audit-search "DELETE"
```

#### `cargo xtask util audit-by-resource <type>`
View audit logs for resource type.

```bash
cargo xtask util audit-by-resource organization
cargo xtask util audit-by-resource dataset
```

#### `cargo xtask util audit-by-user <user_id>`
View audit logs for specific user.

```bash
cargo xtask util audit-by-user 550e8400-e29b-41d4-a716-446655440000
```

#### `cargo xtask util audit-trail <type> <id>`
View complete audit trail for resource.

```bash
cargo xtask util audit-trail organization 550e8400-e29b-41d4-a716-446655440000
```

#### `cargo xtask util audit-export [file]`
Export audit logs to JSON.

```bash
cargo xtask util audit-export
cargo xtask util audit-export audit_backup.json
```

**Default**: `audit_logs.json`

#### `cargo xtask util audit-stats`
Show audit log statistics.

```bash
cargo xtask util audit-stats
```

**Shows**: Action counts, resource types, daily stats

---

## Migration from Just

If you're familiar with Just commands, here's the migration mapping:

### Command Mapping

| Just Command | xtask Equivalent |
|--------------|------------------|
| `just setup` | `cargo xtask setup all` |
| `just db-up` | `cargo xtask db up` |
| `just db-down` | `cargo xtask db down` |
| `just db-migrate` | `cargo xtask db migrate` |
| `just db-migrate-add NAME` | `cargo xtask db migrate-add NAME` |
| `just db-shell` | `cargo xtask db shell` |
| `just db-reset` | `cargo xtask db reset` |
| `just dev` | `cargo xtask dev server` |
| `just web` | `cargo xtask dev web` |
| `just dev-all` | `cargo xtask dev all` |
| `just watch` | `cargo xtask dev watch` |
| `just fmt` | `cargo xtask dev fmt` |
| `just lint` | `cargo xtask dev lint` |
| `just fix` | `cargo xtask dev fix` |
| `just test` | `cargo xtask test all` |
| `just test-unit` | `cargo xtask test unit` |
| `just test-integration` | `cargo xtask test integration` |
| `just test-verbose` | `cargo xtask test verbose` |
| `just test-one TEST` | `cargo xtask test one TEST` |
| `just test-coverage` | `cargo xtask test coverage` |
| `just build` | `cargo xtask build workspace` |
| `just build-release` | `cargo xtask build release` |
| `just build-web` | `cargo xtask dev web-build` |
| `just build-all` | `cargo xtask build all` |
| `just sqlx-prepare` | `cargo xtask sqlx prepare` |
| `just sqlx-check` | `cargo xtask sqlx check` |
| `just clean` | `cargo xtask clean workspace` |
| `just clean-all` | `cargo xtask clean all` |
| `just ci` | `cargo xtask ci all` |
| `just docker-up` | `cargo xtask docker up` |
| `just docker-down` | `cargo xtask docker down` |
| `just minio-up` | `cargo xtask minio up` |
| `just info` | `cargo xtask util info` |
| `just health` | `cargo xtask util health` |
| `just verify` | `cargo xtask setup verify` |

### Key Differences

1. **Namespacing**: xtask uses subcommands (modules) for organization
   ```bash
   # Just
   just db-migrate

   # xtask
   cargo xtask db migrate
   ```

2. **Type safety**: xtask commands are compile-time checked
   ```bash
   # Just: typos fail at runtime
   just db-migreate  # Runs, fails with error

   # xtask: typos caught by compiler
   cargo xtask db migreate  # Won't compile
   ```

3. **Help system**: clap provides better help
   ```bash
   # Just
   just --list

   # xtask
   cargo xtask --help
   cargo xtask db --help
   ```

---

## Adding New Tasks

### 1. Basic Task

To add a new task, edit the appropriate module in `xtask/src/`:

**Example**: Add a new database command

```rust
// File: xtask/src/db.rs

#[derive(Debug, Parser)]
pub enum DbCommand {
    // ... existing commands ...

    /// Backup database to file
    Backup {
        /// Output file path
        #[arg(short, long)]
        output: String,
    },
}

pub fn handle(cmd: DbCommand) -> Result<()> {
    match cmd {
        // ... existing handlers ...
        DbCommand::Backup { output } => backup(&output),
    }
}

fn backup(output: &str) -> Result<()> {
    info(&format!("📦 Backing up database to {}...", output));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());

    run("pg_dump", &[&database_url, "-f", output], "Backup database")?;
    success("Database backed up");
    Ok(())
}
```

Now you can use:
```bash
cargo xtask db backup --output backup.sql
```

### 2. Task with Dependencies

Call other tasks:

```rust
fn deploy() -> Result<()> {
    info("🚀 Deploying...");

    // Run other tasks
    crate::test::handle(crate::test::TestCommand::All)?;
    crate::build::handle(crate::build::BuildCommand::Release)?;
    crate::docker::handle(crate::docker::DockerCommand::Up)?;

    success("Deployment complete");
    Ok(())
}
```

### 3. Cross-Platform Task

Handle platform differences:

```rust
fn my_command() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "Running on Windows..."
# PowerShell commands
"#,
            "Windows task",
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "Running on Unix..."
# Bash commands
"#,
            "Unix task",
        )
    }
}
```

### 4. Utility Functions

Use the provided utility functions:

```rust
use crate::utils::*;

fn my_task() -> Result<()> {
    // Logging
    info("Starting task...");
    success("Task succeeded");
    warning("Something to note");
    error("Task failed");

    // Run commands
    run("cargo", &["build"], "Build")?;
    run_streaming("cargo", &["test"], "Test")?;  // Shows live output
    run_in_dir("web", "yarn", &["build"], "Build web")?;

    // File operations
    if path_exists("target") {
        std::fs::remove_dir_all("target")?;
    }

    // Sleep
    sleep(3);  // Cross-platform sleep

    Ok(())
}
```

### 5. Adding a New Module

To add a completely new command module:

1. **Create module file**: `xtask/src/mymodule.rs`
   ```rust
   use crate::utils::*;
   use anyhow::Result;
   use clap::Parser;

   #[derive(Debug, Parser)]
   pub enum MyModuleCommand {
       /// First command
       First,
       /// Second command
       Second { arg: String },
   }

   pub fn handle(cmd: MyModuleCommand) -> Result<()> {
       match cmd {
           MyModuleCommand::First => first(),
           MyModuleCommand::Second { arg } => second(&arg),
       }
   }

   fn first() -> Result<()> {
       info("Running first command");
       Ok(())
   }

   fn second(arg: &str) -> Result<()> {
       info(&format!("Running second command with: {}", arg));
       Ok(())
   }
   ```

2. **Register module**: Edit `xtask/src/main.rs`
   ```rust
   mod mymodule;  // Add module declaration

   #[derive(Parser)]
   enum Command {
       // ... existing commands ...

       /// My module description
       #[command(subcommand)]
       MyModule(mymodule::MyModuleCommand),
   }

   fn main() -> anyhow::Result<()> {
       match cli.command {
           // ... existing handlers ...
           Command::MyModule(cmd) => mymodule::handle(cmd),
       }
   }
   ```

3. **Use it**:
   ```bash
   cargo xtask mymodule first
   cargo xtask mymodule second "test"
   ```

---

## Troubleshooting

### xtask Won't Compile

**Problem**: `cargo xtask` fails with compilation errors

**Solution**:
```bash
# Clean and rebuild xtask
cd xtask
cargo clean
cargo build
cd ..

# Try again
cargo xtask --help
```

### Command Not Found

**Problem**: `cargo xtask db migrate` says command not found

**Solution**: Check the help to see exact command structure
```bash
cargo xtask db --help
```

### Changes Not Taking Effect

**Problem**: Modified xtask code but changes don't apply

**Solution**: Cargo caches the xtask binary. Force rebuild:
```bash
cd xtask
cargo build --release
cd ..
cargo xtask <your-command>
```

Or clean the cache:
```bash
cargo clean -p xtask
cargo xtask <your-command>
```

### Cross-Platform Issues

**Problem**: Task works on Linux/macOS but fails on Windows

**Solution**: Use platform-specific code blocks:
```rust
#[cfg(target_os = "windows")]
{
    run_powershell(r#"
        # PowerShell version
    "#, "Task")
}

#[cfg(not(target_os = "windows"))]
{
    run_bash(r#"
        # Bash version
    "#, "Task")
}
```

Or use cross-platform Rust code:
```rust
use std::fs;
use std::path::Path;

fn my_task() -> Result<()> {
    let path = Path::new("my/path");
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}
```

### Environment Variables Not Loading

**Problem**: `.env` file variables not available

**Solution**: xtask doesn't automatically load `.env`. Load explicitly:
```rust
fn my_task() -> Result<()> {
    dotenv::dotenv().ok();  // Load .env file

    let var = std::env::var("MY_VAR")?;
    // ...
}
```

Or check the Cargo.toml includes `dotenv` dependency.

### Slow Compilation

**Problem**: `cargo xtask` takes long to compile

**Solution**: xtask is compiled once and cached. Subsequent runs are instant unless you modify xtask code.

To reduce compile time:
```bash
# Use release mode for faster execution
cargo build --release --package xtask

# Run from target
./target/release/xtask db up
```

### Permission Issues (psql, docker, etc.)

**Problem**: `cargo xtask db shell` fails with permission denied

**Solution**: Ensure the underlying tools are in PATH and have proper permissions
```bash
# Check if tool is available
which psql
which docker

# On Linux/macOS, add user to docker group
sudo usermod -aG docker $USER
newgrp docker
```

### Windows PowerShell Execution Policy

**Problem**: PowerShell scripts fail with execution policy error

**Solution**: The xtask uses inline PowerShell, which should work. If issues persist:
```powershell
# Run as Administrator
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
```

---

## Best Practices

### 1. Use Structured Logging

```rust
// Good
info("Starting task...");
success("Task complete");
warning("This is deprecated");
error("Task failed");

// Avoid
println!("Starting task...");
```

### 2. Return Results

```rust
// Good
fn my_task() -> Result<()> {
    run("cargo", &["build"], "Build")?;
    Ok(())
}

// Avoid
fn my_task() {
    run("cargo", &["build"], "Build").unwrap();
}
```

### 3. Add Documentation

```rust
/// Backup database to file
///
/// Creates a PostgreSQL dump file for backup purposes.
/// The output file will be overwritten if it exists.
Backup {
    /// Output file path
    #[arg(short, long)]
    output: String,
},
```

### 4. Cross-Platform by Default

Always consider Windows when writing tasks. Use the provided utilities or platform-specific code blocks.

### 5. Idempotent Tasks

Tasks should be safe to run multiple times:
```rust
fn setup() -> Result<()> {
    // Check before creating
    if !path_exists(".env") {
        fs::copy(".env.example", ".env")?;
    }
    Ok(())
}
```

---

## Resources

- [xtask Pattern Discussion](https://github.com/matklad/cargo-xtask)
- [clap Documentation](https://docs.rs/clap)
- [anyhow Documentation](https://docs.rs/anyhow)
- [BDP xtask Source](../xtask/src/)

## Summary

xtask provides BDP with:

- **Type-safe build automation**: Catch errors at compile time
- **No external dependencies**: Part of the Cargo ecosystem
- **IDE support**: Full autocomplete and refactoring
- **Cross-platform**: Consistent experience on all platforms
- **Extensible**: Easy to add new tasks in pure Rust

The xtask pattern is the modern Rust approach to build automation, combining the power of Rust's type system with the convenience of a task runner.

For any questions or issues, refer to this guide or check the source code in `xtask/src/`.
