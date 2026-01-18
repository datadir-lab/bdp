# BDP Scripts

Utility scripts for development, testing, deployment, and data ingestion.

## Directory Structure

```
scripts/
├── dev/         # Development utilities
├── test/        # Testing scripts
├── deploy/      # Deployment scripts
└── ingest/      # Data ingestion scripts
```

## Development Scripts (`dev/`)

Scripts for local development:

- Database setup and seeding
- Local service management
- Code generation
- Development environment initialization

Example usage:
```bash
./scripts/dev/setup-db.sh
./scripts/dev/seed-test-data.sh
```

## Test Scripts (`test/`)

Scripts for running tests and test infrastructure:

- Integration test setup
- Test database management
- Performance testing
- Load testing

Example usage:
```bash
./scripts/test/run-integration-tests.sh
./scripts/test/benchmark.sh
```

## Deployment Scripts (`deploy/`)

Scripts for deploying the application:

- Production deployment
- Database migrations
- Service configuration
- Health checks

Example usage:
```bash
./scripts/deploy/deploy-production.sh
./scripts/deploy/migrate-db.sh
```

## Ingestion Scripts (`ingest/`)

Scripts for data ingestion workflows:

- Scheduled ingestion jobs
- One-time data imports
- Data validation
- Update automation

Example usage:
```bash
./scripts/ingest/daily-uniprot-update.sh
./scripts/ingest/import-ncbi-batch.sh
```

## Guidelines

### Writing Scripts

1. **Use bash with strict mode:**
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   ```

2. **Add usage documentation:**
   ```bash
   usage() {
       echo "Usage: $0 [options]"
       echo "Options:"
       echo "  -h, --help    Show this help message"
   }
   ```

3. **Validate prerequisites:**
   ```bash
   command -v psql >/dev/null 2>&1 || {
       echo "Error: psql is required but not installed"
       exit 1
   }
   ```

4. **Use environment variables for configuration:**
   ```bash
   DB_HOST="${DB_HOST:-localhost}"
   DB_PORT="${DB_PORT:-5432}"
   ```

5. **Add error handling:**
   ```bash
   cleanup() {
       # Cleanup code here
   }
   trap cleanup EXIT
   ```

## Environment Variables

Scripts expect certain environment variables. Set them in `.env` or export before running:

```bash
export BDP_DB_URL="postgresql://user:pass@localhost/bdp"
export BDP_API_URL="http://localhost:8080"
```

## Making Scripts Executable

```bash
chmod +x scripts/dev/my-script.sh
```

## Testing Scripts

Test scripts in a safe environment before using in production:

```bash
# Use dry-run flags where available
./scripts/deploy/deploy-production.sh --dry-run

# Test with development configuration
./scripts/ingest/import-data.sh --config dev
```
