# Quick Start

**Prerequisites:** Docker, Rust 1.75+, just

```bash
docker-compose up -d postgres minio
cp .env.example .env
just db-migrate
cargo run --bin bdp-server
# Server: http://localhost:8000
```

## Verify

```bash
curl http://localhost:8000/health
# MinIO: http://localhost:9001 (minioadmin/minioadmin)
```

## Test API

```bash
curl -X POST http://localhost:8000/api/v1/organizations -H "Content-Type: application/json" -H "x-user-id: testuser" -d '{"slug":"uniprot","name":"UniProt"}'
curl http://localhost:8000/api/v1/organizations
```

## Common Commands

```bash
just dev               # Run with hot reload
just test              # Run tests
just docker-up         # Start all services
just db-migrate        # Apply migrations
```

## Troubleshooting

**Port in use:** `lsof -i :8000`, change in .env
**DB connection:** `docker-compose restart postgres`
**SQLx errors:** `cargo sqlx prepare` or `export SQLX_OFFLINE=true`
