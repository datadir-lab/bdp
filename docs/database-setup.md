# Database Setup Guide

This guide walks through setting up the PostgreSQL database for BDP development.

## Prerequisites

- PostgreSQL 14 or later
- Rust and Cargo installed
- SQLx CLI installed

## Installing SQLx CLI

The SQLx CLI is required for running migrations and managing the database.

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

## Quick Start

### 1. Create the Database

```bash
# Create development database
createdb bdp

# Create test database (for running tests)
createdb bdp_test
```

### 2. Configure Environment

Copy the example environment file and update it:

```bash
cp .env.example .env
```

Edit `.env` and set your database credentials:

```bash
DATABASE_URL=postgresql://postgres:your_password@localhost:5432/bdp
TEST_DATABASE_URL=postgresql://postgres:your_password@localhost:5432/bdp_test
```

### 3. Run Migrations

```bash
cd crates/bdp-server
sqlx migrate run
```

This will create all the necessary tables and indexes.

### 4. Verify Setup

Run the example program to verify everything is working:

```bash
cargo run --example database_usage
```

## Database Schema

### Organizations Table

```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    slug VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organizations_created_at ON organizations(created_at DESC);
```

### Registry Entries Table

```sql
CREATE TABLE registry_entries (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    slug VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, slug)
);

CREATE INDEX idx_registry_entries_org_id ON registry_entries(organization_id);
CREATE INDEX idx_registry_entries_slug ON registry_entries(slug);
CREATE INDEX idx_registry_entries_public ON registry_entries(is_public) WHERE is_public = true;
```

### Dataset Versions Table

```sql
CREATE TABLE dataset_versions (
    id UUID PRIMARY KEY,
    registry_entry_id UUID NOT NULL REFERENCES registry_entries(id) ON DELETE CASCADE,
    version_number VARCHAR(50) NOT NULL,
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(registry_entry_id, version_number)
);

CREATE INDEX idx_dataset_versions_entry_id ON dataset_versions(registry_entry_id);
CREATE INDEX idx_dataset_versions_number ON dataset_versions(version_number);
CREATE INDEX idx_dataset_versions_metadata ON dataset_versions USING GIN(metadata);
```

## Development Workflow

### Creating Migrations

```bash
# Create a new migration
sqlx migrate add create_users_table

# This creates a new file in migrations/ directory
# Edit the file to add your SQL
```

Example migration file (`migrations/20240101000000_create_users_table.sql`):

```sql
-- Create users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

### Running Migrations

```bash
# Run all pending migrations
sqlx migrate run

