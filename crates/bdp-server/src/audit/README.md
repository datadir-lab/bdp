# Audit Logging System

Comprehensive audit logging system for tracking all actions in the BDP server.

## Overview

The audit logging system provides:

- **Automatic audit logging** via middleware for all command operations (POST, PUT, PATCH, DELETE)
- **Manual audit logging** via function calls for custom scenarios
- **Comprehensive querying** with filters and pagination
- **Before/after state tracking** for updates
- **User context capture** from authentication headers
- **IP address and user agent logging** for security and debugging

## Architecture

The system follows CQRS (Command Query Responsibility Segregation) principles:

- **Commands** (write operations) are audited
- **Queries** (read operations) are not audited to reduce noise

### Components

1. **Database Migration** (`migrations/20260116000016_audit_log.sql`)
   - Creates `audit_log` table with proper indexes
   - Supports UUID user IDs, resource tracking, JSONB for changes/metadata
   - CHECK constraints for valid actions and resource types

2. **Models** (`models.rs`)
   - `AuditEntry`: Database record with all audit data
   - `AuditAction`: Enum of possible actions (Create, Update, Delete, etc.)
   - `ResourceType`: Enum of resource types (Organization, DataSource, etc.)
   - `CreateAuditEntry`: Builder for creating audit entries
   - `AuditQuery`: Query filters for searching audit logs

3. **Queries** (`queries.rs`)
   - `create_audit_entry()`: Insert new audit record
   - `query_audit_logs()`: Search with filters and pagination
   - `get_audit_trail()`: Get full history for a resource
   - `get_user_audit_logs()`: Get all actions by a user

4. **Middleware** (`middleware.rs`)
   - `AuditLayer`: Tower layer for automatic audit logging
   - Captures request body for commands
   - Extracts user info from headers
   - Logs after successful execution
   - Non-blocking database writes

## Usage

### Automatic Audit Logging (Middleware)

Add the audit layer to your Axum application:

```rust
use axum::Router;
use sqlx::PgPool;
use tower::ServiceBuilder;
use bdp_server::audit::AuditLayer;

async fn setup_router(pool: PgPool) -> Router {
    Router::new()
        .route("/api/v1/organizations", post(create_organization))
        .layer(
            ServiceBuilder::new()
                .layer(AuditLayer::new(pool.clone()))
        )
}
```

The middleware will automatically:
- Capture POST/PUT/PATCH/DELETE requests
- Extract user ID from `x-user-id` header
- Log IP address and user agent
- Store request body as changes
- Only log successful operations (2xx responses)

### Manual Audit Logging

For custom audit logging scenarios:

```rust
use bdp_server::audit::{
    create_audit_entry, CreateAuditEntry, AuditAction, ResourceType
};
use sqlx::PgPool;
use uuid::Uuid;

async fn example(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Using the builder pattern
    let entry = CreateAuditEntry::builder()
        .action(AuditAction::Create)
        .resource_type(ResourceType::Organization)
        .resource_id(Some(Uuid::new_v4()))
        .user_id(Some(Uuid::new_v4()))
        .ip_address("192.168.1.1")
        .user_agent("Mozilla/5.0")
        .changes(serde_json::json!({
            "name": "New Organization",
            "website": "https://example.com"
        }))
        .metadata(serde_json::json!({
            "request_id": "req-123",
            "session_id": "sess-456"
        }))
        .build();

    let audit_log = create_audit_entry(pool, entry).await?;
    println!("Created audit log: {}", audit_log.id);

    Ok(())
}
```

### Querying Audit Logs

#### Get audit trail for a specific resource

```rust
use bdp_server::audit::{get_audit_trail, ResourceType};
use uuid::Uuid;

let resource_id = Uuid::parse_str("...")?;
let trail = get_audit_trail(
    &pool,
    ResourceType::Organization,
    resource_id,
    Some(50) // Limit to 50 most recent entries
).await?;

for entry in trail {
    println!("{}: {} on {}",
        entry.timestamp,
        entry.action,
        entry.resource_type
    );
}
```

#### Get all actions by a user

```rust
use bdp_server::audit::get_user_audit_logs;

let user_id = Uuid::parse_str("...")?;
let logs = get_user_audit_logs(&pool, user_id, Some(100)).await?;

for log in logs {
    println!("User {} {} {}",
        log.user_id.unwrap(),
        log.action,
        log.resource_type
    );
}
```

#### Advanced filtering

