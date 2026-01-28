# Database Module

This module provides database connection pooling and configuration for the BDP server.

## Overview

The BDP project uses a **mediator-based CQRS architecture** where database queries are embedded directly in command and query handlers. This module provides only the foundational database infrastructure - no shared query layer.

```
db/
├── mod.rs                  # Connection pool setup and configuration
├── archive/                # Archived shared database layer (deprecated)
│   ├── organizations.rs    # DEPRECATED - migrated to features/organizations/
│   ├── data_sources.rs     # DEPRECATED - migrated to features/data_sources/
│   ├── versions.rs         # DEPRECATED - migrated to features/data_sources/
│   ├── search.rs           # DEPRECATED - migrated to features/search/
│   └── sources.rs          # DEPRECATED - placeholder
└── README.md              # This file
```

## CQRS Architecture

**As of January 2026, BDP uses a pure CQRS architecture with NO SHARED DATABASE LAYER.**

All database operations are contained within feature-specific command and query handlers:

- `features/organizations/commands/` - Organization commands (create, update, delete)
- `features/organizations/queries/` - Organization queries (get, list)
- `features/data_sources/commands/` - Data source commands
- `features/data_sources/queries/` - Data source queries
- `features/search/queries/` - Search queries

Each handler contains its own inline SQL queries using SQLx's `query!` and `query_as!` macros for compile-time verification.

## Features

- **Connection pooling**: Efficient connection management with configurable pool settings
- **Error handling**: Custom error types for database scenarios
- **Health checks**: Database availability monitoring
- **Configuration**: Environment-based or programmatic configuration

## Quick Start

### 1. Setup Database

```bash
# Create database
createdb bdp

# Run migrations
cd crates/bdp-server
sqlx migrate run
```

### 2. Configure Environment

```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/bdp"
export DB_MAX_CONNECTIONS=20
```

### 3. Use in Your Code

```rust
use bdp_server::db::{create_pool, DbConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create connection pool
    let config = DbConfig::from_env()?;
    let pool = create_pool(&config).await?;

    // Pass pool to CQRS handlers
    // Example: use mediator to send commands/queries

    Ok(())
}
```

## Core Module Exports

### `mod.rs` - Core Module

Provides:
- `DbConfig`: Database configuration
- `DbError`: Custom error types
- `DbResult<T>`: Result type alias
- `create_pool()`: Connection pool creation
- `health_check()`: Database health verification

### No Shared Query Layer

Unlike traditional layered architectures, this module **does not export** database query functions. All queries are embedded in CQRS handlers following these patterns:

**Commands** (write operations):
```rust
// features/organizations/commands/create.rs
pub async fn handle(
    pool: PgPool,
    command: CreateOrganizationCommand,
) -> Result<CreateOrganizationResponse, CreateOrganizationError> {
    // Inline SQL query
    let result = sqlx::query_as!(
        OrganizationRecord,
        r#"
        INSERT INTO organizations (slug, name, website)
        VALUES ($1, $2, $3)
        RETURNING id, slug, name, created_at
        "#,
        command.slug,
        command.name,
        command.website
    )
    .fetch_one(&pool)
    .await?;

    Ok(result.into())
}
```

**Queries** (read operations):
```rust
// features/organizations/queries/get.rs
pub async fn handle(
    pool: PgPool,
    query: GetOrganizationQuery,
) -> Result<GetOrganizationResponse, GetOrganizationError> {
    // Inline SQL query
    let record = sqlx::query_as!(
        OrganizationRecord,
        r#"
        SELECT id, slug, name, website, created_at
        FROM organizations
        WHERE slug = $1
        "#,
        query.slug
    )
    .fetch_optional(&pool)
    .await?;

    // ... map to response
}
```

## Error Handling

The module defines custom error types:

```rust
pub enum DbError {
    Sqlx(sqlx::Error),           // Database errors
    NotFound(String),             // Resource not found
    Duplicate(String),            // Unique constraint violation
    Config(String),               // Configuration errors
}
```

Feature-specific handlers define their own error types that wrap these as needed.

## Configuration

### Environment Variables

```bash
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/bdp

# Optional (with defaults)
DB_MAX_CONNECTIONS=20       # Maximum pool size
DB_MIN_CONNECTIONS=5        # Minimum idle connections
DB_CONNECT_TIMEOUT=30       # Connection timeout (seconds)
DB_IDLE_TIMEOUT=600         # Idle timeout (seconds)
DB_MAX_LIFETIME=1800        # Max connection lifetime (seconds)
```

### Programmatic Configuration

```rust
let config = DbConfig {
    url: "postgresql://localhost/bdp".to_string(),
    max_connections: 20,
    min_connections: 5,
    connect_timeout_secs: 30,
    idle_timeout_secs: Some(600),
    max_lifetime_secs: Some(1800),
};
```

## Testing

### Running Tests

