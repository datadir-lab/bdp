# Hetzner Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the OVH/manual deployment setup with fully automated Hetzner VPS infrastructure — Terraform-provisioned, Dokploy-managed, Traefik with persisted Let's Encrypt certs, restic backups to Hetzner Storage Box, and `cargo xtask infra` commands for everything.

**Architecture:** A single Hetzner VPS runs Dokploy (PaaS layer with bundled Traefik). A persistent data volume (`/mnt/data`) is attached to the server and holds all stateful data including Dokploy's config dir (symlinked so certs survive server rebuilds). Restic backs up the volume daily to a Hetzner Storage Box with automatic pruning. Terraform manages all cloud resources; cloud-init bootstraps the server on first boot; xtask provides a single ergonomic CLI for all ops.

**Tech Stack:** Terraform (hcloud + cloudflare + tls + random providers), cloud-init (Ubuntu 24.04), Dokploy, Traefik v3 (bundled with Dokploy), restic, Hetzner Storage Box, Rust xtask (clap), MinIO (S3-compatible object storage on the data volume, replaces OVH S3).

---

## Reference: temnir patterns to reuse

Read these before implementing — they contain the exact patterns to copy:
- `../temnir/infrastructure/infrastructure/providers/hetzner/terraform/main.tf` — primary IP, volume, storage box, cloud-init gzip, deploy_version trigger
- `../temnir/infrastructure/infrastructure/shared/cloud-init/parts/volume-mount.sh` — volume mount with safety checks (copy verbatim)
- `../temnir/infrastructure/infrastructure/shared/cloud-init/parts/backup-restic.sh` — restic backup + prune (copy verbatim)
- `../temnir/infrastructure/infrastructure/shared/cloud-init/prod.yaml` — Dokploy setup pattern
- `../temnir/infrastructure/infrastructure/providers/hetzner/Taskfile.yml` — task naming conventions

---

## File Map

### New files
| Path | Purpose |
|------|---------|
| `infrastructure/hetzner/terraform/main.tf` | VPS, primary IP, data volume, storage box, firewall, cloud-init |
| `infrastructure/hetzner/terraform/variables.tf` | All Terraform input variables |
| `infrastructure/hetzner/terraform/outputs.tf` | server_ipv4, volume_id, etc. |
| `infrastructure/hetzner/terraform/cloudflare.tf` | DNS A record for bdp.dev |
| `infrastructure/hetzner/environments/prod/.secrets.example` | Template for secrets file |
| `infrastructure/hetzner/environments/prod/config.yaml` | Non-secret environment config |
| `infrastructure/shared/cloud-init/prod.yaml` | Full cloud-init: Docker, Dokploy, volume, ufw, restic |
| `infrastructure/shared/cloud-init/parts/volume-mount.sh` | Copied from temnir |
| `infrastructure/shared/cloud-init/parts/backup-restic.sh` | Adapted from temnir |
| `infrastructure/shared/cloud-init/parts/ufw-base.sh` | UFW firewall rules |
| `infrastructure/shared/services/dokploy/setup.sh` | Dokploy admin user bootstrap |
| `infrastructure/hetzner/scripts/bootstrap.sh` | One-time: S3 state bucket + SSH keygen |
| `infrastructure/hetzner/scripts/bootstrap.sh` (only) | `load-env.sh` is NOT needed — `xtask infra` handles env loading inline via `load_env_preamble()` |

### Modified files
| Path | Change |
|------|--------|
| `infrastructure/deploy/docker-compose.prod.yml` | Remove standalone traefik service; move letsencrypt to bind mount; add minio service |
| `xtask/src/infra.rs` | Full rewrite — all new commands |
| `infrastructure/README.md` | Update to reflect Hetzner + Dokploy |

---

## Task 1: Terraform — variables and outputs

**Files:**
- Create: `infrastructure/hetzner/terraform/variables.tf`
- Create: `infrastructure/hetzner/terraform/outputs.tf`

- [ ] Create `infrastructure/hetzner/terraform/variables.tf`:

```hcl
variable "hcloud_token" {
  description = "Hetzner Cloud API token"
  type        = string
  sensitive   = true
}

variable "project_name" {
  description = "Resource name prefix (e.g. bdp-prod)"
  type        = string
  default     = "bdp-prod"
}

variable "server_type" {
  description = "Hetzner server type"
  type        = string
  default     = "cx22" # 2 vCPU, 4GB RAM, ~4.35€/mo
}

variable "server_image" {
  description = "Server OS image"
  type        = string
  default     = "ubuntu-24.04"
}

variable "location" {
  description = "Hetzner datacenter location"
  type        = string
  default     = "nbg1" # Nuremberg — cheapest EU
}

variable "volume_size" {
  description = "Data volume size in GB"
  type        = number
  default     = 80
}

variable "ssh_public_key" {
  description = "SSH public key for server access"
  type        = string
}

variable "ssh_allowed_ips" {
  description = "IPs allowed to SSH and access Dokploy UI (port 3000)"
  type        = list(string)
  default     = ["0.0.0.0/0", "::/0"]
}

variable "domain" {
  description = "Root domain (e.g. bdp.dev)"
  type        = string
}

variable "acme_email" {
  description = "Email for Let's Encrypt registration"
  type        = string
}

variable "cloudflare_api_token" {
  description = "Cloudflare API token for DNS management (leave empty to skip DNS)"
  type        = string
  default     = ""
  sensitive   = true
}

variable "create_dns_record" {
  description = "Create A record pointing domain to server IP"
  type        = bool
  default     = true
}

variable "deploy_version" {
  description = "Bump to trigger server rebuild (volume persists)"
  type        = string
  default     = "1"
}

variable "dokploy_admin_password" {
  description = "Dokploy admin panel password"
  type        = string
  sensitive   = true
}

variable "storage_box_type" {
  description = "Hetzner Storage Box type (bx11=100GB ~3.81€/mo)"
  type        = string
  default     = "bx11"
}

variable "storage_box_location" {
  description = "Location for Storage Box"
  type        = string
  default     = "nbg1"
}

variable "restic_password" {
  description = "Restic encryption passphrase — generate: openssl rand -hex 32"
  type        = string
  sensitive   = true
}

variable "minio_root_user" {
  description = "MinIO root username"
  type        = string
  default     = "bdpadmin"
}

variable "minio_root_password" {
  description = "MinIO root password"
  type        = string
  sensitive   = true
}
```

- [ ] Create `infrastructure/hetzner/terraform/outputs.tf`:

```hcl
output "server_ipv4" {
  description = "Server public IPv4"
  value       = hcloud_primary_ip.main_ipv4.ip_address
}

output "server_id" {
  description = "Hetzner server ID"
  value       = hcloud_server.main.id
}

output "volume_id" {
  description = "Data volume ID"
  value       = hcloud_volume.data.id
}

output "storage_box_host" {
  description = "Restic backup host"
  value       = hcloud_storage_box.backup.server
}

output "storage_box_user" {
  description = "Restic backup username"
  value       = hcloud_storage_box.backup.username
}

output "dokploy_url" {
  description = "Dokploy management UI"
  value       = "https://dokploy.${var.domain}"
}

output "app_url" {
  description = "BDP application URL"
  value       = "https://${var.domain}"
}

output "ssh_command" {
  description = "SSH command to connect to server"
  value       = "ssh root@${hcloud_primary_ip.main_ipv4.ip_address}"
}
```