```rust
use bdp_server::audit::{query_audit_logs, AuditQuery, AuditAction, ResourceType};
use chrono::{Utc, Duration};

let query = AuditQuery {
    user_id: Some(Uuid::parse_str("...")?),
    action: Some(AuditAction::Update),
    resource_type: Some(ResourceType::DataSource),
    start_time: Some(Utc::now() - Duration::days(7)),
    end_time: Some(Utc::now()),
    limit: 100,
    offset: 0,
};

let results = query_audit_logs(&pool, query).await?;
```

## Audit Actions

Supported audit actions:

- `Create`: New resource created
- `Update`: Resource modified
- `Delete`: Resource deleted
- `Read`: Resource accessed (typically not logged)
- `Login`: User authenticated
- `Logout`: User logged out
- `Register`: New user registered
- `Publish`: Resource published
- `Unpublish`: Resource unpublished
- `Archive`: Resource archived
- `Upload`: File uploaded
- `Download`: File downloaded
- `Grant`: Permission granted
- `Revoke`: Permission revoked
- `Other`: Other actions

## Resource Types

Supported resource types:

- `Organization`
- `DataSource`
- `Version`
- `Tool`
- `RegistryEntry`
- `VersionFile`
- `Dependency`
- `Organism`
- `ProteinMetadata`
- `Citation`
- `Tag`
- `Download`
- `VersionMapping`
- `User`
- `Session`
- `ApiKey`
- `Other`

## Database Schema

The `audit_log` table includes:

```sql
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,  -- Nullable for anonymous actions
    action VARCHAR(50) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id UUID,
    changes JSONB,  -- Before/after state
    ip_address VARCHAR(45),  -- IPv4 or IPv6
    user_agent TEXT,
    timestamp TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    metadata JSONB  -- Additional context
);
```

### Indexes

Optimized for common query patterns:

- `audit_log_timestamp_idx`: Timeline queries
- `audit_log_resource_type_idx`: Filter by resource type
- `audit_log_resource_id_idx`: Resource history
- `audit_log_user_id_idx`: User activity
- `audit_log_composite_idx`: Combined resource queries
- `audit_log_user_composite_idx`: Combined user queries
- GIN indexes on JSONB columns for flexible queries

## Best Practices

### 1. Capture Before/After State

For updates, store both before and after state:

```rust
let changes = json!({
    "before": {
        "name": "Old Name",
        "status": "draft"
    },
    "after": {
        "name": "New Name",
        "status": "published"
    }
});
```

### 2. Include Request Context

Use metadata to store request-specific information:

```rust
let metadata = json!({
    "request_id": "req-abc123",
    "session_id": "sess-xyz789",
    "source": "web_ui",
    "correlation_id": "corr-123"
});
```

### 3. Anonymous Actions

For public APIs or unauthenticated actions:

```rust
let entry = CreateAuditEntry::builder()
    .action(AuditAction::Read)
    .resource_type(ResourceType::DataSource)
    .user_id(None)  // Anonymous
    .build();
```

### 4. Limit Query Results

Always set reasonable limits to prevent performance issues:

```rust
let query = AuditQuery {
    limit: 100,  // Cap at 100 results
    ..Default::default()
};
```

The system enforces a maximum limit of 1000 results per query.

## Performance Considerations

- **Non-blocking writes**: Middleware uses `tokio::spawn()` for async audit logging
- **Indexes**: Comprehensive indexes on common query patterns
- **Pagination**: Built-in offset/limit support
- **GIN indexes**: Fast JSONB queries
- **Selective logging**: Only commands are logged, not queries

## Security

- **IP address logging**: Tracks client IP for security analysis
- **User agent capture**: Identifies client applications
- **Immutable records**: Audit logs should never be modified
- **UUID users**: Supports anonymous actions with nullable user_id
- **Metadata flexibility**: Store any additional context needed

## Testing

Comprehensive tests cover:

- Basic audit entry creation
- Changes and metadata storage
- Anonymous entries
- Query filtering
- Pagination
- Time range queries
- All action types
- All resource types
- IP address formats (IPv4/IPv6)
- Limit capping

Run tests:

```bash
cargo test --package bdp-server --test audit_tests
```

## References

- [Axum Middleware Documentation](https://docs.rs/axum/latest/axum/middleware/index.html)
- [Tower Layer Documentation](https://docs.rs/tower/latest/tower/layer/index.html)
- [Building Modular Web Services with Axum Layers](https://leapcell.io/blog/building-modular-web-services-with-axum-layers-for-observability-and-security)