```bash
# Set test database URL
export TEST_DATABASE_URL=postgresql://postgres@localhost/bdp_test

# Run migrations
sqlx migrate run --database-url $TEST_DATABASE_URL

# Run tests
cargo test
```

### Example Test

Tests are written in each CQRS handler file using `#[sqlx::test]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_handle_creates_organization(pool: PgPool) -> sqlx::Result<()> {
        let cmd = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test Organization".to_string(),
            // ...
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        Ok(())
    }
}
```

## SQLx Offline Mode

For CI/CD environments without database access:

```bash
# Generate metadata (requires database)
cargo sqlx prepare

# Build without database
export SQLX_OFFLINE=true
cargo build
```

Metadata is stored in `.sqlx/*.json` files.

## Migration from Shared DB Layer

The `archive/` directory contains the old shared database layer that was deprecated when migrating to CQRS architecture. These files are kept for historical reference but are not compiled or used:

- `organizations.rs` → Migrated to `features/organizations/commands/` and `features/organizations/queries/`
- `data_sources.rs` → Migrated to `features/data_sources/commands/` and `features/data_sources/queries/`
- `versions.rs` → Migrated to `features/data_sources/commands/` and `features/data_sources/queries/`
- `search.rs` → Migrated to `features/search/queries/`
- `sources.rs` → Deprecated placeholder, never implemented

**Migration completed**: January 2026

## Adding New Database Operations

When adding new database operations, follow the CQRS pattern:

### 1. Create Command or Query Handler

Create a new file in the appropriate feature module:

```
features/
└── my_feature/
    ├── commands/
    │   └── create.rs          # Command handler with inline SQL
    ├── queries/
    │   └── get.rs             # Query handler with inline SQL
    └── mod.rs
```

### 2. Implement Handler with Inline SQL

```rust
// features/my_feature/commands/create.rs
use sqlx::PgPool;
use mediator::Request;

#[derive(Debug)]
pub struct CreateMyEntityCommand {
    pub name: String,
}

impl Request<Result<MyEntityResponse, MyEntityError>> for CreateMyEntityCommand {}

pub async fn handle(
    pool: PgPool,
    command: CreateMyEntityCommand,
) -> Result<MyEntityResponse, MyEntityError> {
    // Inline SQL query - no shared database layer
    let result = sqlx::query_as!(
        MyEntityRecord,
        r#"
        INSERT INTO my_entities (name)
        VALUES ($1)
        RETURNING id, name, created_at
        "#,
        command.name
    )
    .fetch_one(&pool)
    .await?;

    Ok(result.into())
}
```

### 3. Update Prepared Queries

```bash
cargo sqlx prepare
```

## Performance Tips

### Connection Pool Sizing

```rust
// Formula: connections = (core_count × 2) + effective_spindle_count
// For 8-core CPU with SSD: 8 × 2 + 1 = 17
let config = DbConfig {
    max_connections: 20,
    min_connections: 5,
    ..Default::default()
};
```

### Query Optimization

- Use indexes for WHERE clauses in your inline queries
- Use `EXPLAIN ANALYZE` to check query plans
- Avoid N+1 queries - use JOINs in your handler SQL
- Use materialized views for complex aggregations (see `features/search/`)

### Batch Operations

```rust
// Use transactions for batch inserts
let mut tx = pool.begin().await?;
for item in items {
    sqlx::query!("INSERT INTO ...")
        .execute(&mut *tx)
        .await?;
}
tx.commit().await?;
```

## Documentation

- [CQRS Architecture](../../../docs/agents/backend-architecture.md) - MANDATORY reading for backend development
- [SQLx Guide](../../../docs/agents/implementation/sqlx-guide.md) - SQLx patterns and best practices
- [Database Setup](../../../docs/database-setup.md) - Installation and configuration
- [Error Handling](../../../docs/agents/error-handling.md) - Error handling policy

## Resources

- [SQLx Repository](https://github.com/launchbadge/sqlx)
- [PostgreSQL Docs](https://www.postgresql.org/docs/)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)

## Troubleshooting

### "Could not find query metadata"

```bash
# Solution: Generate query metadata
cargo sqlx prepare
```

### "Connection refused"

```bash
# Solution: Start PostgreSQL
sudo systemctl start postgresql
```

### "Migration already applied"

```bash
# Solution: Check migration status
sqlx migrate info

# Or reset database
dropdb bdp && createdb bdp
sqlx migrate run
```

## Contributing

When adding new database operations:

1. **DO NOT** add functions to this `db/` module
2. **DO** create CQRS command/query handlers in `features/`
3. **DO** embed SQL queries inline in handlers
4. **DO** add comprehensive doc comments
5. **DO** include usage examples
6. **DO** handle errors appropriately
7. **DO** add tests using `#[sqlx::test]`
8. **DO** update prepared queries (`cargo sqlx prepare`)
