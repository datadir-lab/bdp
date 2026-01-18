# BDP Docker Images

This directory contains Dockerfiles for all BDP services. Each Dockerfile uses multi-stage builds for optimal image size and security.

## Available Dockerfiles

### 1. Dockerfile.server - BDP Server (Rust Backend)
**Purpose:** Main API server handling authentication, data management, and business logic.

**Build:**
```bash
docker build -f docker/Dockerfile.server -t bdp-server:latest .
```

**Run:**
```bash
docker run -d \
  --name bdp-server \
  -p 8000:8000 \
  -e DATABASE_URL=postgresql://user:pass@db:5432/bdp \
  -e RUST_LOG=info \
  bdp-server:latest
```

**Features:**
- Multi-stage build with Rust 1.75
- Runs as non-root user (uid 1000)
- Health check on `/health` endpoint
- Minimal debian:bookworm-slim runtime
- Port 8000 exposed

### 2. Dockerfile.cli - BDP CLI Tool
**Purpose:** Command-line interface for interacting with BDP services.

**Build:**
```bash
docker build -f docker/Dockerfile.cli -t bdp-cli:latest .
```

**Run:**
```bash
docker run --rm bdp-cli:latest --help

# Example: Run a specific command
docker run --rm \
  -v $(pwd)/data:/app/data \
  bdp-cli:latest query --format json
```

**Features:**
- Lightweight CLI tool
- Non-root user execution
- Volume mount support for data access

### 3. Dockerfile.ingest - BDP Ingest Service
**Purpose:** Data ingestion service with scheduled (cron) and on-demand execution.

**Build:**
```bash
docker build -f docker/Dockerfile.ingest -t bdp-ingest:latest .
```

**Run (Cron Mode):**
```bash
docker run -d \
  --name bdp-ingest \
  -e BDP_API_URL=http://bdp-server:8000 \
  -e RUST_LOG=info \
  bdp-ingest:latest cron
```

**Run (One-time Execution):**
```bash
docker run --rm \
  -e BDP_API_URL=http://bdp-server:8000 \
  bdp-ingest:latest once --source github
```

**Features:**
- Includes cron for scheduled ingestion
- Default: runs hourly (configurable via crontab)
- Can run in cron mode or one-time mode
- Logs to `/var/log/cron/ingest.log`

**Execution Modes:**
- `cron` - Run as daemon with scheduled execution (default)
- `once` - Execute ingestion once and exit
- Direct command - Pass through to bdp-ingest binary

### 4. Dockerfile.web - BDP Web (Next.js Frontend)
**Purpose:** Next.js web application providing the user interface.

**Build:**
```bash
docker build -f docker/Dockerfile.web -t bdp-web:latest .
```

**Run:**
```bash
docker run -d \
  --name bdp-web \
  -p 3000:3000 \
  -e NEXT_PUBLIC_API_URL=http://localhost:8000 \
  bdp-web:latest
```

**Features:**
- Node.js 20 Alpine for minimal size
- Multi-stage: deps → builder → runner
- Standalone output mode for optimal Docker builds
- Non-root user (nextjs:nodejs)
- Health check included
- Port 3000 exposed

## Docker Compose

For orchestrated deployment of all services, use the main `docker-compose.yml`:

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down
```

## Build Optimization

All Dockerfiles implement these optimizations:

1. **Multi-stage builds** - Separate build and runtime stages
2. **Layer caching** - Dependencies copied before source code
3. **Minimal base images** - debian:bookworm-slim or alpine
4. **Non-root users** - Security best practice
5. **Health checks** - Container health monitoring
6. **.dockerignore** - Exclude unnecessary files from build context

## Image Sizes (Approximate)

- `bdp-server`: ~150MB
- `bdp-cli`: ~145MB
- `bdp-ingest`: ~160MB (includes cron)
- `bdp-web`: ~180MB (Node.js + Next.js)

## Security

All images:
- Run as non-root users
- Use official base images
- Include only necessary runtime dependencies
- Have CA certificates for HTTPS
- Follow least privilege principle

## Environment Variables

### Server
- `DATABASE_URL` - PostgreSQL connection string
- `RUST_LOG` - Logging level (debug, info, warn, error)
- `BDP_HOST` - Bind address (default: 0.0.0.0)
- `BDP_PORT` - Port to listen on (default: 8000)

### Web
- `NEXT_PUBLIC_API_URL` - Backend API URL
- `PORT` - Next.js port (default: 3000)
- `NODE_ENV` - Environment (production)

### Ingest
- `BDP_API_URL` - BDP server URL
- `RUST_LOG` - Logging level
- `BDP_INGEST_MODE` - Execution mode (scheduled, manual)

## Development

For local development with hot-reload, use docker-compose.dev.yml or run services directly without Docker.

## Troubleshooting

### Build fails with "cannot find package"
Ensure you're building from the repository root:
```bash
docker build -f docker/Dockerfile.server -t bdp-server:latest .
```

### Permission denied in container
Check that volumes are mounted with correct permissions:
```bash
docker run -v $(pwd)/data:/app/data:rw ...
```

### Health check failing
Verify the service is accessible:
```bash
docker exec bdp-server curl -f http://localhost:8000/health
```

## CI/CD

These Dockerfiles are designed for CI/CD pipelines:

```yaml
# Example GitHub Actions snippet
- name: Build Docker image
  run: docker build -f docker/Dockerfile.server -t bdp-server:${{ github.sha }} .

- name: Push to registry
  run: docker push bdp-server:${{ github.sha }}
```

## Production Considerations

1. **Use specific version tags** instead of `latest`
2. **Configure resource limits** in docker-compose or Kubernetes
3. **Set up proper logging** with log aggregation
4. **Use secrets management** for sensitive environment variables
5. **Implement monitoring** with Prometheus/Grafana
6. **Configure backup strategies** for persistent data

## License

See LICENSE file in repository root.
