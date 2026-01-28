# =============================================================================
# Managed PostgreSQL Database
# OVH Public Cloud Database - Essential plan (cheapest)
# =============================================================================

resource "ovh_cloud_project_database" "postgresql" {
  service_name = var.ovh_project_id
  description  = "BDP ${var.environment} PostgreSQL"
  engine       = "postgresql"
  version      = var.db_version
  plan         = var.db_plan
  flavor       = var.db_flavor

  nodes {
    region = var.region
  }

  # Note: Essential plan = 1 node, no HA
  # Upgrade to "business" plan for 2 nodes with HA
}

# Database user
resource "ovh_cloud_project_database_user" "bdp_user" {
  service_name = var.ovh_project_id
  cluster_id   = ovh_cloud_project_database.postgresql.id
  engine       = "postgresql"
  name         = var.db_user
}

# Database
resource "ovh_cloud_project_database_database" "bdp_db" {
  service_name = var.ovh_project_id
  cluster_id   = ovh_cloud_project_database.postgresql.id
  engine       = "postgresql"
  name         = var.db_name
}

# IP Restriction - Allow access from our instance
# Note: For MVP, we allow all IPs. In production, restrict to instance IP only.
resource "ovh_cloud_project_database_ip_restriction" "allow_instance" {
  service_name = var.ovh_project_id
  cluster_id   = ovh_cloud_project_database.postgresql.id
  engine       = "postgresql"
  ip           = "${openstack_compute_instance_v2.bdp_server.access_ip_v4}/32"
}

# Allow your local IP for development (optional)
# resource "ovh_cloud_project_database_ip_restriction" "allow_dev" {
#   service_name = var.ovh_project_id
#   cluster_id   = ovh_cloud_project_database.postgresql.id
#   engine       = "postgresql"
#   ip           = "YOUR_IP/32"
# }
