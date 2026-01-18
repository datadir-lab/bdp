# Logging Best Practices

This document describes the logging infrastructure and best practices for the BDP project.

## Overview

BDP uses a structured logging system based on the `tracing` ecosystem, similar to Serilog in .NET. The system provides:

- **Structured logging** with fields and spans
- **Multiple output targets** (console, file, both)
- **Multiple formats** (text, JSON)
- **Configurable log levels**
- **Automatic daily log rotation**
- **Environment-based configuration**

## Architecture

The logging infrastructure is centralized in `bdp-common/src/logging.rs` and used by all components:

- `bdp-server` - Backend API server
- `bdp-cli` - Command-line interface
- `bdp-ingest` - Data ingestion tool

## Configuration

### Environment Variables

All logging configuration can be controlled via environment variables:

| Variable | Description | Values | Default |
|----------|-------------|---------|---------|
| `LOG_LEVEL` | Minimum log level | `trace`, `debug`, `info`, `warn`, `error` | `info` |
| `LOG_OUTPUT` | Output target | `console`, `file`, `both` | `console` |
| `LOG_FORMAT` | Log format | `text`, `json` | `text` |
| `LOG_DIR` | Log file directory | Any path | `./logs` |
| `LOG_FILE_PREFIX` | Log file name prefix | Any string | `bdp` (or component-specific) |
| `LOG_FILTER` | Filter directives | Comma-separated filters | None |
| `LOG_INCLUDE_LOCATION` | Include file:line | `true`, `false` | `false` |
| `LOG_INCLUDE_THREAD_IDS` | Include thread IDs | `true`, `false` | `false` |
| `LOG_INCLUDE_TARGETS` | Include module names | `true`, `false` | `true` |

### Example Configuration

**.env for development:**
```bash
# Development logging - verbose console output
LOG_LEVEL=debug
LOG_OUTPUT=console
LOG_FORMAT=text
```

**.env for production:**
```bash
# Production logging - JSON to files with daily rotation
LOG_LEVEL=info
LOG_OUTPUT=both
LOG_FORMAT=json
LOG_DIR=/var/log/bdp
LOG_FILTER=sqlx=warn,tower_http=info
```

**.env for debugging:**
```bash
# Debugging - trace everything to both console and file
LOG_LEVEL=trace
LOG_OUTPUT=both
LOG_FORMAT=text
LOG_INCLUDE_LOCATION=true
LOG_INCLUDE_THREAD_IDS=true
```

## Best Practices

### ❌ NEVER Use These

**NEVER** use the following in production code:

```rust
// ❌ DON'T DO THIS
println!("User logged in: {}", user.name);
eprintln!("Error: {}", error);
dbg!(some_value);
console.log("message");  // Not Rust, but don't do this either
```

### ✅ ALWAYS Use Structured Logging

**ALWAYS** use the `tracing` macros:

```rust
use tracing::{trace, debug, info, warn, error};

// ✅ DO THIS
info!(user_id = %user.id, username = %user.name, "User logged in");
error!(error = ?err, path = %file_path, "Failed to read file");
```

### Log Levels

Use appropriate log levels for different situations:

#### `trace!` - Very Detailed Debugging
```rust
trace!(query = %sql, params = ?params, "Executing database query");
trace!(bytes = buffer.len(), "Read data from socket");
```

**When to use:**
- Very detailed information for debugging specific issues
- Function entry/exit points during debugging
- Variable values during complex calculations

#### `debug!` - Development Information
```rust
debug!(config = ?config, "Loaded configuration");
debug!(cache_hit = true, key = %cache_key, "Cache lookup");
```

**When to use:**
- Information useful during development
- Configuration details
- Cache hits/misses
- Internal state changes

#### `info!` - General Information
```rust
info!(addr = %addr, "Server listening");
info!(user_id = %user_id, "User logged in");
info!(count = records.len(), "Processed records");
```

**When to use:**
- Application startup/shutdown
- Important business events
- Successful operations
- Progress indicators

#### `warn!` - Warnings
```rust
warn!(retry_count = attempt, max_retries = MAX_RETRIES, "Retrying failed operation");
warn!(remaining_space = disk_space, threshold = THRESHOLD, "Low disk space");
```

