# SQLx Setup and Configuration Guide

## Table of Contents
- [Introduction](#introduction)
- [SQLx Offline Mode](#sqlx-offline-mode)
- [Environment Variables](#environment-variables)
- [The .sqlx Folder](#the-sqlx-folder)
- [Development Workflow (Online Mode)](#development-workflow-online-mode)
- [CI/CD Workflow (Offline Mode)](#cicd-workflow-offline-mode)
- [cargo sqlx prepare Command](#cargo-sqlx-prepare-command)
- [Testing Setup](#testing-setup)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Introduction

SQLx is a compile-time checked SQL library for Rust. Unlike traditional ORMs, SQLx validates your SQL queries against your actual database schema at compile time, catching errors before runtime. This guide covers the proper setup and usage of SQLx in the BDP project.

## SQLx Offline Mode

SQLx supports two modes of operation:

### Online Mode (Development)
- **What it does**: SQLx connects to a live database to validate queries during compilation
- **When to use**: Local development, when you have a database available
- **Requirements**:
  - Running PostgreSQL instance
  - `DATABASE_URL` environment variable set
  - Network access to database
- **Advantages**:
  - Immediate validation against current schema
  - Automatic query metadata generation
  - Catches schema mismatches early

### Offline Mode (CI/CD, Air-gapped)
- **What it does**: SQLx uses pre-generated metadata files (`.sqlx/*.json`) to validate queries
- **When to use**:
  - Continuous Integration/Deployment pipelines
  - Building in Docker without database access
  - Air-gapped or restricted environments
  - Reproducible builds
- **Requirements**:
  - `SQLX_OFFLINE=true` environment variable
  - Pre-generated `.sqlx/*.json` files committed to repository
- **Advantages**:
  - No database required during compilation
  - Faster builds (no database connection overhead)
  - Works in restricted environments
  - Consistent builds across different environments

## Environment Variables

### DATABASE_URL
- **Purpose**: Connection string for the PostgreSQL database
- **Format**: `postgresql://user:password@host:port/database`
- **Example**: `postgresql://bdp:bdp_dev_password@localhost:5432/bdp`
- **When used**:
  - Development builds (online mode)
  - Running migrations
  - Generating `.sqlx` metadata files
  - Runtime database connections

### TEST_DATABASE_URL
- **Purpose**: Separate database for running tests
- **Format**: Same as `DATABASE_URL`
- **Example**: `postgresql://bdp:bdp_dev_password@localhost:5433/bdp_test`
- **Why separate**:
  - Prevents test data from polluting development database
  - Allows parallel test execution
  - Safe to reset/clean between test runs

### SQLX_OFFLINE
- **Purpose**: Controls SQLx compilation mode
- **Values**:
  - `true`: Use offline mode (requires `.sqlx` files)
  - `false` or unset: Use online mode (requires live database)
- **When to set**:
  - Set to `true` in CI/CD pipelines
  - Set to `true` when building Docker images
  - Leave unset or `false` for local development

### Build Behavior Matrix

| DATABASE_URL | SQLX_OFFLINE | Behavior |
|--------------|--------------|----------|
| Set | `false`/unset | Online mode: validates against live database |
| Set | `true` | Offline mode: uses `.sqlx` files, ignores database |
| Unset | `false`/unset | Build fails: cannot validate queries |
| Unset | `true` | Offline mode: uses `.sqlx` files only |

## The .sqlx Folder

### What is it?
The `.sqlx` folder contains JSON metadata files that describe the structure and types of your SQL queries. Each query in your codebase gets a corresponding JSON file based on a hash of the query text.

### Structure
```
.sqlx/
├── query-<hash1>.json  # Metadata for query 1
├── query-<hash2>.json  # Metadata for query 2
└── query-<hash3>.json  # Metadata for query 3
```

### What's in the files?
Each JSON file contains:
- SQL query text
- Parameter types and names
- Result column types and names
- Whether the query is nullable
- Database type information

### Example
```json
{
  "db_name": "PostgreSQL",
  "query": "SELECT id, name, created_at FROM users WHERE email = $1",
  "describe": {
    "columns": [
      {"ordinal": 0, "name": "id", "type_info": "Uuid"},
      {"ordinal": 1, "name": "name", "type_info": "Text"},
      {"ordinal": 2, "name": "created_at", "type_info": "Timestamptz"}
    ],
    "parameters": {
      "Left": ["Text"]
    },
    "nullable": [false, false, false]
  }
}
```

### Git Management
- **Commit the .sqlx folder**: These files should be committed to version control
- **Why commit**:
  - Enables offline builds in CI/CD
  - Provides documentation of query structure
  - Ensures consistent builds across environments
- **When to regenerate**:
  - After modifying any SQL queries
  - After schema migrations
  - When query validation fails
  - Before committing query changes

## Development Workflow (Online Mode)

### Initial Setup

1. **Start the development database**:
   ```bash
   just db-up
   ```

2. **Set environment variables** (in `.env` file):
   ```bash
   DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp
   SQLX_OFFLINE=false  # or leave unset
   ```

3. **Run migrations**:
   ```bash
   just db-migrate
   ```

4. **Build the project**:
   ```bash
   just build
   ```

### Daily Development

1. **Start services**:
   ```bash
   just dev
   ```

2. **Make code changes**: Edit queries, add new queries, etc.

3. **Test changes**:
   ```bash
   just test
   ```

4. **Build and validate**:
   ```bash
   cargo check  # Quick validation
   just build   # Full build
   ```

### Making Schema Changes

1. **Create a new migration**:
   ```bash
   just db-migrate-add descriptive_name
   # Edit the generated migration file in migrations/
   ```

2. **Apply migration**:
   ```bash
   just db-migrate
   ```

3. **Update queries** as needed in your code

4. **Regenerate .sqlx files** (before committing):
   ```bash
   just sqlx-prepare
   ```

5. **Commit changes**:
   ```bash
   git add migrations/ .sqlx/ src/
   git commit -m "Add new feature with schema changes"
   ```

## CI/CD Workflow (Offline Mode)

### GitHub Actions / GitLab CI

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      # Set offline mode - NO DATABASE REQUIRED
      - name: Set SQLx offline mode
        run: echo "SQLX_OFFLINE=true" >> $GITHUB_ENV

      # Build using pre-committed .sqlx files
      - name: Build
        run: cargo build --verbose

      # For tests, you can either:
      # Option 1: Use offline mode (unit tests only)
      - name: Test (offline)
        run: cargo test --verbose
        env:
          SQLX_OFFLINE: true

      # Option 2: Start a test database for integration tests
      - name: Start PostgreSQL
        run: |
          docker run -d \
            -e POSTGRES_DB=bdp_test \
            -e POSTGRES_USER=bdp \
            -e POSTGRES_PASSWORD=bdp_test_password \
            -p 5432:5432 \
            postgres:16-alpine

      - name: Wait for PostgreSQL
        run: |
          until docker exec $(docker ps -q) pg_isready -U bdp -d bdp_test; do
            sleep 1
          done

      - name: Run migrations
        run: sqlx migrate run
        env:
          DATABASE_URL: postgresql://bdp:bdp_test_password@localhost:5432/bdp_test

      - name: Test (with database)
        run: cargo test --verbose
        env:
          DATABASE_URL: postgresql://bdp:bdp_test_password@localhost:5432/bdp_test
```

### Docker Builds

**Multi-stage Dockerfile with offline mode**:

```dockerfile
FROM rust:1.75 AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Copy SQLx metadata for offline builds
COPY .sqlx/ ./.sqlx/
COPY .cargo/config.toml ./.cargo/config.toml

# Build with offline mode (no database needed)
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/bdp-server /usr/local/bin/
CMD ["bdp-server"]
```

## cargo sqlx prepare Command

### What it does
The `cargo sqlx prepare` command:
1. Connects to your database
2. Scans your Rust code for all SQL queries
3. Validates each query against the current schema
4. Generates/updates `.sqlx/*.json` metadata files
5. Reports any validation errors

### Basic Usage

```bash
# Prepare for all crates in workspace
cargo sqlx prepare

# Prepare for all targets (including tests, benches, examples)
cargo sqlx prepare -- --all-targets

# Prepare for specific crate
cargo sqlx prepare -p bdp-server

# Prepare and check (don't write files, just validate)
cargo sqlx prepare --check
```

### BDP Project Command

We provide a Just command for preparing SQLx metadata:

```bash
just sqlx-prepare
```

This command:
- Ensures database is running
- Executes `cargo sqlx prepare --workspace -- --all-targets`
- Verifies `.sqlx` files were generated
- Provides clear feedback

### When to run

Run `just sqlx-prepare`:

1. **After modifying SQL queries**: When you add, remove, or change any SQL query
2. **After schema migrations**: After running `just db-migrate`
3. **Before committing**: Always regenerate before committing query changes
4. **When offline builds fail**: If CI/CD fails with query validation errors
5. **After pulling changes**: If someone else updated queries or schema
6. **When switching branches**: If the branch has different queries/schema

### Automated Checks

Add a pre-commit hook to ensure `.sqlx` files are up to date:

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check if any Rust files with queries changed
if git diff --cached --name-only | grep -qE '\.rs$'; then
    echo "Rust files changed, verifying .sqlx files are up to date..."

    # Run prepare in check mode
    if ! cargo sqlx prepare --check -- --all-targets 2>/dev/null; then
        echo "Error: .sqlx files are out of date!"
        echo "Run: just sqlx-prepare"
        echo "Then: git add .sqlx/"
        exit 1
    fi
fi
```

## Testing Setup

### Test Database Configuration

BDP uses a separate database for tests to:
- Prevent contamination of development data
- Allow parallel test execution
- Enable safe cleanup between test runs

### Setup Test Database

1. **Start test database** (runs on different port):
   ```bash
   just db-test-up
   ```

2. **Verify it's running**:
   ```bash
   docker compose ps
   ```

3. **Run tests**:
   ```bash
   just test
   ```

### Test Configuration

In your test code:

```rust
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_database_connection() {
    // Reads TEST_DATABASE_URL from environment
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5433/bdp_test".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run your test...
}
```

### Test Database Helpers

```rust
// tests/helpers/database.rs
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

pub async fn cleanup_test_db(pool: &PgPool) {
    // Clean up test data
    sqlx::query!("TRUNCATE users, organizations CASCADE")
        .execute(pool)
        .await
        .expect("Failed to cleanup database");
}
```

## Troubleshooting

### Problem: Compilation fails with "error: no such column"

**Cause**: Query references a column that doesn't exist in the database schema.

**Solutions**:
1. Check if migrations are up to date: `sqlx migrate run`
2. Verify `DATABASE_URL` points to correct database
3. Check if column name is spelled correctly
4. Ensure you're connected to the right database (dev vs test)

### Problem: Compilation fails with "error connecting to database"

**Cause**: SQLx cannot connect to database during compilation (online mode).

**Solutions**:
1. Verify database is running: `docker ps | grep postgres`
2. Check `DATABASE_URL` is set correctly: `echo $DATABASE_URL`
3. Test connection: `psql $DATABASE_URL -c "SELECT 1"`
4. Use offline mode if database not available: `export SQLX_OFFLINE=true`
5. Check firewall/network settings

### Problem: CI/CD build fails with "query metadata not found"

**Cause**: `.sqlx` files not committed or out of date.

**Solutions**:
1. Run locally: `just sqlx-prepare`
2. Commit generated files: `git add .sqlx/ && git commit`
3. Verify `.sqlx/` is not in `.gitignore`
4. Ensure `SQLX_OFFLINE=true` is set in CI/CD

### Problem: Offline mode fails with "query changed"

**Cause**: SQL query text changed but `.sqlx` files not regenerated.

**Solutions**:
1. Regenerate metadata: `just sqlx-prepare`
2. Commit updated files: `git add .sqlx/`
3. Note: Even whitespace changes in queries require regeneration

### Problem: "migration checksum mismatch"

**Cause**: Migration files were modified after being applied.

**Solutions**:
1. **Never modify applied migrations** - create a new migration instead
2. If in development and safe to reset:
   ```bash
   sqlx database drop
   sqlx database create
   sqlx migrate run
   ```
3. For production: create a new migration to fix the issue

### Problem: ".sqlx files contain absolute paths"

**Cause**: Older versions of sqlx-cli generated absolute paths.

**Solutions**:
1. Update sqlx-cli: `cargo install sqlx-cli --force`
2. Regenerate: `just sqlx-clean && just sqlx-prepare`
3. Use workspace-relative paths in newer versions

### Problem: "too many open connections" during tests

**Cause**: Test connection pool too large or not properly closed.

**Solutions**:
1. Reduce max connections in tests: `.max_connections(5)`
2. Ensure proper cleanup: close pools after tests
3. Use test isolation: separate databases per test suite
4. Increase PostgreSQL connection limit (if needed)

### Problem: Schema drift between dev and CI

**Cause**: Migrations not applied consistently.

**Solutions**:
1. Always run migrations before prepare: `sqlx migrate run`
2. Include migration hash in CI cache key
3. Document migration process in README
4. Use identical PostgreSQL versions in dev and CI

## Best Practices

### 1. Always Use Prepared Statements

```rust
// GOOD: Type-safe, compile-time checked
let user = sqlx::query_as!(
    User,
    "SELECT id, name, email FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;

// AVOID: Not type-checked at compile time
let user = sqlx::query("SELECT id, name, email FROM users WHERE id = $1")
    .bind(user_id)
    .fetch_one(&pool)
    .await?;
```

### 2. Keep Migrations Small and Focused

Each migration should:
- Do one thing (add table, add column, etc.)
- Be reversible when possible
- Include comments explaining the purpose
- Be tested before merging

### 3. Regenerate .sqlx Before Committing

```bash
# Before committing query changes
just sqlx-prepare
git add .sqlx/
git commit -m "Add user search query"
```

### 4. Use Offline Mode in CI/CD

```yaml
# Always set in CI/CD pipelines
env:
  SQLX_OFFLINE: true
```

### 5. Separate Test and Development Databases

```bash
# Development
DATABASE_URL=postgresql://bdp:password@localhost:5432/bdp

# Testing
TEST_DATABASE_URL=postgresql://bdp:password@localhost:5433/bdp_test
```

### 6. Use Query Macros for Complex Queries

```rust
// For complex queries with many columns
sqlx::query_as!(
    UserWithOrg,
    r#"
    SELECT
        u.id,
        u.name,
        u.email,
        o.id as "org_id",
        o.name as "org_name"
    FROM users u
    LEFT JOIN organizations o ON u.org_id = o.id
    WHERE u.id = $1
    "#,
    user_id
)
```

### 7. Handle Nullable Columns Properly

```rust
// Use Option<T> for nullable columns
struct User {
    id: Uuid,
    name: String,
    email: String,
    bio: Option<String>,  // Nullable column
}

// Query with explicit NULL handling
sqlx::query_as!(
    User,
    r#"
    SELECT
        id,
        name,
        email,
        bio as "bio?"  -- Mark as nullable with ?
    FROM users
    WHERE id = $1
    "#,
    user_id
)
```

### 8. Use Transactions for Related Operations

```rust
let mut tx = pool.begin().await?;

// Insert user
let user_id = sqlx::query!(
    "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id",
    name,
    email
)
.fetch_one(&mut *tx)
.await?
.id;

// Insert user profile
sqlx::query!(
    "INSERT INTO profiles (user_id, bio) VALUES ($1, $2)",
    user_id,
    bio
)
.execute(&mut *tx)
.await?;

tx.commit().await?;
```

### 9. Document Database Requirements

In your README.md:

```markdown
## Database Setup

1. Ensure PostgreSQL 16+ is running
2. Set DATABASE_URL in .env
3. Run migrations: `sqlx migrate run`
4. For tests, also set TEST_DATABASE_URL
```

### 10. Version Lock SQLx

In `Cargo.toml`:

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "uuid", "chrono", "json"] }
```

### 11. Use Feature Flags Wisely

```toml
# Only include what you need
sqlx = {
    version = "0.7",
    features = [
        "runtime-tokio-native-tls",  # Async runtime
        "postgres",                   # Database driver
        "uuid",                       # UUID support
        "chrono",                     # Date/time support
        "json",                       # JSON support
        "migrate"                     # Migration support
    ]
}
```

### 12. Monitor Query Performance

```rust
use tracing::instrument;

#[instrument(skip(pool))]
async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User> {
    sqlx::query_as!(
        User,
        "SELECT id, name, email FROM users WHERE id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
}
```

### 13. Use Connection Pooling Appropriately

```rust
// Configure pool based on workload
let pool = PgPoolOptions::new()
    .max_connections(20)           // Limit concurrent connections
    .min_connections(5)            // Keep minimum ready
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

### 14. Validate Migrations in CI

```yaml
- name: Check migrations
  run: |
    sqlx migrate run
    cargo sqlx prepare --check
```

### 15. Use Type Overrides When Needed

```rust
// Override SQLx's type inference
struct User {
    id: Uuid,
    // Force interpretation as custom type
    metadata: sqlx::types::Json<UserMetadata>,
}

sqlx::query_as!(
    User,
    r#"
    SELECT
        id,
        metadata as "metadata: Json<UserMetadata>"
    FROM users
    WHERE id = $1
    "#,
    user_id
)
```

## Additional Resources

- [SQLx Documentation](https://docs.rs/sqlx/)
- [SQLx GitHub Repository](https://github.com/launchbadge/sqlx)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [BDP Contributing Guide](../../CONTRIBUTING.md)

## Getting Help

- Check this guide first
- Review BDP documentation in `docs/`
- Check existing issues on GitHub
- Ask in team chat or create an issue
- Consult SQLx documentation for specific features
