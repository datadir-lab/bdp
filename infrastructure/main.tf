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
provider "openstack" {
  auth_url    = "https://auth.cloud.ovh.net/v3/"
  domain_name = "Default"
  user_name   = var.openstack_user_name
  password    = var.openstack_password
  tenant_id   = var.ovh_project_id
  region      = var.region
}
