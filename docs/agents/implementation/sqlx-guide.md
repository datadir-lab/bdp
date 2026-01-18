# SQLx Comprehensive Guide for AI Agents

This guide provides complete reference documentation for working with SQLx in the BDP project.

## Table of Contents

1. [SQLx Architecture Overview](#sqlx-architecture-overview)
2. [Compile-Time vs Runtime Query Checking](#compile-time-vs-runtime-query-checking)
3. [Query Macro Decision Tree](#query-macro-decision-tree)
4. [Offline Mode Deep Dive](#offline-mode-deep-dive)
5. [.sqlx Folder Structure](#sqlx-folder-structure)
6. [DATABASE_URL Precedence Rules](#database_url-precedence-rules)
7. [Migration Workflow](#migration-workflow)
8. [Testing Patterns](#testing-patterns)
9. [Common Pitfalls and Solutions](#common-pitfalls-and-solutions)
10. [Troubleshooting Compilation Errors](#troubleshooting-compilation-errors)
11. [Performance Considerations](#performance-considerations)
12. [Security Best Practices](#security-best-practices)

---

## SQLx Architecture Overview

### What is SQLx?

SQLx is a Rust SQL toolkit that provides:
- **Compile-time checked queries** - Queries are validated against your database schema at compile time
- **Async/await support** - Built on tokio for high-performance async I/O
- **Type safety** - Automatic mapping between SQL types and Rust types
- **Migration management** - Built-in migration runner
- **Multiple database support** - PostgreSQL, MySQL, SQLite, MSSQL

### How SQLx Works

```
┌─────────────────────────────────────────────────────────────┐
│                    Development Workflow                      │
└─────────────────────────────────────────────────────────────┘

1. Write SQL Query with Macro
   ┌────────────────────────────────────────────────────────┐
   │ sqlx::query_as!(User, "SELECT * FROM users WHERE...")  │
   └────────────────────────────────────────────────────────┘
                          │
                          ▼
2. SQLx Connects to Database (compile-time)
   ┌────────────────────────────────────────────────────────┐
   │ • Reads DATABASE_URL from environment                  │
   │ • Executes PREPARE statement                           │
   │ • Gets column types, names, nullability                │
   └────────────────────────────────────────────────────────┘
                          │
                          ▼
3. Macro Expansion (compile-time)
   ┌────────────────────────────────────────────────────────┐
   │ • Validates SQL syntax                                 │
   │ • Checks types match Rust struct                       │
   │ • Generates optimized query code                       │
   └────────────────────────────────────────────────────────┘
                          │
                          ▼
4. Runtime Execution
   ┌────────────────────────────────────────────────────────┐
   │ • Query runs against database                          │
   │ • Results automatically mapped to Rust types           │
   └────────────────────────────────────────────────────────┘
```

### BDP Project Configuration

**Current setup:**
- Database: SQLite (early development) → PostgreSQL (production)
- SQLx version: 0.8
- Features enabled: `runtime-tokio`, `sqlite`, `migrate`
- Location: `crates/bdp-server/Cargo.toml`

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }
```

---

## Compile-Time vs Runtime Query Checking

### Compile-Time Checking (Default)

**Pros:**
- Catches SQL errors before runtime
- Type mismatches caught at compile time
- Better IDE autocomplete
- No runtime overhead for query validation

**Cons:**
- Requires database connection during compilation
- CI/CD needs database access OR offline mode
- Schema changes require recompilation

**Example:**
```rust
// Compile-time checked - will fail compilation if 'name' column doesn't exist
let users = sqlx::query_as!(User, "SELECT id, name FROM users")
    .fetch_all(&pool)
    .await?;
```

### Runtime Query Building

**Pros:**
- No database needed at compile time
- Dynamic queries possible
- Faster compilation

**Cons:**
- SQL errors only discovered at runtime
- No compile-time type checking
- Manual type mapping required

**Example:**
```rust
// Runtime query - errors only at runtime
let users = sqlx::query_as::<_, User>("SELECT id, name FROM users")
    .fetch_all(&pool)
    .await?;
```

### Decision Matrix

```
┌─────────────────────────────────────────────────────────────┐
│              When to Use Each Approach                       │
└─────────────────────────────────────────────────────────────┘

Use Compile-Time (query!, query_as!):
  ✓ Static queries with known schema
  ✓ Core business logic
  ✓ Safety-critical queries
  ✓ Production code
  ✓ When you can use offline mode

Use Runtime (query, query_as):
  ✓ Dynamic query building
  ✓ Queries with variable column selection
  ✓ Rapid prototyping
  ✓ Complex WHERE clauses built at runtime
  ✗ Avoid in production where possible
```

---

## Query Macro Decision Tree

### Available Macros

1. **`query!`** - Raw query with compile-time checking
2. **`query_as!`** - Query mapped to struct with compile-time checking
3. **`query`** - Raw query (runtime)
4. **`query_as`** - Query mapped to struct (runtime)
5. **`query_scalar!`** - Single value query (compile-time)
6. **`query_file!`** - Query from SQL file (compile-time)

### Decision Tree

```
START
  │
  ├─ Need compile-time type safety? ──NO──> Use query() or query_as()
  │                                          (Consider using macros instead!)
  │
 YES
  │
  ├─ Return single scalar value? ──YES──> Use query_scalar!()
  │
  NO
  │
  ├─ Map to Rust struct? ──YES──┬─ Struct defined? ──YES──> Use query_as!()
  │                             │
  │                             └─ NO ──> Define struct OR use query!()
  │
  NO
  │
  └─> Use query!() for anonymous record
```

### Usage Examples

#### 1. `query!()` - Returns anonymous record

```rust
// Returns anonymous struct with fields matching columns
let row = sqlx::query!("SELECT id, name FROM users WHERE id = ?", user_id)
    .fetch_one(&pool)
    .await?;

println!("ID: {}, Name: {}", row.id, row.name);
```

**When to use:**
- One-off queries
- Simple SELECT statements
- Don't want to create a struct

#### 2. `query_as!()` - Maps to named struct

```rust
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: i64,
    name: String,
    email: Option<String>,
}

// Maps directly to User struct
let user = sqlx::query_as!(User, "SELECT id, name, email FROM users WHERE id = ?", user_id)
    .fetch_one(&pool)
    .await?;
```

**When to use:**
- Returning domain models
- Multiple queries returning same type
- Need struct for business logic
- **MOST COMMON PATTERN IN BDP**

#### 3. `query_scalar!()` - Single value

```rust
// Returns single value
let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
    .fetch_one(&pool)
    .await?;

let name: String = sqlx::query_scalar!("SELECT name FROM users WHERE id = ?", user_id)
    .fetch_one(&pool)
    .await?;
```

**When to use:**
- COUNT queries
- EXISTS checks
- Single column returns
- Aggregate functions

#### 4. `query_file!()` - SQL in separate file

```rust
// SQL in queries/get_user.sql
let user = sqlx::query_file_as!(User, "queries/get_user.sql", user_id)
    .fetch_one(&pool)
    .await?;
```

**When to use:**
- Complex queries (>5 lines)
- Queries shared across modules
- Better SQL syntax highlighting
- **RARELY USED IN BDP - prefer inline for simplicity**

#### 5. Runtime variants (no `!`)

```rust
// No compile-time checking
let users = sqlx::query_as::<_, User>("SELECT * FROM users")
    .fetch_all(&pool)
    .await?;
```

**When to use:**
- Dynamic query building
- Development/prototyping
- Cannot use offline mode
- **AVOID IN PRODUCTION**

---

## Offline Mode Deep Dive

### What is Offline Mode?

Offline mode allows SQLx to perform compile-time query checking WITHOUT a database connection. It uses pre-generated metadata stored in `.sqlx/` directory.

### Why Use Offline Mode?

```
Problem: CI/CD needs database for compile-time checking
  ├─ Option 1: Spin up database in CI ──> Slow, complex, fragile
  │
  └─ Option 2: Use offline mode ──> Fast, simple, reliable ✓
```

### Enabling Offline Mode

**Method 1: Environment variable**
```bash
export SQLX_OFFLINE=true
cargo build
```

**Method 2: Cargo feature flag**
```toml
[dependencies]
sqlx = { version = "0.8", features = ["offline"] }
```

**Method 3: cargo-sqlx config**
```toml
# .cargo/config.toml
[env]
SQLX_OFFLINE = "true"
```

### Generating Offline Data

**Step 1: Ensure DATABASE_URL is set**
```bash
export DATABASE_URL="postgresql://user:pass@localhost/bdp"
```

**Step 2: Run sqlx prepare**
```bash
# Install cargo-sqlx if not already installed
cargo install sqlx-cli --no-default-features --features postgres

# Generate .sqlx files
cargo sqlx prepare

# Or for specific package
cargo sqlx prepare --workspace
```

**Expected output:**
```
query data written to `.sqlx` in the current directory; please check this into version control
```

**Step 3: Commit .sqlx files**
```bash
git add .sqlx/
git commit -m "chore: update sqlx offline data"
```

### When to Regenerate

```
Regenerate .sqlx files when:
  ✓ Database schema changes (migrations)
  ✓ New queries added
  ✓ Query text modified
  ✓ Struct definitions changed
  ✗ Only code logic changes (no SQL/schema)
```

### Offline Mode in CI/CD

**GitHub Actions example:**
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      # No database needed!
      - name: Build with offline mode
        run: cargo build
        env:
          SQLX_OFFLINE: true

      - name: Run tests
        run: cargo test
        env:
          SQLX_OFFLINE: true
```

---

## .sqlx Folder Structure

### Directory Layout

```
project/
├── .sqlx/
│   ├── query-<hash1>.json          # Query metadata
│   ├── query-<hash2>.json
│   ├── query-<hash3>.json
│   └── ...
├── migrations/
│   └── ...
└── src/
    └── ...
```

### Query Metadata File Format

**File: `.sqlx/query-<hash>.json`**

```json
{
  "db_name": "PostgreSQL",
  "query": "SELECT id, name, email FROM users WHERE id = $1",
  "describe": {
    "columns": [
      {
        "ordinal": 0,
        "name": "id",
        "type_info": "Int8"
      },
      {
        "ordinal": 1,
        "name": "name",
        "type_info": "Text"
      },
      {
        "ordinal": 2,
        "name": "email",
        "type_info": "Text"
      }
    ],
    "parameters": {
      "Left": [
        "Int8"
      ]
    },
    "nullable": [
      false,
      false,
      true
    ]
  },
  "hash": "abc123def456..."
}
```

### Key Fields Explained

- **`db_name`**: Database type (PostgreSQL, SQLite, etc.)
- **`query`**: The exact SQL query text
- **`columns`**: Column metadata (name, type, order)
- **`parameters`**: Query parameter types
- **`nullable`**: Which columns can be NULL
- **`hash`**: Content hash of the query

### Hash Generation

```
Hash is generated from:
  1. Database name
  2. SQL query text (normalized)
  3. Parameter types

Changes to ANY of these trigger new hash → new file
```

### Version Control

**DO commit:**
- All `.sqlx/query-*.json` files
- Keep in sync with schema

**DO NOT commit:**
- Database files (*.db, *.sqlite)
- Connection strings

**Gitignore example:**
```gitignore
# Databases
*.db
*.sqlite
*.sqlite3

# Environment
.env
.env.local

# Keep .sqlx for offline mode
# (don't add .sqlx/ to .gitignore!)
```

---

## DATABASE_URL Precedence Rules

### Resolution Order

SQLx looks for DATABASE_URL in this order:

```
1. Environment variable (highest priority)
   └─> export DATABASE_URL="..."

2. .env file in current directory
   └─> .env contains DATABASE_URL=...

3. .env file in parent directories (recursive)
   └─> Searches up directory tree

4. Compile-time environment (build.rs)
   └─> Built into binary at compile time

5. No URL found → ERROR (lowest priority)
```

### Examples

**Scenario 1: Development with .env**
```bash
# .env
DATABASE_URL=postgresql://localhost/bdp_dev

# This works
cargo sqlx prepare
```

**Scenario 2: Override with environment**
```bash
# .env has one URL, but override with env var
export DATABASE_URL=postgresql://localhost/bdp_test
cargo sqlx prepare  # Uses bdp_test, not bdp_dev
```

**Scenario 3: CI with no database**
```bash
# CI environment - no DATABASE_URL needed
export SQLX_OFFLINE=true
cargo build  # Uses .sqlx/ files
```

### BDP Project Setup

**Development:**
```bash
# .env
DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp
```

**Testing:**
```bash
# Override for test database
export DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp_test
cargo test
```

**Production:**
```bash
# Use environment variable (never .env in production)
export DATABASE_URL=postgresql://user:pass@prod-db/bdp
cargo run --release
```

### Security Considerations

```
✗ NEVER commit .env with real credentials
✗ NEVER hardcode DATABASE_URL in code
✓ Use .env.example for documentation
✓ Use environment variables in production
✓ Use secrets management (Vault, AWS Secrets Manager)
```

---

## Migration Workflow

### Migration File Structure

```
migrations/
├── 20260116000001_initial_schema.sql
├── 20260116000002_add_users_table.sql
├── 20260116000003_add_indexes.sql
└── ...

Format: {timestamp}_{description}.sql
```

### Creating Migrations

**Using sqlx-cli (recommended):**
```bash
# Create new migration
sqlx migrate add <description>

# Example
sqlx migrate add add_users_table

# Creates:
# migrations/20260116123456_add_users_table.sql
```

**Manual creation:**
```bash
# Create file with timestamp
touch migrations/$(date +%Y%m%d%H%M%S)_description.sql
```

### Migration File Contents

**Up migration (in the .sql file):**
```sql
-- Add up migration script here

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

**Down migration (separate file, optional):**
```sql
-- migrations/20260116123456_add_users_table.down.sql

DROP INDEX IF EXISTS idx_users_email;
DROP TABLE IF EXISTS users;
```

### Running Migrations

**Run all pending:**
```bash
sqlx migrate run
```

**Revert last migration:**
```bash
sqlx migrate revert
```

**Check migration status:**
```bash
sqlx migrate info
```

**Example output:**
```
Applied At                  | Version | Description
============================|=========|================
2026-01-16 12:00:00.000000 | 1       | initial_schema
2026-01-16 12:05:00.000000 | 2       | add_users_table
(pending)                   | 3       | add_indexes
```

### Programmatic Migrations (Runtime)

**In BDP server code:**
```rust
use sqlx::migrate::Migrator;

// Run migrations on startup
pub async fn init(config: &Config) -> anyhow::Result<Storage> {
    let pool = SqlitePoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    // Run migrations from migrations/ directory
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(Storage::new(pool))
}
```

**Location in BDP:** `crates/bdp-server/src/storage/mod.rs`

### Migration Best Practices

```
DO:
  ✓ Use descriptive names
  ✓ One logical change per migration
  ✓ Test migrations on copy of production data
  ✓ Make migrations idempotent (IF NOT EXISTS)
  ✓ Add indexes for foreign keys
  ✓ Include down migrations for development

DON'T:
  ✗ Edit applied migrations (create new one)
  ✗ Delete migration files
  ✗ Mix schema and data changes
  ✗ Use database-specific syntax (if possible)
  ✗ Forget to update .sqlx after schema changes
```

### Migration Testing Workflow

```bash
# 1. Create migration
sqlx migrate add new_feature

# 2. Edit migration file
vim migrations/20260116123456_new_feature.sql

# 3. Test up migration
sqlx migrate run

# 4. Test your queries
cargo test

# 5. Test down migration (if exists)
sqlx migrate revert

# 6. Re-apply
sqlx migrate run

# 7. Update .sqlx data
cargo sqlx prepare

# 8. Commit everything
git add migrations/ .sqlx/
git commit -m "feat: add new_feature migration"
```

---

## Testing Patterns

### Test Database Setup

**Option 1: Separate test database**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect(":memory:")  // In-memory SQLite for tests
            .await
            .unwrap();

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_user() {
        let pool = setup_test_db().await;

        let result = sqlx::query!(
            "INSERT INTO users (name, email) VALUES (?, ?)",
            "Test User",
            "test@example.com"
        )
        .execute(&pool)
        .await;

        assert!(result.is_ok());
    }
}
```

**Option 2: Database per test (isolation)**
```rust
async fn create_test_db() -> SqlitePool {
    let db_url = format!("test_{}.db", uuid::Uuid::new_v4());
    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn isolated_test() {
    let pool = create_test_db().await;
    // Test runs on fresh database
}
```

### Testing Query Logic

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_datasets() {
        let pool = setup_test_db().await;
        let storage = Storage::new(pool);

        // Insert test data
        sqlx::query!("INSERT INTO datasets (name) VALUES (?)", "test-dataset")
            .execute(storage.pool())
            .await
            .unwrap();

        // Test query
        let datasets = storage.list_datasets().await.unwrap();

        assert_eq!(datasets.len(), 1);
        assert_eq!(datasets[0].name, "test-dataset");
    }
}
```

### Mocking vs Real Database

```
Use Real Database Tests When:
  ✓ Testing SQL logic
  ✓ Integration tests
  ✓ Testing migrations
  ✓ Performance testing
  ✓ BDP standard approach

Use Mocks When:
  ✓ Unit testing business logic
  ✓ Testing error handling
  ✓ External dependencies
  ✗ Rarely needed with SQLx
```

### Test Fixtures

```rust
// tests/fixtures/mod.rs
pub async fn create_test_user(pool: &SqlitePool, name: &str) -> i64 {
    sqlx::query_scalar!(
        "INSERT INTO users (name, email) VALUES (?, ?) RETURNING id",
        name,
        format!("{}@test.com", name)
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

// In test
#[tokio::test]
async fn test_with_fixture() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "Alice").await;

    // Test with user_id
}
```

---

## Common Pitfalls and Solutions

### Pitfall 1: Missing DATABASE_URL

**Error:**
```
error: DATABASE_URL must be set to use query macros
```

**Solution:**
```bash
# Development: Create .env file
echo "DATABASE_URL=postgresql://localhost/bdp" > .env

# CI: Use offline mode
export SQLX_OFFLINE=true
```

### Pitfall 2: Schema Mismatch

**Error:**
```
error: mismatched types
  expected struct `User`, found struct `User`

note: column `email` is TEXT (nullable) but struct field is String (non-nullable)
```

**Root cause:** Database schema doesn't match Rust struct

**Solution:**
```rust
// Either make field optional in Rust
struct User {
    email: Option<String>,  // Matches nullable column
}

// Or make column NOT NULL
// ALTER TABLE users ALTER COLUMN email SET NOT NULL;
```

### Pitfall 3: Stale .sqlx Data

**Error:**
```
error: column 'new_column' does not exist
```

**Root cause:** Added column in migration but didn't update .sqlx

**Solution:**
```bash
# Regenerate offline data
cargo sqlx prepare
git add .sqlx/
git commit -m "chore: update sqlx offline data"
```

### Pitfall 4: Wrong Placeholder Syntax

**Error:**
```
error: expected `?` or `$N` placeholder, found `:name`
```

**Root cause:** Different databases use different placeholders

**Solution:**
```rust
// PostgreSQL: $1, $2, $3
sqlx::query!("SELECT * FROM users WHERE id = $1", user_id)

// SQLite: ?
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)

// MySQL: ?
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)
```

### Pitfall 5: Type Inference Failures

**Error:**
```
error: type annotations needed
```

**Root cause:** Rust can't infer return type

**Solution:**
```rust
// Explicit type annotation
let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
    .fetch_one(&pool)
    .await?;

// Or use turbofish
let count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
    .fetch_one(&pool)
    .await?;
let _: i64 = count;
```

### Pitfall 6: Forgetting to Commit .sqlx Files

**Symptom:** CI fails with schema errors

**Solution:**
```bash
# Always commit .sqlx with schema changes
git add .sqlx/ migrations/
git commit -m "feat: add new table with offline data"
```

### Pitfall 7: Using Wrong Fetch Method

**Error:**
```
error: query returned no rows
```

**Cause:** Used `fetch_one()` but query returned 0 or >1 rows

**Solution:**
```rust
// Expects exactly 1 row, errors if 0 or >1
fetch_one()   // Use for: SELECT ... WHERE id = ?

// Returns Option<Row>, None if 0 rows
fetch_optional()  // Use for: SELECT ... WHERE email = ?

// Returns Vec<Row>
fetch_all()   // Use for: SELECT ... (multiple rows)

// Returns impl Stream<Item = Result<Row>>
fetch()       // Use for: Large result sets (streaming)
```

---

## Troubleshooting Compilation Errors

### Error Categories

#### 1. DATABASE_URL Errors

**Error:**
```
error: DATABASE_URL must be set
```

**Diagnosis:**
```bash
# Check if DATABASE_URL is set
echo $DATABASE_URL

# Check .env file exists
cat .env | grep DATABASE_URL
```

**Fix:**
```bash
# Option 1: Set environment variable
export DATABASE_URL="postgresql://localhost/bdp"

# Option 2: Create .env file
echo "DATABASE_URL=postgresql://localhost/bdp" > .env

# Option 3: Use offline mode
export SQLX_OFFLINE=true
```

#### 2. Connection Errors

**Error:**
```
error connecting to database: Connection refused
```

**Diagnosis:**
```bash
# Check if database is running
pg_isready -h localhost -p 5432

# For PostgreSQL
systemctl status postgresql

# For Docker
docker ps | grep postgres
```

**Fix:**
```bash
# Start database
docker-compose up -d postgres

# Or local PostgreSQL
systemctl start postgresql
```

#### 3. Schema Mismatch Errors

**Error:**
```
error: column "x" does not exist
```

**Diagnosis:**
```bash
# Check applied migrations
sqlx migrate info

# Check database schema
psql -U bdp -d bdp -c "\d users"
```

**Fix:**
```bash
# Run pending migrations
sqlx migrate run

# Regenerate .sqlx
cargo sqlx prepare
```

#### 4. Type Mismatch Errors

**Error:**
```
error: mismatched types
  expected `String`, found `Option<String>`
```

**Diagnosis:** Column is nullable but struct field is not

**Fix:**
```rust
// Before (error)
struct User {
    email: String,  // NOT NULL expected
}

// After (fixed)
struct User {
    email: Option<String>,  // Nullable
}
```

#### 5. Offline Mode Errors

**Error:**
```
error: offline mode enabled but no cached data found
```

**Diagnosis:** `.sqlx/` directory missing or stale

**Fix:**
```bash
# Disable offline mode temporarily
unset SQLX_OFFLINE

# Regenerate
cargo sqlx prepare

# Re-enable
export SQLX_OFFLINE=true
```

### Debug Workflow

```
1. Read error message carefully
   └─> Note file, line, and exact error text

2. Identify category
   ├─> Connection → Check database
   ├─> Schema → Check migrations
   ├─> Type → Check struct vs schema
   └─> Offline → Check .sqlx/

3. Verify environment
   ├─> echo $DATABASE_URL
   ├─> echo $SQLX_OFFLINE
   └─> cat .env

4. Check database state
   ├─> sqlx migrate info
   ├─> psql/sqlite3 schema inspection
   └─> Compare with migrations/

5. Regenerate if needed
   ├─> cargo sqlx prepare
   └─> cargo clean && cargo build

6. Test fix
   └─> cargo check
```

### Useful Debug Commands

```bash
# Check SQLx version
cargo tree | grep sqlx

# Verbose compilation
RUST_BACKTRACE=1 cargo build

# Check macro expansion
cargo expand --bin bdp-server

# Force rebuild
cargo clean && cargo build

# Verify .sqlx integrity
ls -la .sqlx/
cat .sqlx/query-*.json | jq .
```

---

## Performance Considerations

### Connection Pooling

**Optimal pool size:**
```rust
// Too small: connection starvation
// Too large: resource waste

// Formula: (CPU cores * 2) + disk spindles
// For most apps: 10-50 connections

let pool = SqlitePoolOptions::new()
    .max_connections(20)  // Adjust based on load
    .min_connections(5)   // Keep warm connections
    .connect(&database_url)
    .await?;
```

### Query Optimization

**Batch queries:**
```rust
// BAD: N+1 queries
for user_id in user_ids {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = ?", user_id)
        .fetch_one(&pool)
        .await?;
}

// GOOD: Single query
let users = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE id = ANY($1)",
    &user_ids[..]
)
.fetch_all(&pool)
.await?;
```

**Use indexes:**
```sql
-- Slow: Full table scan
SELECT * FROM users WHERE email = 'user@example.com';

-- Fast: Index scan
CREATE INDEX idx_users_email ON users(email);
```

**Limit result sets:**
```rust
// Fetch only what you need
sqlx::query_as!(User, "SELECT id, name FROM users LIMIT 100")
    .fetch_all(&pool)
    .await?;
```

### Streaming Large Results

```rust
use futures::TryStreamExt;

// BAD: Loads all rows into memory
let users = sqlx::query_as!(User, "SELECT * FROM users")
    .fetch_all(&pool)  // Could be GBs of data!
    .await?;

// GOOD: Stream rows
let mut stream = sqlx::query_as!(User, "SELECT * FROM users")
    .fetch(&pool);

while let Some(user) = stream.try_next().await? {
    process_user(user).await?;
    // Only one row in memory at a time
}
```

### Prepared Statements

**SQLx automatically prepares statements:**
```rust
// First execution: Prepared
// Subsequent: Uses cached prepared statement
for i in 0..1000 {
    sqlx::query!("SELECT * FROM users WHERE id = ?", i)
        .fetch_one(&pool)
        .await?;
}
```

### Transaction Batching

```rust
// Wrap multiple operations in transaction
let mut tx = pool.begin().await?;

for user in users {
    sqlx::query!("INSERT INTO users (name) VALUES (?)", user.name)
        .execute(&mut *tx)
        .await?;
}

tx.commit().await?;  // Single commit
```

### Performance Monitoring

```rust
// Enable query logging
tracing_subscriber::fmt::init();

// Set log level
export RUST_LOG=sqlx=debug

// Logs will show:
// - Query execution time
// - Rows returned
// - Connection pool stats
```

---

## Security Best Practices

### SQL Injection Prevention

**ALWAYS use parameterized queries:**

```rust
// ✓ SAFE: Parameterized
let user_id = 42;
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)
    .fetch_one(&pool)
    .await?;

// ✗ UNSAFE: String interpolation
let user_id = "42; DROP TABLE users--";
sqlx::query!(&format!("SELECT * FROM users WHERE id = {}", user_id))
    .fetch_one(&pool)
    .await?;  // DANGER: SQL injection!
```

**SQLx macros prevent SQL injection by design:**
```rust
// This won't compile - macros require compile-time string literals
let column = "id";
sqlx::query!("SELECT {} FROM users", column)  // ERROR!

// Correct: Use parameterized WHERE clause
sqlx::query!("SELECT id FROM users WHERE id = ?", user_id)
```

### Input Validation

```rust
// Validate before querying
fn validate_email(email: &str) -> Result<(), Error> {
    if !email.contains('@') {
        return Err(Error::InvalidEmail);
    }
    Ok(())
}

// Use in query
validate_email(&user_input)?;
sqlx::query!("INSERT INTO users (email) VALUES (?)", user_input)
    .execute(&pool)
    .await?;
```

### Secrets Management

**NEVER:**
```rust
// ✗ Hardcoded credentials
let pool = SqlitePoolOptions::new()
    .connect("postgresql://admin:password123@localhost/db")
    .await?;

// ✗ Committed .env
git add .env  // Contains DATABASE_URL with password
```

**ALWAYS:**
```rust
// ✓ Use environment variables
let database_url = std::env::var("DATABASE_URL")
    .expect("DATABASE_URL must be set");

let pool = SqlitePoolOptions::new()
    .connect(&database_url)
    .await?;

// ✓ Use .env.example for documentation
# .env.example
DATABASE_URL=postgresql://user:password@localhost/dbname

// ✓ Add .env to .gitignore
.env
.env.local
```

### Least Privilege Principle

```sql
-- Create dedicated application user
CREATE USER bdp_app WITH PASSWORD 'secure_password';

-- Grant only necessary permissions
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO bdp_app;

-- Don't grant:
-- ✗ SUPERUSER
-- ✗ CREATEDB
-- ✗ DROP TABLE
```

### Connection String Security

```bash
# ✓ Use environment variables
export DATABASE_URL="postgresql://user:pass@host/db"

# ✓ Use secrets management
export DATABASE_URL=$(aws secretsmanager get-secret-value --secret-id db-url --query SecretString --output text)

# ✗ Never log connection strings
println!("DB: {}", database_url);  // Exposes password!
```

### Error Handling

```rust
// Don't expose internal details
match sqlx::query!(...)
    .fetch_one(&pool)
    .await
{
    Ok(row) => Ok(row),
    Err(e) => {
        // ✗ Bad: Exposes internal info
        // return Err(format!("Database error: {}", e));

        // ✓ Good: Generic error to user, log details internally
        tracing::error!("Database query failed: {:?}", e);
        Err("An error occurred".into())
    }
}
```

### Rate Limiting

```rust
// Limit connections per user/IP
let pool = SqlitePoolOptions::new()
    .max_connections(100)  // Global limit
    .connect(&database_url)
    .await?;

// Implement application-level rate limiting
// (Use tower-governor or similar)
```

### Audit Logging

```rust
// Log security-relevant queries
async fn delete_user(pool: &SqlitePool, user_id: i64, admin_id: i64) -> Result<()> {
    tracing::warn!(
        admin_id = admin_id,
        target_user_id = user_id,
        "Admin deleting user"
    );

    sqlx::query!("DELETE FROM users WHERE id = ?", user_id)
        .execute(pool)
        .await?;

    Ok(())
}
```

---

## Quick Reference Card

### Common Commands

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Create migration
sqlx migrate add <description>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check migration status
sqlx migrate info

# Generate offline data
cargo sqlx prepare

# Build with offline mode
SQLX_OFFLINE=true cargo build
```

### Query Macro Cheatsheet

```rust
// Single scalar value
let count: i64 = query_scalar!("SELECT COUNT(*) FROM users")
    .fetch_one(&pool).await?;

// Anonymous struct
let row = query!("SELECT id, name FROM users WHERE id = ?", id)
    .fetch_one(&pool).await?;

// Named struct
let user = query_as!(User, "SELECT * FROM users WHERE id = ?", id)
    .fetch_one(&pool).await?;

// Multiple rows
let users = query_as!(User, "SELECT * FROM users")
    .fetch_all(&pool).await?;

// Optional result
let user = query_as!(User, "SELECT * FROM users WHERE id = ?", id)
    .fetch_optional(&pool).await?;
```

### Environment Variables

```bash
DATABASE_URL          # Required for compile-time checking
SQLX_OFFLINE          # Enable offline mode (true/false)
RUST_LOG              # Enable query logging (sqlx=debug)
```

---

## Related Documentation

- [Adding New Query Workflow](../../workflows/adding-new-query.md)
- [Adding Migration Workflow](../../workflows/adding-migration.md)
- [Quick Start SQLx](../../../QUICK_START_SQLX.md)
- [Rust Backend Guide](../rust-backend.md)
- [Database Schema](../../design/database-schema.md)

---

**Last Updated:** 2026-01-16
**Maintainer:** BDP Team
**Target Audience:** AI Agents working on BDP project
