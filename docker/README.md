# Docker

Multi-stage Dockerfiles with optimized builds, non-root users, health checks.

## Images

**server:** API (Rust, port 8000, ~150MB)
**cli:** CLI tool (~145MB)
**ingest:** ETL (cron support, ~160MB)
**web:** Next.js (port 3000, ~180MB)

```bash
docker build -f docker/Dockerfile.server -t bdp-server .
docker run -d -p 8000:8000 -e DATABASE_URL=... bdp-server
docker-compose up -d
```

## Env Vars

**Server:** DATABASE_URL, RUST_LOG, BDP_HOST, BDP_PORT
**Web:** NEXT_PUBLIC_API_URL, PORT
**Ingest:** BDP_API_URL, RUST_LOG

## Production

Use version tags, set resource limits, secrets management, monitoring, backups
