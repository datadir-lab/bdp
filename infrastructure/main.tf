# BDP Infrastructure - OVH Cloud
# Minimal MVP setup: Single instance, managed PostgreSQL, object storage

terraform {
  required_version = ">= 1.0.0"

  required_providers {
    ovh = {
      source  = "ovh/ovh"
      version = ">= 0.40.0"
    }
    openstack = {
      source  = "terraform-provider-openstack/openstack"
      version = "~> 1.49.0"
    }
  }
}

# OVH Provider - for managed services (database, object storage)
provider "ovh" {
  endpoint           = var.ovh_endpoint
  application_key    = var.ovh_application_key
  application_secret = var.ovh_application_secret
  consumer_key       = var.ovh_consumer_key
}

# OpenStack Provider - for compute instances and networking
# Explicitly configured for OVH Cloud
# Region must be specific (e.g., GRA7, not just GRA) for compute/networking endpoints
provider "openstack" {
  auth_url            = "https://auth.cloud.ovh.net/v3"
  region              = var.region  # Must be specific region ID like GRA7, GRA9, SBG5, etc.
  user_domain_name    = "Default"
  project_domain_name = "Default"
  tenant_id           = var.ovh_project_id
  tenant_name         = "6933015767659822"
  user_name           = var.openstack_user_name
  password            = var.openstack_password
}
