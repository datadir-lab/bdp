# BDP Infrastructure

> **Status**: Managed manually until OVH startup grant approved.

## Current Setup (OVH Cloud - DE Region)

| Resource | Details |
|----------|---------|
| **Instance** | B3-8 (2 vCPU, 8GB RAM, 50GB NVMe) |
| **OS** | Ubuntu 24.04 LTS |
| **SSH Key** | `bdp-production SSH key` |
| **S3 Bucket** | `bdp-production` (DE region, SSE-OMK encrypted) |
| **Domain** | bdp.dev |

## Stack

- PostgreSQL 16 (container)
- BDP Backend (Rust)
- BDP Frontend (Next.js)
- Traefik (reverse proxy + Let's Encrypt TLS)

## Deployment

Automatic via `.github/workflows/deploy.yml` on push to main.

Manual trigger supports environment selection.

## GitHub Configuration

### Secrets (sensitive)

| Secret | Description |
|--------|-------------|
| `SERVER_IP` | OVH instance public IP |
| `DEPLOY_SSH_KEY` | Private SSH key for deployment |
| `POSTGRES_PASSWORD` | PostgreSQL password |
| `STORAGE_S3_ACCESS_KEY` | OVH S3 access key |
| `STORAGE_S3_SECRET_KEY` | OVH S3 secret key |

### Variables (config, non-sensitive)

| Variable | Description | Example |
|----------|-------------|---------|
| `DOMAIN` | Domain name | `bdp.dev` |
| `STORAGE_S3_ENDPOINT` | S3 endpoint | `https://s3.de.io.cloud.ovh.net` |
| `STORAGE_S3_REGION` | S3 region | `de` |
| `STORAGE_S3_BUCKET` | S3 bucket | `bdp-production` |
| `ACME_EMAIL` | Let's Encrypt email | `you@example.com` |
| `RUST_LOG` | Log level (optional) | `info,bdp_server=debug` |

### Setup via gh CLI

```bash
# Secrets
gh secret set SERVER_IP --env production --body "<ip>"
gh secret set DEPLOY_SSH_KEY --env production --body (Get-Content ~/.ssh/bdp-production -Raw)
gh secret set POSTGRES_PASSWORD --env production --body "<password>"
gh secret set STORAGE_S3_ACCESS_KEY --env production --body "<key>"
gh secret set STORAGE_S3_SECRET_KEY --env production --body "<secret>"

# Variables
gh variable set DOMAIN --env production --body "bdp.dev"
gh variable set STORAGE_S3_ENDPOINT --env production --body "https://s3.de.io.cloud.ovh.net"
gh variable set STORAGE_S3_REGION --env production --body "de"
gh variable set STORAGE_S3_BUCKET --env production --body "bdp-production"
gh variable set ACME_EMAIL --env production --body "you@example.com"
gh variable set RUST_LOG --env production --body "info,bdp_server=info,sqlx=warn"
```

## Application Config

App-specific settings are in `infrastructure/deploy/docker-compose.prod.yml`:

```yaml
# Ingestion - general
INGEST_ENABLED: "true"
INGEST_SCHEDULE: "0 2 * * 0"    # Weekly Sunday 2am
INGEST_WORKERS: "2"
INGEST_BATCH_SIZE: "1000"

# Ingestion - sources (enable + version)
INGEST_UNIPROT_ENABLED: "true"
INGEST_UNIPROT_VERSION: "2025_06"   # releases: 2025_06, 2026_01
INGEST_ENSEMBL_ENABLED: "true"
INGEST_ENSEMBL_VERSION: "115"       # releases: 114, 115, 116
INGEST_NCBI_ENABLED: "true"
INGEST_NCBI_VERSION: "229"          # releases: 228, 229, 230

# API
API_RATE_LIMIT: "100"
API_TIMEOUT_SECS: "30"
```

Edit docker-compose.prod.yml and push to deploy changes.

## DNS Setup

Point domain A record to server IP.

## Backups

pg_dump runs daily at 3am, keeps 7 days. Backups stored in `/opt/bdp/backups/`.
