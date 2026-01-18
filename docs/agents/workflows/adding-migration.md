# Workflow: Adding a Database Migration to BDP

This document provides a step-by-step workflow for AI agents to create and apply database migrations in the BDP project using SQLx.

## Overview

```
Adding Migration Workflow
┌────────────────────────────────────────────────────────────┐
│  1. Create Migration File                                   │
│  2. Write SQL (Up Migration)                                │
│  3. Write Rollback (Down Migration - Optional)              │
│  4. Test Migration Up                                       │
│  5. Test Migration Down (if exists)                         │
│  6. Update .sqlx Files                                      │
│  7. Commit Migration + .sqlx Files                          │
│  8. Verify in CI/CD                                         │
└────────────────────────────────────────────────────────────┘
```

---

## Prerequisites

```bash
# SQLx CLI installed
cargo install sqlx-cli --no-default-features --features postgres,sqlite

# Database running
docker-compose up -d postgres

# DATABASE_URL set
echo $DATABASE_URL
# OR
cat .env | grep DATABASE_URL
```

---

## Step 1: Create Migration File

### Step 1.1: Use sqlx migrate add Command

```bash
# Syntax
sqlx migrate add <description>

# Examples
sqlx migrate add initial_schema
sqlx migrate add add_users_table
sqlx migrate add add_email_index
sqlx migrate add alter_users_add_role
```

**Output:**
```
Creating migrations/20260116123456_add_users_table.sql
```

**File naming convention:**
```
Format: {timestamp}_{description}.sql

Components:
- Timestamp: YYYYMMDDHHMMSS (ensures ordering)
- Underscore separator
- Description: snake_case, descriptive

Examples:
✓ 20260116123456_add_users_table.sql
✓ 20260116123500_create_organizations.sql
✓ 20260116123600_add_email_index.sql
✗ add_users.sql (no timestamp)
✗ migration_1.sql (not descriptive)
```

### Step 1.2: Manual Creation (Alternative)

```bash
# Generate timestamp
TIMESTAMP=$(date +%Y%m%d%H%M%S)

# Create file
touch migrations/${TIMESTAMP}_description.sql

# Example
touch migrations/20260116123456_add_users_table.sql
```

---

## Step 2: Write SQL (Up Migration)

### Step 2.1: Open Migration File

```bash
# Edit the created file
vim migrations/20260116123456_add_users_table.sql
```

### Step 2.2: Write SQL Schema Changes

**Example: Creating a Table**

```sql
-- migrations/20260116123456_add_users_table.sql

-- Create users table
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_created_at ON users(created_at DESC);

-- Add comment
COMMENT ON TABLE users IS 'User accounts for authentication';
```

**Example: Adding Columns**

```sql
-- migrations/20260116123500_add_user_profile.sql

-- Add profile columns to users
ALTER TABLE users
ADD COLUMN first_name TEXT,
ADD COLUMN last_name TEXT,
ADD COLUMN bio TEXT,
ADD COLUMN avatar_url TEXT;

-- Create index for name search
CREATE INDEX idx_users_full_name ON users(first_name, last_name);
```

**Example: Creating Relationships**

```sql
-- migrations/20260116123600_add_organizations.sql

-- Create organizations table
CREATE TABLE organizations (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    website TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create user-organization membership
CREATE TABLE organization_members (
    id BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, user_id)
);

-- Create indexes
CREATE INDEX idx_org_members_org_id ON organization_members(organization_id);
CREATE INDEX idx_org_members_user_id ON organization_members(user_id);
CREATE INDEX idx_organizations_slug ON organizations(slug);
```

**Example: Adding Constraints**

```sql
-- migrations/20260116123700_add_constraints.sql

-- Add check constraints
ALTER TABLE users
ADD CONSTRAINT email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}$'),
ADD CONSTRAINT username_length CHECK (LENGTH(username) >= 3 AND LENGTH(username) <= 50);

-- Add foreign key
ALTER TABLE posts
ADD CONSTRAINT fk_posts_author
FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE;
```

**Example: Creating Enums (PostgreSQL)**

