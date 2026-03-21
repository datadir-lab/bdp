# BDP Infrastructure

Hetzner VPS + Dokploy + Terraform. All operations via `cargo xtask infra`.

## Stack

| Component | Details |
|-----------|---------|
| **Server** | Hetzner cx22 (2 vCPU, 4GB RAM) ~4.35€/mo |
| **Data volume** | 80GB ext4, mounted at `/mnt/data` |
| **Backups** | Hetzner Storage Box (bx11, 100GB) via restic, daily |
| **PaaS** | Dokploy (manages Traefik, app deployments) |
| **TLS** | Traefik + Let's Encrypt (persisted on `/mnt/data/dokploy`) |
| **Object storage** | MinIO on `/mnt/data/minio` (S3-compatible) |
| **DNS** | Cloudflare (automated via Terraform) |

## Quick Start

```bash
# 1. Copy and fill secrets
cp infrastructure/hetzner/environments/prod/.secrets.example \
   infrastructure/hetzner/environments/prod/.secrets
# Edit .secrets with real values

# 2. One-time bootstrap
cargo xtask infra bootstrap

# 3. Preview changes
cargo xtask infra plan

# 4. Deploy
cargo xtask infra apply

# 5. Wait for server to initialize (5-10 min first boot)
cargo xtask infra post-deploy
```

## Common Commands

```bash
cargo xtask infra ssh          # SSH into server
cargo xtask infra status       # Docker service health
cargo xtask infra logs         # Tail bdp-server logs
cargo xtask infra logs minio   # Tail minio logs
cargo xtask infra show-secrets # Show all credentials
cargo xtask infra backup-now   # Trigger immediate backup
cargo xtask infra backup-list  # List restic snapshots
cargo xtask infra update       # Pull latest images + restart
cargo xtask infra info         # Terraform outputs (IPs, URLs)
```

## Let's Encrypt Persistence

Traefik's `acme.json` lives at `/mnt/data/dokploy/traefik/acme.json`.
The data volume persists across server rebuilds (`auto_delete = false`).
To trigger a server rebuild: bump `deploy_version` in `.secrets`.

## Backups

Restic backs up `/mnt/data` daily at 3am. Retention: 7 daily, 4 weekly, 3 monthly.
Restore interactively: `cargo xtask infra restore`

## Secrets

All secrets in `infrastructure/hetzner/environments/prod/.secrets` (gitignored).
Template: `.secrets.example`.

## Windows

All `cargo xtask infra` commands require WSL on Windows (for Terraform + shell tools).
SSH commands work natively (Windows 10+ ships OpenSSH).
