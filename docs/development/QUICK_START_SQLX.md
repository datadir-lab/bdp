# SQLx Quick Start Reference

One-page reference for AI agents working with SQLx in the BDP project.

## Essential Commands

```bash
# Install SQLx CLI and Just
cargo install just sqlx-cli --features postgres

# Create migration
just db-migrate-add <description>

# Run migrations
just db-migrate

# Revert last migration
just db-migrate-revert

# Generate offline data (REQUIRED after schema changes)
just sqlx-prepare

# Verify metadata is up to date
just sqlx-check

# Build with offline mode (no database needed)
just ci-offline

# Build with database (compile-time checking)
just build
```

## Environment Variables

```bash
# Required for compile-time checking
DATABASE_URL=postgresql://user:password@localhost:5432/database

# Enable offline mode (CI/CD)
SQLX_OFFLINE=true

# Enable query logging
RUST_LOG=sqlx=debug
```

## Query Patterns

### Select Single Row

```rust
// Returns Option<User>
let user = sqlx::query_as!(
    User,
    "SELECT id, name, email FROM users WHERE id = $1",
    user_id
)
.fetch_optional(&pool)
.await?;
```

### Select Multiple Rows

```rust
// Returns Vec<User>
let users = sqlx::query_as!(
    User,
    "SELECT id, name, email FROM users ORDER BY name LIMIT $1",
    limit
)
.fetch_all(&pool)
.await?;
```

### Select Single Value

```rust
// Returns i64
let count: i64 = sqlx::query_scalar!(
    "SELECT COUNT(*) FROM users"
)
.fetch_one(&pool)
.await?;
```

### Insert with RETURNING

```rust
// Returns newly created User
let user = sqlx::query_as!(
    User,
    r#"
    INSERT INTO users (name, email)
    VALUES ($1, $2)
    RETURNING id, name, email, created_at
    "#,
    name,
    email
)
.fetch_one(&pool)
.await?;
```

### Update

```rust
// Returns updated User
let user = sqlx::query_as!(
    User,
    r#"
    UPDATE users
    SET name = $2, updated_at = NOW()
    WHERE id = $1
    RETURNING id, name, email, updated_at
    "#,
    user_id,
    new_name
)
.fetch_one(&pool)
.await?;
```

### Delete

```rust
// Execute without returning data
sqlx::query!(
    "DELETE FROM users WHERE id = $1",
    user_id
)
.execute(&pool)
.await?;
```

## Common Workflows

### Adding a New Query

```bash
# 1. Define struct with FromRow
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: i64,
    name: String,
    email: Option<String>,  # Use Option for nullable columns
}

# 2. Add query method to storage layer
impl Storage {
    pub async fn get_user(&self, id: i64) -> Result<Option<User>> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_optional(&*self.pool)
            .await
    }
}

# 3. Test locally
just test

# 4. Generate offline data
just sqlx-prepare

# 5. Commit
git add crates/ .sqlx/
git commit -m "feat: add get_user query"

# 6. Verify offline build
just ci-offline
```

### Adding a Migration

```bash
# 1. Create migration file
just db-migrate-add add_users_table

# 2. Write SQL (migrations/TIMESTAMP_add_users_table.sql)
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

# 3. (Optional) Write down migration (.down.sql)
DROP TABLE IF EXISTS users;

# 4. Test migration up
just db-migrate

# 5. Verify in database
just db-shell
\d users

# 6. Test migration down (optional)
just db-migrate-revert
just db-migrate

# 7. Update .sqlx if queries affected
just sqlx-prepare

# 8. Commit
git add migrations/ .sqlx/
git commit -m "feat: add users table migration"
```

### Handling Schema Changes

```bash
# After ANY schema change (new table, column, type change):

# 1. Apply migration
just db-migrate

# 2. Update offline data
just sqlx-prepare

# 3. Verify build
just build

# 4. Commit both
git add migrations/ .sqlx/
git commit -m "feat: schema change description"
```

## Type Mapping

### PostgreSQL → Rust

| PostgreSQL | Rust | Notes |
|------------|------|-------|
| BIGSERIAL, BIGINT | i64 | Use for IDs |
| SERIAL, INT | i32 | Avoid, prefer BIGINT |
| TEXT, VARCHAR | String | Non-nullable |
| TEXT | Option<String> | Nullable |
| BOOLEAN | bool | |
| TIMESTAMPTZ | DateTime<Utc> | Requires chrono |
| TIMESTAMP | DateTime<Utc> | Prefer TIMESTAMPTZ |
| UUID | uuid::Uuid | Requires uuid crate |
| JSON, JSONB | serde_json::Value | |
| BYTEA | Vec<u8> | Binary data |

