# Docker Setup Guide

Complete guide for running BDP in Docker for development and production.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) 20.10+
- [Docker Compose](https://docs.docker.com/compose/install/) v2.0+

Verify installation:
```bash
docker --version
docker-compose --version
```

## Quick Start (5 Minutes)

```bash
# 1. Clone repository
git clone https://github.com/datadir-lab/bdp.git
cd bdp

# 2. Copy environment file
cp .env.example .env

# 3. Start all services
docker-compose up -d

# 4. Check health
docker-compose ps
```

**Expected Output:**
```
NAME               STATUS             PORTS
bdp-postgres       Up (healthy)       0.0.0.0:5432->5432/tcp
bdp-minio          Up (healthy)       0.0.0.0:9000-9001->9000-9001/tcp
bdp-server         Up (healthy)       0.0.0.0:8000->8000/tcp
```

**Services Available:**
- API: http://localhost:8000
- MinIO Console: http://localhost:9001 (minioadmin/minioadmin)
- PostgreSQL: localhost:5432 (bdp/bdp_dev_password)

## Services Overview

### PostgreSQL (Database)
- **Image**: `postgres:16-alpine`
- **Port**: 5432
- **Credentials**: bdp / bdp_dev_password
- **Database**: bdp
- **Volume**: postgres_data (persistent)

### MinIO (S3 Storage)
- **Image**: `minio/minio:latest`
- **Ports**:
  - 9000 (S3 API)
  - 9001 (Web Console)
- **Credentials**: minioadmin / minioadmin
- **Bucket**: bdp-data (auto-created)
- **Volume**: minio_data (persistent)

### BDP Server (API)
- **Build**: Multi-stage Rust build
- **Port**: 8000
- **Depends On**: PostgreSQL + MinIO
- **Features**:
  - REST API
  - Data ingestion pipeline
  - Work unit coordination

## Step-by-Step Setup

### 1. Environment Configuration

```bash
# Copy template
cp .env.example .env

# Edit if needed (optional)
nano .env
```

**Key Variables:**
```bash
# Database
POSTGRES_DB=bdp
POSTGRES_USER=bdp
POSTGRES_PASSWORD=bdp_dev_password

# Storage
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin
MINIO_BUCKET=bdp-data

# Server
SERVER_PORT=8000
RUST_LOG=info,bdp_server=debug,sqlx=warn
INGEST_ENABLED=false
```

### 2. Start Services

#### Start Everything (Recommended)
```bash
docker-compose up -d
```

#### Start Individual Services
```bash
# Database only
docker-compose up -d postgres

# Database + Storage
docker-compose up -d postgres minio minio-init

# All services
docker-compose up -d
```

#### Watch Logs
```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f bdp-server

# Last 100 lines
docker-compose logs --tail=100 bdp-server
```

### 3. Verify Health

```bash
# Check all services
docker-compose ps

# Test API
curl http://localhost:8000/health
# Expected: {"status":"ok"}

# Test MinIO
curl http://localhost:9000/minio/health/live
# Expected: OK

# Test PostgreSQL
docker exec bdp-postgres psql -U bdp -d bdp -c "SELECT version();"
```

### 4. Access Services

#### PostgreSQL
```bash
# Connect from host
psql postgresql://bdp:bdp_dev_password@localhost:5432/bdp

# Or using Docker
docker exec -it bdp-postgres psql -U bdp -d bdp

# Run queries
docker exec bdp-postgres psql -U bdp -d bdp -c "SELECT COUNT(*) FROM ingestion_jobs;"
```

#### MinIO Console
1. Open http://localhost:9001
2. Login: minioadmin / minioadmin
3. Browse buckets: `bdp-data`

#### API Server
```bash
# Health check
curl http://localhost:8000/health

# List endpoints (if available)
curl http://localhost:8000/api/v1/health
```

## Database Migrations

Migrations run automatically on server startup. To run manually:

```bash
# Using Docker
docker-compose exec bdp-server sqlx migrate run

# Check migration status
docker-compose exec bdp-server sqlx migrate info

# Revert last migration
docker-compose exec bdp-server sqlx migrate revert
```

## Building the Server Image

### During Development
```bash
# Rebuild after code changes
docker-compose build bdp-server

# Rebuild without cache
docker-compose build --no-cache bdp-server

# Rebuild and restart
docker-compose up -d --build bdp-server
```

### For Production
```bash
# Build optimized image
docker build -f docker/Dockerfile.server -t bdp-server:latest .

# Tag for registry
docker tag bdp-server:latest your-registry.com/bdp-server:v1.0.0

# Push to registry
docker push your-registry.com/bdp-server:v1.0.0
```

## Common Operations

### Stop Services
```bash
# Stop all services (keeps volumes)
docker-compose down

# Stop and remove volumes (clean slate)
docker-compose down -v

# Stop specific service
docker-compose stop bdp-server
```

### Restart Services
```bash
# Restart all
docker-compose restart

# Restart specific service
docker-compose restart bdp-server
```

### View Resources
```bash
# Container resources
docker stats

# Volumes
docker volume ls | grep bdp

# Networks
docker network ls | grep bdp

# Images
docker images | grep bdp
```

### Clean Up
```bash
# Stop and remove everything
docker-compose down -v

# Remove dangling images
docker image prune

# Remove all unused resources
docker system prune -a
```

## Development Workflow

### Iterative Development

```bash
# Option 1: Rebuild on changes
docker-compose up -d --build bdp-server

# Option 2: Run server natively (faster)
docker-compose up -d postgres minio minio-init
cargo run --bin bdp-server
```

### Debugging

```bash
# Access container shell
docker-compose exec bdp-server /bin/bash

# Check environment variables
docker-compose exec bdp-server env | grep DATABASE

# View configuration
docker-compose config

# Inspect container
docker inspect bdp-server
```

### Logs and Monitoring

```bash
# Follow logs
docker-compose logs -f bdp-server

# Filter errors
docker-compose logs bdp-server | grep ERROR

# Last hour
docker-compose logs --since 60m bdp-server

# Export logs
docker-compose logs --no-color > logs.txt
```

## Troubleshooting

### Server Won't Start

```bash
# Check logs
docker-compose logs bdp-server

# Common issues:
# 1. Database not ready - wait for postgres health check
docker-compose ps postgres

# 2. MinIO not ready - wait for minio health check
docker-compose ps minio

# 3. Port already in use - change in .env
SERVER_PORT=8001

# 4. Migration failed - check migrations
docker-compose exec bdp-server sqlx migrate info
```

### Database Issues

```bash
# Reset database
docker-compose stop postgres
docker volume rm bdp_postgres_data
docker-compose up -d postgres

# Check connection
docker exec bdp-postgres pg_isready -U bdp -d bdp

# View database logs
docker-compose logs postgres | grep ERROR
```

### MinIO Issues

```bash
# Check MinIO is running
docker-compose ps minio

# Recreate bucket
docker-compose up -d minio-init

# Check bucket exists
docker exec bdp-minio mc ls minio/bdp-data

# View MinIO logs
docker-compose logs minio
```

### Build Failures

```bash
# Common causes:
# 1. Outdated SQLx cache
cargo sqlx prepare --workspace
git add .sqlx
git commit -m "chore: update sqlx cache"

# 2. Docker cache issues
docker-compose build --no-cache

# 3. Dependency issues
docker system prune -a
docker-compose up -d --build
```

### Network Issues

```bash
# Recreate network
docker-compose down
docker network rm bdp_bdp-network
docker-compose up -d

# Check network
docker network inspect bdp_bdp-network

# Test connectivity between containers
docker-compose exec bdp-server ping postgres
docker-compose exec bdp-server ping minio
```

## Production Deployment

### Environment Setup

```bash
# Create production .env
cat > .env.prod << EOF
# Production Database (managed service)
DATABASE_URL=postgresql://user:pass@prod-db.example.com:5432/bdp
DATABASE_MAX_CONNECTIONS=50

# Production S3
STORAGE_TYPE=s3
STORAGE_S3_ENDPOINT=https://s3.amazonaws.com
STORAGE_S3_BUCKET=prod-bdp-data
STORAGE_S3_ACCESS_KEY=\${AWS_ACCESS_KEY}
STORAGE_S3_SECRET_KEY=\${AWS_SECRET_KEY}
STORAGE_S3_PATH_STYLE=false

# Security
JWT_SECRET=\$(openssl rand -base64 64)

# Logging
RUST_LOG=info,sqlx=warn

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8000
EOF
```

### Deploy

```bash
# Use production config
docker-compose -f docker-compose.yml --env-file .env.prod up -d

# Or create docker-compose.prod.yml
docker-compose -f docker-compose.prod.yml up -d
```

### Health Checks

```bash
# API health
curl https://api.example.com/health

# Container health
docker-compose ps

# Metrics (if enabled)
curl https://api.example.com/metrics
```

## Performance Tuning

### PostgreSQL

```yaml
# docker-compose.yml
postgres:
  environment:
    POSTGRES_INITDB_ARGS: "--encoding=UTF8 --locale=en_US.UTF-8"
  command:
    - "postgres"
    - "-c"
    - "max_connections=200"
    - "-c"
    - "shared_buffers=256MB"
    - "-c"
    - "effective_cache_size=1GB"
    - "-c"
    - "maintenance_work_mem=64MB"
    - "-c"
    - "checkpoint_completion_target=0.9"
    - "-c"
    - "wal_buffers=16MB"
```

### Server

```yaml
# docker-compose.yml
bdp-server:
  environment:
    DATABASE_MAX_CONNECTIONS: 50
    INGEST_WORKER_THREADS: 8
    RUST_LOG: info,sqlx=warn
  deploy:
    resources:
      limits:
        cpus: '4'
        memory: 4G
      reservations:
        cpus: '2'
        memory: 2G
```

## Backup and Restore

### Database Backup

```bash
# Backup
docker exec bdp-postgres pg_dump -U bdp bdp > backup-$(date +%Y%m%d).sql

# Restore
docker exec -i bdp-postgres psql -U bdp -d bdp < backup-20260118.sql
```

### MinIO Backup

```bash
# Backup bucket
docker run --rm --net bdp_bdp-network \
  -v $(pwd)/backups:/backups \
  minio/mc \
  mirror minio/bdp-data /backups/bdp-data

# Restore bucket
docker run --rm --net bdp_bdp-network \
  -v $(pwd)/backups:/backups \
  minio/mc \
  mirror /backups/bdp-data minio/bdp-data
```

## Security Checklist

- [ ] Change default passwords in production
- [ ] Use strong `JWT_SECRET` (`openssl rand -base64 64`)
- [ ] Enable SSL/TLS for database connections
- [ ] Use managed database in production (not Docker)
- [ ] Use AWS S3 or managed MinIO in production
- [ ] Set `RUST_LOG=info` (not debug) in production
- [ ] Configure firewall rules (only allow necessary ports)
- [ ] Enable Docker content trust
- [ ] Regular security updates (`docker-compose pull`)
- [ ] Monitor logs for security issues

## Additional Resources

- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [PostgreSQL Docker Hub](https://hub.docker.com/_/postgres)
- [MinIO Docker Hub](https://hub.docker.com/r/minio/minio)
- [Main README](./README.md)
- [Parallel ETL Architecture](./docs/parallel-etl-architecture.md)