- [ ] Commit:
```bash
git add infrastructure/hetzner/terraform/variables.tf infrastructure/hetzner/terraform/outputs.tf
git commit -m "feat(infra): add hetzner terraform variables and outputs"
```

---

## Task 2: Terraform — main.tf

**Files:**
- Create: `infrastructure/hetzner/terraform/main.tf`

This is the core. It follows temnir's pattern exactly: persistent primary IP, data volume with `prevent_destroy`, storage box for restic, firewall, cloud-init via gzip, `deploy_version` rebuild trigger.

- [ ] Create `infrastructure/hetzner/terraform/main.tf`:

```hcl
terraform {
  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.60"
    }
    cloudinit = {
      source  = "hashicorp/cloudinit"
      version = "~> 2.3"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
    tls = {
      source  = "hashicorp/tls"
      version = "~> 4.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
  required_version = ">= 1.0"
  # Backend: local state by default.
  # For S3 state (recommended for teams), create backend.tf:
  #   terraform { backend "s3" { ... } }
}

provider "hcloud" {
  token = var.hcloud_token
}

# SSH key
resource "hcloud_ssh_key" "default" {
  name       = "${var.project_name}-ssh-key"
  public_key = var.ssh_public_key
  lifecycle {
    ignore_changes = [name]
  }
}

# Primary IPv4 — persists independently of server (survives rebuilds)
resource "hcloud_primary_ip" "main_ipv4" {
  name          = "${var.project_name}-main-ipv4"
  type          = "ipv4"
  location      = var.location
  assignee_type = "server"
  auto_delete   = false
  labels        = { project = var.project_name }
}

# Firewall
resource "hcloud_firewall" "main" {
  name = "${var.project_name}-firewall"

  # SSH
  rule {
    direction  = "in"
    protocol   = "tcp"
    port       = "22"
    source_ips = var.ssh_allowed_ips
  }

  # Dokploy UI (restrict to your IP in .secrets for security)
  rule {
    direction  = "in"
    protocol   = "tcp"
    port       = "3000"
    source_ips = var.ssh_allowed_ips
  }

  # HTTPS (Traefik — all services)
  rule {
    direction  = "in"
    protocol   = "tcp"
    port       = "443"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  # HTTP (Let's Encrypt ACME challenge)
  rule {
    direction  = "in"
    protocol   = "tcp"
    port       = "80"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  labels = { project = var.project_name }
}

# Restic SSH key (dedicated for Storage Box auth)
resource "tls_private_key" "backup" {
  algorithm = "ED25519"
}

resource "random_password" "storage_box" {
  length           = 24
  special          = true
  min_upper        = 2
  min_lower        = 2
  min_numeric      = 2
  min_special      = 2
  override_special = "!@#$%"
}

# Hetzner Storage Box for restic backups
resource "hcloud_storage_box" "backup" {
  name             = "${var.project_name}-backup"
  location         = var.storage_box_location
  storage_box_type = var.storage_box_type
  password         = random_password.storage_box.result
  ssh_keys         = [tls_private_key.backup.public_key_openssh]
  labels           = { project = var.project_name, purpose = "backup" }
  # Note: SSH access is enabled by the presence of ssh_keys — no access_settings block needed.

  lifecycle {
    ignore_changes = [ssh_keys]
  }
}

# Data volume — ALL persistent state lives here (/mnt/data)
# prevent_destroy: volume survives terraform destroy (must remove manually)
resource "hcloud_volume" "data" {
  name              = "${var.project_name}-data"
  size              = var.volume_size
  location          = var.location
  format            = "ext4"
  delete_protection = false

  lifecycle {
    prevent_destroy = true
  }

  labels = { project = var.project_name }
}

# Cloud-init variables
locals {
  cloud_init_vars = {
    volume_device          = "/dev/disk/by-id/scsi-0HC_Volume_${hcloud_volume.data.id}"
    domain                 = var.domain
    acme_email             = var.acme_email
    dokploy_admin_password = var.dokploy_admin_password
    minio_root_user        = var.minio_root_user
    minio_root_password    = var.minio_root_password

    # Restic backup credentials
    storage_box_user   = hcloud_storage_box.backup.username
    storage_box_host   = hcloud_storage_box.backup.server
    backup_ssh_key_b64 = base64encode(tls_private_key.backup.private_key_openssh)
    restic_password    = var.restic_password

    # Shared scripts embedded as base64 (Hetzner 32KB user_data limit — must gzip)
    volume_mount_sh_b64  = base64encode(file("${path.root}/../../shared/cloud-init/parts/volume-mount.sh"))
    ufw_base_sh_b64      = base64encode(file("${path.root}/../../shared/cloud-init/parts/ufw-base.sh"))
    backup_restic_sh_b64 = base64encode(file("${path.root}/../../shared/cloud-init/parts/backup-restic.sh"))
    dokploy_setup_sh_b64 = base64encode(file("${path.root}/../../shared/services/dokploy/setup.sh"))
  }
}

# Cloud-init with gzip compression (Hetzner 32KB user_data limit)
data "cloudinit_config" "server" {
  gzip          = true
  base64_encode = true

  part {
    content_type = "text/cloud-config"
    content      = templatefile("${path.root}/../../shared/cloud-init/prod.yaml", local.cloud_init_vars)
  }
}

# Rebuild trigger — server replaces ONLY when deploy_version is bumped
resource "terraform_data" "deploy_trigger" {
  input = var.deploy_version
}

# Main server
resource "hcloud_server" "main" {
  name         = var.project_name
  server_type  = var.server_type
  image        = var.server_image
  location     = var.location
  ssh_keys     = [hcloud_ssh_key.default.id]
  firewall_ids = [hcloud_firewall.main.id]
  backups      = false # We use restic, not Hetzner backups

  public_net {
    ipv4_enabled = true
    ipv4         = hcloud_primary_ip.main_ipv4.id
    ipv6_enabled = true
  }

  user_data = data.cloudinit_config.server.rendered

  labels = {
    project        = var.project_name
    deploy_version = var.deploy_version
  }

  lifecycle {
    replace_triggered_by = [terraform_data.deploy_trigger]
    ignore_changes        = [user_data, image]
  }
}

# Attach data volume to server
resource "hcloud_volume_attachment" "main" {
  volume_id = hcloud_volume.data.id
  server_id = hcloud_server.main.id
  automount = false # cloud-init handles mounting
}
```

- [ ] Commit:
```bash
git add infrastructure/hetzner/terraform/main.tf
git commit -m "feat(infra): add hetzner terraform main configuration"
```