```sql
-- migrations/20260116123800_add_user_roles.sql

-- Create role enum type
CREATE TYPE user_role AS ENUM ('admin', 'moderator', 'user', 'guest');

-- Add role column
ALTER TABLE users
ADD COLUMN role user_role NOT NULL DEFAULT 'user';

-- Create index
CREATE INDEX idx_users_role ON users(role);
```

### Step 2.3: Migration Best Practices

```sql
-- ✓ GOOD: Make migrations idempotent
CREATE TABLE IF NOT EXISTS users (...);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT;

-- ✓ GOOD: Use explicit column types
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,          -- Explicit
    name TEXT NOT NULL,                -- Clear
    created_at TIMESTAMPTZ NOT NULL    -- With timezone
);

-- ✗ AVOID: Implicit types
CREATE TABLE users (
    id SERIAL PRIMARY KEY,             -- SERIAL vs BIGSERIAL
    created_at TIMESTAMP               -- Without timezone
);

-- ✓ GOOD: Add indexes for foreign keys
CREATE INDEX idx_posts_author_id ON posts(author_id);

-- ✓ GOOD: Use meaningful constraint names
ALTER TABLE users
ADD CONSTRAINT email_unique UNIQUE(email);

-- ✗ AVOID: Auto-generated names
ALTER TABLE users ADD UNIQUE(email);

-- ✓ GOOD: Include comments
COMMENT ON TABLE users IS 'User accounts';
COMMENT ON COLUMN users.email IS 'Primary contact email';

-- ✓ GOOD: Set defaults
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
is_active BOOLEAN NOT NULL DEFAULT TRUE

-- ✓ GOOD: Use CASCADE for cleanup
FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE

-- ✗ CAREFUL: Data migrations
-- Keep schema and data migrations separate
-- Use separate migration for large data changes
```

---

## Step 3: Write Rollback (Down Migration)

### Step 3.1: Create Down Migration File (Optional)

```bash
# SQLx supports down migrations with .down.sql suffix
# File: migrations/{timestamp}_{description}.down.sql

# Example
vim migrations/20260116123456_add_users_table.down.sql
```

### Step 3.2: Write Rollback SQL

**Example: Dropping a Table**

```sql
-- migrations/20260116123456_add_users_table.down.sql

-- Drop indexes first
DROP INDEX IF EXISTS idx_users_created_at;
DROP INDEX IF EXISTS idx_users_username;
DROP INDEX IF EXISTS idx_users_email;

-- Drop table
DROP TABLE IF EXISTS users;
```

**Example: Removing Columns**

```sql
-- migrations/20260116123500_add_user_profile.down.sql

-- Drop index
DROP INDEX IF EXISTS idx_users_full_name;

-- Remove columns
ALTER TABLE users
DROP COLUMN IF EXISTS avatar_url,
DROP COLUMN IF EXISTS bio,
DROP COLUMN IF EXISTS last_name,
DROP COLUMN IF EXISTS first_name;
```

**Example: Complex Rollback**

```sql
-- migrations/20260116123600_add_organizations.down.sql

-- Drop tables in reverse order (respect foreign keys)
DROP TABLE IF EXISTS organization_members;
DROP TABLE IF EXISTS organizations;
```

### Step 3.3: When to Create Down Migrations

```
CREATE down migrations for:
  ✓ Development environment (easy rollback)
  ✓ Staging environment testing
  ✓ Non-destructive changes (adding columns/tables)
  ✓ Reversible schema changes

SKIP down migrations for:
  ✗ Production-only migrations
  ✗ Data migrations (often irreversible)
  ✗ Destructive operations (dropping columns with data)
  ✓ Complex migrations (document rollback separately)
```

---

## Step 4: Test Migration Up

### Step 4.1: Check Current Migration Status

```bash
# View migration history
sqlx migrate info

# Expected output
Applied At                  | Version | Description
============================|=========|================
2026-01-16 12:00:00.000000 | 1       | initial_schema
2026-01-16 12:05:00.000000 | 2       | add_users_table
(pending)                   | 3       | add_organizations
```