**When to use:**
- Recoverable errors
- Deprecated functionality
- Resource constraints
- Unexpected but handled situations

#### `error!` - Errors
```rust
error!(error = ?err, user_id = %user_id, "Failed to process payment");
error!(error = %e, path = %file_path, "Failed to read configuration file");
```

**When to use:**
- Operation failures
- Unhandled errors
- Data corruption
- Security violations

### Structured Fields

Always use structured fields for important data:

```rust
// ✅ Good - structured fields
info!(
    user_id = %user.id,
    username = %user.name,
    role = %user.role,
    "User logged in"
);

// ❌ Bad - string interpolation
info!("User {} (id: {}, role: {}) logged in", user.name, user.id, user.role);
```

**Field formatting:**
- `%` - Display format (implements `Display`)
- `?` - Debug format (implements `Debug`)
- No prefix - efficient format (implements `Value`)

### Using Spans

Use spans for operations that have a duration:

```rust
use tracing::{info_span, instrument};

// Manual span
async fn process_order(order_id: &str) -> Result<()> {
    let span = info_span!("process_order", order_id = %order_id);
    let _enter = span.enter();

    info!("Processing order");
    // ... operation logic

    Ok(())
}

// Automatic span with #[instrument]
#[instrument(skip(db))]
async fn save_user(user: &User, db: &PgPool) -> Result<()> {
    info!("Saving user to database");
    // ... database logic
    Ok(())
}
```

**When to use spans:**
- HTTP request handling
- Database transactions
- File operations
- Complex business operations

### Error Logging

Always log context when errors occur:

```rust
use tracing::error;

// ✅ Good - context included
match read_config(&path).await {
    Ok(config) => config,
    Err(e) => {
        error!(
            error = ?e,
            path = %path,
            "Failed to read configuration"
        );
        return Err(e);
    }
}

// ✅ Even better - use Result with context
read_config(&path)
    .await
    .map_err(|e| {
        error!(error = ?e, path = %path, "Failed to read configuration");
        e
    })?
```

### Module-Level Filtering

Use the `LOG_FILTER` environment variable to control specific modules:

```bash
# Reduce noise from SQLx and tower_http
LOG_FILTER=sqlx=warn,tower_http=info

# Only show errors from most modules, but debug from your feature
LOG_FILTER=sqlx=error,tower_http=error,bdp_server::features::my_feature=debug
```

## Component-Specific Guidelines

### Server (bdp-server)

**Default configuration:**
- Level: `info`
- Output: `both` (console + file)
- Format: `json` in production, `text` in development
- File: `./logs/bdp-server.YYYY-MM-DD.log`

**What to log:**
```rust
// Startup/shutdown
info!(addr = %addr, "Server listening");
info!("Server shut down gracefully");

// Request handling (via middleware)
// Automatically logged by tower_http tracing layer

// Business operations
info!(organization_id = %org_id, "Created organization");
warn!(user_id = %user_id, retry = attempt, "Payment retry");
error!(error = ?err, order_id = %order_id, "Failed to process order");
```

### CLI (bdp-cli)

**Default configuration:**
- Level: `warn` (normal), `debug` (--verbose)
- Output: `console`
- Format: `text`

**What to log:**
```rust
// Only important events
info!(project = %name, "Initialized project");
info!(source = %source_id, "Added data source");

// Errors (also shown via eprintln! for user feedback)
error!(error = ?err, command = "pull", "Command failed");
```

### Ingest (bdp-ingest)

**Default configuration:**
- Level: `info` (normal), `debug` (--verbose)
- Output: `both`
- Format: `text`
- File: `./logs/bdp-ingest.YYYY-MM-DD.log`

**What to log:**
```rust
// Progress
info!(source = "uniprot", release = %version, "Starting ingestion");
info!(files = count, "Downloaded files");

// Warnings
warn!(file = %filename, "Skipping malformed record");

// Errors
error!(error = ?err, url = %url, "Failed to download file");
```

## Log File Management

### File Rotation