---

## Task 3: Terraform — Cloudflare DNS

**Files:**
- Create: `infrastructure/hetzner/terraform/cloudflare.tf`

- [ ] Create `infrastructure/hetzner/terraform/cloudflare.tf`:

```hcl
provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

# Look up the zone ID from the domain name.
# NOTE: cloudflare provider v4 uses `cloudflare_zone` (singular), NOT `cloudflare_zones`.
data "cloudflare_zone" "domain" {
  count = var.create_dns_record && var.cloudflare_api_token != "" ? 1 : 0
  name  = var.domain
}

# A record: bdp.dev → server IP
resource "cloudflare_record" "apex" {
  count   = var.create_dns_record && var.cloudflare_api_token != "" ? 1 : 0
  zone_id = data.cloudflare_zone.domain[0].id
  name    = "@"
  type    = "A"
  content = hcloud_primary_ip.main_ipv4.ip_address
  ttl     = 300
  proxied = false # Direct — Traefik handles TLS
}

# A record: *.bdp.dev → server IP (for dokploy.bdp.dev etc.)
resource "cloudflare_record" "wildcard" {
  count   = var.create_dns_record && var.cloudflare_api_token != "" ? 1 : 0
  zone_id = data.cloudflare_zone.domain[0].id
  name    = "*"
  type    = "A"
  content = hcloud_primary_ip.main_ipv4.ip_address
  ttl     = 300
  proxied = false
}
```

- [ ] Commit:
```bash
git add infrastructure/hetzner/terraform/cloudflare.tf
git commit -m "feat(infra): add cloudflare DNS records for bdp.dev"
```

---

## Task 4: Environment config and secrets template

**Files:**
- Create: `infrastructure/hetzner/environments/prod/config.yaml`
- Create: `infrastructure/hetzner/environments/prod/.secrets.example`

- [ ] Create `infrastructure/hetzner/environments/prod/config.yaml`:

```yaml
# Non-secret configuration for BDP production environment.
# Committed to version control.
project_name: bdp-prod
server_type: cx22        # 2 vCPU, 4GB RAM, ~4.35€/mo. Upgrade to cx32 if needed.
location: nbg1           # Nuremberg, Germany
volume_size: 80          # GB — stores Dokploy data, PostgreSQL, MinIO, backups
storage_box_type: bx11   # 100GB Storage Box for restic backups, ~3.81€/mo
storage_box_location: nbg1
domain: bdp.dev
deploy_version: "1"      # Bump to trigger server rebuild (volume persists)
create_dns_record: true
```

- [ ] Create `infrastructure/hetzner/environments/prod/.secrets.example`:

```bash
# Copy to .secrets and fill in real values.
# NEVER commit .secrets to version control.

# Hetzner Cloud API token (read+write)
# Create at: https://console.hetzner.cloud → Project → Security → API Tokens
TF_VAR_hcloud_token=

# SSH public key for server access
# Generate: ssh-keygen -t ed25519 -C "bdp-prod" -f ~/.ssh/bdp_prod_ed25519
TF_VAR_ssh_public_key=

# SSH key path (used by xtask for SSH/SCP commands — not passed to Terraform)
SSH_KEY_PATH=~/.ssh/bdp_prod_ed25519

# Cloudflare API token (Zone:DNS:Edit permission for bdp.dev)
# Create at: https://dash.cloudflare.com → Profile → API Tokens
# Leave empty to skip DNS automation (set records manually)
TF_VAR_cloudflare_api_token=

# Dokploy admin password (used for initial setup)
# Generate: openssl rand -base64 24
TF_VAR_dokploy_admin_password=

# MinIO root credentials
TF_VAR_minio_root_user=bdpadmin
TF_VAR_minio_root_password=

# Restic encryption passphrase — KEEP THIS SAFE, losing it = losing backups
# Generate: openssl rand -hex 32
TF_VAR_restic_password=

# Let's Encrypt email for certificate notifications
TF_VAR_acme_email=sebastian.stupak@pm.me

# Admin email for Dokploy login
DOKPLOY_ADMIN_EMAIL=sebastian.stupak@pm.me

# App environment variables (used in docker-compose deployed via Dokploy)
POSTGRES_PASSWORD=
PUBLIC_URL=https://bdp.dev
INGEST_ENABLED=true
```

- [ ] Commit:
```bash
git add infrastructure/hetzner/environments/prod/config.yaml infrastructure/hetzner/environments/prod/.secrets.example
git commit -m "feat(infra): add prod environment config and secrets template"
```

---

## Task 5: Shared shell scripts

**Files:**
- Create: `infrastructure/shared/cloud-init/parts/volume-mount.sh` (copy from temnir verbatim)
- Create: `infrastructure/shared/cloud-init/parts/backup-restic.sh` (copy from temnir verbatim)
- Create: `infrastructure/shared/cloud-init/parts/ufw-base.sh`

- [ ] Copy `volume-mount.sh` verbatim from `../temnir/infrastructure/infrastructure/shared/cloud-init/parts/volume-mount.sh`

- [ ] Before copying `backup-restic.sh`, verify the temnir script reads `RESTIC_REPOSITORY` and `RESTIC_PASSWORD` from environment variables (not hardcoded paths). Open `../temnir/infrastructure/infrastructure/shared/cloud-init/parts/backup-restic.sh` and confirm the lines `export RESTIC_REPOSITORY RESTIC_PASSWORD` and `restic backup "$MOUNT_POINT"` are present. If so, copy verbatim.

- [ ] Copy `backup-restic.sh` from `../temnir/infrastructure/infrastructure/shared/cloud-init/parts/backup-restic.sh` (after verifying above)

- [ ] Create `infrastructure/shared/cloud-init/parts/ufw-base.sh`:

```bash
#!/bin/bash
# UFW firewall baseline for BDP production server.
# Allows: SSH (22), HTTP (80), HTTPS (443), Dokploy UI (3000 — restrict after setup)
set -euo pipefail

echo "=== UFW Firewall Setup ==="

ufw --force reset
ufw default deny incoming
ufw default allow outgoing

ufw allow 22/tcp   comment "SSH"
ufw allow 80/tcp   comment "HTTP (ACME challenge)"
ufw allow 443/tcp  comment "HTTPS (Traefik)"
ufw allow 3000/tcp comment "Dokploy UI (restrict to your IP post-setup)"

ufw --force enable
echo "  UFW status:"
ufw status verbose
echo "=== UFW setup complete ==="
```

- [ ] Commit:
```bash
git add infrastructure/shared/cloud-init/parts/
git commit -m "feat(infra): add shared cloud-init scripts (volume-mount, restic, ufw)"
```

---

## Task 6: Dokploy admin setup script

**Files:**
- Create: `infrastructure/shared/services/dokploy/setup.sh`

This script is deployed to the server via cloud-init and creates the initial admin user in Dokploy's database. Pattern copied from temnir prod.yaml.