### Step 4.2: Run Migration

```bash
# Apply pending migrations
sqlx migrate run

# Expected output
Applied 3/add_organizations (Xs)
```

### Step 4.3: Verify Schema Changes

**Using psql:**
```bash
# Connect to database
psql -U bdp -d bdp

# List tables
\dt

# Describe table structure
\d users
\d organizations

# View indexes
\di

# View constraints
\d+ users

# Exit
\q
```

**Using SQL queries:**
```bash
# Check table exists
psql -U bdp -d bdp -c "SELECT tablename FROM pg_tables WHERE schemaname = 'public';"

# Check columns
psql -U bdp -d bdp -c "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = 'users';"

# Check indexes
psql -U bdp -d bdp -c "SELECT indexname FROM pg_indexes WHERE tablename = 'users';"
```

### Step 4.4: Test with Sample Data

```sql
-- Insert test data
INSERT INTO users (email, username, password_hash)
VALUES ('test@example.com', 'testuser', 'hashed_password');

-- Verify
SELECT * FROM users;

-- Clean up
DELETE FROM users WHERE email = 'test@example.com';
```

### Step 4.5: Troubleshooting Migration Errors

**Error: Relation already exists**
```
ERROR: relation "users" already exists
```

**Solution:**
```sql
-- Use IF NOT EXISTS
CREATE TABLE IF NOT EXISTS users (...);
```

**Error: Column already exists**
```
ERROR: column "bio" of relation "users" already exists
```

**Solution:**
```sql
-- Use IF NOT EXISTS (PostgreSQL 9.6+)
ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT;

-- Or check first
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'bio'
    ) THEN
        ALTER TABLE users ADD COLUMN bio TEXT;
    END IF;
END $$;
```

**Error: Foreign key constraint fails**
```
ERROR: insert or update on table "posts" violates foreign key constraint
```

**Solution:**
```bash
# Check data before migration
psql -U bdp -d bdp -c "SELECT * FROM posts WHERE author_id NOT IN (SELECT id FROM users);"

# Fix orphaned records
# Then re-run migration
```

---

## Step 5: Test Migration Down

### Step 5.1: Revert Migration

```bash
# Revert last applied migration
sqlx migrate revert

# Expected output
Applied 3/add_organizations (revert) (Xs)
```

### Step 5.2: Verify Rollback

```bash
# Check migration status
sqlx migrate info

# Should show migration as pending again
(pending) | 3 | add_organizations

# Verify schema
psql -U bdp -d bdp -c "\dt"
# organizations table should be gone
```

### Step 5.3: Re-apply Migration

```bash
# Re-apply to verify idempotency
sqlx migrate run

# Should succeed without errors
```

---

## Step 6: Update .sqlx Files

### Step 6.1: Why Update .sqlx?

```
Schema changes affect:
  ✓ Table structures
  ✓ Column types
  ✓ Column nullability
  ✓ Existing queries

Result: Stale .sqlx data causes compilation errors
```

### Step 6.2: Regenerate Offline Data

```bash
# Ensure migrations are applied
sqlx migrate run

# Regenerate all query metadata
cargo sqlx prepare

# Or for entire workspace
cargo sqlx prepare --workspace

# Expected output
query data written to `.sqlx` in the current directory
```

### Step 6.3: Review .sqlx Changes

```bash
# Check what changed
git status .sqlx/

# View changes
git diff .sqlx/

# New files for new queries
# Modified files for queries touching changed tables
```

### Step 6.4: When to Update .sqlx

```
Update .sqlx files when:
  ✓ New table created (if queried)
  ✓ Column added/removed
  ✓ Column type changed
  ✓ Column nullability changed
  ✓ Any schema change affecting queries
  ✗ Migration only adds indexes (usually no change)
  ✗ Migration only adds constraints (no query impact)
```

---

## Step 7: Commit Migration + .sqlx Files

### Step 7.1: Review All Changes

```bash
# Check status
git status

# Expected files
#   new file:   migrations/20260116123456_add_organizations.sql
#   new file:   migrations/20260116123456_add_organizations.down.sql
#   modified:   .sqlx/query-*.json (multiple files)
```

