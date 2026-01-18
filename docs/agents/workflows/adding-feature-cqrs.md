# Workflow: Adding a New Feature with CQRS

Step-by-step guide for AI agents to add new features to BDP using CQRS architecture with vertical slices.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Step 1: Create Feature Module](#step-1-create-feature-module)
4. [Step 2: Define Models](#step-2-define-models)
5. [Step 3: Add Commands with Audit Logging](#step-3-add-commands-with-audit-logging)
6. [Step 4: Add Queries](#step-4-add-queries)
7. [Step 5: Register Routes](#step-5-register-routes)
8. [Step 6: Test Commands and Queries](#step-6-test-commands-and-queries)
9. [Step 7: Generate SQLx Metadata](#step-7-generate-sqlx-metadata)
10. [Complete Example](#complete-example)
11. [Quick Reference Checklist](#quick-reference-checklist)

---

## Overview

This workflow shows how to add a new feature following CQRS (Command Query Responsibility Segregation) pattern with vertical slice architecture.

```
Feature Development Flow
┌─────────────────────────────────────────────────────────┐
│  1. Create feature directory structure                  │
│  2. Define DTOs and command/query types                 │
│  3. Implement commands (with audit logging)             │
│  4. Implement queries (read-only, no audit)             │
│  5. Register API routes                                 │
│  6. Write integration tests                             │
│  7. Generate SQLx offline metadata                      │
└─────────────────────────────────────────────────────────┘
```

**Key Principles**:
- **Commands** modify state and require audit logging
- **Queries** read state without side effects
- All feature code lives in one directory
- Commands and queries are separate

---

## Prerequisites

Before starting:

```bash
# Ensure database is running
docker-compose up -d postgres

# Run migrations
sqlx migrate run

# Verify database connection
psql -h localhost -U bdp -d bdp -c "SELECT 1;"

# Check audit_log table exists
psql -h localhost -U bdp -d bdp -c "\d audit_log"
```

If audit_log doesn't exist, create it:

```sql
-- migrations/<timestamp>_create_audit_log.sql

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action VARCHAR(100) NOT NULL,
    user_id UUID,
    entity_id UUID NOT NULL,
    metadata JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX audit_log_action_idx ON audit_log(action);
CREATE INDEX audit_log_user_id_idx ON audit_log(user_id);
CREATE INDEX audit_log_entity_id_idx ON audit_log(entity_id);
CREATE INDEX audit_log_created_at_idx ON audit_log(created_at DESC);
CREATE INDEX audit_log_entity_created_idx ON audit_log(entity_id, created_at DESC);
```

---

## Step 1: Create Feature Module

### Step 1.1: Create Directory Structure

```bash
# Example: Adding a "tags" feature
cd crates/bdp-server/src

# Create feature directory
mkdir -p features/tags/commands
mkdir -p features/tags/queries

# Create module files
touch features/tags/commands/mod.rs
touch features/tags/queries/mod.rs
touch features/tags/models.rs
touch features/tags/errors.rs
touch features/tags/mod.rs
```

**Expected structure**:
```
features/tags/
├── commands/
│   └── mod.rs
├── queries/
│   └── mod.rs
├── models.rs
├── errors.rs
└── mod.rs
```

### Step 1.2: Initialize Feature Module

**File**: `features/tags/mod.rs`

```rust
//! Tags feature
//!
//! Provides tagging functionality for registry entries.

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

### Step 1.3: Register Feature in Parent Module

**File**: `features/mod.rs`

```rust
pub mod organizations;
pub mod sources;
pub mod tags;  // Add new feature
```

---

## Step 2: Define Models

### Step 2.1: Define DTOs

**File**: `features/tags/models.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Full tag DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDto {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Lightweight tag summary for lists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
}

/// Tag with usage count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagWithCount {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub usage_count: i64,
}
```

### Step 2.2: Define Command Types

**File**: `features/tags/models.rs` (continued)

```rust
/// Command to create a new tag
#[derive(Debug, Clone)]
pub struct CreateTagCommand {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

/// Result of creating a tag
#[derive(Debug, Clone)]
pub struct CreateTagResult {
    pub id: Uuid,
    pub name: String,
}

/// Command to update a tag
#[derive(Debug, Clone)]
pub struct UpdateTagCommand {
    pub id: Uuid,
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
}

/// Command to delete a tag
#[derive(Debug, Clone)]
pub struct DeleteTagCommand {
    pub id: Uuid,
}

/// Command to assign tag to entry
#[derive(Debug, Clone)]
pub struct AssignTagCommand {
    pub entry_id: Uuid,
    pub tag_id: Uuid,
}
```

### Step 2.3: Define Query Types

**File**: `features/tags/models.rs` (continued)

```rust
/// Query to get tag by ID
#[derive(Debug, Clone)]
pub struct GetTagQuery {
    pub id: Uuid,
}

/// Query to get tag by name
#[derive(Debug, Clone)]
pub struct GetTagByNameQuery {
    pub name: String,
}

/// Query to list tags
#[derive(Debug, Clone)]
pub struct ListTagsQuery {
    pub category: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Query to get tags for an entry
#[derive(Debug, Clone)]
pub struct GetEntryTagsQuery {
    pub entry_id: Uuid,
}

/// Query to get popular tags
#[derive(Debug, Clone)]
pub struct GetPopularTagsQuery {
    pub limit: i64,
}
```

### Step 2.4: Define Error Types

**File**: `features/tags/errors.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TagError {
    #[error("Tag with name '{0}' already exists")]
    DuplicateName(String),

    #[error("Tag not found")]
    NotFound,

    #[error("Invalid tag name")]
    InvalidName,

    #[error("Tag name too long (max 100 characters)")]
    NameTooLong,

    #[error("Cannot delete tag with active assignments")]
    HasAssignments,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Audit error: {0}")]
    Audit(#[from] crate::infrastructure::audit::AuditError),
}

// Convert to HTTP response
impl axum::response::IntoResponse for TagError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, message) = match self {
            Self::DuplicateName(_) => (StatusCode::CONFLICT, self.to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::InvalidName | Self::NameTooLong => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::HasAssignments => (StatusCode::CONFLICT, self.to_string()),
            Self::Database(_) | Self::Audit(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## Step 3: Add Commands with Audit Logging

Commands modify state and MUST include audit logging.

### Step 3.1: Implement Create Command

**File**: `features/tags/commands/create.rs`

```rust
use crate::features::tags::{CreateTagCommand, CreateTagResult, TagError};
use crate::infrastructure::audit::{AuditLogger, AuditAction};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Handler for CreateTagCommand
pub struct CreateTagHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
}

impl CreateTagHandler {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self { pool, audit_logger }
    }

    /// Execute the command
    pub async fn handle(
        &self,
        cmd: CreateTagCommand,
        user_id: Option<Uuid>,
    ) -> Result<CreateTagResult, TagError> {
        // 1. Validate input
        self.validate(&cmd)?;

        // 2. Check for duplicates
        if self.name_exists(&cmd.name).await? {
            return Err(TagError::DuplicateName(cmd.name.clone()));
        }

        // 3. Create tag within transaction
        let tag_id = Uuid::new_v4();
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO tags (id, name, category, description, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            tag_id,
            cmd.name,
            cmd.category,
            cmd.description,
            now
        )
        .execute(&mut *tx)
        .await?;

        // 4. Audit log
        self.audit_logger
            .log(
                &mut tx,
                AuditAction::CreateTag,
                user_id,
                tag_id,
                serde_json::json!({
                    "name": cmd.name,
                    "category": cmd.category
                }),
            )
            .await?;

        tx.commit().await?;

        tracing::info!(
            tag_id = %tag_id,
            name = %cmd.name,
            "Tag created"
        );

        // 5. Return minimal result
        Ok(CreateTagResult {
            id: tag_id,
            name: cmd.name,
        })
    }

    fn validate(&self, cmd: &CreateTagCommand) -> Result<(), TagError> {
        // Name validation
        if cmd.name.is_empty() {
            return Err(TagError::InvalidName);
        }

        if cmd.name.len() > 100 {
            return Err(TagError::NameTooLong);
        }

        // Alphanumeric and hyphens only
        if !cmd.name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(TagError::InvalidName);
        }

        Ok(())
    }

    async fn name_exists(&self, name: &str) -> Result<bool, TagError> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE name = $1)",
            name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::audit::AuditLogger;

    #[sqlx::test]
    async fn test_create_tag(pool: PgPool) {
        let audit_logger = AuditLogger::new(pool.clone());
        let handler = CreateTagHandler::new(pool.clone(), audit_logger);

        let cmd = CreateTagCommand {
            name: "test-tag".to_string(),
            category: Some("organism".to_string()),
            description: Some("Test description".to_string()),
        };

        let result = handler.handle(cmd, None).await.unwrap();
        assert_eq!(result.name, "test-tag");

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
    async fn test_create_duplicate_name(pool: PgPool) {
        let audit_logger = AuditLogger::new(pool.clone());
        let handler = CreateTagHandler::new(pool.clone(), audit_logger);

        let cmd = CreateTagCommand {
            name: "duplicate-tag".to_string(),
            category: None,
            description: None,
        };

        // First create succeeds
        handler.handle(cmd.clone(), None).await.unwrap();

        // Second create fails
        let result = handler.handle(cmd, None).await;
        assert!(matches!(result, Err(TagError::DuplicateName(_))));
    }
}
```

### Step 3.2: Implement Update Command

**File**: `features/tags/commands/update.rs`

```rust
use crate::features::tags::{UpdateTagCommand, TagError};
use crate::infrastructure::audit::{AuditLogger, AuditAction};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UpdateTagHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
}

impl UpdateTagHandler {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self { pool, audit_logger }
    }

    pub async fn handle(
        &self,
        cmd: UpdateTagCommand,
        user_id: Option<Uuid>,
    ) -> Result<(), TagError> {
        let mut tx = self.pool.begin().await?;

        // 1. Fetch current state for audit trail
        let old_tag = sqlx::query!(
            r#"
            SELECT name, category, description
            FROM tags
            WHERE id = $1
            "#,
            cmd.id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(TagError::NotFound)?;

        // 2. Build update fields
        let new_name = cmd.name.as_ref().unwrap_or(&old_tag.name);
        let new_category = cmd.category.as_ref().or(old_tag.category.as_ref());
        let new_description = cmd.description.as_ref().or(old_tag.description.as_ref());

        // 3. Validate new name if changed
        if let Some(ref name) = cmd.name {
            if name.len() > 100 {
                return Err(TagError::NameTooLong);
            }
        }

        // 4. Execute update
        sqlx::query!(
            r#"
            UPDATE tags
            SET
                name = $2,
                category = $3,
                description = $4,
                updated_at = NOW()
            WHERE id = $1
            "#,
            cmd.id,
            new_name,
            new_category,
            new_description
        )
        .execute(&mut *tx)
        .await?;

        // 5. Audit log with before/after
        self.audit_logger
            .log(
                &mut tx,
                AuditAction::UpdateTag,
                user_id,
                cmd.id,
                serde_json::json!({
                    "before": {
                        "name": old_tag.name,
                        "category": old_tag.category,
                        "description": old_tag.description
                    },
                    "after": {
                        "name": new_name,
                        "category": new_category,
                        "description": new_description
                    }
                }),
            )
            .await?;

        tx.commit().await?;

        tracing::info!(
            tag_id = %cmd.id,
            "Tag updated"
        );

        Ok(())
    }
}
```

### Step 3.3: Implement Delete Command

**File**: `features/tags/commands/delete.rs`

```rust
use crate::features::tags::TagError;
use crate::infrastructure::audit::{AuditLogger, AuditAction};
use sqlx::PgPool;
use uuid::Uuid;

pub struct DeleteTagHandler {
    pool: PgPool,
    audit_logger: AuditLogger,
}

impl DeleteTagHandler {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self { pool, audit_logger }
    }

    pub async fn handle(
        &self,
        tag_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<(), TagError> {
        let mut tx = self.pool.begin().await?;

        // 1. Check if tag has assignments
        let assignment_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM entry_tags WHERE tag_id = $1",
            tag_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if assignment_count.unwrap_or(0) > 0 {
            return Err(TagError::HasAssignments);
        }

        // 2. Delete tag
        let result = sqlx::query!(
            "DELETE FROM tags WHERE id = $1",
            tag_id
        )
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TagError::NotFound);
        }

        // 3. Audit log
        self.audit_logger
            .log(
                &mut tx,
                AuditAction::DeleteTag,
                user_id,
                tag_id,
                serde_json::json!({}),
            )
            .await?;

        tx.commit().await?;

        tracing::info!(
            tag_id = %tag_id,
            "Tag deleted"
        );

        Ok(())
    }
}
```

### Step 3.4: Export Commands

**File**: `features/tags/commands/mod.rs`

```rust
mod create;
mod update;
mod delete;
mod assign;

pub use create::*;
pub use update::*;
pub use delete::*;
pub use assign::*;
```

### Step 3.5: Update Audit Actions

**File**: `infrastructure/audit.rs`

```rust
#[derive(Debug, Clone, Copy)]
pub enum AuditAction {
    // ... existing actions ...

    // Tags
    CreateTag,
    UpdateTag,
    DeleteTag,
    AssignTag,
    UnassignTag,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ... existing cases ...
            Self::CreateTag => "create_tag",
            Self::UpdateTag => "update_tag",
            Self::DeleteTag => "delete_tag",
            Self::AssignTag => "assign_tag",
            Self::UnassignTag => "unassign_tag",
        }
    }
}
```

---

## Step 4: Add Queries

Queries are read-only and DO NOT require audit logging.

### Step 4.1: Implement Get Query

**File**: `features/tags/queries/get.rs`

```rust
use crate::features::tags::{GetTagQuery, TagDto, TagError};
use sqlx::PgPool;

/// Handler for GetTagQuery
pub struct GetTagHandler {
    pool: PgPool,
}

impl GetTagHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Execute the query
    pub async fn handle(&self, query: GetTagQuery) -> Result<Option<TagDto>, TagError> {
        let tag = sqlx::query_as!(
            TagDto,
            r#"
            SELECT id, name, category, description, created_at
            FROM tags
            WHERE id = $1
            "#,
            query.id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[sqlx::test]
    async fn test_get_tag(pool: PgPool) {
        let tag_id = Uuid::new_v4();

        // Insert test data
        sqlx::query!(
            r#"
            INSERT INTO tags (id, name, created_at)
            VALUES ($1, $2, NOW())
            "#,
            tag_id,
            "test-tag"
        )
        .execute(&pool)
        .await
        .unwrap();

        let handler = GetTagHandler::new(pool);
        let query = GetTagQuery { id: tag_id };

        let result = handler.handle(query).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "test-tag");
    }
}
```

### Step 4.2: Implement List Query

**File**: `features/tags/queries/list.rs`

```rust
use crate::features::tags::{ListTagsQuery, TagSummary, TagError};
use sqlx::PgPool;

pub struct ListTagsHandler {
    pool: PgPool,
}

impl ListTagsHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn handle(&self, query: ListTagsQuery) -> Result<Vec<TagSummary>, TagError> {
        let tags = if let Some(category) = query.category {
            sqlx::query_as!(
                TagSummary,
                r#"
                SELECT id, name, category
                FROM tags
                WHERE category = $1
                ORDER BY name ASC
                LIMIT $2 OFFSET $3
                "#,
                category,
                query.limit,
                query.offset
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                TagSummary,
                r#"
                SELECT id, name, category
                FROM tags
                ORDER BY name ASC
                LIMIT $1 OFFSET $2
                "#,
                query.limit,
                query.offset
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(tags)
    }
}
```

### Step 4.3: Implement Search Query

**File**: `features/tags/queries/search.rs`

```rust
use crate::features::tags::{TagSummary, TagError};
use sqlx::PgPool;

pub struct SearchTagsQuery {
    pub search_term: String,
    pub limit: i64,
}

pub struct SearchTagsHandler {
    pool: PgPool,
}

impl SearchTagsHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn handle(&self, query: SearchTagsQuery) -> Result<Vec<TagSummary>, TagError> {
        let pattern = format!("%{}%", query.search_term);

        let tags = sqlx::query_as!(
            TagSummary,
            r#"
            SELECT id, name, category
            FROM tags
            WHERE name ILIKE $1 OR description ILIKE $1
            ORDER BY name ASC
            LIMIT $2
            "#,
            pattern,
            query.limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tags)
    }
}
```

### Step 4.4: Export Queries

**File**: `features/tags/queries/mod.rs`

```rust
mod get;
mod get_by_name;
mod list;
mod search;
mod get_entry_tags;
mod get_popular;

pub use get::*;
pub use get_by_name::*;
pub use list::*;
pub use search::*;
pub use get_entry_tags::*;
pub use get_popular::*;
```

---

## Step 5: Register Routes

### Step 5.1: Create API Module

**File**: `api/tags.rs`

```rust
use crate::features::tags::*;
use crate::infrastructure::audit::AuditLogger;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// App state with handlers
#[derive(Clone)]
pub struct TagsAppState {
    // Command handlers
    create_tag_handler: CreateTagHandler,
    update_tag_handler: UpdateTagHandler,
    delete_tag_handler: DeleteTagHandler,

    // Query handlers
    get_tag_handler: GetTagHandler,
    list_tags_handler: ListTagsHandler,
    search_tags_handler: SearchTagsHandler,
}

impl TagsAppState {
    pub fn new(pool: PgPool, audit_logger: AuditLogger) -> Self {
        Self {
            create_tag_handler: CreateTagHandler::new(pool.clone(), audit_logger.clone()),
            update_tag_handler: UpdateTagHandler::new(pool.clone(), audit_logger.clone()),
            delete_tag_handler: DeleteTagHandler::new(pool.clone(), audit_logger),

            get_tag_handler: GetTagHandler::new(pool.clone()),
            list_tags_handler: ListTagsHandler::new(pool.clone()),
            search_tags_handler: SearchTagsHandler::new(pool),
        }
    }
}

/// Create tags router
pub fn tags_router(state: TagsAppState) -> Router {
    Router::new()
        .route("/tags", post(create_tag))
        .route("/tags/:id", get(get_tag))
        .route("/tags/:id", put(update_tag))
        .route("/tags/:id", delete(delete_tag_route))
        .route("/tags", get(list_tags))
        .route("/tags/search", get(search_tags))
        .with_state(state)
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTagResponse {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTagsParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub category: Option<String>,
}

fn default_limit() -> i64 {
    50
}

// ============================================================================
// Route Handlers
// ============================================================================

/// POST /tags - Create a new tag
async fn create_tag(
    State(state): State<TagsAppState>,
    Json(req): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<CreateTagResponse>), TagError> {
    let cmd = CreateTagCommand {
        name: req.name,
        category: req.category,
        description: req.description,
    };

    // TODO: Extract user_id from authentication token
    let user_id = None;

    let result = state.create_tag_handler.handle(cmd, user_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateTagResponse {
            id: result.id,
            name: result.name,
        }),
    ))
}

/// GET /tags/:id - Get tag by ID
async fn get_tag(
    State(state): State<TagsAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TagDto>, TagError> {
    let query = GetTagQuery { id };

    let tag = state
        .get_tag_handler
        .handle(query)
        .await?
        .ok_or(TagError::NotFound)?;

    Ok(Json(tag))
}

/// PUT /tags/:id - Update tag
async fn update_tag(
    State(state): State<TagsAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<StatusCode, TagError> {
    let cmd = UpdateTagCommand {
        id,
        name: req.name,
        category: req.category,
        description: req.description,
    };

    let user_id = None;

    state.update_tag_handler.handle(cmd, user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /tags/:id - Delete tag
async fn delete_tag_route(
    State(state): State<TagsAppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, TagError> {
    let user_id = None;

    state.delete_tag_handler.handle(id, user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /tags - List tags
async fn list_tags(
    State(state): State<TagsAppState>,
    Query(params): Query<ListTagsParams>,
) -> Result<Json<Vec<TagSummary>>, TagError> {
    let query = ListTagsQuery {
        category: params.category,
        limit: params.limit,
        offset: params.offset,
    };

    let tags = state.list_tags_handler.handle(query).await?;

    Ok(Json(tags))
}

/// GET /tags/search - Search tags
async fn search_tags(
    State(state): State<TagsAppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<TagSummary>>, TagError> {
    let query = SearchTagsQuery {
        search_term: params.q,
        limit: params.limit.unwrap_or(20),
    };

    let tags = state.search_tags_handler.handle(query).await?;

    Ok(Json(tags))
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<i64>,
}
```

### Step 5.2: Register in Main Router

**File**: `api/mod.rs`

```rust
pub mod organizations;
pub mod sources;
pub mod tags;  // Add new module

use axum::Router;
use crate::infrastructure::audit::AuditLogger;
use sqlx::PgPool;

pub fn create_api_router(pool: PgPool, audit_logger: AuditLogger) -> Router {
    let tags_state = tags::TagsAppState::new(pool.clone(), audit_logger.clone());

    Router::new()
        // ... existing routes ...
        .nest("/api/v1", tags::tags_router(tags_state))
}
```

---

## Step 6: Test Commands and Queries

### Step 6.1: Integration Tests for Commands

**File**: `tests/tags_tests.rs`

```rust
use bdp_server::features::tags::*;
use bdp_server::infrastructure::audit::AuditLogger;
use sqlx::PgPool;

#[sqlx::test]
async fn test_create_tag_command(pool: PgPool) {
    let audit_logger = AuditLogger::new(pool.clone());
    let handler = CreateTagHandler::new(pool.clone(), audit_logger);

    let cmd = CreateTagCommand {
        name: "test-tag".to_string(),
        category: Some("organism".to_string()),
        description: Some("Test".to_string()),
    };

    let result = handler.handle(cmd, None).await.unwrap();

    assert!(!result.id.is_nil());
    assert_eq!(result.name, "test-tag");

    // Verify in database
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM tags WHERE id = $1",
        result.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count.unwrap(), 1);

    // Verify audit log
    let audit_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM audit_log WHERE entity_id = $1 AND action = 'create_tag'",
        result.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(audit_count.unwrap(), 1);
}

#[sqlx::test]
async fn test_update_tag_command(pool: PgPool) {
    // Setup: Create tag
    let tag_id = uuid::Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO tags (id, name, created_at) VALUES ($1, $2, NOW())",
        tag_id,
        "original-name"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Test: Update tag
    let audit_logger = AuditLogger::new(pool.clone());
    let handler = UpdateTagHandler::new(pool.clone(), audit_logger);

    let cmd = UpdateTagCommand {
        id: tag_id,
        name: Some("updated-name".to_string()),
        category: None,
        description: None,
    };

    handler.handle(cmd, None).await.unwrap();

    // Verify: Name updated
    let name = sqlx::query_scalar!(
        "SELECT name FROM tags WHERE id = $1",
        tag_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(name.unwrap(), "updated-name");

    // Verify: Audit log with before/after
    let audit = sqlx::query!(
        r#"SELECT metadata FROM audit_log WHERE entity_id = $1 AND action = 'update_tag'"#,
        tag_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(audit.metadata.to_string().contains("original-name"));
    assert!(audit.metadata.to_string().contains("updated-name"));
}
```

### Step 6.2: Integration Tests for Queries

**File**: `tests/tags_tests.rs` (continued)

```rust
#[sqlx::test]
async fn test_get_tag_query(pool: PgPool) {
    let tag_id = uuid::Uuid::new_v4();

    // Setup: Insert tag
    sqlx::query!(
        r#"
        INSERT INTO tags (id, name, category, created_at)
        VALUES ($1, $2, $3, NOW())
        "#,
        tag_id,
        "test-tag",
        Some("organism")
    )
    .execute(&pool)
    .await
    .unwrap();

    // Test: Get tag
    let handler = GetTagHandler::new(pool);
    let query = GetTagQuery { id: tag_id };

    let result = handler.handle(query).await.unwrap();

    assert!(result.is_some());
    let tag = result.unwrap();
    assert_eq!(tag.name, "test-tag");
    assert_eq!(tag.category.as_deref(), Some("organism"));
}

#[sqlx::test]
async fn test_list_tags_query(pool: PgPool) {
    // Setup: Insert multiple tags
    for i in 1..=5 {
        sqlx::query!(
            "INSERT INTO tags (id, name, created_at) VALUES ($1, $2, NOW())",
            uuid::Uuid::new_v4(),
            format!("tag-{}", i)
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    // Test: List tags
    let handler = ListTagsHandler::new(pool);
    let query = ListTagsQuery {
        category: None,
        limit: 10,
        offset: 0,
    };

    let result = handler.handle(query).await.unwrap();

    assert_eq!(result.len(), 5);
}
```

### Step 6.3: API Integration Tests

**File**: `tests/api_tags_tests.rs`

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[sqlx::test]
async fn test_create_tag_api(pool: PgPool) {
    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tags")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name": "test-tag", "category": "organism"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["id"].is_string());
    assert_eq!(json["name"], "test-tag");
}

#[sqlx::test]
async fn test_get_tag_api(pool: PgPool) {
    let tag_id = uuid::Uuid::new_v4();

    // Setup
    sqlx::query!(
        "INSERT INTO tags (id, name, created_at) VALUES ($1, $2, NOW())",
        tag_id,
        "test-tag"
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tags/{}", tag_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

---

## Step 7: Generate SQLx Metadata

### Step 7.1: Run sqlx prepare

```bash
# Ensure database is running
docker-compose up -d postgres

# Set DATABASE_URL
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"

# Generate .sqlx metadata
cargo sqlx prepare

# Expected output:
# query data written to `.sqlx` in the current directory
```

### Step 7.2: Verify Generated Files

```bash
# Check .sqlx directory
ls -la .sqlx/

# Should see new query-*.json files for tag queries
# Verify one file
cat .sqlx/query-<hash>.json | jq .
```

### Step 7.3: Test Offline Build

```bash
# Clean build
cargo clean

# Build with offline mode
SQLX_OFFLINE=true cargo build

# Should succeed without database connection
```

### Step 7.4: Commit Changes

```bash
# Stage all changes
git add crates/bdp-server/src/features/tags/
git add crates/bdp-server/src/api/tags.rs
git add crates/bdp-server/tests/tags_tests.rs
git add .sqlx/

# Commit with descriptive message
git commit -m "feat: add tags feature with CQRS

- Implement CreateTagCommand with audit logging
- Implement UpdateTagCommand with before/after audit
- Implement DeleteTagCommand with assignment check
- Add read-only queries (get, list, search)
- Register API routes for tags
- Add integration tests for commands and queries
- Generate SQLx offline metadata

Commands:
- CreateTag: Creates new tag with validation
- UpdateTag: Updates tag with audit trail
- DeleteTag: Deletes tag if no assignments

Queries:
- GetTag: Retrieve single tag by ID
- ListTags: List tags with pagination and category filter
- SearchTags: Full-text search on tags

All commands include audit logging to track state changes."

# Push to remote
git push origin feature/tags-cqrs
```

---

## Complete Example

See the full implementation in:
- `crates/bdp-server/src/features/tags/` - Complete tags feature
- `crates/bdp-server/tests/tags_tests.rs` - Integration tests

Key files:
- `features/tags/commands/create.rs` - Command with audit logging
- `features/tags/queries/get.rs` - Read-only query
- `api/tags.rs` - API routes with handlers

---

## Quick Reference Checklist

Use this checklist when adding a new feature:

```
Feature Setup
□ Create feature directory (features/<name>/)
□ Create commands/, queries/ subdirectories
□ Create models.rs, errors.rs, mod.rs files
□ Define DTOs and command/query types
□ Define error enum with IntoResponse

Commands (Write Operations)
□ Implement command handlers
□ Add validation logic
□ Wrap in database transaction
□ Add audit logging within transaction
□ Return minimal result (ID only)
□ Write integration tests
□ Test audit log creation

Queries (Read Operations)
□ Implement query handlers
□ No audit logging (read-only)
□ Return full DTOs
□ Support pagination where applicable
□ Write integration tests
□ Consider caching (future)

API Layer
□ Create api/<feature>.rs module
□ Define request/response types
□ Create handler state with dependencies
□ Implement route handlers
□ Register routes in api/mod.rs
□ Write API integration tests

Infrastructure
□ Add AuditAction variants
□ Implement AuditAction::as_str()
□ Ensure audit_log table exists
□ Create database migrations if needed

Testing
□ Write command tests with audit verification
□ Write query tests
□ Write API integration tests
□ Test validation errors
□ Test duplicate detection
□ Test not found cases

Finalization
□ Run cargo sqlx prepare
□ Verify .sqlx files created
□ Test offline build (SQLX_OFFLINE=true)
□ Review all changes with git diff
□ Commit code and .sqlx together
□ Create PR with description
□ Verify CI passes
```

---

## Related Documentation

- [CQRS Architecture](../implementation/cqrs-architecture.md) - Architecture overview
- [SQLx Guide](../implementation/sqlx-guide.md) - Database queries
- [Adding Migration](./adding-migration.md) - Schema changes
- [Database Schema](../design/database-schema.md) - Database design

---

**Last Updated**: 2026-01-16
**Version**: 1.0.0
**Target Audience**: AI Agents
