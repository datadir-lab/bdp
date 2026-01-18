# CQRS Architecture for BDP Backend

Comprehensive guide to implementing Command Query Responsibility Segregation (CQRS) pattern with vertical slice architecture in the BDP Rust backend.

## Table of Contents

1. [Overview](#overview)
2. [CQRS Pattern Explanation](#cqrs-pattern-explanation)
3. [Vertical Slice Architecture](#vertical-slice-architecture)
4. [Command vs Query Separation](#command-vs-query-separation)
5. [Folder Structure](#folder-structure)
6. [Handler Pattern](#handler-pattern)
7. [Audit Logging Integration](#audit-logging-integration)
8. [Error Handling](#error-handling)
9. [Testing Strategy](#testing-strategy)
10. [Migration Strategy](#migration-strategy)
11. [Performance Considerations](#performance-considerations)
12. [Examples](#examples)

---

## Overview

### What is CQRS?

**Command Query Responsibility Segregation (CQRS)** is an architectural pattern that separates read operations (queries) from write operations (commands). This separation provides:

- **Clear separation of concerns**: Commands change state, queries retrieve state
- **Independent optimization**: Optimize reads and writes separately
- **Better security**: Fine-grained permissions (who can read vs write)
- **Audit trails**: Track all state changes through commands
- **Scalability**: Scale read and write paths independently

### Why CQRS for BDP?

BDP is a registry for biological data sources with:

1. **Write-heavy ingestion**: Automated scrapers ingesting proteins (commands)
2. **Read-heavy API**: Users browsing and searching (queries)
3. **Audit requirements**: Track who created/updated entries (commands)
4. **Different optimization needs**:
   - Commands: Transactional consistency, validation
   - Queries: Performance, caching, pagination

### CQRS + Vertical Slices

We combine CQRS with **vertical slice architecture**:

```
Traditional Layered              Vertical Slices (Feature-based)
┌─────────────────┐              ┌──────────────────────────────┐
│   Controllers   │              │      organizations/          │
├─────────────────┤              │  ├─ commands/                │
│    Services     │              │  │   ├─ create.rs            │
├─────────────────┤              │  │   └─ update.rs            │
│   Repositories  │              │  ├─ queries/                 │
├─────────────────┤              │  │   ├─ get_by_slug.rs       │
│     Models      │              │  │   └─ list.rs              │
└─────────────────┘              │  └─ mod.rs                   │
                                 └──────────────────────────────┘
```

**Benefits**:
- All organization-related code in one place
- Easy to find and modify features
- Reduced merge conflicts
- Clear feature boundaries
- Easy to add/remove features

---

## CQRS Pattern Explanation

### Core Principles

```rust
// Command: Changes state, returns success/failure
// - Validates input
// - Modifies database
// - Emits audit logs
// - Returns minimal data (ID, success)
pub async fn create_organization(cmd: CreateOrganizationCommand) -> Result<OrganizationId> {
    // 1. Validate
    // 2. Execute state change
    // 3. Audit log
    // 4. Return minimal result
}

// Query: Reads state, never modifies
// - No side effects
// - No audit logging
// - Can use read replicas
// - Can be cached
pub async fn get_organization(query: GetOrganizationQuery) -> Result<Organization> {
    // 1. Fetch from database
    // 2. Transform/map data
    // 3. Return result
}
```

### Command Characteristics

**Commands are imperative** (verb-based):
- `CreateOrganizationCommand`
- `UpdateOrganizationCommand`
- `DeleteOrganizationCommand`
- `PublishVersionCommand`

**Properties**:
- Change system state
- Require validation
- Need audit logging
- Return minimal data
- Are transactional
- Check permissions

### Query Characteristics

**Queries are interrogative** (noun-based):
- `GetOrganizationQuery`
- `ListOrganizationsQuery`
- `SearchProteinsQuery`
- `GetDependenciesQuery`

**Properties**:
- Read-only operations
- No side effects
- No audit logging
- Return full DTOs
- Can use caching
- Can use read replicas

---

## Vertical Slice Architecture

### Feature-Based Organization

Instead of organizing by technical layers (controllers, services, repositories), organize by features (organizations, sources, versions).

```
crates/bdp-server/src/
├── features/                    # All features here
│   ├── organizations/           # Organization feature
│   │   ├── commands/            # Write operations
│   │   │   ├── create.rs        # Create organization command
│   │   │   ├── update.rs        # Update organization command
│   │   │   ├── delete.rs        # Delete organization command
│   │   │   └── mod.rs           # Command exports
│   │   ├── queries/             # Read operations
│   │   │   ├── get_by_slug.rs   # Get single organization
│   │   │   ├── get_by_id.rs     # Get by UUID
│   │   │   ├── list.rs          # List with pagination
│   │   │   ├── search.rs        # Full-text search
│   │   │   └── mod.rs           # Query exports
│   │   ├── models.rs            # Shared types (Organization, DTOs)
│   │   ├── errors.rs            # Feature-specific errors
│   │   └── mod.rs               # Feature root
│   │
│   ├── sources/                 # Data sources feature
│   │   ├── commands/
│   │   │   ├── create.rs
│   │   │   ├── update.rs
│   │   │   └── mod.rs
│   │   ├── queries/
│   │   │   ├── get.rs
│   │   │   ├── list.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   │
│   ├── versions/                # Version management feature
│   │   ├── commands/
│   │   │   ├── create.rs
│   │   │   ├── publish.rs
│   │   │   └── mod.rs
│   │   ├── queries/
│   │   │   ├── get.rs
│   │   │   ├── list.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   │
│   └── mod.rs                   # Feature module exports
│
├── infrastructure/              # Cross-cutting concerns
│   ├── audit.rs                 # Audit logging
│   ├── database.rs              # Database connection
│   ├── validation.rs            # Validation utilities
│   └── mod.rs
│
├── api/                         # HTTP layer
│   ├── organizations.rs         # Organization routes
│   ├── sources.rs               # Source routes
│   └── mod.rs
│
└── main.rs
```

### Feature Module Structure

Each feature follows this template:

```rust
// features/organizations/mod.rs

pub mod commands;
pub mod queries;
pub mod models;
pub mod errors;

// Re-export for convenience
pub use commands::*;
pub use queries::*;
pub use models::*;
pub use errors::*;
```

---

## Command vs Query Separation

### Commands: Write Operations

Commands modify state and require audit logging.

```rust
// features/organizations/commands/create.rs

use crate::infrastructure::audit::{AuditLogger, AuditAction};
use sqlx::PgPool;
use uuid::Uuid;

/// Command to create a new organization
#[derive(Debug, Clone)]
pub struct CreateOrganizationCommand {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub is_system: bool,
}

/// Result of creating an organization
#[derive(Debug)]
pub struct CreateOrganizationResult {
    pub id: Uuid,
    pub slug: String,
}

/// Handler for CreateOrganizationCommand
pub struct CreateOrganizationHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
}

impl CreateOrganizationHandler {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self { pool, audit_logger }
    }

    /// Execute the command
    pub async fn handle(
        &self,
        cmd: CreateOrganizationCommand,
        user_id: Option<Uuid>,
    ) -> Result<CreateOrganizationResult, OrganizationError> {
        // 1. Validate input
        self.validate(&cmd)?;

        // 2. Check for duplicates
        if self.slug_exists(&cmd.slug).await? {
            return Err(OrganizationError::DuplicateSlug(cmd.slug.clone()));
        }

        // 3. Create organization (within transaction)
        let org_id = Uuid::new_v4();
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, description, website, is_system, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            org_id,
            cmd.slug,
            cmd.name,
            cmd.description,
            cmd.website,
            cmd.is_system,
            now,
            now
        )
        .execute(&mut *tx)
        .await?;

        // 4. Log audit trail
        self.audit_logger.log(
            &mut tx,
            AuditAction::CreateOrganization,
            user_id,
            org_id,
            serde_json::json!({
                "slug": cmd.slug,
                "name": cmd.name,
                "is_system": cmd.is_system
            })
        ).await?;

        tx.commit().await?;

        // 5. Return minimal result
        Ok(CreateOrganizationResult {
            id: org_id,
            slug: cmd.slug,
        })
    }

    fn validate(&self, cmd: &CreateOrganizationCommand) -> Result<(), OrganizationError> {
        // Slug validation
        if !cmd.slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(OrganizationError::InvalidSlug);
        }

        // Name validation
        if cmd.name.is_empty() || cmd.name.len() > 256 {
            return Err(OrganizationError::InvalidName);
        }

        Ok(())
    }

    async fn slug_exists(&self, slug: &str) -> Result<bool, OrganizationError> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM organizations WHERE slug = $1)",
            slug
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }
}
```

### Queries: Read Operations

Queries retrieve state without side effects or audit logging.

```rust
// features/organizations/queries/get_by_slug.rs

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Query to get organization by slug
#[derive(Debug, Clone)]
pub struct GetOrganizationBySlugQuery {
    pub slug: String,
}

/// Full organization DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationDto {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo_url: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Handler for GetOrganizationBySlugQuery
pub struct GetOrganizationBySlugHandler {
    pool: PgPool,
}

impl GetOrganizationBySlugHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Execute the query
    pub async fn handle(
        &self,
        query: GetOrganizationBySlugQuery,
    ) -> Result<Option<OrganizationDto>, OrganizationError> {
        // Simple query - no validation, no audit logging
        let org = sqlx::query_as!(
            OrganizationDto,
            r#"
            SELECT id, slug, name, description, website, logo_url, is_system, created_at, updated_at
            FROM organizations
            WHERE slug = $1
            "#,
            query.slug
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(org)
    }
}
```

---

## Folder Structure

### Complete Example: Organizations Feature

```
features/organizations/
├── commands/
│   ├── create.rs              # CreateOrganizationCommand + Handler
│   ├── update.rs              # UpdateOrganizationCommand + Handler
│   ├── delete.rs              # DeleteOrganizationCommand + Handler
│   └── mod.rs                 # Command exports
│
├── queries/
│   ├── get_by_slug.rs         # GetOrganizationBySlugQuery + Handler
│   ├── get_by_id.rs           # GetOrganizationByIdQuery + Handler
│   ├── list.rs                # ListOrganizationsQuery + Handler
│   ├── search.rs              # SearchOrganizationsQuery + Handler
│   ├── get_statistics.rs      # GetOrganizationStatisticsQuery + Handler
│   └── mod.rs                 # Query exports
│
├── models.rs                  # Shared types
│   ├── OrganizationDto        # Full organization data
│   ├── OrganizationSummary    # Lightweight list item
│   ├── OrganizationStatistics # Statistics DTO
│
├── errors.rs                  # Feature-specific errors
│   ├── OrganizationError      # Error enum
│
└── mod.rs                     # Feature root
```

### File Naming Conventions

**Commands** (verb-based):
- `create.rs` - Create new entity
- `update.rs` - Update existing entity
- `delete.rs` - Delete entity
- `publish.rs` - Publish/activate entity
- `archive.rs` - Archive/deactivate entity

**Queries** (noun-based):
- `get.rs` or `get_by_slug.rs` - Get single entity
- `list.rs` - List with pagination
- `search.rs` - Full-text search
- `get_statistics.rs` - Aggregate statistics
- `get_dependencies.rs` - Related entities

---

## Handler Pattern

### Handler Structure

Each command/query has a dedicated handler:

```rust
// Generic handler pattern

pub struct CommandHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
    // Other dependencies
}

impl CommandHandler {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self { pool, audit_logger }
    }

    pub async fn handle(
        &self,
        cmd: Command,
        user_id: Option<Uuid>,
    ) -> Result<CommandResult, Error> {
        // 1. Validate
        // 2. Execute
        // 3. Audit
        // 4. Return
    }
}
```

### Handler Composition

Handlers can depend on other handlers:

```rust
// features/versions/commands/publish.rs

pub struct PublishVersionHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
    // Depends on query handler
    get_version_handler: GetVersionHandler,
}

impl PublishVersionHandler {
    pub async fn handle(
        &self,
        cmd: PublishVersionCommand,
        user_id: Option<Uuid>,
    ) -> Result<PublishVersionResult, VersionError> {
        // 1. Use query handler to verify version exists
        let version = self.get_version_handler
            .handle(GetVersionQuery { id: cmd.version_id })
            .await?
            .ok_or(VersionError::NotFound)?;

        // 2. Validate version is publishable
        if version.published {
            return Err(VersionError::AlreadyPublished);
        }

        // 3. Execute publish
        // ...
    }
}
```

### Dependency Injection

Use constructor injection for handler dependencies:

```rust
// main.rs or api module

use crate::features::organizations::commands::*;
use crate::features::organizations::queries::*;

pub struct AppState {
    // Command handlers
    pub create_org_handler: CreateOrganizationHandler,
    pub update_org_handler: UpdateOrganizationHandler,

    // Query handlers
    pub get_org_by_slug_handler: GetOrganizationBySlugHandler,
    pub list_orgs_handler: ListOrganizationsHandler,
}

impl AppState {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self {
            // Initialize command handlers with audit logger
            create_org_handler: CreateOrganizationHandler::new(pool.clone(), audit_logger.clone()),
            update_org_handler: UpdateOrganizationHandler::new(pool.clone(), audit_logger.clone()),

            // Initialize query handlers (no audit logger needed)
            get_org_by_slug_handler: GetOrganizationBySlugHandler::new(pool.clone()),
            list_orgs_handler: ListOrganizationsHandler::new(pool.clone()),
        }
    }
}
```

---

## Audit Logging Integration

### Audit Logger Design

```rust
// infrastructure/audit.rs

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Audit action types
#[derive(Debug, Clone, Copy)]
pub enum AuditAction {
    // Organizations
    CreateOrganization,
    UpdateOrganization,
    DeleteOrganization,

    // Sources
    CreateSource,
    UpdateSource,
    DeleteSource,

    // Versions
    CreateVersion,
    PublishVersion,
    DeprecateVersion,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateOrganization => "create_organization",
            Self::UpdateOrganization => "update_organization",
            Self::DeleteOrganization => "delete_organization",
            Self::CreateSource => "create_source",
            Self::UpdateSource => "update_source",
            Self::DeleteSource => "delete_source",
            Self::CreateVersion => "create_version",
            Self::PublishVersion => "publish_version",
            Self::DeprecateVersion => "deprecate_version",
        }
    }
}

/// Audit logger for tracking state changes
#[derive(Clone)]
pub struct AuditLogger {
    pool: PgPool,
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Log an audit event within a transaction
    pub async fn log(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        action: AuditAction,
        user_id: Option<Uuid>,
        entity_id: Uuid,
        metadata: Value,
    ) -> Result<(), AuditError> {
        let event_id = Uuid::new_v4();
        let timestamp = Utc::now();

        sqlx::query!(
            r#"
            INSERT INTO audit_log (id, action, user_id, entity_id, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            event_id,
            action.as_str(),
            user_id,
            entity_id,
            metadata,
            timestamp
        )
        .execute(&mut **tx)
        .await?;

        tracing::info!(
            audit_event_id = %event_id,
            action = %action.as_str(),
            user_id = ?user_id,
            entity_id = %entity_id,
            "Audit log entry created"
        );

        Ok(())
    }

    /// Log without transaction (use sparingly)
    pub async fn log_standalone(
        &self,
        action: AuditAction,
        user_id: Option<Uuid>,
        entity_id: Uuid,
        metadata: Value,
    ) -> Result<(), AuditError> {
        let mut tx = self.pool.begin().await?;
        self.log(&mut tx, action, user_id, entity_id, metadata).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### Audit Log Schema

```sql
-- Migration: Add audit_log table

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action VARCHAR(100) NOT NULL,        -- 'create_organization', 'update_source'
    user_id UUID,                        -- NULL for system actions
    entity_id UUID NOT NULL,             -- ID of affected entity
    metadata JSONB NOT NULL,             -- Action-specific data
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for querying
CREATE INDEX audit_log_action_idx ON audit_log(action);
CREATE INDEX audit_log_user_id_idx ON audit_log(user_id);
CREATE INDEX audit_log_entity_id_idx ON audit_log(entity_id);
CREATE INDEX audit_log_created_at_idx ON audit_log(created_at DESC);

-- Composite index for entity history
CREATE INDEX audit_log_entity_created_idx ON audit_log(entity_id, created_at DESC);
```

### Using Audit Logger in Commands

```rust
// features/organizations/commands/update.rs

pub async fn handle(
    &self,
    cmd: UpdateOrganizationCommand,
    user_id: Option<Uuid>,
) -> Result<UpdateOrganizationResult, OrganizationError> {
    let mut tx = self.pool.begin().await?;

    // 1. Fetch current state
    let old_org = sqlx::query_as!(
        Organization,
        "SELECT * FROM organizations WHERE id = $1",
        cmd.id
    )
    .fetch_one(&mut *tx)
    .await?;

    // 2. Update
    sqlx::query!(
        r#"
        UPDATE organizations
        SET name = $2, description = $3, updated_at = NOW()
        WHERE id = $1
        "#,
        cmd.id,
        cmd.name,
        cmd.description
    )
    .execute(&mut *tx)
    .await?;

    // 3. Audit log with before/after
    self.audit_logger.log(
        &mut tx,
        AuditAction::UpdateOrganization,
        user_id,
        cmd.id,
        serde_json::json!({
            "before": {
                "name": old_org.name,
                "description": old_org.description
            },
            "after": {
                "name": cmd.name,
                "description": cmd.description
            }
        })
    ).await?;

    tx.commit().await?;

    Ok(UpdateOrganizationResult { id: cmd.id })
}
```

### Querying Audit Logs

```rust
// features/organizations/queries/get_audit_history.rs

pub struct GetOrganizationAuditHistoryQuery {
    pub org_id: Uuid,
    pub limit: i64,
    pub offset: i64,
}

pub struct AuditHistoryDto {
    pub id: Uuid,
    pub action: String,
    pub user_id: Option<Uuid>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

pub async fn handle(
    &self,
    query: GetOrganizationAuditHistoryQuery,
) -> Result<Vec<AuditHistoryDto>, OrganizationError> {
    let history = sqlx::query_as!(
        AuditHistoryDto,
        r#"
        SELECT id, action, user_id, metadata, created_at
        FROM audit_log
        WHERE entity_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        query.org_id,
        query.limit,
        query.offset
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(history)
}
```

---

## Error Handling

### Feature-Specific Errors

Each feature defines its own error types:

```rust
// features/organizations/errors.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrganizationError {
    #[error("Organization with slug '{0}' already exists")]
    DuplicateSlug(String),

    #[error("Organization not found")]
    NotFound,

    #[error("Invalid slug format")]
    InvalidSlug,

    #[error("Invalid name")]
    InvalidName,

    #[error("Cannot delete system organization")]
    CannotDeleteSystem,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Audit error: {0}")]
    Audit(#[from] crate::infrastructure::audit::AuditError),
}

// Convert to HTTP response
impl axum::response::IntoResponse for OrganizationError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, message) = match self {
            Self::DuplicateSlug(_) => (StatusCode::CONFLICT, self.to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::InvalidSlug | Self::InvalidName => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::CannotDeleteSystem => (StatusCode::FORBIDDEN, self.to_string()),
            Self::Database(_) | Self::Audit(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## Testing Strategy

### Command Testing

Commands require integration tests with database:

```rust
// features/organizations/commands/create.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::audit::AuditLogger;

    #[sqlx::test]
    async fn test_create_organization(pool: PgPool) {
        let audit_logger = AuditLogger::new(pool.clone());
        let handler = CreateOrganizationHandler::new(pool.clone(), audit_logger);

        let cmd = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test Organization".to_string(),
            description: Some("Test description".to_string()),
            website: None,
            is_system: false,
        };

        let result = handler.handle(cmd, None).await.unwrap();

        assert_eq!(result.slug, "test-org");

        // Verify audit log
        let audit_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM audit_log WHERE entity_id = $1",
            result.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(audit_count.unwrap(), 1);
    }

    #[sqlx::test]
    async fn test_create_duplicate_slug(pool: PgPool) {
        let audit_logger = AuditLogger::new(pool.clone());
        let handler = CreateOrganizationHandler::new(pool.clone(), audit_logger);

        let cmd = CreateOrganizationCommand {
            slug: "test-org".to_string(),
            name: "Test".to_string(),
            description: None,
            website: None,
            is_system: false,
        };

        // First create succeeds
        handler.handle(cmd.clone(), None).await.unwrap();

        // Second create fails
        let result = handler.handle(cmd, None).await;
        assert!(matches!(result, Err(OrganizationError::DuplicateSlug(_))));
    }
}
```

### Query Testing

Queries can use simpler tests:

```rust
// features/organizations/queries/get_by_slug.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_organization_by_slug(pool: PgPool) {
        // Insert test data
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            "#,
            Uuid::new_v4(),
            "test-org",
            "Test Org"
        )
        .execute(&pool)
        .await
        .unwrap();

        let handler = GetOrganizationBySlugHandler::new(pool);
        let query = GetOrganizationBySlugQuery {
            slug: "test-org".to_string(),
        };

        let result = handler.handle(query).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().slug, "test-org");
    }

    #[sqlx::test]
    async fn test_get_nonexistent_organization(pool: PgPool) {
        let handler = GetOrganizationBySlugHandler::new(pool);
        let query = GetOrganizationBySlugQuery {
            slug: "nonexistent".to_string(),
        };

        let result = handler.handle(query).await.unwrap();
        assert!(result.is_none());
    }
}
```

---

## Migration Strategy

### Phase 1: Add CQRS Infrastructure

1. Create `features/` directory
2. Add `infrastructure/audit.rs`
3. Add audit_log table migration
4. Create first feature (organizations)

### Phase 2: Migrate Existing Code

Current structure:
```
db/organizations.rs     (mixed read/write)
api/organizations.rs    (API routes)
```

New structure:
```
features/organizations/
  commands/create.rs     (from db/organizations.rs create function)
  commands/update.rs     (from db/organizations.rs update function)
  queries/get_by_slug.rs (from db/organizations.rs get function)
  queries/list.rs        (from db/organizations.rs list function)
```

**Migration steps**:

1. **Extract queries** (no audit needed):
   ```bash
   # Move read functions to queries/
   cp db/organizations.rs features/organizations/queries/get_by_slug.rs
   # Edit to keep only get_organization_by_slug function
   ```

2. **Extract commands** (add audit logging):
   ```bash
   # Move write functions to commands/
   cp db/organizations.rs features/organizations/commands/create.rs
   # Edit to:
   # - Keep only create_organization function
   # - Add audit logging
   # - Wrap in transaction
   ```

3. **Update API routes**:
   ```rust
   // Before
   use crate::db::organizations;

   async fn create_org(State(state): State<AppState>, Json(req): Json<CreateOrgRequest>) {
       organizations::create_organization(&state.pool, params).await?;
   }

   // After
   use crate::features::organizations::commands::CreateOrganizationHandler;

   async fn create_org(
       State(state): State<AppState>,
       Json(req): Json<CreateOrgRequest>
   ) {
       state.create_org_handler.handle(cmd, user_id).await?;
   }
   ```

4. **Deprecate old code**:
   ```rust
   // db/organizations.rs
   #[deprecated(note = "Use features/organizations/commands/create.rs")]
   pub async fn create_organization(...) { ... }
   ```

### Phase 3: Add New Features with CQRS

All new features must follow CQRS from the start:
- Create feature directory
- Add commands/ and queries/ subdirectories
- Implement handlers
- Add audit logging to commands
- Write tests

---

## Performance Considerations

### Command Performance

**Use transactions**:
```rust
let mut tx = pool.begin().await?;
// Execute queries
tx.commit().await?;
```

**Batch operations**:
```rust
// Good: Single query
sqlx::query!("INSERT INTO ... SELECT * FROM unnest($1)")
    .bind(&ids)
    .execute(&mut tx)
    .await?;

// Bad: N queries
for id in ids {
    sqlx::query!("INSERT INTO ... VALUES ($1)").bind(id).execute(&mut tx).await?;
}
```

### Query Performance

**Use pagination**:
```rust
pub struct ListOrganizationsQuery {
    pub limit: i64,    // Default: 50
    pub offset: i64,   // Default: 0
}
```

**Add caching** (future):
```rust
// Cache query results
#[cached(time = 300)]  // 5 minutes
pub async fn list_organizations(...) -> Result<Vec<OrganizationDto>> {
    // Query database
}
```

**Read replicas** (future):
```rust
// Queries use read replica
let read_pool = PgPool::connect(&read_replica_url).await?;

pub struct GetOrganizationBySlugHandler {
    pool: PgPool,  // Read replica
}
```

---

## Examples

### Complete Feature Example: Sources

```
features/sources/
├── commands/
│   ├── create.rs
│   ├── update.rs
│   ├── delete.rs
│   └── mod.rs
├── queries/
│   ├── get.rs
│   ├── list.rs
│   ├── search.rs
│   └── mod.rs
├── models.rs
├── errors.rs
└── mod.rs
```

**models.rs**:
```rust
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDto {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub slug: String,
    pub name: String,
    pub source_type: String,
    pub external_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSourceCommand {
    pub organization_id: Uuid,
    pub slug: String,
    pub name: String,
    pub source_type: String,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetSourceQuery {
    pub org_slug: String,
    pub source_slug: String,
}
```

**commands/create.rs**:
```rust
use crate::features::sources::{SourceDto, SourceError};
use crate::infrastructure::audit::{AuditLogger, AuditAction};

pub struct CreateSourceHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
}

impl CreateSourceHandler {
    pub async fn handle(
        &self,
        cmd: CreateSourceCommand,
        user_id: Option<Uuid>,
    ) -> Result<Uuid, SourceError> {
        // Validate
        self.validate(&cmd)?;

        let source_id = Uuid::new_v4();
        let entry_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;

        // Insert into registry_entries
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            "#,
            entry_id,
            cmd.organization_id,
            cmd.slug,
            cmd.name
        )
        .execute(&mut *tx)
        .await?;

        // Insert into data_sources
        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type, external_id)
            VALUES ($1, $2, $3)
            "#,
            entry_id,
            cmd.source_type,
            cmd.external_id
        )
        .execute(&mut *tx)
        .await?;

        // Audit log
        self.audit_logger.log(
            &mut tx,
            AuditAction::CreateSource,
            user_id,
            entry_id,
            serde_json::json!({
                "slug": cmd.slug,
                "source_type": cmd.source_type
            })
        ).await?;

        tx.commit().await?;

        Ok(entry_id)
    }

    fn validate(&self, cmd: &CreateSourceCommand) -> Result<(), SourceError> {
        // Validation logic
        Ok(())
    }
}
```

**queries/get.rs**:
```rust
use crate::features::sources::{SourceDto, SourceError};

pub struct GetSourceHandler {
    pool: PgPool,
}

impl GetSourceHandler {
    pub async fn handle(
        &self,
        query: GetSourceQuery,
    ) -> Result<Option<SourceDto>, SourceError> {
        let source = sqlx::query_as!(
            SourceDto,
            r#"
            SELECT
                re.id,
                re.organization_id,
                re.slug,
                re.name,
                ds.source_type,
                ds.external_id,
                re.created_at,
                re.updated_at
            FROM registry_entries re
            JOIN data_sources ds ON ds.id = re.id
            JOIN organizations o ON o.id = re.organization_id
            WHERE o.slug = $1 AND re.slug = $2
            "#,
            query.org_slug,
            query.source_slug
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(source)
    }
}
```

---

## Related Documentation

- [Adding Feature with CQRS](../workflows/adding-feature-cqrs.md) - Step-by-step workflow
- [Database Schema](../design/database-schema.md) - Database design
- [API Design](../design/api-design.md) - API endpoints
- [SQLx Guide](./sqlx-guide.md) - Database query patterns

---

**Last Updated**: 2026-01-16
**Version**: 1.0.0
**Status**: Active