### Step 7.2: Stage Files

```bash
# Stage migration files
git add migrations/

# Stage .sqlx updates
git add .sqlx/

# Verify staged changes
git diff --cached
```

### Step 7.3: Write Descriptive Commit Message

```bash
git commit -m "feat: add organizations and membership schema

- Create organizations table with slug, name, description
- Create organization_members junction table
- Add foreign key constraints with CASCADE delete
- Add indexes for slug and membership lookups
- Add down migration for rollback
- Update sqlx offline data for new schema

Closes #42"
```

**Commit message template:**
```
<type>: <short summary>

- <change 1>
- <change 2>
- <change 3>
- Update sqlx offline data

<optional footer>
```

### Step 7.4: Push Changes

```bash
# Push to remote
git push origin feature/add-organizations

# Create pull request
gh pr create --title "feat: add organizations schema" --body "..."
```

---

## Step 8: Verify in CI/CD

### Step 8.1: CI Pipeline Expectations

**Successful CI workflow:**
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Start PostgreSQL
        run: docker-compose up -d postgres

      - name: Wait for database
        run: |
          until pg_isready -h localhost -p 5432; do
            sleep 1
          done

      - name: Run migrations
        run: cargo sqlx migrate run
        env:
          DATABASE_URL: postgresql://bdp:bdp_dev_password@localhost:5432/bdp

      - name: Build (with offline mode)
        run: cargo build
        env:
          SQLX_OFFLINE: true

      - name: Run tests
        run: cargo test
        env:
          DATABASE_URL: postgresql://bdp:bdp_dev_password@localhost:5432/bdp
```

### Step 8.2: Common CI Failures

**Failure: Migration fails in CI**
```
Error: error returned from database: relation "users" does not exist
```

**Cause:** Migration depends on previous migration that wasn't applied

**Solution:**
```bash
# Run all migrations locally first
sqlx migrate run

# Ensure all migrations committed
git log --oneline migrations/
```

**Failure: Offline data not updated**
```
Error: column "new_column" does not exist in cached query data
```

**Cause:** Forgot to run `cargo sqlx prepare`

**Solution:**
```bash
cargo sqlx prepare
git add .sqlx/
git commit --amend --no-edit
git push --force-with-lease
```

### Step 8.3: Verify CI Success

```bash
# Check CI status
gh pr checks

# View details
gh pr view

# Merge when CI passes
gh pr merge
```

---

## Complete Example: Adding Organizations Table

### Full workflow from start to finish

**Step 1: Create migration**
```bash
sqlx migrate add add_organizations_table
```

**Step 2: Write up migration**
```sql
-- migrations/20260116123456_add_organizations_table.sql

