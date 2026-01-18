# Database Module

This module provides database operations for the BDP server using SQLx.

## Overview

The database module is organized as follows:

```
db/
├── mod.rs                  # Connection pool setup and configuration
├── organizations.rs        # CRUD operations for organizations
└── README.md              # This file
```

## Features

- **Type-safe queries**: Using SQLx's `query_as!` macro for compile-time verification
- **Connection pooling**: Efficient connection management with configurable pool settings
- **Error handling**: Custom error types for common database scenarios
- **Pagination**: Built-in pagination support for list queries
- **Transactions**: Support for multi-step atomic operations
- **Health checks**: Database availability monitoring

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
use bdp_server::db::{create_pool, organizations, DbConfig};
use bdp_common::types::Pagination;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create connection pool
    let config = DbConfig::from_env()?;
    let pool = create_pool(&config).await?;

    // Create an organization
    let org = organizations::create_organization(
        &pool,
        "acme-corp",
        "ACME Corporation",
        Some("Leading provider of datasets"),
    ).await?;

    // Get organization by slug
    let org = organizations::get_organization_by_slug(&pool, "acme-corp").await?;

    // List organizations with pagination
    let orgs = organizations::list_organizations(&pool, Pagination::default()).await?;

    Ok(())
}
```

## Module Structure

### `mod.rs` - Core Module

Provides:
- `DbConfig`: Database configuration
- `DbError`: Custom error types
- `DbResult<T>`: Result type alias
- `create_pool()`: Connection pool creation
- `health_check()`: Database health verification

### `organizations.rs` - Organizations Table

CRUD operations:
- `create_organization()`: Insert new organization
- `get_organization_by_slug()`: Fetch by slug
- `get_organization_by_id()`: Fetch by UUID
- `list_organizations()`: List with pagination
- `count_organizations()`: Total count
- `update_organization()`: Update fields
- `delete_organization()`: Remove organization
- `search_organizations()`: Full-text search

## Error Handling

The module defines custom error types:

```rust
pub enum DbError {
    Sqlx(sqlx::Error),           // Database errors
    NotFound(String),             // Resource not found
    Duplicate(String),            // Unique constraint violation
}
```

Example error handling:

```rust
match organizations::get_organization_by_slug(&pool, "acme-corp").await {
    Ok(org) => println!("Found: {}", org.name),
    Err(DbError::NotFound(msg)) => println!("Not found: {}", msg),
    Err(e) => println!("Error: {}", e),
}
```

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

## Pagination

All list queries support pagination:

```rust
use bdp_common::types::Pagination;

// Default pagination (50 items, offset 0)
let page1 = organizations::list_organizations(&pool, Pagination::default()).await?;

// Custom pagination
let page2 = organizations::list_organizations(&pool, Pagination::new(20, 20)).await?;

// Page-based pagination
let page3 = organizations::list_organizations(&pool, Pagination::page(2, 20)).await?;
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

```rust
#[tokio::test]
async fn test_create_organization() {
    let pool = create_test_pool().await;

    let org = create_organization(&pool, "test-org", "Test", None)
        .await
        .unwrap();

    assert_eq!(org.slug, "test-org");

    // Cleanup
    delete_organization(&pool, "test-org").await.unwrap();
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

## Adding New Database Operations

### 1. Create a New Module

Create `src/db/registry_entries.rs`:

```rust
use sqlx::PgPool;
use uuid::Uuid;
use super::{DbError, DbResult};

pub async fn create_registry_entry(
    pool: &PgPool,
    org_id: Uuid,
    slug: &str,
    name: &str,
) -> DbResult<RegistryEntry> {
    let id = Uuid::new_v4();

    let entry = sqlx::query_as!(
        RegistryEntry,
        r#"
        INSERT INTO registry_entries (id, organization_id, slug, name)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        id, org_id, slug, name
    )
    .fetch_one(pool)
    .await?;

    Ok(entry)
}
```

### 2. Export the Module

In `src/db/mod.rs`:

```rust
pub mod organizations;
pub mod registry_entries;  // Add this
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

```rust
// Use indexes for WHERE clauses
CREATE INDEX idx_organizations_slug ON organizations(slug);

// Use EXPLAIN ANALYZE to check query plans
EXPLAIN ANALYZE SELECT * FROM organizations WHERE slug = 'acme-corp';

// Avoid N+1 queries - use JOINs
SELECT o.*, COUNT(r.id) as entry_count
FROM organizations o
LEFT JOIN registry_entries r ON r.organization_id = o.id
GROUP BY o.id;
```

### Batch Operations

```rust
// Instead of multiple individual inserts
for item in items {
    insert_one(&pool, item).await?;  // Slow
}

// Use a transaction with batch insert
let mut tx = pool.begin().await?;
for item in items {
    insert_one(&mut tx, item).await?;
}
tx.commit().await?;  // Fast
```

## Common Patterns

### Transaction Example

```rust
pub async fn transfer_ownership(
    pool: &PgPool,
    entry_id: Uuid,
    new_org_id: Uuid,
) -> DbResult<()> {
    let mut tx = pool.begin().await?;

    // Update registry entry
    sqlx::query!(
        "UPDATE registry_entries SET organization_id = $1 WHERE id = $2",
        new_org_id,
        entry_id
    )
    .execute(&mut *tx)
    .await?;

    // Log the transfer
    sqlx::query!(
        "INSERT INTO audit_log (action, entry_id) VALUES ('transfer', $1)",
        entry_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
```

### Optional Updates

```rust
pub async fn update_organization(
    pool: &PgPool,
    slug: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> DbResult<Organization> {
    // Fetch current state
    let mut org = get_organization_by_slug(pool, slug).await?;

    // Apply changes
    if let Some(n) = name {
        org.name = n.to_string();
    }
    if let Some(d) = description {
        org.description = Some(d.to_string());
    }

    // Update in database
    let org = sqlx::query_as!(
        Organization,
        "UPDATE organizations SET name = $2, description = $3 WHERE slug = $1 RETURNING *",
        slug,
        org.name,
        org.description
    )
    .fetch_one(pool)
    .await?;

    Ok(org)
}
```

## Documentation

- [SQLx Usage Guide](../../../docs/sqlx-guide.md) - Comprehensive SQLx patterns
- [Database Setup](../../../docs/database-setup.md) - Installation and configuration
- [Example Code](../examples/database_usage.rs) - Working examples
- [.sqlx Metadata](../../../.sqlx/README.md) - Offline compilation

## Resources

- [SQLx Repository](https://github.com/launchbadge/sqlx)
- [PostgreSQL Docs](https://www.postgresql.org/docs/)
- [SQLx Book](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

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

### Type mismatch errors

```rust
// Make sure struct fields match database types
pub struct Organization {
    pub id: Uuid,              // UUID in database
    pub slug: String,          // VARCHAR in database
    pub description: Option<String>,  // TEXT NULL in database
    pub created_at: DateTime<Utc>,    // TIMESTAMPTZ in database
}
```

## Contributing

When adding new database operations:

1. Add comprehensive doc comments
2. Include usage examples
3. Handle errors appropriately
4. Add tests
5. Update prepared queries (`cargo sqlx prepare`)
6. Update this README if adding new modules
