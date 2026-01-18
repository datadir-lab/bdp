# BDP Backend Architecture

**CRITICAL: All backend features MUST follow this CQRS architecture pattern.**

This document defines the mandatory backend architecture for BDP. All AI agents and developers must follow these patterns when implementing features.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [CQRS Pattern](#cqrs-pattern)
3. [Vertical Slice Architecture](#vertical-slice-architecture)
4. [Audit Logging](#audit-logging)
5. [Folder Structure](#folder-structure)
6. [Implementation Checklist](#implementation-checklist)
7. [Code Examples](#code-examples)
8. [Anti-Patterns](#anti-patterns)

---

## Architecture Overview

BDP uses **CQRS (Command Query Responsibility Segregation)** with **Vertical Slice Architecture**.

### Key Principles

1. **Commands** (Write operations) - Modify state, require audit logging
2. **Queries** (Read operations) - Read-only, no audit logging
3. **Features** - Self-contained vertical slices organized by domain
4. **Audit Trail** - Automatic logging of all state changes
5. **Type Safety** - Rust's type system + SQLx compile-time verification

### Technology Stack

- **Framework**: Axum 0.7 (Rust async web framework)
- **Database**: PostgreSQL 16+ with SQLx 0.8
- **Middleware**: Tower layers for cross-cutting concerns
- **Validation**: Type-driven validation in command structs
- **Testing**: SQLx test harness with database isolation

---

## CQRS Pattern

### Commands (Write Operations)

Commands **modify state** and **must be audited**.

**Characteristics:**
- Named as actions: `CreateOrganization`, `UpdateDataSource`, `DeleteVersion`
- Contain validation logic
- Use database transactions
- Log to `audit_log` table
- Return success/failure

**HTTP Methods:** POST, PUT, PATCH, DELETE

**Example:**
```rust
pub struct CreateOrganizationCommand {
    pub slug: String,
    pub name: String,
    pub website: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub is_system: bool,
}

impl CreateOrganizationCommand {
    /// Validate command before execution
    pub fn validate(&self) -> Result<(), CommandError> {
        // Slug validation: 1-100 chars, lowercase alphanumeric + hyphens
        if self.slug.is_empty() || self.slug.len() > 100 {
            return Err(CommandError::InvalidSlug("must be 1-100 characters"));
        }
        // ... more validation
        Ok(())
    }
}

pub struct CreateOrganizationHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
}

impl CreateOrganizationHandler {
    #[tracing::instrument(skip(self))]
    pub async fn handle(
        &self,
        command: CreateOrganizationCommand,
        user_id: Option<Uuid>,
    ) -> Result<CreateOrganizationResponse, HandlerError> {
        // 1. Validate
        command.validate()?;

        // 2. Begin transaction
        let mut tx = self.pool.begin().await?;

        // 3. Execute command
        let org = db::organizations::create_organization(
            &mut tx,
            &command.slug,
            &command.name,
            command.website.as_deref(),
            command.description.as_deref(),
            command.logo_url.as_deref(),
            command.is_system,
        ).await?;

        // 4. Log audit entry
        self.audit_logger.log(
            &mut tx,
            AuditAction::CreateOrganization,
            ResourceType::Organization,
            org.id,
            user_id,
            serde_json::json!({
                "slug": command.slug,
                "name": command.name,
            }),
        ).await?;

        // 5. Commit transaction
        tx.commit().await?;

        Ok(CreateOrganizationResponse { organization: org })
    }
}
```

### Queries (Read Operations)

Queries **read state** and **are NOT audited**.

**Characteristics:**
- Named as questions: `GetOrganization`, `ListDataSources`, `SearchProteins`
- Read-only (no writes)
- No audit logging (performance optimization)
- Return data
- Can use database read replicas (future optimization)

**HTTP Method:** GET

**Example:**
```rust
pub struct ListOrganizationsQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub is_system: Option<bool>,
    pub name_contains: Option<String>,
}

impl ListOrganizationsQuery {
    pub fn validate(&self) -> Result<(), QueryError> {
        if let Some(page) = self.page {
            if page < 1 {
                return Err(QueryError::InvalidPage);
            }
        }
        // Limit per_page to prevent DOS
        if let Some(per_page) = self.per_page {
            if per_page > 100 {
                return Err(QueryError::PerPageTooLarge);
            }
        }
        Ok(())
    }
}

pub struct ListOrganizationsHandler {
    pool: PgPool,
}

impl ListOrganizationsHandler {
    #[tracing::instrument(skip(self))]
    pub async fn handle(
        &self,
        query: ListOrganizationsQuery,
    ) -> Result<ListOrganizationsResponse, HandlerError> {
        // 1. Validate
        query.validate()?;

        // 2. Execute query (NO transaction, NO audit)
        let organizations = db::organizations::list_organizations(
            &self.pool,
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(20),
            query.is_system,
            query.name_contains.as_deref(),
        ).await?;

        Ok(ListOrganizationsResponse {
            items: organizations,
            pagination: /* ... */
        })
    }
}
```

### Command vs Query Decision Matrix

| Operation | Type | Audited? | Transaction? | Example |
|-----------|------|----------|--------------|---------|
| Create record | Command | ✅ Yes | ✅ Yes | POST /organizations |
| Update record | Command | ✅ Yes | ✅ Yes | PUT /organizations/:id |
| Delete record | Command | ✅ Yes | ✅ Yes | DELETE /organizations/:id |
| Get single record | Query | ❌ No | ❌ No | GET /organizations/:id |
| List records | Query | ❌ No | ❌ No | GET /organizations |
| Search records | Query | ❌ No | ❌ No | GET /search?q=... |
| Download file | Query | ❌ No | ❌ No | GET /sources/:id/download |

---

## Vertical Slice Architecture

Features are organized as **vertical slices** - each feature contains all layers needed for that feature.

### Feature Structure

```
crates/bdp-server/src/features/
├── organizations/           # Feature: Organization management
│   ├── mod.rs              # Re-exports commands, queries, routes
│   ├── commands/           # Write operations
│   │   ├── mod.rs
│   │   ├── create.rs       # CreateOrganization command
│   │   ├── update.rs       # UpdateOrganization command
│   │   └── delete.rs       # DeleteOrganization command
│   ├── queries/            # Read operations
│   │   ├── mod.rs
│   │   ├── get.rs          # GetOrganization query
│   │   └── list.rs         # ListOrganizations query
│   └── routes.rs           # HTTP handlers for this feature
│
├── data_sources/           # Feature: Data source management
│   ├── mod.rs
│   ├── commands/
│   │   ├── create.rs
│   │   ├── update.rs
│   │   └── delete.rs
│   ├── queries/
│   │   ├── get.rs
│   │   ├── list.rs
│   │   └── search.rs
│   └── routes.rs
│
└── mod.rs                  # Feature registry
```

### Why Vertical Slices?

**Traditional Layered Architecture:**
```
controllers/
  organizations_controller.rs
  data_sources_controller.rs
services/
  organizations_service.rs
  data_sources_service.rs
repositories/
  organizations_repository.rs
  data_sources_repository.rs
```

**Problems with layers:**
- Changes to one feature touch multiple directories
- Hard to understand feature boundaries
- Difficult to work on features in parallel
- Unclear ownership

**Vertical Slices:**
```
features/
  organizations/  ← Everything for organizations
  data_sources/   ← Everything for data sources
```

**Benefits:**
- ✅ All code for a feature in one place
- ✅ Easy to understand and modify
- ✅ Features can be developed independently
- ✅ Clear ownership and boundaries
- ✅ Easier to delete unused features

---

## Audit Logging

**CRITICAL: All commands must log to the audit trail.**

### Audit Log Table

```sql
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,                           -- Who performed the action
    action VARCHAR(50) NOT NULL,            -- What action (CreateOrganization, UpdateDataSource)
    resource_type VARCHAR(50) NOT NULL,     -- What type (Organization, DataSource, Version)
    resource_id UUID,                       -- Which specific resource
    changes JSONB,                          -- What changed (before/after for updates)
    ip_address VARCHAR(45),                 -- From where
    user_agent TEXT,                        -- Using what client
    timestamp TIMESTAMPTZ DEFAULT NOW(),    -- When
    metadata JSONB                          -- Additional context
);
```

### Automatic Audit via Middleware

Audit logging happens automatically via Tower middleware:

```rust
// In main.rs
let app = Router::new()
    .nest("/api/v1", feature_routes)
    .layer(
        ServiceBuilder::new()
            .layer(AuditLayer::new(pool.clone()))  // ← Audits all commands
            .layer(cors)
            .layer(TraceLayer::new_for_http())
    );
```

**How it works:**
1. Middleware intercepts all requests
2. For commands (POST/PUT/PATCH/DELETE):
   - Captures request body
   - Extracts user ID from headers
   - Extracts IP address and user agent
3. After successful command execution:
   - Logs audit entry to database
   - Runs asynchronously (non-blocking)
4. Queries (GET) are ignored (performance)

### Manual Audit Logging

In handlers, you can also log explicitly:

```rust
self.audit_logger.log(
    &mut tx,
    AuditAction::UpdateOrganization,
    ResourceType::Organization,
    org.id,
    user_id,
    serde_json::json!({
        "before": { "name": old_name },
        "after": { "name": new_name },
    }),
).await?;
```

### Querying Audit Logs

```bash
# View recent logs
just audit-logs 100

# Search logs
just audit-search "organization"

# View logs for specific resource
just audit-trail Organization "uuid-here"

# View logs for specific user
just audit-by-user "uuid-here"

# Export to JSON
just audit-export audit.json

# View statistics
just audit-stats
```

**API Endpoint:**
```http
GET /api/v1/audit?limit=100&resource_type=Organization&action=Create
```

---

## Folder Structure

### Complete Backend Structure

```
crates/bdp-server/
├── src/
│   ├── main.rs                    # Server entry point, middleware setup
│   ├── lib.rs                     # Library exports
│   ├── config.rs                  # Configuration from environment
│   ├── error.rs                   # Global error types
│   │
│   ├── features/                  # ⭐ VERTICAL SLICES ⭐
│   │   ├── mod.rs                # Feature registry
│   │   ├── organizations/        # Organization feature
│   │   │   ├── mod.rs
│   │   │   ├── commands/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── create.rs
│   │   │   │   ├── update.rs
│   │   │   │   └── delete.rs
│   │   │   ├── queries/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── get.rs
│   │   │   │   └── list.rs
│   │   │   └── routes.rs
│   │   ├── data_sources/
│   │   ├── versions/
│   │   └── search/
│   │
│   ├── db/                        # Database layer (used by features)
│   │   ├── mod.rs
│   │   ├── organizations.rs       # CRUD operations
│   │   ├── data_sources.rs
│   │   ├── versions.rs
│   │   └── search.rs
│   │
│   ├── audit/                     # Audit logging system
│   │   ├── mod.rs
│   │   ├── models.rs              # AuditEntry, AuditAction, ResourceType
│   │   ├── queries.rs             # Database operations for audit logs
│   │   └── middleware.rs          # Tower middleware for auto-audit
│   │
│   ├── middleware/                # Cross-cutting concerns
│   │   ├── mod.rs
│   │   ├── cors.rs
│   │   ├── logging.rs
│   │   └── audit.rs (re-export)
│   │
│   └── storage/                   # S3/MinIO integration
│       ├── mod.rs
│       └── s3_client.rs
│
├── tests/
│   ├── helpers/
│   │   ├── mod.rs
│   │   └── fixtures.rs
│   ├── db_tests.rs
│   ├── api_tests.rs
│   ├── audit_tests.rs
│   └── cqrs_integration_tests.rs
│
└── Cargo.toml
```

---

## Implementation Checklist

When implementing a new feature, follow this checklist:

### 1. Create Feature Module

```bash
mkdir -p crates/bdp-server/src/features/my_feature/{commands,queries}
touch crates/bdp-server/src/features/my_feature/{mod.rs,routes.rs}
touch crates/bdp-server/src/features/my_feature/commands/{mod.rs,create.rs}
touch crates/bdp-server/src/features/my_feature/queries/{mod.rs,list.rs}
```

### 2. Define Models

- [ ] Create command structs (e.g., `CreateMyFeatureCommand`)
- [ ] Create query structs (e.g., `ListMyFeaturesQuery`)
- [ ] Create response structs (e.g., `MyFeatureResponse`)
- [ ] Create error types (e.g., `MyFeatureError`)

### 3. Implement Commands

- [ ] Create command handler (e.g., `CreateMyFeatureHandler`)
- [ ] Add validation logic in `Command::validate()`
- [ ] Use database transaction
- [ ] **Add audit logging**
- [ ] Commit transaction
- [ ] Add tracing instrumentation
- [ ] Write unit tests

### 4. Implement Queries

- [ ] Create query handler (e.g., `ListMyFeaturesHandler`)
- [ ] Add validation logic in `Query::validate()`
- [ ] Execute read-only database query
- [ ] **No audit logging**
- [ ] Add tracing instrumentation
- [ ] Write unit tests

### 5. Create HTTP Routes

- [ ] Add Axum route handlers in `routes.rs`
- [ ] Map commands to POST/PUT/PATCH/DELETE
- [ ] Map queries to GET
- [ ] Add proper error handling
- [ ] Return correct HTTP status codes

### 6. Register Routes

- [ ] Export feature router from `features/my_feature/mod.rs`
- [ ] Register in `features/mod.rs`
- [ ] Add to main router in `main.rs`

### 7. Add Tests

- [ ] Unit tests for command/query validation
- [ ] Integration tests for handlers
- [ ] API endpoint tests
- [ ] **Test audit log creation for commands**
- [ ] Test queries don't create audit logs

### 8. Update Database Layer

- [ ] Add database functions in `db/my_feature.rs`
- [ ] Use SQLx `query!` or `query_as!` macros
- [ ] Run `just sqlx-prepare` to generate metadata

### 9. Documentation

- [ ] Add doc comments to all public functions
- [ ] Include usage examples in doc comments
- [ ] Update API documentation if needed

---

## Code Examples

### Minimal Feature Implementation

#### `features/tags/commands/create.rs`

```rust
use sqlx::{PgPool, Postgres, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateTagCommand {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

impl CreateTagCommand {
    pub fn validate(&self) -> Result<(), CommandError> {
        if self.name.is_empty() || self.name.len() > 100 {
            return Err(CommandError::InvalidName);
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct CreateTagResponse {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
}

pub struct CreateTagHandler {
    pool: PgPool,
}

impl CreateTagHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[tracing::instrument(skip(self))]
    pub async fn handle(
        &self,
        command: CreateTagCommand,
        user_id: Option<Uuid>,
    ) -> Result<CreateTagResponse, HandlerError> {
        // 1. Validate
        command.validate()?;

        // 2. Begin transaction
        let mut tx = self.pool.begin().await?;

        // 3. Execute
        let tag = db::tags::create_tag(
            &mut tx,
            &command.name,
            command.category.as_deref(),
            command.description.as_deref(),
        ).await?;

        // 4. Audit log
        audit::log(
            &mut tx,
            AuditAction::CreateTag,
            ResourceType::Tag,
            tag.id,
            user_id,
            serde_json::json!({ "name": tag.name }),
        ).await?;

        // 5. Commit
        tx.commit().await?;

        Ok(CreateTagResponse {
            id: tag.id,
            name: tag.name,
            category: tag.category,
        })
    }
}
```

#### `features/tags/queries/list.rs`

```rust
use sqlx::PgPool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListTagsQuery {
    pub category: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl ListTagsQuery {
    pub fn validate(&self) -> Result<(), QueryError> {
        if let Some(per_page) = self.per_page {
            if per_page > 100 {
                return Err(QueryError::PerPageTooLarge);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ListTagsResponse {
    pub items: Vec<Tag>,
    pub total: i64,
}

pub struct ListTagsHandler {
    pool: PgPool,
}

impl ListTagsHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[tracing::instrument(skip(self))]
    pub async fn handle(
        &self,
        query: ListTagsQuery,
    ) -> Result<ListTagsResponse, HandlerError> {
        // 1. Validate
        query.validate()?;

        // 2. Execute (NO transaction, NO audit)
        let tags = db::tags::list_tags(
            &self.pool,
            query.category.as_deref(),
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(20),
        ).await?;

        let total = db::tags::count_tags(
            &self.pool,
            query.category.as_deref(),
        ).await?;

        Ok(ListTagsResponse { items: tags, total })
    }
}
```

#### `features/tags/routes.rs`

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, delete},
    Json, Router,
};

pub fn routes() -> Router<FeatureState> {
    Router::new()
        .route("/tags", post(create_tag))
        .route("/tags", get(list_tags))
        .route("/tags/:id", delete(delete_tag))
}

async fn create_tag(
    State(state): State<FeatureState>,
    Json(command): Json<CreateTagCommand>,
) -> Result<(StatusCode, Json<CreateTagResponse>), HandlerError> {
    let handler = CreateTagHandler::new(state.pool.clone());
    let response = handler.handle(command, None).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn list_tags(
    State(state): State<FeatureState>,
    Query(query): Query<ListTagsQuery>,
) -> Result<Json<ListTagsResponse>, HandlerError> {
    let handler = ListTagsHandler::new(state.pool.clone());
    let response = handler.handle(query).await?;
    Ok(Json(response))
}
```

---

## Anti-Patterns

### ❌ DON'T: Mix commands and queries

```rust
// BAD: Single function does both read and write
pub async fn update_and_return_organization(
    pool: &PgPool,
    id: Uuid,
    name: String,
) -> Result<Organization, Error> {
    // Updates AND returns in one operation
    sqlx::query_as!(/* ... */).fetch_one(pool).await
}
```

```rust
// GOOD: Separate command and query
pub mod commands {
    pub async fn update_organization(/* ... */) -> Result<(), Error> {
        // Only updates
    }
}

pub mod queries {
    pub async fn get_organization(/* ... */) -> Result<Organization, Error> {
        // Only reads
    }
}
```

### ❌ DON'T: Skip audit logging

```rust
// BAD: Command without audit logging
pub async fn create_organization(command: CreateCommand) -> Result<Uuid, Error> {
    let org = db::create(command).await?;
    Ok(org.id) // No audit log!
}
```

```rust
// GOOD: Command with audit logging
pub async fn create_organization(
    command: CreateCommand,
    user_id: Option<Uuid>,
) -> Result<Uuid, Error> {
    let mut tx = pool.begin().await?;
    let org = db::create(&mut tx, command).await?;

    audit::log(&mut tx, AuditAction::Create, org.id, user_id).await?; // ✅

    tx.commit().await?;
    Ok(org.id)
}
```

### ❌ DON'T: Audit queries

```rust
// BAD: Auditing a read operation (wastes space, slows down queries)
pub async fn list_organizations(query: ListQuery) -> Result<Vec<Org>, Error> {
    let orgs = db::list(query).await?;
    audit::log(AuditAction::ListOrganizations).await?; // Don't do this!
    Ok(orgs)
}
```

```rust
// GOOD: Queries don't log
pub async fn list_organizations(query: ListQuery) -> Result<Vec<Org>, Error> {
    let orgs = db::list(query).await?;
    Ok(orgs) // No audit logging for reads
}
```

### ❌ DON'T: Put features in traditional layers

```rust
// BAD: Traditional layers
src/
  controllers/organizations.rs  ← Feature code scattered
  services/organizations.rs     ← across multiple directories
  repositories/organizations.rs ← Hard to find related code
```

```rust
// GOOD: Vertical slices
src/features/
  organizations/  ← Everything in one place
    commands/
    queries/
    routes.rs
```

### ❌ DON'T: Skip validation

```rust
// BAD: No validation
pub async fn handle(command: CreateCommand) -> Result<Response, Error> {
    db::create(command).await?; // What if command.name is empty?
}
```

```rust
// GOOD: Always validate
pub async fn handle(command: CreateCommand) -> Result<Response, Error> {
    command.validate()?; // Fail fast
    db::create(command).await?;
}
```

---

## Summary

### Mandatory Patterns

1. **✅ Use CQRS**: Separate commands (write) from queries (read)
2. **✅ Use Vertical Slices**: Organize by feature, not by layer
3. **✅ Audit Commands**: All state changes must be logged
4. **✅ Don't Audit Queries**: Read operations are not logged
5. **✅ Validate Early**: Check inputs before executing
6. **✅ Use Transactions**: Commands must be atomic
7. **✅ Instrument with Tracing**: All handlers must have `#[tracing::instrument]`
8. **✅ Test Thoroughly**: Commands test audit logs, queries test output

### References

- **Detailed CQRS Guide**: [docs/agents/implementation/cqrs-architecture.md](./cqrs-architecture.md)
- **Feature Workflow**: [docs/agents/workflows/adding-feature-cqrs.md](../workflows/adding-feature-cqrs.md)
- **SQLx Guide**: [docs/agents/implementation/sqlx-guide.md](./sqlx-guide.md)
- **Testing Strategy**: [docs/agents/testing.md](../testing.md)

### Research Sources

This architecture is based on industry best practices:

- [CQRS by Martin Fowler](https://martinfowler.com/bliki/CQRS.html)
- [Vertical Slice Architecture](https://jimmybogard.com/vertical-slice-architecture/)
- [serverlesstechnology/cqrs - Rust CQRS Framework](https://github.com/serverlesstechnology/cqrs)
- [Building APIs with Rust, CQRS, and Axum](https://blog.devgenius.io/creating-an-api-with-rust-clean-architecture-cqrs-axum-and-surrealdb-part-2-99a48b2d10bc)
- [Axum Middleware and Tower Layers](https://leapcell.io/blog/building-modular-web-services-with-axum-layers-for-observability-and-security)

---

**This architecture is MANDATORY for all backend features in BDP.**