- [ ] Create `infrastructure/shared/services/dokploy/setup.sh`:

```bash
#!/bin/bash
# Dokploy admin user bootstrap.
# Run once after first boot, sourcing credentials from /mnt/data/.secrets/env
set -euo pipefail

SECRETS_FILE="/mnt/data/.secrets/env"
[ -f "$SECRETS_FILE" ] && source "$SECRETS_FILE"

DOMAIN="${DOMAIN:-bdp.dev}"
ADMIN_EMAIL="${DOKPLOY_ADMIN_EMAIL:-admin@${DOMAIN}}"
ADMIN_PASSWORD="${DOKPLOY_ADMIN_PASSWORD:?DOKPLOY_ADMIN_PASSWORD is required}"

echo "=== Dokploy Admin Setup ==="
echo "Domain:      $DOMAIN"
echo "Admin email: $ADMIN_EMAIL"

# Wait for Dokploy to be ready (up to 5 minutes)
echo "Waiting for Dokploy..."
for i in $(seq 1 60); do
  if curl -sf http://localhost:3000 >/dev/null 2>&1; then
    echo "  Dokploy ready after ${i}x5s"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: Dokploy not ready after 5 minutes"
    exit 1
  fi
  sleep 5
done

# Check if admin already exists
USER_COUNT=$(docker exec "$(docker ps -q --filter 'name=dokploy-postgres')" \
  psql -U dokploy -d dokploy -t -c 'SELECT COUNT(*) FROM "user";' 2>/dev/null | tr -d ' ' || echo "0")

if [ "$USER_COUNT" = "0" ]; then
  echo "  Creating admin user..."

  PASSWORD_HASH=$(docker run --rm -w /tmp \
    -e PASSWORD="$ADMIN_PASSWORD" \
    node:lts-alpine sh -c "
      npm install bcryptjs >/dev/null 2>&1 && \
      node -e \"
        const bcrypt = require('bcryptjs');
        console.log(bcrypt.hashSync(process.env.PASSWORD, 10));
      \"
    ")

  USER_ID=$(cat /proc/sys/kernel/random/uuid)
  ACCOUNT_ID=$(cat /proc/sys/kernel/random/uuid)

  docker exec "$(docker ps -q --filter 'name=dokploy-postgres')" \
    psql -U dokploy -d dokploy -c "
      INSERT INTO \"user\" (id, email, email_verified, role, \"createdAt\", \"isRegistered\", \"expirationDate\", updated_at)
      VALUES ('$USER_ID', '$ADMIN_EMAIL', true, 'admin', NOW(), true, '', NOW())
      ON CONFLICT (email) DO NOTHING;

      INSERT INTO \"account\" (id, account_id, provider_id, user_id, password, created_at, updated_at)
      VALUES ('$ACCOUNT_ID', '$USER_ID', 'credential', '$USER_ID', '$PASSWORD_HASH', NOW(), NOW())
      ON CONFLICT DO NOTHING;
    " && echo "  Admin created: $ADMIN_EMAIL" || echo "  Admin already exists"
else
  echo "  Admin user already exists (count: $USER_COUNT)"
fi

echo "=== Dokploy setup complete ==="
echo ""
echo "  URL:      https://dokploy.$DOMAIN"
echo "  Email:    $ADMIN_EMAIL"
echo "  Password: (from .secrets)"
```

- [ ] Commit:
```bash
git add infrastructure/shared/services/dokploy/setup.sh
git commit -m "feat(infra): add dokploy admin bootstrap script"
```

---

## Task 7: Cloud-init prod.yaml

**Files:**
- Create: `infrastructure/shared/cloud-init/prod.yaml`

This is the main cloud-init. Key design: before Dokploy installs, create `/mnt/data/dokploy` and symlink `/etc/dokploy` → `/mnt/data/dokploy`. This means all Dokploy data (Traefik certs, app configs, postgres data) survives server rebuilds.

- [ ] Create `infrastructure/shared/cloud-init/prod.yaml`:

```yaml
#cloud-config
# BDP Production — Dokploy on Hetzner
# Stack: Docker + Dokploy (Traefik + PostgreSQL + Redis bundled) + MinIO + restic backups
# All persistent state on /mnt/data (attached Hetzner volume)

package_update: true
package_upgrade: true

packages:
  - curl
  - jq
  - openssl
  - htop
  - ufw
  - fail2ban
  - restic

write_files:

  # =========================================================================
  # Shared bootstrap scripts
  # =========================================================================
  - path: /opt/bdp/scripts/volume-mount.sh
    encoding: b64
    permissions: '0755'
    content: ${volume_mount_sh_b64}

  - path: /opt/bdp/scripts/ufw-setup.sh
    encoding: b64
    permissions: '0755'
    content: ${ufw_base_sh_b64}

  - path: /opt/bdp/scripts/backup-restic.sh
    encoding: b64
    permissions: '0755'
    content: ${backup_restic_sh_b64}

  - path: /opt/bdp/scripts/dokploy-setup.sh
    encoding: b64
    permissions: '0755'
    content: ${dokploy_setup_sh_b64}

  # =========================================================================
  # Show credentials helper
  # =========================================================================
  - path: /opt/bdp/scripts/show-secrets.sh
    permissions: '0755'
    content: |
      #!/bin/bash
      SECRETS="/mnt/data/.secrets/env"
      [ ! -f "$SECRETS" ] && echo "Secrets not found at $SECRETS" && exit 1
      source "$SECRETS"
      echo "============================================"
      echo "  BDP PRODUCTION CREDENTIALS"
      echo "============================================"
      echo ""
      echo "  Dokploy UI:  https://dokploy.${domain}"
      echo "  Email:       $DOKPLOY_ADMIN_EMAIL"
      echo "  Password:    $DOKPLOY_ADMIN_PASSWORD"
      echo ""
      echo "  App URL:     https://${domain}"
      echo "  MinIO:       https://minio.${domain}"
      echo "  MinIO user:  $MINIO_ROOT_USER"
      echo "  MinIO pass:  $MINIO_ROOT_PASSWORD"
      echo "============================================"

runcmd:
  # ---------------------------------------------------------------------------
  # 1. Mount data volume
  # ---------------------------------------------------------------------------
  - VOLUME_DEVICE="${volume_device}" /opt/bdp/scripts/volume-mount.sh

  # ---------------------------------------------------------------------------
  # 2. Write secrets to volume (persisted, survives rebuilds)
  # IMPORTANT: use tee + printf, NOT a heredoc with YAML indentation.
  # YAML literal blocks preserve indentation — a heredoc written with 4-space
  # indent produces lines like "    DOMAIN=..." which break `source` in strict mode.
  # ---------------------------------------------------------------------------
  - mkdir -p /mnt/data/.secrets
  - chmod 700 /mnt/data/.secrets
  - |
    printf '%s\n' \
      'DOMAIN=${domain}' \
      'ACME_EMAIL=${acme_email}' \
      'DOKPLOY_ADMIN_EMAIL=admin@${domain}' \
      'DOKPLOY_ADMIN_PASSWORD=${dokploy_admin_password}' \
      'MINIO_ROOT_USER=${minio_root_user}' \
      'MINIO_ROOT_PASSWORD=${minio_root_password}' \
      'STORAGE_BOX_USER=${storage_box_user}' \
      'STORAGE_BOX_HOST=${storage_box_host}' \
      'RESTIC_REPOSITORY=sftp:${storage_box_user}@${storage_box_host}:/bdp-backup' \
      'RESTIC_PASSWORD=${restic_password}' \
      > /mnt/data/.secrets/env
  - chmod 600 /mnt/data/.secrets/env

  # ---------------------------------------------------------------------------
  # 3. Write restic SSH key to volume
  # ---------------------------------------------------------------------------
  - mkdir -p /mnt/data/.secrets
  - echo "${backup_ssh_key_b64}" | base64 -d > /mnt/data/.secrets/backup_ssh_key
  - chmod 600 /mnt/data/.secrets/backup_ssh_key

  # ---------------------------------------------------------------------------
  # 4. UFW firewall
  # ---------------------------------------------------------------------------
  - /opt/bdp/scripts/ufw-setup.sh

  # ---------------------------------------------------------------------------
  # 5. Install Docker
  # ---------------------------------------------------------------------------
  - curl -fsSL https://get.docker.com | sh
  - systemctl enable docker
  - systemctl start docker

  # ---------------------------------------------------------------------------
  # 6. Pre-create Dokploy directory on volume BEFORE installing Dokploy
  #    This ensures /etc/dokploy symlinks to the volume so certs persist.
  # ---------------------------------------------------------------------------
  - mkdir -p /mnt/data/dokploy
  - ln -sfn /mnt/data/dokploy /etc/dokploy

  # ---------------------------------------------------------------------------
  # 7. Install Dokploy
  # ---------------------------------------------------------------------------
  - curl -sSL https://dokploy.com/install.sh | sh

  # ---------------------------------------------------------------------------
  # 8. Create Dokploy admin user
  # ---------------------------------------------------------------------------
  - /opt/bdp/scripts/dokploy-setup.sh

  # ---------------------------------------------------------------------------
  # 9. Restic backup cron — daily at 3am, prune old backups automatically
  # ---------------------------------------------------------------------------
  - echo "0 3 * * * root MOUNT_POINT=/mnt/data /opt/bdp/scripts/backup-restic.sh >> /var/log/restic-backup.log 2>&1" > /etc/cron.d/restic-backup
  - chmod 644 /etc/cron.d/restic-backup

  # ---------------------------------------------------------------------------
  # 10. Run initial backup
  # ---------------------------------------------------------------------------
  - MOUNT_POINT=/mnt/data /opt/bdp/scripts/backup-restic.sh || true

  # ---------------------------------------------------------------------------
  # 11. Sentinel — marks cloud-init as complete
  # ---------------------------------------------------------------------------
  - touch /mnt/data/.initialized
  - echo "BDP cloud-init complete: $(date)" >> /mnt/data/.initialized
```

- [ ] Commit:
```bash
git add infrastructure/shared/cloud-init/prod.yaml
git commit -m "feat(infra): add cloud-init for BDP production server"
```

---

## Task 8: Update docker-compose.prod.yml for Dokploy

**Files:**
- Modify: `infrastructure/deploy/docker-compose.prod.yml`

Changes:
1. Remove standalone `traefik` service (Dokploy bundles Traefik)
2. Remove `traefik_letsencrypt` volume (certs live in `/mnt/data/dokploy/traefik`)
3. Add `minio` service (replaces OVH S3)
4. Postgres data on `/mnt/data/postgres` (bind mount, not Docker volume)

- [ ] Read `infrastructure/deploy/docker-compose.prod.yml` first (already done above)

- [ ] Rewrite `infrastructure/deploy/docker-compose.prod.yml`:

```yaml
# BDP Production — deployed as a Docker Compose project via Dokploy
# Dokploy provides Traefik (with Let's Encrypt) — do NOT add a traefik service here.
# All persistent data is on /mnt/data (Hetzner attached volume).

services:
  postgres:
    image: postgres:16-alpine
    container_name: bdp-postgres
    restart: unless-stopped
    environment:
      POSTGRES_DB: bdp
      POSTGRES_USER: bdp
      POSTGRES_PASSWORD: "${POSTGRES_PASSWORD}"
    volumes:
      - /mnt/data/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U bdp -d bdp"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s

  minio:
    image: minio/minio:latest
    container_name: bdp-minio
    restart: unless-stopped
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: "${MINIO_ROOT_USER}"
      MINIO_ROOT_PASSWORD: "${MINIO_ROOT_PASSWORD}"
    volumes:
      - /mnt/data/minio:/data
    healthcheck:
      test: ["CMD", "mc", "ready", "local"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.minio-console.rule=Host(`minio.${DOMAIN}`)"
      - "traefik.http.routers.minio-console.entrypoints=websecure"
      - "traefik.http.routers.minio-console.tls.certresolver=letsencrypt"
      - "traefik.http.routers.minio-console.service=minio-console"
      - "traefik.http.services.minio-console.loadbalancer.server.port=9001"
      - "traefik.http.routers.minio-api.rule=Host(`s3.${DOMAIN}`)"
      - "traefik.http.routers.minio-api.entrypoints=websecure"
      - "traefik.http.routers.minio-api.tls.certresolver=letsencrypt"
      - "traefik.http.routers.minio-api.service=minio-api"
      - "traefik.http.services.minio-api.loadbalancer.server.port=9000"

  bdp-server:
    image: ghcr.io/datadir-lab/bdp-server:latest
    container_name: bdp-server
    restart: unless-stopped
    environment:
      SERVER_HOST: "0.0.0.0"
      SERVER_PORT: "8000"
      RUST_LOG: "${RUST_LOG:-info,bdp_server=info,sqlx=warn}"
      DATABASE_URL: "postgresql://bdp:${POSTGRES_PASSWORD}@postgres:5432/bdp"
      STORAGE_TYPE: "s3"
      STORAGE_S3_ENDPOINT: "http://minio:9000"
      STORAGE_S3_REGION: "us-east-1"
      STORAGE_S3_BUCKET: "bdp-production"
      STORAGE_S3_ACCESS_KEY: "${MINIO_ROOT_USER}"
      STORAGE_S3_SECRET_KEY: "${MINIO_ROOT_PASSWORD}"
      INGEST_ENABLED: "${INGEST_ENABLED:-true}"
      INGEST_WORKER_THREADS: "2"
      INGEST_MAX_RETRIES: "3"
      INGEST_JOB_TIMEOUT_SECS: "7200"
      INGEST_START_FROM_VERSION: "2025_01"
      INGEST_UNIPROT_FTP_HOST: "ftp.uniprot.org"
      INGEST_UNIPROT_FTP_TIMEOUT_SECS: "300"
      INGEST_UNIPROT_BATCH_SIZE: "5000"
      INGEST_UNIPROT_MODE: "latest"
      INGEST_UNIPROT_AUTO_INGEST: "true"
      INGEST_NCBI_ENABLED: "${INGEST_NCBI_ENABLED:-true}"
      INGEST_NCBI_START_DATE: "${INGEST_NCBI_START_DATE:-2025-01-01}"
      INGEST_GENBANK_ENABLED: "${INGEST_GENBANK_ENABLED:-true}"
      INGEST_GENBANK_SOURCE_DATABASE: "genbank"
      INGEST_GENBANK_BATCH_SIZE: "500"
      INGEST_GENBANK_CONCURRENCY: "1"
      INGEST_GO_ENABLED: "${INGEST_GO_ENABLED:-true}"
      INGEST_INTERPRO_ENABLED: "${INGEST_INTERPRO_ENABLED:-true}"
      INGEST_INTERPRO_BATCH_SIZE: "500"
      API_RATE_LIMIT: "100"
      API_TIMEOUT_SECS: "30"
      CORS_ALLOWED_ORIGINS: "https://${DOMAIN}"
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.api.rule=Host(`${DOMAIN}`) && (PathPrefix(`/api`) || PathPrefix(`/health`))"
      - "traefik.http.routers.api.entrypoints=websecure"
      - "traefik.http.routers.api.tls.certresolver=letsencrypt"
      - "traefik.http.services.api.loadbalancer.server.port=8000"
    depends_on:
      postgres:
        condition: service_healthy
      minio:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      start_period: 60s
      retries: 3

  bdp-web:
    image: ghcr.io/datadir-lab/bdp-web:latest
    container_name: bdp-web
    restart: unless-stopped
    environment:
      INTERNAL_API_URL: "http://bdp-server:8000"
      NEXT_PUBLIC_API_URL: "${PUBLIC_URL}"
      NODE_ENV: "production"
    healthcheck:
      test: ["CMD", "node", "-e", "require('http').get('http://localhost:3000/', (r) => {process.exit(r.statusCode < 400 ? 0 : 1)})"]
      interval: 30s
      timeout: 3s
      start_period: 15s
      retries: 3
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.web.rule=Host(`${DOMAIN}`)"
      - "traefik.http.routers.web.entrypoints=websecure"
      - "traefik.http.routers.web.tls.certresolver=letsencrypt"
      - "traefik.http.routers.web.priority=1"
      - "traefik.http.services.web.loadbalancer.server.port=3000"
    depends_on:
      bdp-server:
        condition: service_healthy
```

