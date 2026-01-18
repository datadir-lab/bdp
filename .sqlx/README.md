# SQLx Prepared Query Metadata

This directory contains prepared query metadata for SQLx's compile-time verification.

## Overview

SQLx's `query!` and `query_as!` macros perform compile-time SQL verification by connecting to your database and checking the queries against the actual schema. However, this requires a database connection at compile time, which isn't always available (e.g., in CI/CD pipelines, offline development).

The `.sqlx` directory solves this by storing pre-computed query metadata that SQLx can use for offline compilation.

## How It Works

1. **During development** (with database access):
   - SQLx macros connect to the database
   - They verify queries and extract type information
   - You can save this metadata using `cargo sqlx prepare`

2. **During offline compilation** (without database access):
   - SQLx reads the prepared metadata from `.sqlx/*.json`
   - Compilation succeeds without needing a database connection
   - Type safety is still enforced at compile time

## File Structure

Each JSON file corresponds to a specific SQL query in your code:

```json
{
  "db_name": "PostgreSQL",
  "query": "SELECT ...",
  "describe": {
    "columns": [...],      // Column names and types
    "parameters": {...},   // Parameter types
    "nullable": [...]      // Nullability information
  },
  "hash": "..."           // Query hash for verification
}
```

## Usage

### Generating Metadata

Run this command when you have database access:

```bash
# Set your database URL
export DATABASE_URL="postgresql://user:pass@localhost/bdp"

# Generate metadata for all queries
cargo sqlx prepare
```

This will create/update all `query-*.json` files in this directory.

### Offline Compilation

To enable offline mode, set the environment variable:

```bash
export SQLX_OFFLINE=true
cargo build
```

Or add it to your CI/CD pipeline.

## Benefits

1. **CI/CD Support**: Build without running a database in CI
2. **Faster Builds**: No database connection overhead during compilation
3. **Version Control**: Track query changes in git
4. **Type Safety**: Still get compile-time SQL verification
5. **Cross-Platform**: Works on any machine without database setup

## Maintenance

- **Keep in sync**: Run `cargo sqlx prepare` after changing any queries
- **Commit changes**: Include updated `.sqlx/*.json` files in version control
- **Review diffs**: Query metadata changes show up in git diffs

## Example Files

This directory contains example metadata files showing the structure:

- `query-get_organization_by_slug.json`: Single row SELECT
- `query-list_organizations.json`: Paginated SELECT with LIMIT/OFFSET
- `query-create_organization.json`: INSERT with RETURNING
- `query-update_organization.json`: UPDATE with RETURNING

## Real vs Example Files

**Note**: The files in this directory are examples for documentation purposes. They show the structure but use placeholder hashes. In a real project, run `cargo sqlx prepare` to generate actual metadata from your database schema.

To generate real metadata:

1. Set up your PostgreSQL database with the schema from `migrations/`
2. Set `DATABASE_URL` environment variable
3. Run `cargo sqlx prepare`
4. The real metadata files will replace these examples

## Further Reading

- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [Compile-time verification](https://github.com/launchbadge/sqlx#compile-time-verification)
- [Offline mode](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query)
