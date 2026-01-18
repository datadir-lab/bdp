# Mediator-Based CQRS Architecture

**Status**: ✅ CURRENT ARCHITECTURE (January 2026)
**Replaces**: Shared DB layer with handler structs

## Overview

BDP uses a mediator pattern for CQRS command/query dispatching. This provides clean separation between API endpoints and business logic, with no shared database layer.

## Core Principles

### 1. Command/Query Separation
- **Commands** (writes): Change state, create audit logs
- **Queries** (reads): Read-only, no audit logs

### 2. Function-Based Handlers
- Handlers are **standalone async functions**, not structs
- Each handler contains ALL business logic and SQL queries inline
- No shared `db/` layer

### 3. Mediator Dispatch
- API endpoints send commands/queries to mediator
- Mediator routes to appropriate handler function
- Middleware (audit, validation) wraps execution

### 4. Vertical Slicing
- Features are self-contained in `features/feature_name/`
- No cross-feature dependencies
- Each command/query is independent

## Architecture Flow

```
API Endpoint (axum handler)
  ↓ Creates Command struct (pure data)
  ↓ Sends to Mediator (mediator.send(command))
  ↓ Mediator dispatches to Handler function
  ↓ Tower middleware wraps execution (audit, logging)
  ↓ Handler executes business logic + inline SQL
  ↓ Response returned through chain
```

## Implementation

### Dependencies

```toml
# Cargo.toml workspace dependencies
[workspace.dependencies]
mediator = { version = "0.2", features = ["async"] }

# bdp-server dependencies
[dependencies]
mediator = { workspace = true }
```

### Command Structure

Commands are **pure data structures** with no behavior except validation helpers:

```rust
// features/organizations/commands/create.rs

use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Command - just data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationCommand {
    pub slug: String,
    pub name: String,
    pub website: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub is_system: bool,
}

/// Response type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    // ... other fields
}

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum CreateOrganizationError {
    #[error("Slug is required")]
    SlugRequired,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// Implement mediator Request trait
impl Request<Result<CreateOrganizationResponse, CreateOrganizationError>>
    for CreateOrganizationCommand
{
}

// Mark as Command for middleware
impl crate::cqrs::middleware::Command for CreateOrganizationCommand {}

impl CreateOrganizationCommand {
    /// Optional validation helper
    pub fn validate(&self) -> Result<(), CreateOrganizationError> {
        if self.slug.is_empty() {
            return Err(CreateOrganizationError::SlugRequired);
        }
        Ok(())
    }
}
```

### Handler Function

Handlers are **standalone async functions** with inline SQL:

```rust
/// Handler function - contains ALL business logic and SQL
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: CreateOrganizationCommand,
) -> Result<CreateOrganizationResponse, CreateOrganizationError> {
    // 1. Validate
    command.validate()?;

    // 2. Execute inline SQL (NO shared db layer)
    let result = sqlx::query_as!(
        OrganizationRecord,
        r#"
        INSERT INTO organizations (slug, name, website, description, logo_url, is_system)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, slug, name, website, description, logo_url, is_system, created_at, updated_at
        "#,
        command.slug,
        command.name,
        command.website,
        command.description,
        command.logo_url,
        command.is_system
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return CreateOrganizationError::DuplicateSlug(command.slug.clone());
            }
        }
        CreateOrganizationError::Database(e)
    })?;

    // 3. Return response
    Ok(CreateOrganizationResponse {
        id: result.id,
        slug: result.slug,
        name: result.name,
        website: result.website,
        description: result.description,
        logo_url: result.logo_url,
        is_system: result.is_system,
        created_at: result.created_at,
    })
}

// Helper struct for sqlx
#[derive(Debug)]
struct OrganizationRecord {
    id: Uuid,
    slug: String,
    name: String,
    website: Option<String>,
    description: Option<String>,
    logo_url: Option<String>,
    is_system: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

### Mediator Setup

The mediator is configured once at startup and shared across the app:

```rust
// cqrs/mod.rs

use mediator::DefaultAsyncMediator;
use sqlx::PgPool;

pub type AppMediator = DefaultAsyncMediator;

pub fn build_mediator(pool: PgPool) -> AppMediator {
    let pool_clone = pool.clone();

    DefaultAsyncMediator::builder()
        // Register handlers as closures
        .add_handler(move |cmd| {
            let pool = pool_clone.clone();
            async move {
                crate::features::organizations::commands::create::handle(pool, cmd).await
            }
        })
        // Add more handlers here
        .build()
}
```

### API Endpoint

API endpoints create commands and dispatch through mediator:

```rust
// api/organizations.rs

use axum::{Extension, Json};
use crate::cqrs::AppMediator;
use crate::features::organizations::commands::create::{
    CreateOrganizationCommand,
    CreateOrganizationResponse,
    CreateOrganizationError,
};

async fn create_organization(
    Extension(mediator): Extension<AppMediator>,
    Json(command): Json<CreateOrganizationCommand>,
) -> Result<Json<CreateOrganizationResponse>, CreateOrganizationError> {
    // Dispatch to mediator - it handles routing to handler
    let response = mediator.send(command).await?;
    Ok(Json(response))
}
```

### App Setup

Wire the mediator into the axum app:

```rust
// main.rs or app setup

use axum::{Extension, Router};
use sqlx::PgPool;

let pool = PgPool::connect(&database_url).await?;
let mediator = crate::cqrs::build_mediator(pool.clone());