- [ ] Commit:
```bash
git add infrastructure/deploy/docker-compose.prod.yml
git commit -m "feat(infra): update docker-compose for dokploy + minio + persistent volumes"
```

---

## Task 9: Rewrite xtask/src/infra.rs

**Files:**
- Modify: `xtask/src/infra.rs`

New commands: `bootstrap`, `init`, `plan`, `apply`, `destroy`, `ssh`, `status`, `info`, `post-deploy`, `show-secrets`, `backup-now`, `backup-list`, `restore`, `logs`, `update`.

All commands load `.secrets` from `infrastructure/hetzner/environments/prod/.secrets` before invoking Terraform. SSH key path comes from `SSH_KEY_PATH` in `.secrets`.

- [ ] Read current `xtask/src/infra.rs` (already done — it has 8 basic commands calling cd infrastructure && terraform ...)

- [ ] Read `xtask/src/utils.rs` to understand available helpers.

**Cross-platform note:** `run_bash` is only defined for `#[cfg(not(target_os = "windows"))]`. On Windows, use `run_powershell`. **However**, `infra` commands all require `terraform`, `ssh`, and `sh` — tools that on Windows are only reliably available inside WSL. The pragmatic approach (matching the old `infra.rs` pattern) is:
- Every function that calls `run_bash` wraps its body in `#[cfg(not(target_os = "windows"))]` / `#[cfg(target_os = "windows")]` blocks
- The Windows variant uses `run_powershell` with `wsl bash -c "..."` to invoke shell commands inside WSL
- `get_server_ip()` likewise needs a `#[cfg]` split: on Windows use `std::process::Command::new("wsl").args(["bash", "-c", &script])`, on Unix use `std::process::Command::new("sh").args(["-c", &script])`
- `ssh_connect()` can use `ssh` directly (Windows 10+ ships OpenSSH natively)
- Document in `infrastructure/README.md` that infra commands require WSL on Windows

The implementation below shows the Unix logic only; wrap each function with the appropriate `#[cfg]` guards as described:

- [ ] Rewrite `xtask/src/infra.rs` — add `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` guards to every function that calls `run_bash` or `run_powershell`, following the exact pattern in the old `infra.rs`. The implementation below shows the logic; wrap each function body appropriately:

```rust
//! Infrastructure operations — Hetzner VPS via Terraform + Dokploy
//!
//! All commands load environment from infrastructure/hetzner/environments/prod/.secrets
//! before running. Set SSH_KEY_PATH in .secrets to control which key is used for SSH ops.
use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

use crate::utils::*;

const SECRETS_PATH: &str = "infrastructure/hetzner/environments/prod/.secrets";
const TF_DIR: &str = "infrastructure/hetzner/terraform";

#[derive(Debug, Parser)]
pub enum InfraCommand {
    /// One-time setup: generate SSH key + initialize Terraform
    Bootstrap,
    /// Initialize Terraform (after bootstrap)
    Init,
    /// Preview infrastructure changes
    Plan,
    /// Apply infrastructure changes (provisions/updates VPS)
    Apply,
    /// Destroy infrastructure — volume persists (requires confirmation)
    Destroy,
    /// Show Terraform outputs (server IP, URLs, etc.)
    Info,
    /// SSH into production server
    Ssh,
    /// Check live server status (Docker services health)
    Status,
    /// Wait for cloud-init to complete and show credentials
    PostDeploy,
    /// Show all production credentials
    ShowSecrets,
    /// Trigger immediate restic backup
    BackupNow,
    /// List restic snapshots on Storage Box
    BackupList,
    /// Restore from restic backup (interactive)
    Restore,
    /// Tail logs from a service (usage: infra logs [service])
    Logs {
        /// Service name: bdp-server, bdp-web, postgres, minio (default: bdp-server)
        #[arg(default_value = "bdp-server")]
        service: String,
    },
    /// Pull latest Docker images and restart services via Dokploy
    Update,
    /// Validate Terraform configuration
    Validate,
}

pub fn handle(cmd: InfraCommand) -> Result<()> {
    match cmd {
        InfraCommand::Bootstrap => bootstrap(),
        InfraCommand::Init => tf_init(),
        InfraCommand::Plan => tf_plan(),
        InfraCommand::Apply => tf_apply(),
        InfraCommand::Destroy => tf_destroy(),
        InfraCommand::Info => tf_info(),
        InfraCommand::Ssh => ssh_connect(),
        InfraCommand::Status => server_status(),
        InfraCommand::PostDeploy => post_deploy(),
        InfraCommand::ShowSecrets => show_secrets(),
        InfraCommand::BackupNow => backup_now(),
        InfraCommand::BackupList => backup_list(),
        InfraCommand::Restore => backup_restore(),
        InfraCommand::Logs { service } => logs(&service),
        InfraCommand::Update => update_services(),
        InfraCommand::Validate => tf_validate(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the path to .secrets, erroring with a helpful message if missing.
fn secrets_path() -> Result<PathBuf> {
    let path = PathBuf::from(SECRETS_PATH);
    if !path.exists() {
        bail!(
            "Secrets file not found: {}\n\
             Copy the example and fill in your values:\n\
             cp {}.example {}",
            SECRETS_PATH,
            SECRETS_PATH,
            SECRETS_PATH
        );
    }
    Ok(path)
}

/// Build the shell preamble that sources .secrets and exports TF_VAR_* vars.
/// Merges YAML config (config.yaml) into environment as TF_VAR_* too.
fn load_env_preamble() -> String {
    format!(
        r#"
set -euo pipefail
# Load secrets
if [ -f "{secrets}" ]; then
  set -a
  source "{secrets}"
  set +a
fi
# Ensure Terraform uses our directory
TF_DIR="{tf_dir}"
"#,
        secrets = SECRETS_PATH,
        tf_dir = TF_DIR,
    )
}

/// Get server IP from Terraform outputs (SSH key path from .secrets)
fn get_server_ip() -> Result<String> {
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
cd "$TF_DIR"
terraform output -raw server_ipv4 2>/dev/null
"#,
        preamble
    );
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .output()?;
    if !output.status.success() {
        bail!("Failed to get server IP. Is infrastructure deployed? Run: cargo xtask infra apply");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn ssh_key_path() -> String {
    // Read SSH_KEY_PATH from .secrets, fallback to default
    if let Ok(content) = std::fs::read_to_string(SECRETS_PATH) {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("SSH_KEY_PATH=") {
                return val.trim().replace("~", &std::env::var("HOME").unwrap_or_default());
            }
        }
    }
    format!(
        "{}/.ssh/bdp_prod_ed25519",
        std::env::var("HOME").unwrap_or_default()
    )
}

fn ssh_cmd(ip: &str, remote_cmd: &str) -> String {
    format!(
        "ssh -i {key} -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 root@{ip} '{cmd}'",
        key = ssh_key_path(),
        ip = ip,
        cmd = remote_cmd
    )
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn bootstrap() -> Result<()> {
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
echo "=== BDP Infrastructure Bootstrap ==="
echo ""

# 1. Generate SSH key if it doesn't exist
SSH_KEY="${{SSH_KEY_PATH:-$HOME/.ssh/bdp_prod_ed25519}}"
SSH_KEY=$(echo "$SSH_KEY" | sed "s|~|$HOME|")
if [ ! -f "$SSH_KEY" ]; then
  echo "Generating SSH key: $SSH_KEY"
  ssh-keygen -t ed25519 -C "bdp-prod" -f "$SSH_KEY" -N ""
  echo ""
  echo "SSH public key (add to .secrets as TF_VAR_ssh_public_key):"
  cat "${{SSH_KEY}}.pub"
  echo ""
else
  echo "SSH key already exists: $SSH_KEY"
fi

# 2. Initialize Terraform
echo "Initializing Terraform..."
cd "$TF_DIR"
terraform init

echo ""
echo "Bootstrap complete."
echo ""
echo "Next steps:"
echo "  1. Ensure {secrets} is filled with all required values"
echo "  2. Run: cargo xtask infra plan"
echo "  3. Run: cargo xtask infra apply"
"#,
            preamble,
            secrets = SECRETS_PATH
        ),
        "Bootstrap infrastructure",
    )
}

fn tf_init() -> Result<()> {
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
echo "Initializing Terraform..."
cd "$TF_DIR"
terraform init
"#,
            preamble
        ),
        "Terraform init",
    )
}

fn tf_plan() -> Result<()> {
    secrets_path()?;
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
echo "Planning infrastructure changes..."
cd "$TF_DIR"
terraform plan
"#,
            preamble
        ),
        "Terraform plan",
    )
}

fn tf_apply() -> Result<()> {
    secrets_path()?;
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
echo "Applying infrastructure..."
cd "$TF_DIR"
terraform apply
echo ""
echo "Done. Run 'cargo xtask infra post-deploy' to wait for cloud-init."
"#,
            preamble
        ),
        "Terraform apply",
    )
}

fn tf_destroy() -> Result<()> {
    secrets_path()?;
    print!("This will DESTROY the server (volume persists). Type 'yes' to confirm: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() != "yes" {
        println!("Aborted.");
        return Ok(());
    }
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
echo "Destroying infrastructure (volume persists)..."
cd "$TF_DIR"
terraform destroy
"#,
            preamble
        ),
        "Terraform destroy",
    )
}

fn tf_info() -> Result<()> {
    secrets_path()?;
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
echo "Infrastructure outputs:"
echo "=================================="
cd "$TF_DIR"
terraform output
"#,
            preamble
        ),
        "Terraform info",
    )
}

fn tf_validate() -> Result<()> {
    let preamble = load_env_preamble();
    run_bash(
        &format!(
            r#"{}
cd "$TF_DIR"
terraform validate && terraform fmt -check
echo "Terraform configuration is valid."
"#,
            preamble
        ),
        "Terraform validate",
    )
}

fn ssh_connect() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    println!("Connecting to root@{ip}...");
    std::process::Command::new("ssh")
        .args([
            "-i", &key,
            "-o", "StrictHostKeyChecking=accept-new",
            &format!("root@{ip}"),
        ])
        .status()?;
    Ok(())
}

fn server_status() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"
echo "=== BDP Production Status ==="
echo "  Server: {ip}"
echo ""
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
  "docker ps --format 'table {{{{.Names}}}}\t{{{{.Status}}}}\t{{{{.Ports}}}}'"
"#,
            ip = ip,
            key = key
        ),
        "Server status",
    )
}

fn post_deploy() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"
echo "=== Waiting for cloud-init to complete ==="
echo "  Server: {ip}"
echo "  This may take 5-10 minutes on first boot..."
echo ""

for i in $(seq 1 60); do
  if ssh -i {key} -o StrictHostKeyChecking=accept-new -o ConnectTimeout=5 root@{ip} \
      "test -f /mnt/data/.initialized" 2>/dev/null; then
    echo "  Cloud-init complete after ${{i}}x10s"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: Cloud-init did not complete after 10 minutes."
    echo "Check logs: cargo xtask infra ssh, then: tail -f /var/log/cloud-init-output.log"
    exit 1
  fi
  printf "  Waiting... ($i/60)\r"
  sleep 10
done

echo ""
echo "=== Credentials ==="
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "/opt/bdp/scripts/show-secrets.sh"
"#,
            ip = ip,
            key = key
        ),
        "Post-deploy",
    )
}

fn show_secrets() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "/opt/bdp/scripts/show-secrets.sh""#,
            ip = ip,
            key = key
        ),
        "Show secrets",
    )
}

fn backup_now() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"
echo "Triggering restic backup on {ip}..."
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
  "MOUNT_POINT=/mnt/data /opt/bdp/scripts/backup-restic.sh"
"#,
            ip = ip,
            key = key
        ),
        "Backup now",
    )
}

fn backup_list() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"
echo "Restic snapshots on {ip}:"
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
  "source /mnt/data/.secrets/env && restic snapshots --repo \$RESTIC_REPOSITORY"
"#,
            ip = ip,
            key = key
        ),
        "Backup list",
    )
}

fn backup_restore() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    println!("WARNING: This will restore files from a restic snapshot.");
    println!("Run the following to restore interactively:");
    println!();
    println!(
        "  ssh -i {} root@{} \\",
        key, ip
    );
    println!("    'source /mnt/data/.secrets/env && restic restore latest --target /mnt/data'");
    println!();
    println!("Or to restore to a temporary location first:");
    println!(
        "  ssh -i {} root@{} \\",
        key, ip
    );
    println!("    'source /mnt/data/.secrets/env && restic restore latest --target /tmp/restore'");
    Ok(())
}

fn logs(service: &str) -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "docker logs -f --tail=100 {service}""#,
            ip = ip,
            key = key,
            service = service
        ),
        &format!("Logs for {service}"),
    )
}

fn update_services() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    run_bash(
        &format!(
            r#"
echo "Pulling latest images and restarting services on {ip}..."
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "
  docker pull ghcr.io/datadir-lab/bdp-server:latest
  docker pull ghcr.io/datadir-lab/bdp-web:latest
  docker restart bdp-server bdp-web
  docker ps --format 'table {{{{.Names}}}}\t{{{{.Status}}}}'
"
echo "Services updated."
"#,
            ip = ip,
            key = key
        ),
        "Update services",
    )
}
```

