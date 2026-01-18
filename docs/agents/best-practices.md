# Best Practices

This document outlines coding standards and best practices for the BDP project.

## 🚨 CRITICAL: Logging Rules (MANDATORY)

### **NEVER Use Console Logging**

**The following are FORBIDDEN in all code:**

```rust
// ❌ FORBIDDEN - NEVER USE THESE
println!("...");
eprintln!("...");
dbg!(...);
print!("...");
eprint!("...");
```

**Violations will be rejected in code review.**

### **ALWAYS Use Structured Logging**

```rust
// ✅ REQUIRED - ALWAYS USE THESE
use tracing::{trace, debug, info, warn, error};

info!("User logged in");
error!(error = ?err, "Operation failed");
warn!(count = n, "Low disk space");
debug!(value = ?data, "Debug info");
trace!("Entering function");
```

### Why This Matters

1. **Production Debugging**: Structured logs can be filtered, searched, and analyzed
2. **Performance Monitoring**: Log levels can be changed without code changes
3. **Security Auditing**: All events are tracked with context
4. **File Rotation**: Logs are automatically rotated and managed
5. **JSON Format**: Production logs integrate with monitoring tools

### Full Documentation

See [Logging Best Practices](./logging.md) for complete guide.

---

## Code Organization

### File Structure

- **Backend**: Use vertical slices in `features/feature_name/`
- **Shared Code**: Place in `bdp-common` crate
- **Tests**: Co-locate with code in `mod tests`

### Module Organization

```rust
// Good - Clear module structure
mod commands;
mod queries;
mod models;

pub use commands::*;
pub use queries::*;
pub use models::*;
```

---

## Error Handling

### Use Result Types

```rust
use anyhow::Result;

fn process_data() -> Result<()> {
    let data = read_file()?;
    validate_data(&data)?;
    Ok(())
}
```

### Log Errors with Context

```rust
use tracing::error;

match process_file(&path).await {
    Ok(result) => result,
    Err(e) => {
        error!(
            error = ?e,
            path = %path,
            "Failed to process file"
        );
        return Err(e);
    }
}
```

### Use `thiserror` for Custom Errors

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Not found: {0}")]
    NotFound(String),
}
```

---

## Async Patterns

### Use `#[instrument]` for Async Functions

```rust
use tracing::instrument;

#[instrument(skip(db))]
async fn fetch_user(user_id: &str, db: &PgPool) -> Result<User> {
    info!("Fetching user");
    // All logs will include user_id context
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!(error = ?e, "Database query failed");
            e.into()
        })
}
```

### Proper Error Propagation

```rust
// ✅ Good - propagate with context
async fn process() -> Result<()> {
    let data = fetch_data().await?;
    validate(data).await?;
    Ok(())
}

// ❌ Bad - swallowing errors
async fn process() {
    let _ = fetch_data().await;  // Error ignored!
}
```

---

## Database Patterns

### Always Use Transactions for Writes

```rust
let mut tx = db.begin().await?;

sqlx::query!("INSERT INTO users ...")
    .execute(&mut *tx)
    .await?;

sqlx::query!("INSERT INTO audit_logs ...")
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
```

### Use SQLx Compile-Time Checks

```rust
// ✅ Good - compile-time verified
let users = sqlx::query_as!(
    User,
    "SELECT id, name FROM users WHERE active = $1",
    true
)
.fetch_all(db)
.await?;

// ❌ Bad - runtime verification only
let users = sqlx::query("SELECT * FROM users")
    .fetch_all(db)
    .await?;
```

---

## Testing

### Write Tests for All Features

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_user() {
        let result = create_user("test@example.com").await;
        assert!(result.is_ok());
    }
}
```

### Use Meaningful Test Names

```rust
// ✅ Good - descriptive
#[test]
fn test_email_validation_rejects_invalid_format() { }

// ❌ Bad - unclear
#[test]
fn test1() { }
```

---

## Security

### Never Log Sensitive Data

```rust
// ❌ BAD - Exposes password
error!(password = %user.password, "Login failed");

// ✅ GOOD - No sensitive data
error!(user_id = %user.id, "Login failed");
```

### Sanitize User Input

```rust
use regex::Regex;