# Revert the last migration
sqlx migrate revert
```

### Checking Migration Status

```bash
# List migration status
sqlx migrate info
```

## SQLx Offline Mode

For CI/CD environments without a database, SQLx supports offline mode using prepared query metadata.

### Preparing Queries

With database access, generate query metadata:

```bash
export DATABASE_URL=postgresql://postgres:password@localhost/bdp
cargo sqlx prepare
```

This creates `.sqlx/*.json` files with query metadata.

### Building Offline

In CI/CD or without database access:

```bash
export SQLX_OFFLINE=true
cargo build
```

## Testing

### Running Database Tests

The test suite includes integration tests that require a database:

```bash
# Set test database URL
export TEST_DATABASE_URL=postgresql://postgres:password@localhost/bdp_test

# Run migrations on test database
sqlx migrate run --database-url $TEST_DATABASE_URL

# Run tests
cargo test

# Run specific test
cargo test test_create_organization
```

### Test Database Best Practices

1. **Use a separate test database** - Never run tests against your development database
2. **Clean state** - Each test should clean up after itself
3. **Parallel safety** - Use unique identifiers to avoid conflicts
4. **Reset between runs** - Consider recreating the test database periodically

Example test cleanup:

```rust
#[tokio::test]
async fn test_organization() {
    let pool = create_test_pool().await;

    // Create test data
    let org = create_organization(&pool, "test-org", "Test", None)
        .await
        .unwrap();

    // Run test assertions
    assert_eq!(org.slug, "test-org");

    // Cleanup
    delete_organization(&pool, "test-org").await.unwrap();
}
```

## Troubleshooting

### Connection Refused

```
Error: Connection refused (os error 111)
```

**Solution**: Ensure PostgreSQL is running:

```bash
# Check if PostgreSQL is running
sudo systemctl status postgresql

# Start PostgreSQL
sudo systemctl start postgresql
```

### Permission Denied

```
Error: permission denied for database
```

**Solution**: Grant proper permissions:

```sql
GRANT ALL PRIVILEGES ON DATABASE bdp TO your_user;
```

### Migration Failed

```
Error: migration failed: relation already exists
```

**Solution**: Reset the database:

```bash
# Drop and recreate
dropdb bdp
createdb bdp
sqlx migrate run
```

### Cannot Connect to Database

```
Error: invalid connection string
```

**Solution**: Check your `DATABASE_URL` format:

```bash
# Correct format
postgresql://user:password@host:port/database

# Examples
postgresql://postgres:postgres@localhost:5432/bdp
postgresql://localhost/bdp  # Uses default user and no password
```

### SQLx Offline Mode Issues

```
Error: query metadata not found
```

**Solution**: Generate query metadata:

```bash
# Make sure DATABASE_URL is set
export DATABASE_URL=postgresql://localhost/bdp

# Prepare queries
cargo sqlx prepare

# Verify files were created
ls .sqlx/
```

## Production Considerations

### Connection Pooling

Configure appropriate pool sizes for production:

```bash
DB_MAX_CONNECTIONS=50  # Based on database server capacity
DB_MIN_CONNECTIONS=10  # Maintain warm connections
DB_CONNECT_TIMEOUT=30  # Fail fast
DB_IDLE_TIMEOUT=600    # 10 minutes
DB_MAX_LIFETIME=1800   # 30 minutes - rotate connections
```

### Connection Limits

PostgreSQL has a maximum connection limit (default: 100). Calculate:

```
max_connections = (number_of_app_instances × DB_MAX_CONNECTIONS) + buffer
```

Example with 3 app instances:
- Each instance: 50 connections
- Total needed: 150 connections
- Set PostgreSQL `max_connections = 200` (includes buffer for admin)

### Performance Tuning

```sql
-- Add indexes for common queries
CREATE INDEX idx_organizations_name ON organizations(name);
CREATE INDEX idx_registry_entries_updated ON registry_entries(updated_at DESC);

-- Analyze tables regularly
ANALYZE organizations;
ANALYZE registry_entries;

-- Monitor slow queries
ALTER SYSTEM SET log_min_duration_statement = 1000;  -- Log queries > 1s
```

### Backups

```bash
# Backup database
pg_dump -U postgres bdp > bdp_backup.sql

# Restore database
psql -U postgres bdp < bdp_backup.sql

# Continuous backup with WAL archiving
# See PostgreSQL documentation for PITR setup
```

### Monitoring

Key metrics to monitor:

- Active connections
- Query latency (p50, p95, p99)
- Connection pool utilization
- Slow query log
- Table/index sizes
- Cache hit ratio

## Advanced Topics

### Transactions

```rust
use sqlx::Transaction;

async fn complex_operation(pool: &PgPool) -> DbResult<()> {
    let mut tx = pool.begin().await?;

    // Multiple operations in transaction
    create_organization(&mut tx, "org1", "Org 1", None).await?;
    create_registry_entry(&mut tx, org_id, "entry1").await?;

    // Commit or rollback
    tx.commit().await?;
    Ok(())
}
```

### Database Functions

Create reusable SQL functions:

```sql
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
```

### Full-Text Search

```sql
-- Add tsvector column
ALTER TABLE organizations
ADD COLUMN search_vector tsvector;

-- Create GIN index
CREATE INDEX idx_organizations_search
ON organizations USING GIN(search_vector);

-- Update search vector
UPDATE organizations
SET search_vector =
    to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''));

-- Search query
SELECT * FROM organizations
WHERE search_vector @@ to_tsquery('english', 'biological & data');
```

## Resources

- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [SQLx CLI Guide](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
- [Connection Pooling Best Practices](https://wiki.postgresql.org/wiki/Number_Of_Database_Connections)

## Next Steps

1. Review the [SQLx Usage Guide](./sqlx-guide.md) for query patterns
2. Explore the [Database Examples](../crates/bdp-server/examples/database_usage.rs)
3. Read the [API Documentation](../crates/bdp-server/src/db/README.md)
4. Set up continuous migration testing in CI/CD