### SQLite → Rust

| SQLite | Rust | Notes |
|--------|------|-------|
| INTEGER | i64 | Primary keys |
| TEXT | String | |
| REAL | f64 | Floating point |
| BLOB | Vec<u8> | Binary |

## Struct Definition

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,                        // BIGSERIAL PRIMARY KEY
    pub name: String,                   // TEXT NOT NULL
    pub email: Option<String>,          // TEXT (nullable)
    pub is_active: bool,                // BOOLEAN
    pub created_at: DateTime<Utc>,      // TIMESTAMPTZ
    pub updated_at: DateTime<Utc>,      // TIMESTAMPTZ
}
```

**Key rules:**
- Derive `FromRow` for SQLx mapping
- Use `Option<T>` for nullable columns
- Match types exactly to database schema
- Use `DateTime<Utc>` for TIMESTAMPTZ

## Fetch Methods

```rust
// Expects exactly 1 row, errors if 0 or >1
.fetch_one(&pool)

// Returns Option<Row>, None if 0 rows
.fetch_optional(&pool)

// Returns Vec<Row>, empty vec if 0 rows
.fetch_all(&pool)

// Returns Stream for large datasets
.fetch(&pool)
```

## Error Handling

```rust
use crate::error::ServerResult;

pub async fn get_user(&self, id: i64) -> ServerResult<Option<User>> {
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&*self.pool)
    .await?;  // ? operator converts SQLx error to ServerError

    Ok(user)
}
```

## Troubleshooting

### DATABASE_URL must be set

```bash
# Solution 1: Setup environment
just env-setup

# Solution 2: Use offline mode for builds
just ci-offline
```

### Connection refused

```bash
# Start database
just db-up

# Verify connection
just check-db
```

### Column does not exist

```bash
# Run migrations
just db-migrate

# Regenerate .sqlx
just sqlx-prepare
```

### Type mismatch (expected String, found Option<String>)

```rust
// Column is nullable in database but struct field is not
// Fix: Make field Option<T>
pub struct User {
    pub email: Option<String>,  // Add Option
}
```

### Offline mode enabled but no cached data found

```bash
# Regenerate metadata
just sqlx-prepare
```

### Stale .sqlx data after schema change

```bash
# Always regenerate after migrations
just db-migrate
just sqlx-prepare
git add .sqlx/
```

## Best Practices

### DO

- ✓ Use `query_as!` for compile-time checking
- ✓ Commit `.sqlx/` files to version control
- ✓ Run `cargo sqlx prepare` after schema changes
- ✓ Use `Option<T>` for nullable columns
- ✓ Use explicit column names in SELECT
- ✓ Use parameterized queries ($1, $2)
- ✓ Use `BIGSERIAL` for primary keys
- ✓ Use `TIMESTAMPTZ` instead of TIMESTAMP
- ✓ Add indexes for foreign keys
- ✓ Test migrations both up and down

### DON'T

- ✗ Use `SELECT *` (be explicit)
- ✗ Use runtime `query_as()` without reason
- ✗ Forget to run `cargo sqlx prepare`
- ✗ Commit `.env` with credentials
- ✗ Edit applied migrations (create new one)
- ✗ Use string interpolation for SQL
- ✗ Skip down migrations in development
- ✗ Mix schema and data migrations

## Macro Decision Tree

```
Need type safety? ──YES──> Use query! macros
      │
      NO
      │
      └──> Use query() or query_as() (runtime)

Which macro?
├─ Single value? ──YES──> query_scalar!()
├─ Map to struct? ──YES──> query_as!()
└─ Anonymous struct? ──YES──> query!()
```

## Placeholder Syntax

```rust
// PostgreSQL: $1, $2, $3, ...
sqlx::query!("SELECT * FROM users WHERE id = $1 AND email = $2", id, email)

// SQLite: ? (positional)
sqlx::query!("SELECT * FROM users WHERE id = ? AND email = ?", id, email)

