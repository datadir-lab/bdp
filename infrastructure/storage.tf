# =============================================================================
# Object Storage (S3-compatible)
# For storing data source files (proteins, genomes, etc.)
# =============================================================================

# S3 credentials for the application
resource "ovh_cloud_project_user" "s3_user" {
  service_name = var.ovh_project_id
  description  = "bdp-s3-access"
  role_name    = "objectstore_operator"
}

resource "ovh_cloud_project_user_s3_credential" "s3_creds" {
  service_name = var.ovh_project_id
  user_id      = ovh_cloud_project_user.s3_user.id
}

# Note: OVH Object Storage containers are created via OpenStack Swift API
# or through the S3 API directly. The bucket will be created on first use
# by the application, or you can create it manually:
#
# Using AWS CLI:
#   aws s3 mb s3://bdp-data \
#     --endpoint-url https://s3.${var.storage_region}.cloud.ovh.net
#
# The application should create the bucket if it doesn't exist.

# Output the S3 endpoint for the application
locals {
  s3_endpoint = "https://s3.${var.storage_region}.cloud.ovh.net"
}
