# =============================================================================
# OVH API Credentials
# Generate at: https://api.ovh.com/createToken/
# Required permissions: GET/POST/PUT/DELETE on /cloud/project/*
# =============================================================================

variable "ovh_endpoint" {
  description = "OVH API endpoint (ovh-eu, ovh-ca, ovh-us)"
  type        = string
  default     = "ovh-eu"
}

variable "ovh_application_key" {
  description = "OVH API application key"
  type        = string
  sensitive   = true
}

variable "ovh_application_secret" {
  description = "OVH API application secret"
  type        = string
  sensitive   = true
}

variable "ovh_consumer_key" {
  description = "OVH API consumer key"
  type        = string
  sensitive   = true
}

variable "ovh_project_id" {
  description = "OVH Public Cloud project ID"
  type        = string
}

# =============================================================================
# OpenStack Credentials
# Get from OVH Control Panel > Public Cloud > Users & Roles > Create User
# =============================================================================

variable "openstack_user_name" {
  description = "OpenStack username (from OVH Public Cloud user)"
  type        = string
}

variable "openstack_password" {
  description = "OpenStack password"
  type        = string
  sensitive   = true
}

# =============================================================================
# Region Configuration
# =============================================================================

variable "region" {
  description = "OVH region (GRA7, GRA9, GRA11, SBG5, DE1, UK1, WAW1, BHS5) - OpenStack region ID"
  type        = string
  default     = "GRA7" # Gravelines, France - good for EU
}

# =============================================================================
# Compute Configuration
# =============================================================================

variable "instance_name" {
  description = "Name for the compute instance"
  type        = string
  default     = "bdp-mvp"
}

variable "instance_flavor" {
  description = "Instance flavor (d2-2, d2-4, d2-8, s1-2, s1-4)"
  type        = string
  default     = "d2-2" # Discovery: 2 vCPU, 4GB RAM, 25GB SSD - cheapest
}

variable "instance_image" {
  description = "OS image name"
  type        = string
  default     = "Ubuntu 24.04"
}

# =============================================================================
# Database Configuration
# =============================================================================

variable "db_plan" {
  description = "Database plan (essential, business, enterprise)"
  type        = string
  default     = "essential" # Cheapest option
}

variable "db_flavor" {
  description = "Database flavor"
  type        = string
  default     = "db1-4" # 1 vCPU, 4GB RAM - smallest
}

variable "db_version" {
  description = "PostgreSQL version"
  type        = string
  default     = "16"
}

variable "db_name" {
  description = "Database name"
  type        = string
  default     = "bdp"
}

variable "db_user" {
  description = "Database admin username"
  type        = string
  default     = "bdp_admin"
}

# =============================================================================
# Object Storage Configuration
# =============================================================================

variable "storage_bucket_name" {
  description = "S3 bucket name for data storage"
  type        = string
  default     = "bdp-data"
}

variable "storage_region" {
  description = "Object storage region"
  type        = string
  default     = "gra" # Gravelines
}

# =============================================================================
# SSH Configuration
# =============================================================================

variable "ssh_public_key" {
  description = "SSH public key for instance access"
  type        = string
}

variable "ssh_key_name" {
  description = "Name for the SSH keypair"
  type        = string
  default     = "bdp-mvp-key"
}

variable "deploy_ssh_public_key" {
  description = "SSH public key for CI/CD deployments (optional)"
  type        = string
  default     = ""
}

# =============================================================================
# Application Configuration
# =============================================================================

variable "domain" {
  description = "Domain name for the application (optional)"
  type        = string
  default     = ""
}

variable "environment" {
  description = "Environment name (mvp, staging, production)"
  type        = string
  default     = "mvp"
}