let app = Router::new()
    .route("/api/v1/organizations", post(create_organization))
    .layer(Extension(mediator))
    .layer(Extension(pool));
```

## Directory Structure

```
features/
  organizations/
    commands/
      create.rs       - Command struct + handler function + inline SQL
      update.rs       - Command struct + handler function + inline SQL
      delete.rs       - Command struct + handler function + inline SQL
      mod.rs          - Re-exports
    queries/
      list.rs         - Query struct + handler function + inline SQL
      get.rs          - Query struct + handler function + inline SQL
      mod.rs          - Re-exports
    mod.rs            - Feature module exports
    routes.rs         - Optional: API route definitions
```

**NO `db/` directory** - all SQL is inline in handlers!

## Testing

Tests are inline with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_validation() {
        let command = CreateOrganizationCommand {
            slug: "valid-slug".to_string(),
            name: "Valid Name".to_string(),
            // ...
        };
        assert!(command.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_handle_creates_organization(pool: PgPool) -> sqlx::Result<()> {
        let command = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test Org".to_string(),
            // ...
        };

        let result = handle(pool.clone(), command).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-org");

        Ok(())
    }
}
```

## Audit Logging

Audit logging is handled at the HTTP layer via Tower middleware (see `audit/middleware.rs`). It:

- Captures POST/PUT/PATCH/DELETE requests (commands)
- Ignores GET requests (queries)
- Logs after successful command execution
- Non-blocking async writes

Commands don't need to explicitly call audit logging - it's automatic!

## Migration Path

### Old Architecture (deprecated):
```
API → Handler struct → Shared db/organizations.rs → Database
```

### New Architecture (current):
```
API → Mediator → Handler function with inline SQL → Database
```

### Steps to Migrate a Feature:

1. **Create command struct**:
   - Pure data with `#[derive(Serialize, Deserialize)]`
   - Implement `Request` trait
   - Implement `Command` or `Query` marker trait
   - Add validation helpers if needed

2. **Create handler function**:
   - Standalone `pub async fn handle(pool, command) -> Result<Response, Error>`
   - Move ALL SQL inline (from shared db layer)
   - Include business logic
   - Add tracing instrumentation

3. **Register in mediator**:
   - Add `.add_handler()` in `cqrs/mod.rs` `build_mediator()`

4. **Update API endpoint**:
   - Replace direct handler calls with `mediator.send(command)`

5. **Remove shared DB functions**:
   - Delete corresponding functions from `db/` layer
   - Eventually delete entire `db/` directory

6. **Update tests**:
   - Test command validation
   - Test handler with `#[sqlx::test]`

## Benefits

### ✅ Vertical Slicing
- Each feature is completely self-contained
- No shared code between features
- Easy to understand and modify

### ✅ Clean Separation
- Commands/queries are just data
- Handlers are just functions
- Mediator handles routing

### ✅ No Shared DB Layer
- No coupling between features
- Each handler owns its SQL
- Easy to optimize per-handler

### ✅ Testability
- Commands easy to construct for tests
- Handlers easy to call directly
- No mocking needed (use test database)

### ✅ Type Safety
- SQLx compile-time verified queries
- Mediator type-safe dispatch
- Rust type system enforces correctness

## Common Patterns

### Pattern: Command with Transaction

```rust
pub async fn handle(pool: PgPool, command: MyCommand) -> Result<MyResponse, MyError> {
    let mut tx = pool.begin().await?;

    // Do work with &mut tx
    sqlx::query!("INSERT ...").execute(&mut *tx).await?;
    sqlx::query!("UPDATE ...").execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(response)
}
```

### Pattern: Query with Pagination

```rust
pub async fn handle(pool: PgPool, query: ListQuery) -> Result<ListResponse, ListError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let items = sqlx::query_as!(
        Item,
        "SELECT * FROM items ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        limit as i64,
        offset as i64
    )
    .fetch_all(&pool)
    .await?;

    Ok(ListResponse { items })
}
```

### Pattern: Complex Business Logic

```rust
pub async fn handle(pool: PgPool, command: ComplexCommand) -> Result<Response, Error> {
    // 1. Validate
    command.validate()?;

    // 2. Check preconditions
    let existing = sqlx::query!("SELECT ...").fetch_optional(&pool).await?;
    if let Some(record) = existing {
        return Err(Error::AlreadyExists);
    }

    // 3. Business logic
    let computed_value = calculate_something(&command);

    // 4. Transaction for multiple operations
    let mut tx = pool.begin().await?;

    sqlx::query!("INSERT ...").execute(&mut *tx).await?;
    sqlx::query!("UPDATE ...").execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(Response { /* ... */ })
}
```

## References

- [mediator crate docs](https://docs.rs/mediator)
- [MediatR pattern (C#)](https://github.com/jbogard/MediatR) - inspiration
- [SQLx docs](https://docs.rs/sqlx)
- [BDP CQRS Architecture](./cqrs-architecture.md) - detailed CQRS guide

## Sources

Research for this architecture:

- [mediator - crates.io](https://crates.io/crates/mediator/)
- [mediator - docs.rs](https://docs.rs/mediator)
- [CQRS and Mediator pattern - DEV Community](https://dev.to/shashanksaini203/cqrs-and-mediator-pattern-159k)
- [Mediator Pattern in Rust](https://refactoring.guru/design-patterns/mediator/rust/example)

