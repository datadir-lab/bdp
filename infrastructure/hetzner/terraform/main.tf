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
    ignore_changes       = [user_data, image]
  }
}

# Attach data volume to server
resource "hcloud_volume_attachment" "main" {
  volume_id = hcloud_volume.data.id
  server_id = hcloud_server.main.id
  automount = false # cloud-init handles mounting
}