// MySQL: ? (positional)
sqlx::query!("SELECT * FROM users WHERE id = ? AND email = ?", id, email)
```

## Connection Pool

```rust
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(20)        // Adjust based on load
    .min_connections(5)         // Keep warm connections
    .connect(&database_url)
    .await?;
```

**Formula:** `(CPU cores × 2) + disk spindles`
**Typical:** 10-50 connections

## Transactions

```rust
// Begin transaction
let mut tx = pool.begin().await?;

// Execute queries
sqlx::query!("INSERT INTO users (name) VALUES ($1)", "Alice")
    .execute(&mut *tx)
    .await?;

sqlx::query!("INSERT INTO logs (message) VALUES ($1)", "User created")
    .execute(&mut *tx)
    .await?;

// Commit (or rollback on error)
tx.commit().await?;
```

## Migration File Structure

```
Format: {timestamp}_{description}.sql

Example:
  20260116123456_add_users_table.sql
  20260116123456_add_users_table.down.sql (optional)

Location:
  migrations/YYYYMMDDHHMMSS_description.sql
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_get_user() {
        let pool = setup_test_db().await;
        let storage = Storage::new(pool);

        // Test logic
        let user = storage.get_user(1).await.unwrap();
        assert!(user.is_some());
    }
}
```

## CI/CD Setup

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      # No database needed for compilation!
      - name: Build
        run: cargo build
        env:
          SQLX_OFFLINE: true

      # Database needed for tests
      - name: Test
        run: |
          docker-compose up -d postgres
          cargo test
        env:
          DATABASE_URL: postgresql://bdp:password@localhost/bdp
```

## Security

```rust
// ✓ SAFE: Parameterized queries
sqlx::query!("SELECT * FROM users WHERE id = $1", user_input)

// ✗ UNSAFE: String interpolation
sqlx::query!(&format!("SELECT * FROM users WHERE id = {}", user_input))
```

**Always use:**
- Parameterized queries
- Environment variables for credentials
- `.env.example` for documentation
- `.gitignore` for `.env`

## Quick Links

- **Detailed Guide:** [docs/agents/implementation/sqlx-guide.md](docs/agents/implementation/sqlx-guide.md)
- **Query Workflow:** [docs/agents/workflows/adding-new-query.md](docs/agents/workflows/adding-new-query.md)
- **Migration Workflow:** [docs/agents/workflows/adding-migration.md](docs/agents/workflows/adding-migration.md)
- **Database Schema:** [docs/agents/design/database-schema.md](docs/agents/design/database-schema.md)

## File Locations in BDP

```
Project Structure:
├── .env.example                    # Environment template
├── .sqlx/                          # Offline query data (commit!)
│   └── query-*.json
├── migrations/                     # Database migrations (commit!)
│   ├── TIMESTAMP_description.sql
│   └── TIMESTAMP_description.down.sql
├── crates/bdp-server/
│   ├── src/
│   │   ├── models/mod.rs          # Database models
│   │   └── storage/mod.rs         # Query methods
│   └── Cargo.toml
└── Cargo.toml                      # Workspace config
```

## Common Patterns in BDP

### Model Definition
```rust
// crates/bdp-server/src/models/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Storage Methods
```rust
// crates/bdp-server/src/storage/mod.rs
impl Storage {
    pub async fn get_organization(&self, id: i64) -> ServerResult<Option<Organization>> {
        sqlx::query_as!(
            Organization,
            "SELECT id, name, slug, description, created_at, updated_at FROM organizations WHERE id = $1",
            id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(Into::into)
    }
}
```

### Running Migrations on Startup
```rust
// crates/bdp-server/src/storage/mod.rs
pub async fn init(config: &Config) -> anyhow::Result<Storage> {
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    // Run migrations automatically
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(Storage::new(pool))
}
```

## Remember

1. **After every schema change:** Run `just sqlx-prepare`
2. **Before committing:** Verify `.sqlx/` files are included
3. **Use Just commands:** `just --list` to see all available commands
4. **For nullable columns:** Use `Option<T>` in structs
5. **For primary keys:** Use `BIGSERIAL` → `i64`
6. **For timestamps:** Use `TIMESTAMPTZ` → `DateTime<Utc>`

---

**Last Updated:** 2026-01-16
**BDP Project:** Bioinformatics Dependencies Platform
**For detailed documentation, see:** [docs/agents/implementation/sqlx-guide.md](docs/agents/implementation/sqlx-guide.md)
