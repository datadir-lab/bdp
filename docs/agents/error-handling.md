# Error Handling Policy for BDP

This document establishes the comprehensive error handling policy for the BDP Rust codebase. All contributors must follow these guidelines to ensure consistent, robust, and maintainable error handling across the project.

## Table of Contents

- [Principles](#principles)
- [Error Type Strategy](#error-type-strategy)
- [Patterns](#patterns)
- [Rules](#rules)
- [Examples](#examples)
- [Migration Guide](#migration-guide)
- [Quick Reference](#quick-reference)

---

## Principles

### 1. Fail Fast, Fail Explicitly

Errors should be detected and reported as early as possible. Never silently ignore errors or allow undefined behavior.

```rust
// ✅ Good - explicit error handling
let config = load_config().map_err(|e| {
    error!(error = ?e, "Failed to load configuration");
    e
})?;

// ❌ Bad - silently ignoring errors
let _ = load_config();  // Error completely ignored!
```

### 2. Recoverable vs Unrecoverable Errors

Use `Result<T, E>` for **recoverable errors** - situations where the caller can meaningfully handle the failure:
- File not found
- Network timeout
- Invalid user input
- Database connection failure

Use `panic!` **only** for **unrecoverable errors** - situations indicating programmer error or invariant violations:
- Index out of bounds on verified data
- Violated internal invariants
- Impossible states (should never happen if code is correct)

```rust
// ✅ Recoverable - use Result
pub async fn get_user(id: Uuid) -> Result<User, DbError> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(&pool)
        .await?
        .ok_or(DbError::NotFound(format!("User {} not found", id)))
}

// ✅ Unrecoverable - panic is acceptable for invariant violations
fn process_validated_batch(items: &[Item]) {
    // At this point, items has been validated to be non-empty
    let first = items.first().expect("Batch must not be empty - already validated");
}
```

### 3. Error Context is Critical

Always provide enough context for debugging. Include:
- What operation failed
- What resource was involved
- Relevant identifiers (IDs, names, paths)

```rust
// ✅ Good - rich context
async fn download_file(url: &str, path: &Path) -> Result<()> {
    tokio::fs::write(path, data)
        .await
        .with_context(|| format!(
            "Failed to write downloaded file from {} to {:?}",
            url, path
        ))?;
    Ok(())
}

// ❌ Bad - no context
async fn download_file(url: &str, path: &Path) -> Result<()> {
    tokio::fs::write(path, data).await?;  // What failed? Which file?
    Ok(())
}
```

### 4. User-Facing vs Internal Errors

Distinguish between errors shown to users and internal errors:
- **User-facing**: Clear, actionable message without implementation details
- **Internal/logs**: Full technical details for debugging

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            // Log full details internally
            AppError::Database(ref e) => {
                error!(error = ?e, "Database error occurred");
                // Return sanitized message to user
                (StatusCode::INTERNAL_SERVER_ERROR, "A database error occurred".to_string())
            }
            // User errors can be more descriptive
            AppError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Validation(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        };
        // ...
    }
}
```

---

## Error Type Strategy

### Use `thiserror` for Library/Domain Errors

For **specific, domain-level errors** where you need:
- Custom error types with variants
- Automatic `From` implementations
- Clean error messages

```rust
use thiserror::Error;

/// Errors specific to CLI operations
#[derive(Error, Debug)]
pub enum CliError {
    #[error("API error: {0}")]
    Api(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    // Automatic From implementation via #[from]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### Use `anyhow` for Application-Level Errors

For **application code** (binaries, examples, tests) where you need:
- Easy error propagation with `?`
- Rich context via `.context()` and `.with_context()`
- Ad-hoc error messages

```rust
use anyhow::{Context, Result, bail, anyhow};

async fn run_ingestion() -> Result<()> {
    let config = load_config()
        .context("Failed to load ingestion configuration")?;

    let pool = PgPoolOptions::new()
        .connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    if config.batch_size == 0 {
        bail!("Batch size must be greater than zero");
    }

    // For one-off errors
    let org_id = get_org_id()
        .ok_or_else(|| anyhow!("Organization ID not found"))?;

    Ok(())
}
```

### When to Use Which

| Scenario | Error Type | Rationale |
|----------|------------|-----------|
| Public library API | `thiserror` custom enum | Callers need to match on specific variants |
| Internal service errors | `thiserror` custom enum | Structured logging and HTTP responses |
| CLI commands | Custom `CliError` with `thiserror` | User-facing error messages |
| Binaries/examples | `anyhow::Result` | Convenience, rich context |
| Tests | `anyhow::Result` or unwrap | Panic on failure is acceptable |
| One-off internal errors | `anyhow::anyhow!()` | Quick, contextual errors |

---

## Patterns

### Pattern 1: Feature-Specific Error Types

Each feature module should define its own error type when it has distinct failure modes:

```rust
// crates/bdp-server/src/features/data_sources/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataSourceError {
    #[error("Data source not found: {org}/{name}@{version}")]
    NotFound {
        org: String,
        name: String,
        version: String,
    },

    #[error("Version conflict: {0} already exists")]
    VersionConflict(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub type DataSourceResult<T> = Result<T, DataSourceError>;
```

### Pattern 2: Error Conversion Chain

Set up proper error conversion for seamless propagation:

```rust
// Domain error
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Resource not found: {0}")]
    NotFound(String),
}

// Application error - converts from domain error
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    // Manual conversion for more complex cases
    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::Sqlx(e) => AppError::Database(e),
            DbError::NotFound(msg) => AppError::NotFound(msg),
        }
    }
}
```

### Pattern 3: Logging Errors at Boundaries

Log errors at system boundaries (HTTP handlers, job processors) not deep in the call stack:

```rust
// ✅ Good - log at handler level
async fn get_user_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    match get_user(&state.db, id).await {
        Ok(user) => Ok(Json(user)),
        Err(e) => {
            error!(
                error = ?e,
                user_id = %id,
                "Failed to fetch user"
            );
            Err(e.into())
        }
    }
}

// ❌ Avoid - logging deep in implementation
async fn get_user(db: &PgPool, id: Uuid) -> Result<User, DbError> {
    let user = sqlx::query_as!(/* ... */)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!("DB error: {:?}", e);  // Don't log here - let caller decide
            e
        })?;
    // ...
}
```

### Pattern 4: Contextual Error Wrapping

Use `.context()` to add information as errors propagate:

```rust
use anyhow::{Context, Result};

async fn process_uniprot_version(version: &str) -> Result<()> {
    let raw_data = download_dat_file(version)
        .await
        .with_context(|| format!("Failed to download UniProt version {}", version))?;

    let entries = parse_dat_entries(&raw_data)
        .with_context(|| format!("Failed to parse DAT file for version {}", version))?;

    store_entries(&entries)
        .await
        .with_context(|| format!(
            "Failed to store {} entries for version {}",
            entries.len(),
            version
        ))?;

    Ok(())
}
```

### Pattern 5: Builder Pattern with Required Fields

For builders, use `.expect()` only for truly required fields that indicate programmer error if missing:

```rust
pub struct AuditEntryBuilder {
    action: Option<AuditAction>,
    resource_type: Option<ResourceType>,
    // ...
}

impl AuditEntryBuilder {
    pub fn build(self) -> AuditEntry {
        AuditEntry {
            // These are programming errors if not set
            action: self.action.expect("action is required"),
            resource_type: self.resource_type.expect("resource_type is required"),
            // ...
        }
    }
}
```

---

## Rules

### RULE 1: Never Use `.unwrap()` in Production Code

`.unwrap()` panics on error with no context. **It is forbidden in production code paths.**

```rust
// ❌ FORBIDDEN in production
let user = get_user(id).await.unwrap();
let config = std::fs::read_to_string("config.toml").unwrap();

// ✅ Use ? with proper error types
let user = get_user(id).await?;
let config = std::fs::read_to_string("config.toml")?;

// ✅ Or handle explicitly
let user = get_user(id).await
    .map_err(|e| AppError::NotFound(format!("User {}: {}", id, e)))?;
```

### RULE 2: `.expect()` Only for Invariants with Descriptive Messages

`.expect()` is acceptable **only** when:
1. The value being unwrapped was **just validated/created**
2. A `None` or `Err` would indicate a **programmer bug**
3. The message clearly explains **why this should never fail**

```rust
// ✅ Acceptable - just inserted, must exist
let entry_id = entry_id_map.get(&taxonomy_id)
    .expect("Entry ID must exist in map - was just inserted in batch_upsert");

// ✅ Acceptable - startup configuration
let db_url = std::env::var("DATABASE_URL")
    .expect("DATABASE_URL must be set - required for server operation");

// ✅ Acceptable - signal handlers (once at startup)
ctrlc::set_handler(move || { /* ... */ })
    .expect("failed to install Ctrl+C handler");

// ❌ NOT acceptable - external data that could be invalid
let user_id = request.headers().get("X-User-Id")
    .expect("User ID header must exist");  // External input!

// ✅ Correct approach for external data
let user_id = request.headers().get("X-User-Id")
    .ok_or(AppError::BadRequest("Missing X-User-Id header".into()))?;
```

### RULE 3: `.unwrap()` is Acceptable in Tests

In test code, `.unwrap()` and `.expect()` are acceptable because test failures should panic:

```rust
#[tokio::test]
async fn test_create_organization() {
    let pool = setup_test_db().await;

    let org = create_organization(&pool, params).await.unwrap();
    assert_eq!(org.name, "Test Org");

    let fetched = get_organization_by_slug(&pool, "test-org").await.unwrap();
    assert_eq!(fetched.id, org.id);

    // Cleanup
    delete_organization(&pool, "test-org").await.unwrap();
}
```

### RULE 4: All Public Functions Must Return `Result` or Document Infallibility

Public functions that can fail **must** return `Result`. If a function cannot fail, document why:

```rust
// ✅ Good - returns Result
pub async fn fetch_user(db: &PgPool, id: Uuid) -> Result<User, DbError> {
    // ...
}

// ✅ Good - documents why it cannot fail
/// Formats a byte count as a human-readable string.
///
/// # Infallibility
/// This function cannot fail as it only performs string formatting
/// operations on valid numeric inputs.
pub fn format_bytes(bytes: u64) -> String {
    // Pure computation, no failure modes
    if bytes < 1024 {
        format!("{} B", bytes)
    } else {
        // ...
    }
}

// ❌ Bad - public function with hidden panic
pub fn get_first_item<T>(items: &[T]) -> &T {
    &items[0]  // Panics if empty!
}

// ✅ Good - explicit Option return
pub fn get_first_item<T>(items: &[T]) -> Option<&T> {
    items.first()
}
```

### RULE 5: Use `?` Operator for Propagation

Prefer the `?` operator over manual error matching for clean propagation:

```rust
// ✅ Good - clean propagation with ?
async fn process_request(id: Uuid) -> Result<Response> {
    let user = fetch_user(id).await?;
    let data = process_data(&user).await?;
    let result = store_result(data).await?;
    Ok(Response::new(result))
}

// ❌ Avoid - verbose manual matching
async fn process_request(id: Uuid) -> Result<Response> {
    let user = match fetch_user(id).await {
        Ok(u) => u,
        Err(e) => return Err(e.into()),
    };
    // ... repeated for each step
}
```

### RULE 6: Never Ignore `Result` Values

The `#[must_use]` attribute on `Result` exists for a reason. Never ignore results:

```rust
// ❌ FORBIDDEN - ignoring result
let _ = delete_file(path);
save_to_database(data);  // Compiler warning ignored

// ✅ Handle the result
delete_file(path)?;

// ✅ Or explicitly acknowledge you're ignoring it (rarely appropriate)
// Only if the operation is truly optional
if let Err(e) = cleanup_temp_file(path) {
    warn!(error = ?e, path = ?path, "Failed to cleanup temp file");
}
```

---

## Examples

### Good Patterns

#### 1. Comprehensive Error Type with Helpers

```rust
// crates/bdp-cli/src/error.rs
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("API error: {0}")]
    Api(String),

    #[error("Checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl CliError {
    /// Create an API error with a message
    pub fn api(msg: impl Into<String>) -> Self {
        Self::Api(msg.into())
    }

    /// Create a checksum mismatch error
    pub fn checksum_mismatch(
        file: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::ChecksumMismatch {
            file: file.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }
}
```

#### 2. HTTP Error Response Implementation

```rust
// crates/bdp-server/src/error.rs
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                // Log full error internally
                tracing::error!(error = ?e, "Database error");
                // Return sanitized message to user
                (StatusCode::INTERNAL_SERVER_ERROR, "A database error occurred".to_string())
            }
            AppError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Validation(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(ref msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "status": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}
```

#### 3. Proper Context Chain

```rust
use anyhow::{Context, Result};

async fn ingest_uniprot_release(release: &str) -> Result<IngestStats> {
    let config = UniProtConfig::load()
        .context("Failed to load UniProt configuration")?;

    let pool = create_db_pool(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    let org_id = ensure_organization(&pool, "uniprot")
        .await
        .context("Failed to ensure UniProt organization exists")?;

    let versions = discover_versions(&config)
        .await
        .with_context(|| format!("Failed to discover versions for release {}", release))?;

    let mut total_stats = IngestStats::default();

    for version in versions {
        let stats = process_version(&pool, &version, org_id)
            .await
            .with_context(|| format!("Failed to process version {}", version.id))?;

        total_stats.merge(stats);
    }

    Ok(total_stats)
}
```

### Bad Patterns to Avoid

#### 1. Silent Error Swallowing

```rust
// ❌ BAD - Error completely lost
fn save_config(config: &Config) {
    let _ = std::fs::write("config.json", serde_json::to_string(config).unwrap());
}

// ✅ GOOD - Propagate or handle
fn save_config(config: &Config) -> Result<()> {
    let json = serde_json::to_string(config)?;
    std::fs::write("config.json", json)?;
    Ok(())
}
```

#### 2. Panic in Library Code

```rust
// ❌ BAD - Library function panics on invalid input
pub fn parse_version(s: &str) -> Version {
    let parts: Vec<_> = s.split('.').collect();
    Version {
        major: parts[0].parse().unwrap(),
        minor: parts[1].parse().unwrap(),
        patch: parts[2].parse().unwrap(),
    }
}

// ✅ GOOD - Returns Result for invalid input
pub fn parse_version(s: &str) -> Result<Version, ParseError> {
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(ParseError::InvalidFormat(s.to_string()));
    }
    Ok(Version {
        major: parts[0].parse().map_err(|_| ParseError::InvalidNumber(parts[0].into()))?,
        minor: parts[1].parse().map_err(|_| ParseError::InvalidNumber(parts[1].into()))?,
        patch: parts[2].parse().map_err(|_| ParseError::InvalidNumber(parts[2].into()))?,
    })
}
```

#### 3. Generic Error Messages

```rust
// ❌ BAD - Unhelpful error message
Err(AppError::Internal("Something went wrong".to_string()))

// ✅ GOOD - Specific, actionable message
Err(AppError::NotFound(format!(
    "Data source '{}/{}@{}' not found",
    org, name, version
)))
```

#### 4. Unwrap on External Input

```rust
// ❌ BAD - External data can be anything
fn handle_request(body: &str) -> Response {
    let data: RequestData = serde_json::from_str(body).unwrap();  // PANIC!
    // ...
}

// ✅ GOOD - Validate external input
fn handle_request(body: &str) -> Result<Response, AppError> {
    let data: RequestData = serde_json::from_str(body)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;
    // ...
}
```

---

## Migration Guide

### Fixing Existing `.unwrap()` Calls

#### Step 1: Identify the Context

Determine if the unwrap is:
- In test code → **Keep it** (tests should panic on failure)
- After just-validated data → **Convert to `.expect()` with message**
- On external/uncertain data → **Convert to `?` with proper error type**

#### Step 2: Choose the Right Fix

```rust
// Before
let user = users.get(&id).unwrap();

// After - Option 1: If id was just validated to exist
let user = users.get(&id)
    .expect("User ID was validated to exist in the map");

// After - Option 2: If id might not exist (most cases)
let user = users.get(&id)
    .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

// After - Option 3: If in application code with anyhow
let user = users.get(&id)
    .ok_or_else(|| anyhow!("User {} not found", id))?;
```

#### Step 3: Add Context Where Needed

```rust
// Before
let data = std::fs::read(path).unwrap();

// After
let data = std::fs::read(&path)
    .with_context(|| format!("Failed to read file: {:?}", path))?;
```

### Common Migration Patterns

| Before | After |
|--------|-------|
| `.unwrap()` | `.expect("reason")` or `?` |
| `.unwrap_or(default)` | Keep as-is (this is fine) |
| `.unwrap_or_else(\|\| ...)` | Keep as-is (this is fine) |
| `match x { Ok(v) => v, Err(_) => panic!() }` | `x?` |
| `if let Some(x) = opt { x } else { panic!() }` | `opt.ok_or(Error)?` |

---

## Quick Reference

### Error Type Decision Tree

```
Is this library/public API code?
├─ YES → Use thiserror custom enum
│        Return Result<T, YourError>
│
└─ NO → Is this application/binary code?
         ├─ YES → Use anyhow::Result<T>
         │        Add .context() for clarity
         │
         └─ NO → Is this test code?
                  ├─ YES → .unwrap() is acceptable
                  │
                  └─ NO → Evaluate case-by-case
```

### When `.expect()` is OK

1. **Startup configuration** - Server won't run without it anyway
2. **Just-validated data** - You literally just checked it
3. **Infallible operations** - e.g., `Mutex::lock()` (poisoning is rare)
4. **Static data** - Regex compilation, constant parsing

### Checklist Before PR

- [ ] No `.unwrap()` in production code paths
- [ ] All `.expect()` have descriptive messages
- [ ] Errors include enough context for debugging
- [ ] User-facing errors are clear and actionable
- [ ] Internal errors are logged at boundaries
- [ ] Public functions return `Result` or document infallibility
- [ ] No ignored `Result` values (check for `let _ =`)

---

## Related Documentation

- [Best Practices](./best-practices.md) - General coding standards
- [Logging Best Practices](./logging.md) - How to log errors properly
- [Backend Architecture](./backend-architecture.md) - CQRS patterns including error handling
- [Rust Backend](./rust-backend.md) - axum error handling patterns

---

**Note**: This policy applies to all Rust code in the BDP project. When in doubt, prefer explicit error handling over convenience. A clear error message during debugging is worth more than a few saved keystrokes during development.
