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
  description = "IPs allowed to SSH and access Dokploy UI (port 3000). Example: [\"1.2.3.4/32\"]"
  type        = list(string)
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
