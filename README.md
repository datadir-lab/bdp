# BDP - Bioinformatics Dependencies Platform

Version-controlled registry for biological data sources with reproducible lockfiles. Think npm/cargo for bioinformatics data.

## Features

- 🧬 **UniProt Protein Ingestion** - Robust parallel ETL pipeline with S3 storage
- 📦 **Version Control** - Track data sources like code dependencies
- 🔒 **Reproducible** - Lockfiles ensure consistent data across environments
- 📝 **Audit & Provenance** - Local audit trail for regulatory compliance (FDA, NIH, EMA)
- 🔍 **Integrity Verification** - Hash-chain tamper detection and checksum validation
- ⚡ **Parallel Processing** - Distributed workers with automatic load balancing
- 🐳 **Docker Ready** - Complete containerized development environment
- 🚀 **Production Ready** - Fault-tolerant, idempotent batch processing

## Quick Start

### For Users (CLI)

```bash
# Install BDP CLI
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh

# Initialize project
bdp init

# Add data sources
bdp source add "uniprot:P01308-fasta@1.0"

# Download and cache (with automatic audit logging)
bdp pull

# Verify integrity
bdp audit

# Export audit report for publication
bdp audit export --format das --output data-availability.md
```

See [INSTALL.md](./INSTALL.md) for all installation methods.

### For Developers (Full Stack)

```bash
# Clone repository
git clone https://github.com/datadir-lab/bdp.git
cd bdp

# Start all services (PostgreSQL + MinIO + BDP Server)
docker-compose up -d

# Check service health
docker-compose ps

# View logs
docker-compose logs -f bdp-server
```

**Services will be available at:**
- **API Server**: http://localhost:8000
- **MinIO Console**: http://localhost:9001 (minioadmin/minioadmin)
- **PostgreSQL**: localhost:5432 (bdp/bdp_dev_password)

