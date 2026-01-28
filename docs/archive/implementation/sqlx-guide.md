# SQLx Usage Guide for BDP

This guide covers how SQLx is used in the BDP project, including best practices, examples, and common patterns.

## Table of Contents

1. [Overview](#overview)
2. [Setup](#setup)
3. [Query Macros](#query-macros)
4. [CRUD Operations](#crud-operations)
5. [Error Handling](#error-handling)
6. [Testing](#testing)
7. [Offline Mode](#offline-mode)
8. [Best Practices](#best-practices)

## Overview

BDP uses [SQLx](https://github.com/launchbadge/sqlx) as its async, compile-time checked SQL query library. SQLx provides:

- **Compile-time verification**: Queries are checked against your database schema at compile time
- **Type safety**: Automatic mapping between Rust types and SQL types
- **Async/await**: Native support for async Rust with Tokio
- **No ORM overhead**: Direct SQL with zero runtime cost abstraction
- **Database agnostic**: Support for PostgreSQL, MySQL, SQLite, and MSSQL

### Why SQLx?

- **Safety**: Catch SQL errors at compile time, not runtime
- **Performance**: No query parsing overhead at runtime
- **Flexibility**: Write raw SQL without ORM limitations
- **Productivity**: Automatic type inference and mapping

## Setup

### 1. Dependencies

In `Cargo.toml`:

```toml
[dependencies]
sqlx = { version = "0.8", features = [
    "runtime-tokio",  # Tokio async runtime
    "postgres",       # PostgreSQL driver
    "macros",         # query! and query_as! macros
    "uuid",           # UUID support
    "chrono",         # DateTime support
    "json",           # JSON/JSONB support
    "migrate",        # Migration support
] }
```

### 2. Database Configuration

Create a `.env` file:

```bash
DATABASE_URL=postgresql://username:password@localhost:5432/bdp
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5
DB_CONNECT_TIMEOUT=30
DB_IDLE_TIMEOUT=600
DB_MAX_LIFETIME=1800
```

### 3. Connection Pool

The connection pool is set up in `crates/bdp-server/src/db/mod.rs`:

```rust
use sqlx::postgres::PgPool;

// Create pool from configuration
let config = DbConfig::from_env()?;
let pool = create_pool(&config).await?;

// Pool is cloneable and can be shared across the application
let pool_clone = pool.clone();
```

## Query Macros

SQLx provides two main macros for queries:

### `query!` Macro

Returns an anonymous record type:

```rust
let result = sqlx::query!(
    r#"
    SELECT id, name, created_at
    FROM organizations
    WHERE slug = $1
    "#,
    slug
)
.fetch_one(&pool)
.await?;

// Access fields directly
println!("ID: {}", result.id);
println!("Name: {}", result.name);
```

### `query_as!` Macro

Maps results to a struct:

```rust
let org = sqlx::query_as!(
    Organization,
    r#"
    SELECT id, slug, name, description, created_at, updated_at
    FROM organizations
    WHERE slug = $1
    "#,
    slug
)
.fetch_one(&pool)
.await?;

// org is typed as Organization
println!("Organization: {:?}", org);
```

### Fetch Methods

- `fetch_one()`: Returns exactly one row (errors if 0 or >1)
- `fetch_optional()`: Returns `Option<Row>` (None if no rows)
- `fetch_all()`: Returns `Vec<Row>` (all matching rows)
- `execute()`: For INSERT/UPDATE/DELETE (returns rows affected)

## CRUD Operations

### Create (INSERT)

```rust
pub async fn create_organization(
    pool: &PgPool,
    slug: &str,
    name: &str,
    description: Option<&str>,
) -> DbResult<Organization> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let org = sqlx::query_as!(
        Organization,
        r#"
        INSERT INTO organizations (id, slug, name, description, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, slug, name, description, created_at, updated_at
        "#,
        id,
        slug,
        name,
        description,
        now,
        now
    )
    .fetch_one(pool)
    .await?;

    Ok(org)
}
```

**Key Points:**
- Use `RETURNING` clause to get the inserted row
- Generate UUIDs in application code
- Set timestamps explicitly for consistency

### Read (SELECT)

#### Single Row

```rust
pub async fn get_organization_by_slug(pool: &PgPool, slug: &str) -> DbResult<Organization> {
    let org = sqlx::query_as!(
        Organization,
        r#"
        SELECT id, slug, name, description, created_at, updated_at
        FROM organizations
        WHERE slug = $1
        "#,
        slug
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Organization '{}' not found", slug)))?;

    Ok(org)
}
```

**Key Points:**
- Use `fetch_optional()` to handle "not found" gracefully
- Convert `None` to a custom error type
- Provide meaningful error messages

#### Multiple Rows with Pagination

```rust
pub async fn list_organizations(
    pool: &PgPool,
    pagination: Pagination,
) -> DbResult<Vec<Organization>> {
    let orgs = sqlx::query_as!(
        Organization,
        r#"
        SELECT id, slug, name, description, created_at, updated_at
        FROM organizations
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        pagination.limit,
        pagination.offset
    )
    .fetch_all(pool)
    .await?;

    Ok(orgs)
}
```

**Key Points:**
- Always use `LIMIT` to prevent unbounded result sets
- Use `OFFSET` for pagination
- Order results consistently

#### Count Queries

```rust
pub async fn count_organizations(pool: &PgPool) -> DbResult<i64> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM organizations
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(result.count)
}
```

**Key Points:**
- Use `as "column_name!"` to mark non-nullable columns
- Useful for pagination (calculating total pages)

### Update (UPDATE)

```rust
pub async fn update_organization(
    pool: &PgPool,
    slug: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> DbResult<Organization> {
    let mut org = get_organization_by_slug(pool, slug).await?;

    if let Some(new_name) = name {
        org.name = new_name.to_string();
    }
    if let Some(new_desc) = description {
        org.description = Some(new_desc.to_string());
    }

    let now = Utc::now();

    let org = sqlx::query_as!(
        Organization,
        r#"
        UPDATE organizations
        SET name = $2, description = $3, updated_at = $4
        WHERE slug = $1
        RETURNING id, slug, name, description, created_at, updated_at
        "#,
        slug,
        org.name,
        org.description,
        now
    )
    .fetch_one(pool)
    .await?;

    Ok(org)
}
```

**Key Points:**
- Fetch current record first for partial updates
- Update `updated_at` timestamp
- Use `RETURNING` to get updated data

### Delete (DELETE)

```rust
pub async fn delete_organization(pool: &PgPool, slug: &str) -> DbResult<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM organizations
        WHERE slug = $1
        "#,
        slug
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Organization '{}' not found", slug)));
    }

    Ok(())
}
```

**Key Points:**
- Check `rows_affected()` to verify deletion
- Return error if row doesn't exist
- Consider soft deletes for important data

## Error Handling

### Custom Error Type

```rust
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    Duplicate(String),
}

pub type DbResult<T> = Result<T, DbError>;
```

### Handling Specific Errors

```rust
// Unique constraint violations
.map_err(|e| {
    if let sqlx::Error::Database(ref db_err) = e {
        if db_err.is_unique_violation() {
            return DbError::Duplicate(format!("Slug already exists"));
        }
    }
    DbError::from(e)
})

// Foreign key violations
.map_err(|e| {
    if let sqlx::Error::Database(ref db_err) = e {
        if db_err.is_foreign_key_violation() {
            return DbError::Constraint(format!("Referenced entity not found"));
        }
    }
    DbError::from(e)
})
```

### Transaction Error Handling

```rust
let mut tx = pool.begin().await?;

match execute_query(&mut tx).await {
    Ok(result) => {
        tx.commit().await?;
        Ok(result)
    }
    Err(e) => {
        tx.rollback().await?;
        Err(e)
    }
}
```

## Testing

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres@localhost/bdp_test".to_string());

        PgPool::connect(&url).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_organization() {
        let pool = create_test_pool().await;

        let org = create_organization(
            &pool,
            "test-org",
            "Test Organization",
            None,
        ).await.unwrap();

        assert_eq!(org.slug, "test-org");

        // Cleanup
        delete_organization(&pool, "test-org").await.unwrap();
    }
}
```

### Test Database Setup

1. Create a separate test database:
```bash
createdb bdp_test
```

2. Run migrations:
```bash
DATABASE_URL=postgresql://postgres@localhost/bdp_test sqlx migrate run
```

3. Set test environment variable:
```bash
export TEST_DATABASE_URL=postgresql://postgres@localhost/bdp_test
```

## Offline Mode

SQLx can verify queries at compile time without a database connection using prepared query metadata.

### Generating Metadata

```bash
# With database connection available
export DATABASE_URL=postgresql://user:pass@localhost/bdp
cargo sqlx prepare
```

This creates `.sqlx/*.json` files with query metadata.

### Using Offline Mode

```bash
# Enable offline mode for compilation
export SQLX_OFFLINE=true
cargo build
```

Or in CI/CD:

```yaml
# .github/workflows/ci.yml
- name: Build
  run: cargo build --release
  env:
    SQLX_OFFLINE: true
```

### Keeping Metadata Updated

1. Run `cargo sqlx prepare` after changing queries
2. Commit `.sqlx/*.json` files to version control
3. Review metadata diffs in pull requests
4. Re-run preparation when schema changes

## Best Practices

### 1. Always Use Prepared Statements

```rust
// Good - parameterized query
sqlx::query!("SELECT * FROM users WHERE id = $1", user_id)

// Bad - string concatenation (SQL injection risk!)
sqlx::query(&format!("SELECT * FROM users WHERE id = {}", user_id))
```

### 2. Use Transactions for Multiple Operations

```rust
pub async fn transfer_data(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
) -> DbResult<()> {
    let mut tx = pool.begin().await?;

    // Both operations must succeed
    update_source(&mut tx, from_id).await?;
    update_destination(&mut tx, to_id).await?;

    tx.commit().await?;
    Ok(())
}
```

### 3. Handle Nullable Columns Explicitly

```rust
// Use Option<T> for nullable columns
#[derive(Debug)]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,  // NULL in database
    pub created_at: DateTime<Utc>,
}
```

### 4. Use Type Aliases for Clarity

```rust
// Column types that appear frequently
pub type OrganizationId = Uuid;
pub type DatasetId = Uuid;
pub type Slug = String;

// Result types
pub type DbResult<T> = Result<T, DbError>;
```

### 5. Log Database Operations

```rust
pub async fn create_organization(...) -> DbResult<Organization> {
    let org = sqlx::query_as!(...)
        .fetch_one(pool)
        .await?;

    tracing::info!(
        org_id = %org.id,
        org_slug = %org.slug,
        "Created new organization"
    );

    Ok(org)
}
```

### 6. Use Connection Pooling

```rust
// Good - reuse pool
let pool = create_pool(&config).await?;
app_state.pool = pool.clone();

// Bad - creating new connections
for _ in 0..100 {
    let conn = PgConnection::connect(&url).await?; // Don't do this!
}
```

### 7. Set Appropriate Pool Limits

```rust
DbConfig {
    max_connections: 20,      // Based on database limits
    min_connections: 5,       // Maintain warm connections
    connect_timeout_secs: 30, // Fail fast on connection issues
    idle_timeout_secs: Some(600),   // Close idle connections
    max_lifetime_secs: Some(1800),  // Rotate old connections
}
```

### 8. Use Migrations

```bash
# Create migration
sqlx migrate add create_organizations_table

# Run migrations
sqlx migrate run

# Revert migration
sqlx migrate revert
```

### 9. Document Query Intent

```rust
/// Retrieves an organization by its URL-safe slug.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `slug` - URL-safe organization identifier (e.g., "acme-corp")
///
/// # Errors
/// Returns `DbError::NotFound` if organization doesn't exist
pub async fn get_organization_by_slug(...) -> DbResult<Organization> {
    // Implementation
}
```

### 10. Test Database Code

```rust
#[tokio::test]
async fn test_organization_not_found() {
    let pool = create_test_pool().await;

    let result = get_organization_by_slug(&pool, "nonexistent").await;

    assert!(matches!(result, Err(DbError::NotFound(_))));
}
```

## Advanced Patterns

### Bulk Inserts

```rust
pub async fn bulk_create_files(
    pool: &PgPool,
    files: &[FileData],
) -> DbResult<()> {
    let mut tx = pool.begin().await?;

    for file in files {
        sqlx::query!(
            "INSERT INTO files (id, name, size) VALUES ($1, $2, $3)",
            file.id,
            file.name,
            file.size
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
```

### JSON Columns

```rust
pub async fn update_metadata(
    pool: &PgPool,
    id: Uuid,
    metadata: serde_json::Value,
) -> DbResult<()> {
    sqlx::query!(
        "UPDATE datasets SET metadata = $2 WHERE id = $1",
        id,
        metadata
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

### Array Columns

```rust
pub async fn find_by_tags(
    pool: &PgPool,
    tags: &[String],
) -> DbResult<Vec<Dataset>> {
    sqlx::query_as!(
        Dataset,
        r#"
        SELECT * FROM datasets
        WHERE tags && $1
        "#,
        tags
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::from)
}
```

## Resources

- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [SQLx Book](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Tokio Documentation](https://tokio.rs/)

## Summary

SQLx provides compile-time verified SQL queries with excellent type safety and performance. Key takeaways:

1. Use `query_as!` for type-safe queries
2. Handle errors explicitly with custom error types
3. Use transactions for multi-step operations
4. Enable offline mode for CI/CD
5. Keep query metadata in sync with schema
6. Write integration tests for database code
7. Use connection pooling effectively

For more examples, see:
- `crates/bdp-server/src/db/organizations.rs` - Full CRUD implementation
- `crates/bdp-server/examples/database_usage.rs` - Working examples
- `.sqlx/README.md` - Offline mode documentation
