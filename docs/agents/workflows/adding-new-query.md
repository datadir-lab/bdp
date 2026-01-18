# Workflow: Adding a New Query to BDP

This document provides a step-by-step workflow for AI agents to add new SQLx queries to the BDP project with compile-time checking and offline mode support.

## Overview

```
Adding New Query Workflow
┌────────────────────────────────────────────────────────────┐
│  1. Write Query                                             │
│  2. Test Locally with DATABASE_URL                          │
│  3. Run sqlx prepare                                        │
│  4. Commit .sqlx Files                                      │
│  5. Verify Offline Build                                    │
│  6. CI/CD Verification                                      │
└────────────────────────────────────────────────────────────┘
```

---

## Prerequisites

Before starting, ensure:

```bash
# SQLx CLI is installed
cargo install sqlx-cli --no-default-features --features postgres,sqlite

# DATABASE_URL is set (for local development)
cat .env | grep DATABASE_URL

# Database is running
docker-compose ps postgres
# OR
systemctl status postgresql

# Migrations are up to date
sqlx migrate info
```

---

## Step 1: Write the Query

### Step 1.1: Define the Data Model

**Location:** `crates/bdp-server/src/models/mod.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Organization model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Key points:**
- Derive `FromRow` for SQLx mapping
- Use `Option<T>` for nullable columns
- Match database types:
  - `BIGSERIAL` → `i64`
  - `TEXT` → `String`
  - `TIMESTAMPTZ` → `DateTime<Utc>`
  - `BOOLEAN` → `bool`
  - `UUID` → `uuid::Uuid` (requires uuid crate)

### Step 1.2: Add Query to Storage Layer

**Location:** `crates/bdp-server/src/storage/mod.rs` or domain-specific module

**Decision tree:**
```
What type of query?
├─ Single row → fetch_one() or fetch_optional()
├─ Multiple rows → fetch_all()
├─ Single value → query_scalar!() + fetch_one()
└─ Large dataset → fetch() (streaming)
```

**Example: Get Organization by Slug**

```rust
impl Storage {
    /// Get organization by slug
    pub async fn get_organization_by_slug(
        &self,
        slug: &str,
    ) -> ServerResult<Option<Organization>> {
        let org = sqlx::query_as!(
            Organization,
            r#"
            SELECT id, name, slug, description, website, created_at, updated_at
            FROM organizations
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(org)
    }
}
```

**Example: List Organizations with Pagination**

```rust
impl Storage {
    /// List organizations with pagination
    pub async fn list_organizations(
        &self,
        limit: i64,
        offset: i64,
    ) -> ServerResult<Vec<Organization>> {
        let orgs = sqlx::query_as!(
            Organization,
            r#"
            SELECT id, name, slug, description, website, created_at, updated_at
            FROM organizations
            ORDER BY name ASC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(orgs)
    }
}
```

**Example: Count Query**

```rust
impl Storage {
    /// Count total organizations
    pub async fn count_organizations(&self) -> ServerResult<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM organizations"
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }
}
```

**Example: Insert Query**

```rust
impl Storage {
    /// Create a new organization
    pub async fn create_organization(
        &self,
        name: &str,
        slug: &str,
        description: Option<&str>,
    ) -> ServerResult<Organization> {
        let org = sqlx::query_as!(
            Organization,
            r#"
            INSERT INTO organizations (name, slug, description)
            VALUES ($1, $2, $3)
            RETURNING id, name, slug, description, website, created_at, updated_at
            "#,
            name,
            slug,
            description
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(org)
    }
}
```

**Example: Update Query**

```rust
impl Storage {
    /// Update organization
    pub async fn update_organization(
        &self,
        id: i64,
        name: &str,
        description: Option<&str>,
    ) -> ServerResult<Organization> {
        let org = sqlx::query_as!(
            Organization,
            r#"
            UPDATE organizations
            SET name = $2,
                description = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, slug, description, website, created_at, updated_at
            "#,
            id,
            name,
            description
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(org)
    }
}
```

### Step 1.3: Query Best Practices

```rust
// ✓ GOOD: Explicit column selection
SELECT id, name, email FROM users

// ✗ AVOID: SELECT *
SELECT * FROM users

// ✓ GOOD: Use query_as! with named struct
query_as!(User, "SELECT id, name FROM users")

// ✗ AVOID: Runtime query for static queries
query_as::<_, User>("SELECT id, name FROM users")

// ✓ GOOD: Parameterized queries
WHERE slug = $1

// ✗ NEVER: String interpolation
WHERE slug = '{}'  // SQL injection risk!

// ✓ GOOD: Use r#"..."# for multiline
r#"
    SELECT *
    FROM users
    WHERE id = $1
"#

// ✓ GOOD: Handle nullable columns
description: Option<String>

// ✗ WRONG: Non-nullable for nullable column
description: String  // Panics if NULL!
```

---

## Step 2: Test Locally with DATABASE_URL

### Step 2.1: Ensure Database is Running

```bash
# Start database with Docker Compose
docker-compose up -d postgres

# Or start local PostgreSQL
systemctl start postgresql

# Verify connection
psql -h localhost -U bdp -d bdp -c "SELECT 1;"
```

### Step 2.2: Run Migrations

```bash
# Check migration status
sqlx migrate info

# Run pending migrations
sqlx migrate run

# Verify schema
psql -h localhost -U bdp -d bdp -c "\d organizations"
```

### Step 2.3: Compile and Test

```bash
# Compile with database connection (compile-time checking)
cargo check

# Expected output if successful:
#   Checking bdp-server v0.1.0
#   Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs

# If errors occur, see troubleshooting section below
```

### Step 2.4: Write and Run Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_get_organization_by_slug() {
        let pool = setup_test_db().await;
        let storage = Storage::new(pool);

        // Insert test data
        let org = storage
            .create_organization("Test Org", "test-org", Some("Description"))
            .await
            .unwrap();

        // Test query
        let result = storage
            .get_organization_by_slug("test-org")
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, org.id);
    }

    #[tokio::test]
    async fn test_list_organizations() {
        let pool = setup_test_db().await;
        let storage = Storage::new(pool);

        // Insert test data
        storage.create_organization("Org 1", "org-1", None).await.unwrap();
        storage.create_organization("Org 2", "org-2", None).await.unwrap();

        // Test query
        let orgs = storage.list_organizations(10, 0).await.unwrap();

        assert_eq!(orgs.len(), 2);
    }
}
```

**Run tests:**
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_get_organization_by_slug

# Run with output
cargo test -- --nocapture
```

---

## Step 3: Run sqlx prepare

### Step 3.1: Generate Offline Query Data

```bash
# Generate .sqlx metadata for all queries
cargo sqlx prepare

# Or for specific workspace member
cargo sqlx prepare -p bdp-server

# Or for entire workspace
cargo sqlx prepare --workspace
```

**Expected output:**
```
query data written to `.sqlx` in the current directory; please check this into version control
```

### Step 3.2: Verify .sqlx Files Created

```bash
# List generated files
ls -la .sqlx/

# Should see files like:
# query-<hash1>.json
# query-<hash2>.json
# ...

# Check a query file
cat .sqlx/query-*.json | jq .
```

**Example .sqlx file content:**
```json
{
  "db_name": "PostgreSQL",
  "query": "SELECT id, name, slug, description, website, created_at, updated_at FROM organizations WHERE slug = $1",
  "describe": {
    "columns": [
      {
        "ordinal": 0,
        "name": "id",
        "type_info": "Int8"
      },
      {
        "ordinal": 1,
        "name": "name",
        "type_info": "Text"
      }
    ],
    "parameters": {
      "Left": ["Text"]
    },
    "nullable": [false, false, false, true, true, false, false]
  },
  "hash": "abc123..."
}
```

### Step 3.3: Troubleshooting sqlx prepare

**Error: DATABASE_URL not set**
```bash
# Solution: Set DATABASE_URL
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"

# Or use .env
echo "DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp" >> .env
```

**Error: Connection refused**
```bash
# Solution: Start database
docker-compose up -d postgres

# Verify
docker-compose ps
```

**Error: Schema mismatch**
```bash
# Solution: Run migrations
sqlx migrate run

# Verify schema
psql -U bdp -d bdp -c "\d+ organizations"
```

---

## Step 4: Commit .sqlx Files

### Step 4.1: Review Changes

```bash
# Check what changed
git status

# Should see:
#   modified:   crates/bdp-server/src/storage/mod.rs
#   modified:   crates/bdp-server/src/models/mod.rs
#   new file:   .sqlx/query-<hash>.json

# Review .sqlx changes
git diff .sqlx/
```

### Step 4.2: Stage and Commit

```bash
# Stage all changes
git add crates/bdp-server/src/storage/mod.rs
git add crates/bdp-server/src/models/mod.rs
git add .sqlx/

# Commit with descriptive message
git commit -m "feat: add organization queries

- Add Organization model
- Implement get_organization_by_slug
- Implement list_organizations with pagination
- Add sqlx offline data for new queries"

# Push to remote
git push origin feature/organization-queries
```

### Step 4.3: Commit Message Guidelines

```
Format:
<type>: <subject>

<body>

Types:
  feat:     New feature
  fix:      Bug fix
  docs:     Documentation changes
  test:     Adding tests
  refactor: Code refactoring
  chore:    Maintenance tasks

Examples:
  feat: add user authentication queries
  fix: correct organization slug lookup
  refactor: optimize list queries with indexes
  chore: update sqlx offline data
```

---

## Step 5: Verify Offline Build

### Step 5.1: Enable Offline Mode

```bash
# Set environment variable
export SQLX_OFFLINE=true

# Verify
echo $SQLX_OFFLINE
```

### Step 5.2: Clean and Rebuild

```bash
# Clean build artifacts
cargo clean

# Build with offline mode (no database connection)
SQLX_OFFLINE=true cargo build

# Expected: Successful build without database
```

### Step 5.3: Run Tests in Offline Mode

```bash
# Run tests (they still need database for runtime)
cargo test

# Note: Offline mode only affects compile-time checking
# Runtime tests still require database connection
```

### Step 5.4: Troubleshooting Offline Build

**Error: offline mode enabled but no cached data found**
```bash
# Solution: Regenerate .sqlx
unset SQLX_OFFLINE
cargo sqlx prepare
export SQLX_OFFLINE=true
cargo build
```

**Error: query hash mismatch**
```bash
# Cause: Query changed but .sqlx not updated
# Solution: Regenerate
cargo sqlx prepare
git add .sqlx/
git commit --amend --no-edit
```

---

## Step 6: CI/CD Verification

### Step 6.1: Create Pull Request

```bash
# Push branch
git push origin feature/organization-queries

# Create PR using GitHub CLI
gh pr create --title "feat: add organization queries" --body "
## Summary
- Added Organization model with FromRow derive
- Implemented get_organization_by_slug query
- Implemented list_organizations with pagination
- Added unit tests for new queries
- Generated sqlx offline data

## Testing
- [x] Local compilation with DATABASE_URL
- [x] Unit tests pass
- [x] Offline mode build succeeds
- [ ] CI/CD verification pending

## Related
- Closes #123
"
```

### Step 6.2: CI/CD Pipeline

**Expected CI workflow:**
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      # No database setup needed!
      - name: Build with offline mode
        run: cargo build
        env:
          SQLX_OFFLINE: true

      - name: Run tests (with database)
        run: |
          docker-compose up -d postgres
          cargo test
        env:
          DATABASE_URL: postgresql://bdp:bdp_dev_password@localhost:5432/bdp
```

### Step 6.3: Verify CI Success

```bash
# Check CI status
gh pr checks

# View CI logs if failures
gh run view <run-id> --log
```

### Step 6.4: Common CI Failures

**Failure: .sqlx files not committed**
```
Error: offline mode enabled but no cached data found
```

**Solution:**
```bash
# Ensure .sqlx files are committed
git add .sqlx/
git commit --amend --no-edit
git push --force-with-lease
```

**Failure: Type mismatch**
```
Error: expected String, found Option<String>
```

**Solution:**
```bash
# Fix struct definition
# Update .sqlx
cargo sqlx prepare
git add .sqlx/
git commit --amend --no-edit
git push --force-with-lease
```

---

## Complete Example: Adding a Search Query

### Example: Full-text search for organizations

**Step 1: Write query in storage layer**

```rust
impl Storage {
    /// Search organizations by name or description
    pub async fn search_organizations(
        &self,
        query: &str,
        limit: i64,
    ) -> ServerResult<Vec<Organization>> {
        let search_pattern = format!("%{}%", query);

        let orgs = sqlx::query_as!(
            Organization,
            r#"
            SELECT id, name, slug, description, website, created_at, updated_at
            FROM organizations
            WHERE name ILIKE $1 OR description ILIKE $1
            ORDER BY name ASC
            LIMIT $2
            "#,
            search_pattern,
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(orgs)
    }
}
```

**Step 2: Test locally**

```bash
# Ensure database is running
docker-compose up -d postgres

# Test compilation
cargo check

# Run test
cargo test test_search_organizations
```

**Step 3: Generate offline data**

```bash
cargo sqlx prepare
```

**Step 4: Commit**

```bash
git add crates/bdp-server/src/storage/mod.rs .sqlx/
git commit -m "feat: add organization search query"
```

**Step 5: Verify offline**

```bash
SQLX_OFFLINE=true cargo build
```

**Step 6: Create PR and verify CI**

```bash
git push origin feature/org-search
gh pr create
```

---

## Quick Reference Checklist

Use this checklist when adding a new query:

```
□ Define struct with FromRow derive
□ Match struct fields to database column types
□ Use Option<T> for nullable columns
□ Add query method to Storage implementation
□ Use query_as! for compile-time checking
□ Write unit tests
□ Test locally with DATABASE_URL
□ Run cargo sqlx prepare
□ Verify .sqlx files created
□ Review .sqlx changes with git diff
□ Commit source code and .sqlx files together
□ Test offline build with SQLX_OFFLINE=true
□ Create PR with descriptive title
□ Verify CI passes
□ Merge after approval
```

---

## Troubleshooting Guide

### Compilation Errors

| Error | Cause | Solution |
|-------|-------|----------|
| DATABASE_URL must be set | Env var not set | `export DATABASE_URL=...` or create `.env` |
| Connection refused | Database not running | `docker-compose up -d postgres` |
| Column does not exist | Schema mismatch | `sqlx migrate run` |
| Type mismatch | Struct doesn't match schema | Fix struct or schema |
| Offline data not found | Missing .sqlx files | `cargo sqlx prepare` |

### Runtime Errors

| Error | Cause | Solution |
|-------|-------|----------|
| Query returned no rows | fetch_one() but 0 rows | Use fetch_optional() |
| Too many rows | fetch_one() but >1 rows | Use fetch_all() or add LIMIT |
| NULL constraint violation | Inserting NULL into NOT NULL | Ensure value is provided |

---

## Related Documentation

- [SQLx Comprehensive Guide](../implementation/sqlx-guide.md) - Deep dive into SQLx
- [Adding Migration Workflow](./adding-migration.md) - Schema changes
- [Quick Start SQLx](../../QUICK_START_SQLX.md) - Quick reference
- [Database Schema](../design/database-schema.md) - Schema documentation

---

**Last Updated:** 2026-01-16
**Maintainer:** BDP Team
**Target Audience:** AI Agents working on BDP project