fn sanitize_input(input: &str) -> String {
    // Remove potentially harmful characters
    input.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect()
}
```

### Use Environment Variables for Secrets

```rust
// ✅ Good
let api_key = std::env::var("API_KEY")
    .expect("API_KEY must be set");

// ❌ Bad - hardcoded secret
let api_key = "sk-1234567890";
```

---

## Performance

### Use Async Where Appropriate

```rust
// ✅ Good - parallel execution
let (users, posts) = tokio::join!(
    fetch_users(),
    fetch_posts()
);

// ❌ Bad - sequential execution
let users = fetch_users().await;
let posts = fetch_posts().await;
```

### Avoid Unnecessary Clones

```rust
// ✅ Good - borrow
fn process_data(data: &str) -> Result<()> {
    // Process without cloning
}

// ❌ Bad - unnecessary clone
fn process_data(data: String) -> Result<()> {
    // Forces caller to clone
}
```

---

## Documentation

### Document Public APIs

```rust
/// Fetches user information from the database.
///
/// # Arguments
///
/// * `user_id` - The unique identifier for the user
/// * `db` - Database connection pool
///
/// # Returns
///
/// Returns the user if found, or an error if not found or database error occurs.
///
/// # Example
///
/// ```no_run
/// let user = fetch_user("123", &pool).await?;
/// ```
pub async fn fetch_user(user_id: &str, db: &PgPool) -> Result<User> {
    // Implementation
}
```

### Use Inline Comments for Complex Logic

```rust
// Calculate the weighted average based on user activity
let weighted_score = posts.iter()
    .map(|p| p.score * p.engagement_weight)
    .sum::<f64>() / total_weight;
```

---

## Code Style

### Follow Rust Conventions

- Use `snake_case` for functions and variables
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Maximum line length: 100 characters

### Format Code

```bash
cargo fmt
```

### Run Clippy

```bash
cargo clippy -- -D warnings
```

---

## Git Practices

### Write Meaningful Commit Messages

```
✅ Good:
feat: add user authentication endpoints
fix: resolve race condition in job scheduler
docs: update logging best practices

❌ Bad:
fixed stuff
update
wip
```

### Keep Commits Focused

- One logical change per commit
- Don't mix refactoring with new features
- Test before committing

---

## Review Checklist

Before submitting code, verify:

- [ ] **NO** `println!`, `eprintln!`, or `dbg!` macros
- [ ] All logging uses `tracing` macros
- [ ] Structured logging fields are used
- [ ] Errors are logged with context
- [ ] Tests are written and passing
- [ ] No sensitive data in logs
- [ ] SQLx queries are compile-time checked
- [ ] Async functions use `#[instrument]` where appropriate
- [ ] Code is formatted (`cargo fmt`)
- [ ] Clippy passes (`cargo clippy`)
- [ ] Documentation is updated

---

## Common Mistakes to Avoid

### 1. Using Console Logging
```rust
// ❌ NEVER DO THIS
println!("User: {}", user.name);

// ✅ ALWAYS DO THIS
info!(user_id = %user.id, username = %user.name, "User logged in");
```

### 2. Ignoring Errors
```rust
// ❌ Bad
let _ = some_operation();

// ✅ Good
some_operation().map_err(|e| {
    error!(error = ?e, "Operation failed");
    e
})?;
```

### 3. Missing Transactions
```rust
// ❌ Bad - no transaction
sqlx::query!("INSERT INTO users ...").execute(db).await?;
sqlx::query!("INSERT INTO audit ...").execute(db).await?;

// ✅ Good - atomic transaction
let mut tx = db.begin().await?;
sqlx::query!("INSERT INTO users ...").execute(&mut *tx).await?;
sqlx::query!("INSERT INTO audit ...").execute(&mut *tx).await?;
tx.commit().await?;
```

### 4. Logging Sensitive Data
```rust
// ❌ Bad
error!(password = %pwd, "Auth failed");

// ✅ Good
error!(user_id = %id, "Auth failed");
```

---

## Resources

- [Logging Best Practices](./logging.md)
- [Backend Architecture](./backend-architecture.md)
- [SQLx Guide](./implementation/sqlx-guide.md)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Book](https://doc.rust-lang.org/book/)
