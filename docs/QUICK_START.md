# BDP Quick Start Guide

Get the BDP backend up and running in 5 minutes.

## Prerequisites

- Docker and Docker Compose
- Rust toolchain (1.75+)
- Just command runner (`cargo install just`)

## Quick Start

### 1. Start Services

```bash
# Start PostgreSQL and MinIO
docker-compose up -d postgres minio

# Wait for services to be healthy (about 10 seconds)
docker-compose ps
```

### 2. Configure Environment

```bash
# Copy environment template
cp .env.example .env

# The defaults work for local development
# No changes needed for quick start
```

### 3. Run Database Migrations

```bash
# Apply all migrations
just db-migrate

# Verify database is ready
just db-status
```

### 4. Start the Server

```bash
# Run in development mode
cargo run --bin bdp-server

# Or use just
just dev
```

The server will start on `http://localhost:8000`

## Verify Installation

### Health Check

```bash
curl http://localhost:8000/health
```

Expected response:
```json
{
  "status": "healthy",
  "database": "connected"
}
```

### API Root

```bash
curl http://localhost:8000/
```

Expected response:
```json
{
  "name": "BDP Server",
  "version": "0.1.0",
  "status": "running"
}
```

### MinIO Console

Open http://localhost:9001 in your browser
- Username: `minioadmin`
- Password: `minioadmin`

You should see the `bdp-data` bucket created automatically.

## Test the API

### Create an Organization

```bash
curl -X POST http://localhost:8000/api/v1/organizations \
  -H "Content-Type: application/json" \
  -H "x-user-id: testuser" \
  -d '{
    "slug": "uniprot",
    "name": "UniProt Consortium",
    "description": "Universal Protein Resource",
    "website": "https://www.uniprot.org",
    "is_system": true
  }'
```

### List Organizations

```bash
curl http://localhost:8000/api/v1/organizations
```

### Upload a File

```bash
# Create a test file
echo "Hello, BDP!" > test.txt

# Upload it
curl -X POST \
  http://localhost:8000/api/v1/files/uniprot/test-data/1.0.0/test.txt \
  -F "file=@test.txt" \
  -H "x-user-id: testuser"
```

Response includes a presigned download URL:
```json
{
  "key": "data-sources/uniprot/test-data/1.0.0/test.txt",
  "checksum": "abc123...",
  "size": 13,
  "presigned_url": "http://localhost:9000/..."
}
```

### Get Download URL

```bash
curl http://localhost:8000/api/v1/files/uniprot/test-data/1.0.0/test.txt
```

## Run Tests

### Unit Tests

```bash
cargo test --lib
```

### Integration Tests (Requires Database)

```bash
# With services running
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"

cargo test --test '*'
```

### Storage Tests (Requires MinIO)

```bash
# With MinIO running
export S3_ENDPOINT=http://localhost:9000
export S3_ACCESS_KEY=minioadmin
export S3_SECRET_KEY=minioadmin
export S3_BUCKET=bdp-data
export S3_PATH_STYLE=true

cargo test --test storage_tests
```

## Common Commands

```bash
# Database
just db-start          # Start PostgreSQL
just db-stop           # Stop PostgreSQL
just db-migrate        # Run migrations
just db-reset          # Drop and recreate database

# Development
just dev               # Run server in dev mode
just check             # Check code compiles
just test              # Run all tests
just fmt               # Format code
just lint              # Run clippy

# Docker
just docker-up         # Start all services
just docker-down       # Stop all services
just docker-logs       # View logs
```

## API Endpoints

### Organizations
- `POST /api/v1/organizations` - Create organization
- `GET /api/v1/organizations` - List organizations
- `GET /api/v1/organizations/:slug` - Get organization
- `PUT /api/v1/organizations/:slug` - Update organization
- `DELETE /api/v1/organizations/:slug` - Delete organization

### Data Sources
- `POST /api/v1/sources` - Create data source
- `GET /api/v1/sources` - List data sources
- `GET /api/v1/sources/:org/:slug` - Get data source
- `PUT /api/v1/sources/:org/:slug` - Update data source
- `DELETE /api/v1/sources/:org/:slug` - Delete data source
- `POST /api/v1/sources/:org/:slug/versions` - Publish version
- `GET /api/v1/sources/:org/:slug/:version` - Get version details
- `GET /api/v1/sources/:org/:slug/:version/dependencies` - List dependencies

### Files
- `POST /api/v1/files/:org/:name/:version/:filename` - Upload file
- `GET /api/v1/files/:org/:name/:version/:filename` - Get download URL

### Search
- `GET /api/v1/search?query=protein` - Unified search

### Resolve
- `POST /api/v1/resolve` - Resolve manifest to lockfile

## Troubleshooting

### Port Already in Use

```bash
# Check what's using the ports
lsof -i :8000  # Server
lsof -i :5432  # PostgreSQL
lsof -i :9000  # MinIO API
lsof -i :9001  # MinIO Console

# Kill the process or change ports in .env
```

### Database Connection Failed

```bash
# Check if PostgreSQL is running
docker-compose ps postgres

# Check logs
docker-compose logs postgres

# Restart
docker-compose restart postgres
```

### MinIO Not Accessible

```bash
# Check if MinIO is running
docker-compose ps minio

# Check logs
docker-compose logs minio

# Restart
docker-compose restart minio
```

### SQLx Compilation Errors

```bash
# Generate query metadata
export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"
cargo sqlx prepare

# Or use offline mode
export SQLX_OFFLINE=true
cargo build
```

## Next Steps

1. **Read the Documentation**
   - [SETUP.md](./SETUP.md) - Detailed setup guide
   - [TESTING.md](./TESTING.md) - Testing guide
   - [ROADMAP.md](./ROADMAP.md) - Project roadmap
   - [docs/](./docs/) - Architecture and design docs

2. **Explore the API**
   - Try creating data sources
   - Upload and download files
   - Test search functionality
   - Experiment with dependency resolution

3. **Contribute**
   - Check [ROADMAP.md](./ROADMAP.md) for pending tasks
   - Phase 2 (Data Ingestion) is ready to start
   - Phase 3 (CLI) and Phase 4 (Frontend) can begin in parallel

4. **Deploy**
   - See [docs/agents/implementation/deployment.md](./docs/agents/implementation/deployment.md) for production deployment guide

## Support

- Issues: https://github.com/datadir-lab/bdp/issues
- Documentation: [docs/](./docs/)
- Architecture: [docs/agents/](./docs/agents/)

---

**Ready to build the npm for bioinformatics!** 🧬🚀