CREATE TABLE organizations (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    website TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organizations_name ON organizations(name);

COMMENT ON TABLE organizations IS 'Organizations that can own data sources';
```

**Step 3: Write down migration**
```sql
-- migrations/20260116123456_add_organizations_table.down.sql

DROP INDEX IF EXISTS idx_organizations_name;
DROP INDEX IF EXISTS idx_organizations_slug;
DROP TABLE IF EXISTS organizations;
```

**Step 4: Test migration up**
```bash
sqlx migrate run

psql -U bdp -d bdp -c "\d organizations"
```

**Step 5: Test with data**
```bash
psql -U bdp -d bdp -c "INSERT INTO organizations (name, slug) VALUES ('Test Org', 'test-org');"
psql -U bdp -d bdp -c "SELECT * FROM organizations;"
psql -U bdp -d bdp -c "DELETE FROM organizations WHERE slug = 'test-org';"
```

**Step 6: Test migration down**
```bash
sqlx migrate revert

psql -U bdp -d bdp -c "\dt"
# organizations should be gone

sqlx migrate run
# Re-apply
```

**Step 7: Update .sqlx (if queries exist)**
```bash
cargo sqlx prepare
```

**Step 8: Commit**
```bash
git add migrations/ .sqlx/
git commit -m "feat: add organizations table

- Create organizations table with name, slug, description
- Add unique constraint on slug
- Add indexes for common lookups
- Include down migration for rollback
- Update sqlx offline data"
```

**Step 9: Push and verify CI**
```bash
git push origin feature/organizations
gh pr create
gh pr checks
```

---

## Handling Migration Conflicts

### Scenario: Parallel Development

**Problem:**
```
Branch A: migrations/20260116120000_add_users.sql
Branch B: migrations/20260116120000_add_orgs.sql  # Same timestamp!
```

**Solution 1: Rename migration (preferred)**
```bash
# In branch B, after pulling A
git mv migrations/20260116120000_add_orgs.sql \
      migrations/20260116120100_add_orgs.sql
```

**Solution 2: Regenerate migration**
```bash
# Delete old migration
rm migrations/20260116120000_add_orgs.sql

# Create new with current timestamp
sqlx migrate add add_orgs

# Copy SQL from old file
```

### Scenario: Migration Order Dependencies

**Problem:**
```sql
-- Migration B tries to add foreign key to table from Migration A
-- But Migration A not yet applied
```

**Solution:**
```bash
# Check migration order
sqlx migrate info

# Ensure dependency order in filenames
# migrations/20260116120000_add_users.sql
# migrations/20260116120100_add_posts.sql (references users)
```

### Scenario: Production Migration Failures

**Problem:** Migration fails in production, database in inconsistent state

**Solution:**
```bash
# 1. DO NOT panic-revert (may cause data loss)

# 2. Check migration status
sqlx migrate info

# 3. Check database state
psql -c "SELECT * FROM _sqlx_migrations;"

# 4. Fix forward (create new migration)
sqlx migrate add fix_migration_issue

# 5. Or manually fix and mark as applied
psql -c "INSERT INTO _sqlx_migrations (version, description, success) VALUES (...);"
```

---

## Migration Checklist

Use this checklist for every migration:

```
□ Create migration file with descriptive name
□ Write SQL for schema changes
□ Use IF NOT EXISTS for idempotency
□ Add indexes for foreign keys
□ Add appropriate constraints
□ Include comments for documentation
□ Write down migration (optional)
□ Test migration up (sqlx migrate run)
□ Verify schema changes in database
□ Test with sample data
□ Test migration down (if exists)
□ Re-apply migration
□ Update .sqlx files (cargo sqlx prepare)
□ Review .sqlx changes
□ Commit migration + .sqlx together
□ Push to remote
□ Verify CI passes
□ Document breaking changes (if any)
```

---

## Quick Reference

### Common Commands

```bash
# Create migration
sqlx migrate add <description>

# Apply migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check status
sqlx migrate info

# Update offline data
cargo sqlx prepare

# Test offline build
SQLX_OFFLINE=true cargo build
```

### File Locations

```
migrations/                          # All migration files
migrations/{timestamp}_{desc}.sql    # Up migration
migrations/{timestamp}_{desc}.down.sql  # Down migration
.sqlx/                               # Offline query data
.sqlx/query-*.json                   # Query metadata
```

### SQL Templates

**Create table:**
```sql
CREATE TABLE IF NOT EXISTS table_name (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Add column:**
```sql
ALTER TABLE table_name
ADD COLUMN IF NOT EXISTS column_name TEXT;
```

**Add index:**
```sql
CREATE INDEX IF NOT EXISTS idx_name ON table(column);
```

**Add foreign key:**
```sql
ALTER TABLE table_name
ADD CONSTRAINT fk_name
FOREIGN KEY (column_id) REFERENCES other_table(id) ON DELETE CASCADE;
```

---

## Related Documentation

- [SQLx Comprehensive Guide](../implementation/sqlx-guide.md) - SQLx deep dive
- [Adding New Query Workflow](./adding-new-query.md) - Query workflow
- [Database Schema](../design/database-schema.md) - Schema documentation
- [Quick Start SQLx](../../QUICK_START_SQLX.md) - Quick reference

---

**Last Updated:** 2026-01-16
**Maintainer:** BDP Team
**Target Audience:** AI Agents working on BDP project