- [ ] Verify it compiles:
```bash
cargo build -p xtask 2>&1
```
Expected: compiles cleanly. Fix any `run_bash` signature mismatches by checking `xtask/src/utils.rs`.

- [ ] Commit:
```bash
git add xtask/src/infra.rs
git commit -m "feat(infra): rewrite xtask infra module with full hetzner command set"
```

---

## Task 10: Bootstrap script and .gitignore

**Files:**
- Create: `infrastructure/hetzner/scripts/bootstrap.sh`
- Modify: `infrastructure/.gitignore`

- [ ] Create `infrastructure/hetzner/scripts/bootstrap.sh`:

```bash
#!/bin/bash
# One-time bootstrap helper.
# Usage: bash infrastructure/hetzner/scripts/bootstrap.sh
# Or use: cargo xtask infra bootstrap
set -euo pipefail

SECRETS="infrastructure/hetzner/environments/prod/.secrets"
TF_DIR="infrastructure/hetzner/terraform"

echo "=== BDP Infrastructure Bootstrap ==="
echo ""

if [ ! -f "$SECRETS" ]; then
  echo "Creating secrets file from example..."
  cp "${SECRETS}.example" "$SECRETS"
  echo ""
  echo "Edit $SECRETS and fill in all required values, then run again."
  exit 0
fi

source "$SECRETS"

# Generate SSH key
SSH_KEY="${SSH_KEY_PATH:-$HOME/.ssh/bdp_prod_ed25519}"
SSH_KEY="${SSH_KEY/#\~/$HOME}"
if [ ! -f "$SSH_KEY" ]; then
  echo "Generating SSH key: $SSH_KEY"
  ssh-keygen -t ed25519 -C "bdp-prod" -f "$SSH_KEY" -N ""
  echo ""
  echo "Add this to your .secrets as TF_VAR_ssh_public_key:"
  echo "TF_VAR_ssh_public_key=$(cat ${SSH_KEY}.pub)"
  echo ""
fi

# Terraform init
echo "Initializing Terraform..."
cd "$TF_DIR"
terraform init

echo ""
echo "Bootstrap complete. Next: cargo xtask infra plan"
```

- [ ] Ensure `infrastructure/hetzner/environments/prod/.secrets` is gitignored. Read `infrastructure/.gitignore`:

The existing `.gitignore` likely only covers the old structure. Add:
```
# Hetzner environment secrets
hetzner/environments/**/.secrets
# Terraform generated files
hetzner/terraform/.terraform/
hetzner/terraform/*.tfstate
hetzner/terraform/*.tfstate.backup
hetzner/terraform/*.tfplan
```

- [ ] Commit:
```bash
git add infrastructure/hetzner/scripts/bootstrap.sh infrastructure/.gitignore
git commit -m "feat(infra): add bootstrap script and gitignore for secrets/state"
```

---

## Task 11: Update infrastructure/README.md

**Files:**
- Modify: `infrastructure/README.md`

- [ ] Rewrite `infrastructure/README.md` to document the new setup:

```markdown
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
```

- [ ] Commit:
```bash
git add infrastructure/README.md
git commit -m "docs(infra): update README for hetzner + dokploy setup"
```

---

## Task 12: End-to-end smoke test

- [ ] Validate Terraform config locally (no credentials needed):
```bash
cd infrastructure/hetzner/terraform
terraform init
terraform validate
```
Expected: `Success! The configuration is valid.`

- [ ] Verify xtask commands are listed:
```bash
cargo xtask infra --help
```
Expected: lists Bootstrap, Init, Plan, Apply, Destroy, Info, Ssh, Status, PostDeploy, ShowSecrets, BackupNow, BackupList, Restore, Logs, Update, Validate

- [ ] Check .secrets is gitignored:
```bash
git status infrastructure/hetzner/environments/prod/.secrets
```
Expected: file not shown (ignored)

- [ ] Final commit (cleanup/fmt):
```bash
cargo fmt -p xtask
git add -A
git commit -m "chore(infra): fmt and final cleanup"
```

---

## Deployment Runbook (after implementation)

```bash
# 1. Fill secrets
cp infrastructure/hetzner/environments/prod/.secrets.example \
   infrastructure/hetzner/environments/prod/.secrets
# Set: hcloud_token, ssh_public_key, cloudflare_api_token, dokploy_admin_password,
#      minio_root_password, restic_password, acme_email

# 2. Bootstrap
cargo xtask infra bootstrap

# 3. Plan (review what will be created)
cargo xtask infra plan

# 4. Apply (~2-3 min to provision)
cargo xtask infra apply

# 5. Wait for cloud-init (~8-10 min first boot)
cargo xtask infra post-deploy

# 6. Open Dokploy: https://dokploy.bdp.dev
#    Create Docker Compose project, paste docker-compose.prod.yml
#    Set env vars from .secrets

# 7. Verify
cargo xtask infra status
```
