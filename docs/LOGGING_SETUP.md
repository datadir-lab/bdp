# Logging System Setup Summary

This document summarizes the structured logging infrastructure that has been set up for the BDP project.

## What Was Implemented

### 1. Centralized Logging Module
**Location:** `crates/bdp-common/src/logging.rs`

A comprehensive logging module similar to Serilog in .NET, providing:
- Configuration-driven logging setup
- Multiple output targets (console, file, both)
- Multiple formats (text for development, JSON for production)
- Automatic daily log file rotation
- Environment-based configuration

### 2. Dependencies Added
**Added to workspace** (`Cargo.toml`):
```toml
tracing-appender = "0.2"
```

All three main crates now use the centralized logging from `bdp-common`.

### 3. Updated Entry Points

All three main applications now use the centralized logging:

- **`bdp-server`** - Logs to both console and file by default
- **`bdp-cli`** - Logs to console (verbose with `--verbose` flag)
- **`bdp-ingest`** - Logs to both console and file with progress tracking

### 4. Environment Configuration
**Updated:** `.env.example`

Added comprehensive logging configuration with:
- `LOG_LEVEL` - Control log verbosity
- `LOG_OUTPUT` - Choose output target
- `LOG_FORMAT` - Text or JSON format
- `LOG_DIR` - Custom log directory
- `LOG_FILE_PREFIX` - Component-specific log files
- `LOG_FILTER` - Module-specific filtering
- `LOG_INCLUDE_*` - Control metadata inclusion

### 5. Documentation
**Created:** `docs/agents/logging.md`

Complete logging best practices guide covering:
- Why we use structured logging
- How to use logging macros properly
- Migration from `println!` to `tracing`
- Production recommendations
- Security considerations

**Updated:** `AGENTS.md`

Added logging as a mandatory reference in the quick reference section.

## How to Use

### Development Setup

1. **Copy environment template:**
   ```bash
   cp .env.example .env
   ```

2. **Configure for development:**
   ```bash
   # In .env
   LOG_LEVEL=debug
   LOG_OUTPUT=console
   LOG_FORMAT=text
   LOG_INCLUDE_LOCATION=true
   ```

3. **Run any component:**
   ```bash
   cargo run --bin bdp-server
   cargo run --bin bdp -- init --verbose
   cargo run --bin bdp-ingest -- uniprot -v
   ```

### Production Setup

1. **Configure for production:**
   ```bash
   # In .env or systemd service file
   LOG_LEVEL=info
   LOG_OUTPUT=both
   LOG_FORMAT=json
   LOG_DIR=/var/log/bdp
   LOG_FILTER=sqlx=warn,tower_http=info
   ```

2. **Ensure log directory exists:**
   ```bash
   sudo mkdir -p /var/log/bdp
   sudo chown bdp:bdp /var/log/bdp
   ```

3. **Set up log rotation (optional, already built-in):**
   The system automatically rotates logs daily. For cleanup:
   ```bash
   # Add to crontab
   0 2 * * * find /var/log/bdp -name "*.log" -mtime +30 -delete
   ```

## Code Examples

### ❌ OLD WAY (DO NOT USE)
```rust
println!("Server started on port {}", port);
eprintln!("Error: {}", error);
dbg!(user_data);
```

### ✅ NEW WAY (ALWAYS USE)
```rust
use tracing::{info, error, debug};

info!(port = %port, "Server started");
error!(error = ?err, "Operation failed");
debug!(user_id = %user.id, "User data loaded");
```

## Log File Locations

### Development
```
./logs/
├── bdp-server.2024-01-18.log
├── bdp-cli.2024-01-18.log
└── bdp-ingest.2024-01-18.log
```

### Production
```
/var/log/bdp/
├── bdp-server.2024-01-18.log
├── bdp-server.2024-01-17.log
├── bdp-ingest.2024-01-18.log
└── bdp-ingest.2024-01-17.log
```

## Key Features

### 1. Structured Logging
```rust
// Fields are structured, not just string interpolation
info!(
    user_id = %user.id,
    username = %user.name,
    action = "login",
    "User logged in successfully"
);
```

### 2. Module-Level Filtering
```bash
# Reduce noise from SQLx, increase detail for your feature
LOG_FILTER=sqlx=error,tower_http=warn,bdp_server::features::my_feature=debug
```

### 3. Automatic File Rotation
Files are automatically rotated daily with format: `{prefix}.{YYYY-MM-DD}.log`

### 4. JSON for Production
Production logs use JSON format for easy integration with log aggregation tools:
```json
{"timestamp":"2024-01-18T10:30:45Z","level":"INFO","target":"bdp_server","fields":{"user_id":"123","action":"login"},"message":"User logged in"}
```

### 5. Spans for Operations
```rust
use tracing::instrument;

#[instrument(skip(db))]
async fn process_order(order_id: &str, db: &PgPool) -> Result<()> {
    info!("Processing order");
    // All logs within this function will include order_id context
    Ok(())
}
```

## Configuration Quick Reference

| Environment | LOG_LEVEL | LOG_OUTPUT | LOG_FORMAT | LOG_DIR |
|-------------|-----------|------------|------------|---------|
| **Development** | `debug` | `console` | `text` | `./logs` |
| **Staging** | `info` | `both` | `json` | `/var/log/bdp` |
| **Production** | `info` | `both` | `json` | `/var/log/bdp` |
| **Debugging** | `trace` | `both` | `text` | `./logs` |

## Migration Guide

To migrate existing code:

1. **Find console logging:**
   ```bash
   rg "println!|eprintln!|dbg!" --type rust
   ```

2. **Replace with structured logging:**
   - Import: `use tracing::{info, warn, error};`
   - Replace `println!` → `info!`
   - Replace `eprintln!` → `error!` or `warn!`
   - Replace `dbg!` → `debug!`
   - Add structured fields

3. **Example migration:**
   ```rust
   // Before
   println!("Processing file: {}", filename);

   // After
   info!(file = %filename, "Processing file");
   ```

## Testing

Logging is automatically disabled in tests unless explicitly initialized:

```rust
#[test]
fn test_with_logging() {
    use tracing_subscriber::fmt::Subscriber;

    let subscriber = Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        // Test code here - logs will be visible
        info!("This will be logged during the test");
    });
}
```

## Next Steps

1. **Review the full documentation:** `docs/agents/logging.md`
2. **Update existing code:** Replace all `println!`, `eprintln!`, `dbg!` with structured logging
3. **Test in development:** Run with `LOG_LEVEL=debug LOG_OUTPUT=console`
4. **Configure for production:** Use `LOG_OUTPUT=both LOG_FORMAT=json`

## Resources

- **Full documentation:** [docs/agents/logging.md](./agents/logging.md)
- **Configuration reference:** [.env.example](../.env.example)
- **Code examples:** [crates/bdp-common/src/logging.rs](../crates/bdp-common/src/logging.rs)
- **tracing docs:** https://docs.rs/tracing/

---

**Important:** This logging system is now mandatory for all BDP components. Never use `println!`, `eprintln!`, or `dbg!` in production code.