See [Development Setup](#development-setup) for detailed instructions.

## Status

| Component | Status | Description |
|-----------|--------|-------------|
| **CLI Tool** | ✅ Complete | 78 tests passing, multi-platform releases |
| **Audit & Provenance** | ✅ Complete | Local audit trail with hash-chain integrity |
| **Backend Server** | ✅ Complete | REST API, CQRS, parallel ETL |
| **Ingestion Pipeline** | ✅ Complete | Robust parallel processing with S3 |
| **Database Schema** | ✅ Complete | PostgreSQL with full migrations |
| **Docker Setup** | ✅ Complete | Multi-container development environment |
| **Export Formats** | 🚧 Planned | FDA, NIH, EMA compliance reports |
| **Post-Pull Hooks** | 🚧 Planned | Auto-processing (samtools, BLAST, BWA) |
| **Web Frontend** | 🚧 Planned | Next.js 16 + Nextra |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     BDP Platform                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                │
│  │   CLI    │  │  Server  │  │   Web    │                │
│  │  (Rust)  │  │  (Rust)  │  │(Next.js) │                │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                │
│       │             │              │                       │
│       └─────────────┼──────────────┘                       │
│                     │                                      │
│  ┌──────────────────┴──────────────────┐                  │
│  │     PostgreSQL Database              │                  │
│  │  • Data sources registry             │                  │
│  │  • Version tracking                  │                  │
│  │  • Work unit coordination            │                  │
│  │  • SKIP LOCKED for parallelism       │                  │
│  └─────────────────┬────────────────────┘                  │
│                    │                                       │
│  ┌─────────────────┴────────────────────┐                  │
│  │     MinIO / S3 Storage               │                  │
│  │  • Raw ingestion files               │                  │
│  │  • Data source artifacts             │                  │
│  │  • MD5 checksums                     │                  │
│  └──────────────────────────────────────┘                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Audit & Provenance System

BDP includes a comprehensive audit trail system for regulatory compliance and research documentation:

### Features

- **Local Audit Trail** - SQLite database (`.bdp/bdp.db`) tracking all CLI operations
- **Hash-Chain Integrity** - Tamper detection via cryptographic hash linking
- **Machine ID Tracking** - Privacy-conscious identification without personal data
- **CQRS Middleware** - All commands automatically logged with start/success/failure events
- **Editable by Design** - Audit trail is intended for research documentation, not legal evidence

### What Gets Logged

```json
{
  "event_type": "init_success",
  "timestamp": "2026-01-18T14:30:00Z",
  "source_spec": null,
  "details": {
    "path": "./my-project",
    "name": "my-project",
    "version": "0.1.0"
  },
  "machine_id": "workstation-abc123",
  "event_hash": "sha256:...",
  "previous_hash": "sha256:..."
}
```

### Export Formats (Coming Soon)

Generate compliance reports for scientific publications:

```bash
# FDA 21 CFR Part 11 compliance report
bdp audit export --format fda --output audit-fda.json

# NIH Data Management & Sharing (DMS) report
bdp audit export --format nih --output data-availability.md

# EMA ALCOA++ compliance report
bdp audit export --format ema --output audit-ema.yaml

# Data Availability Statement for papers
bdp audit export --format das --output methods/data-availability.md
```

### Verify Integrity

```bash
# Verify audit chain integrity
bdp audit verify

# Check file checksums against lockfile
bdp audit
```

**Important**: The audit trail is stored locally in `.bdp/bdp.db` and is editable. It is intended for research documentation and report generation, not legal evidence. See [docs/agents/design/cli-audit-provenance.md](./docs/agents/design/cli-audit-provenance.md) for detailed design.

## Development Setup

### Prerequisites

**Required:**
- [Docker](https://docs.docker.com/get-docker/) & Docker Compose v2
- [Rust](https://rustup.rs/) 1.70+ (for native development)
- [cargo-sqlx](https://crates.io/crates/sqlx-cli): `cargo install sqlx-cli --features postgres`
- [just](https://github.com/casey/just): `cargo install just`

**Optional:**
- [Node.js](https://nodejs.org/) 20+ (for web frontend)
- [cargo-watch](https://crates.io/crates/cargo-watch): `cargo install cargo-watch`

### 1. Clone and Setup

```bash
# Clone repository
git clone https://github.com/datadir-lab/bdp.git
cd bdp

# Copy environment template
cp .env.example .env

# (Optional) Edit .env for custom configuration
nano .env
```

### 2. Start Services with Docker

#### Option A: All Services (Recommended)

```bash
# Start everything: PostgreSQL + MinIO + BDP Server
docker-compose up -d

# Check status
docker-compose ps

# Expected output:
# bdp-postgres   Up (healthy)
# bdp-minio      Up (healthy)
# bdp-server     Up (healthy)

# View logs
docker-compose logs -f bdp-server
```

#### Option B: Database + Storage Only

```bash
# Start just PostgreSQL and MinIO
docker-compose up -d postgres minio minio-init

# Run server natively for faster iteration
cargo run --bin bdp-server
```

### 3. Verify Setup

```bash
# Check API health
curl http://localhost:8000/health

# Expected: {"status":"ok"}

# Check MinIO is accessible
curl http://localhost:9000/minio/health/live

# Expected: OK

# Check database connection
docker exec bdp-postgres psql -U bdp -d bdp -c "SELECT version();"
```

### 4. Run Database Migrations

Migrations run automatically when using Docker. For manual migration:

```bash
# Using Docker
docker-compose exec bdp-server sqlx migrate run

# Or locally (requires DATABASE_URL in .env)
sqlx migrate run
```

### 5. Access Services

| Service | URL | Credentials |
|---------|-----|-------------|
| API Server | http://localhost:8000 | - |
| API Docs | http://localhost:8000/api/docs | - |
| MinIO Console | http://localhost:9001 | minioadmin / minioadmin |
| PostgreSQL | localhost:5432 | bdp / bdp_dev_password |
| MinIO S3 | localhost:9000 | minioadmin / minioadmin |

## Development Workflows

### Backend Development

```bash
# Hot reload during development
cargo watch -x 'run --bin bdp-server'

# Run tests
cargo test

# Run specific test
cargo test --test test_name

# Check code (fast)
cargo check

# Full build with optimizations
cargo build --release
```

### Database Operations

```bash
# Create new migration
sqlx migrate add create_my_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check migration status
sqlx migrate info

# Prepare SQLx offline mode (for Docker builds)
cargo sqlx prepare --workspace
```

### Docker Commands

```bash
# Start services
docker-compose up -d

# Stop services
docker-compose down

# Stop and remove volumes (clean slate)
docker-compose down -v

# Rebuild server image
docker-compose build bdp-server

# View logs
docker-compose logs -f [service-name]

# Execute command in container
docker-compose exec bdp-server /bin/bash

# Restart single service
docker-compose restart bdp-server
```

### Data Ingestion

#### Manual Ingestion (Example)

```bash
# Run UniProt protein ingestion
cargo run --example run_uniprot_ingestion

# Monitor progress
docker exec bdp-postgres psql -U bdp -d bdp -c \
  "SELECT status, records_processed, total_records FROM ingestion_jobs ORDER BY created_at DESC LIMIT 1;"
```

#### Parallel Processing (Multiple Workers)

```bash
# Terminal 1
cargo run --example run_uniprot_ingestion &

# Terminal 2
cargo run --example run_uniprot_ingestion &

# Terminal 3
cargo run --example run_uniprot_ingestion &

# All 3 processes will coordinate via database (SKIP LOCKED)
# and process different batches in parallel!
```

See [docs/parallel-etl-architecture.md](./docs/parallel-etl-architecture.md) for complete guide.

## Testing

### Run All Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific package
cargo test -p bdp-server

# Integration tests only
cargo test --test '*'

# Specific test
cargo test test_uniprot_parser
```

### Test Database

```bash
# Start test database (port 5433)
docker-compose --profile test up -d postgres-test

# Run tests against test database
TEST_DATABASE_URL=postgresql://bdp:bdp_test_password@localhost:5433/bdp_test cargo test
```

## Common Tasks (Just)

We use [just](https://github.com/casey/just) for common tasks:

```bash
# Show all available commands
just --list

# Docker operations
just docker-up          # Start all Docker services
just docker-down        # Stop all Docker services
just docker-build       # Rebuild server image
just docker-logs        # View logs

# Database operations
just db-migrate         # Run migrations
just db-reset           # Drop and recreate database
just db-seed            # Seed with test data

# Development
just dev                # Run server with hot reload
just test               # Run all tests
just check              # Run cargo check
just fmt                # Format code
just lint               # Run clippy

# CI/CD
just ci                 # Run all CI checks locally
```

## Project Structure

```
bdp/
├── crates/
│   ├── bdp-cli/              # CLI tool
│   ├── bdp-server/           # Backend server
│   │   ├── src/
│   │   │   ├── features/     # Feature modules (CQRS)
│   │   │   ├── ingest/       # Ingestion pipeline
│   │   │   │   ├── framework/  # Reusable ETL framework
│   │   │   │   └── uniprot/    # UniProt-specific implementation
│   │   │   ├── storage/      # S3/MinIO client
│   │   │   └── main.rs       # Server entry point
│   │   ├── examples/         # Runnable examples
│   │   └── tests/            # Integration tests
│   └── bdp-common/           # Shared code
├── migrations/               # Database migrations
├── docker/                   # Dockerfiles
├── docs/                     # Documentation
│   ├── parallel-etl-architecture.md  # ETL system guide
│   └── ...
├── web/                      # Next.js frontend (planned)
├── docker-compose.yml        # Development environment
├── .env.example              # Environment template
├── Cargo.toml                # Workspace configuration
├── justfile                  # Task runner
└── README.md                 # This file
```

## Stack & Technologies

### Backend
- **Framework**: [axum](https://github.com/tokio-rs/axum) (async web framework)
- **Database**: PostgreSQL 16 with [SQLx](https://github.com/launchbadge/sqlx) (compile-time SQL verification)
- **Storage**: MinIO / S3 ([aws-sdk-s3](https://crates.io/crates/aws-sdk-s3))
- **Architecture**: CQRS with [Mediator pattern](https://crates.io/crates/mediator)
- **Logging**: [tracing](https://crates.io/crates/tracing) (structured logging)

### CLI
- **Framework**: [clap](https://crates.io/crates/clap) (CLI parsing)
- **Cache**: SQLite with SQLx
- **HTTP Client**: [reqwest](https://crates.io/crates/reqwest)

### Infrastructure
- **Containerization**: Docker & Docker Compose
- **CI/CD**: GitHub Actions
- **Release**: [cargo-dist](https://opensource.axo.dev/cargo-dist/) (multi-platform binaries)

### Frontend (Planned)
- **Framework**: Next.js 16
- **Docs**: Nextra
- **UI**: Tailwind CSS

## Environment Variables

Key environment variables (see [.env.example](./.env.example) for complete list):

```bash
# Database
DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp

# Storage
STORAGE_TYPE=s3
STORAGE_S3_ENDPOINT=http://localhost:9000
STORAGE_S3_BUCKET=bdp-data
STORAGE_S3_ACCESS_KEY=minioadmin
STORAGE_S3_SECRET_KEY=minioadmin

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8000
RUST_LOG=info,bdp_server=debug,sqlx=warn

# Ingestion
INGEST_ENABLED=false
INGEST_WORKER_THREADS=4
INGEST_UNIPROT_BATCH_SIZE=1000
```

## Production Deployment

### Building for Production

```bash
# Build optimized binary
cargo build --release --bin bdp-server

# Binary will be at: ./target/release/bdp-server

# Or build Docker image
docker build -f docker/Dockerfile.server -t bdp-server:latest .
```

### Docker Production Deployment

```yaml
# docker-compose.prod.yml
services:
  bdp-server:
    image: bdp-server:latest
    environment:
      DATABASE_URL: postgresql://user:pass@prod-db:5432/bdp
      STORAGE_S3_ENDPOINT: https://s3.amazonaws.com
      STORAGE_S3_BUCKET: prod-bdp-data
      STORAGE_S3_ACCESS_KEY: ${AWS_ACCESS_KEY}
      STORAGE_S3_SECRET_KEY: ${AWS_SECRET_KEY}
      JWT_SECRET: ${PRODUCTION_JWT_SECRET}
      RUST_LOG: info,sqlx=warn
    ports:
      - "8000:8000"
    restart: always
```

```bash
docker-compose -f docker-compose.prod.yml up -d
```

### Environment Checklist

- [ ] Set strong `JWT_SECRET` (use `openssl rand -base64 64`)
- [ ] Configure production database URL
- [ ] Set up S3 bucket with proper IAM policies
- [ ] Enable SSL/TLS for database connections
- [ ] Set `RUST_LOG=info` (not debug)
- [ ] Configure CORS for production domains
- [ ] Set up monitoring and alerts
- [ ] Configure automated backups

## Monitoring & Observability

### Health Check

```bash
curl http://localhost:8000/health
```

### Database Queries

```sql
-- Job status
SELECT id, status, records_processed, total_records,
       ROUND(100.0 * records_processed / NULLIF(total_records, 0), 2) as progress_pct
FROM ingestion_jobs
ORDER BY created_at DESC LIMIT 10;

-- Active workers
SELECT worker_hostname, COUNT(*) as active_units, MAX(heartbeat_at) as last_heartbeat
FROM ingestion_work_units
WHERE status = 'processing'
GROUP BY worker_hostname;

-- Failed work units
SELECT id, batch_number, retry_count, last_error
FROM ingestion_work_units
WHERE status = 'failed'
ORDER BY updated_at DESC;
```

### Logs

```bash
# Docker logs
docker-compose logs -f bdp-server

# Filter by level
docker-compose logs bdp-server | grep ERROR

# Structured logging (JSON in production)
RUST_LOG=info,bdp_server=debug cargo run --bin bdp-server
```

## Troubleshooting

### Docker Build Fails

```bash
# Clean rebuild
docker-compose down -v
docker-compose build --no-cache
docker-compose up -d
```

### Database Connection Issues

```bash
# Check PostgreSQL is running
docker-compose ps postgres

# Check connection
docker exec bdp-postgres psql -U bdp -d bdp -c "SELECT 1;"

# Reset database
docker-compose down postgres
docker volume rm bdp_postgres_data
docker-compose up -d postgres
```

### MinIO Connection Issues

```bash
# Check MinIO is running
docker-compose ps minio

# Re-initialize buckets
docker-compose up -d minio-init

# Check bucket exists
docker exec bdp-minio mc ls minio/bdp-data
```

### SQLx Offline Mode Errors

```bash
# Regenerate .sqlx metadata
cargo sqlx prepare --workspace

# Commit the changes
git add .sqlx
git commit -m "chore: update sqlx offline cache"
```

### Port Already in Use

```bash
# Find process using port 8000
lsof -i :8000  # macOS/Linux
netstat -ano | findstr :8000  # Windows

# Kill process or change port in .env
SERVER_PORT=8001
```

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

**Key Requirements:**
- ✅ Use `tracing` for logging (no `println!` / `dbg!`)
- ✅ Write tests for new features
- ✅ Run `cargo fmt` and `cargo clippy`
- ✅ Update documentation
- ✅ Read [AGENTS.md](./AGENTS.md) for development context

## Documentation

- [INSTALL.md](./INSTALL.md) - Installation guide
- [CONTRIBUTING.md](./CONTRIBUTING.md) - Contribution guidelines
- [AGENTS.md](./AGENTS.md) - Development context and history
- [docs/parallel-etl-architecture.md](./docs/parallel-etl-architecture.md) - Parallel ETL system
- [docs/sqlx-guide.md](./docs/sqlx-guide.md) - SQLx offline mode guide
- [docs/database-setup.md](./docs/database-setup.md) - Database schema and migrations

## License

See [LICENSE](./LICENSE) for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/datadir-lab/bdp/issues)
- **Discussions**: [GitHub Discussions](https://github.com/datadir-lab/bdp/discussions)
- **Email**: support@datadir.dev

## Acknowledgments

Built with ❤️ using Rust, PostgreSQL, and modern cloud-native technologies.