Log files are automatically rotated **daily**:
- Format: `{prefix}.{YYYY-MM-DD}.log`
- Example: `bdp-server.2024-01-18.log`
- Old files are kept indefinitely (manual cleanup required)

### Log Directory Structure

Recommended production structure:
```
/var/log/bdp/
├── bdp-server.2024-01-18.log
├── bdp-server.2024-01-17.log
├── bdp-ingest.2024-01-18.log
└── bdp-ingest.2024-01-17.log
```

Development structure:
```
./logs/
├── bdp-server.2024-01-18.log
├── bdp-cli.2024-01-18.log
└── bdp-ingest.2024-01-18.log
```

### Cleanup Strategy

**Manual cleanup:**
```bash
# Delete logs older than 30 days
find /var/log/bdp -name "*.log" -mtime +30 -delete
```

**Automated cleanup (cron):**
```bash
# Add to crontab: daily cleanup at 2 AM
0 2 * * * find /var/log/bdp -name "*.log" -mtime +30 -delete
```

## Production Recommendations

### System Service Configuration

When running as a systemd service:

```ini
[Service]
Environment="LOG_LEVEL=info"
Environment="LOG_OUTPUT=both"
Environment="LOG_FORMAT=json"
Environment="LOG_DIR=/var/log/bdp"
Environment="LOG_FILTER=sqlx=warn"
```

### Monitoring Integration

For log aggregation systems (ELK, Splunk, CloudWatch):

1. **Use JSON format:**
   ```bash
   LOG_FORMAT=json
   ```

2. **Configure appropriate fields:**
   ```bash
   LOG_INCLUDE_TARGETS=true
   LOG_INCLUDE_THREAD_IDS=true  # For async debugging
   ```

3. **Point log aggregator to log directory:**
   ```bash
   LOG_DIR=/var/log/bdp
   ```

### Security Considerations

**NEVER log sensitive data:**

```rust
// ❌ DON'T DO THIS
error!(password = %user.password, "Login failed");
info!(credit_card = %card_number, "Payment processed");

// ✅ DO THIS
error!(user_id = %user.id, "Login failed");
info!(user_id = %user.id, last_4_digits = %card.last_4(), "Payment processed");
```

**Sanitize user input before logging:**

```rust
// Sanitize potentially malicious input
let sanitized = input.replace('\n', "\\n").replace('\r', "\\r");
info!(input = %sanitized, "Processing user input");
```

## Testing

### Disable Logging in Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Logging automatically disabled in tests
        // unless explicitly initialized
    }
}
```

### Enable Logging for Specific Tests

```rust
use tracing_subscriber::fmt::Subscriber;

#[test]
fn test_with_logging() {
    let subscriber = Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        // Test code here - logs will be visible
    });
}
```

## Migration from Console Logging

If you find code using `println!`, `eprintln!`, or `dbg!`, migrate it:

### Before:
```rust
println!("Processing file: {}", filename);
eprintln!("Error reading file: {}", error);
dbg!(some_value);
```

### After:
```rust
use tracing::{info, error, debug};

info!(file = %filename, "Processing file");
error!(error = ?error, file = %filename, "Failed to read file");
debug!(value = ?some_value, "Debug value");
```

## Quick Reference

### Import Statements
```rust
use tracing::{trace, debug, info, warn, error, instrument};
use tracing::{info_span, Span};
```

### Basic Logging
```rust
info!("Simple message");
info!(key = %value, "Message with field");
info!(key1 = %val1, key2 = ?val2, "Multiple fields");
```

### Spans
```rust
// Manual span
let span = info_span!("operation_name", field = %value);
let _guard = span.enter();

// Automatic span
#[instrument]
fn my_function() { }

#[instrument(skip(complex_param))]
fn with_skip(simple: u32, complex_param: &Complex) { }
```

### Error Logging
```rust
error!(error = ?err, context = %ctx, "Operation failed");
```

## Resources

- [tracing documentation](https://docs.rs/tracing/)
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber/)
- [tracing-appender documentation](https://docs.rs/tracing-appender/)
- [Structured logging best practices](https://www.honeycomb.io/blog/structured-logging-and-observability)
