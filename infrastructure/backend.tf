# =============================================================================
# Terraform Backend Configuration
# Using Terraform Cloud for secure state management
# =============================================================================
#
# Setup Instructions:
# 1. Create free Terraform Cloud account: https://app.terraform.io/signup
# 2. Create organization (e.g., "bdp-project")
# 3. Create workspace named "bdp-mvp" (CLI-driven workflow)
# 4. In workspace settings, set Execution Mode to "Local"
# 5. Generate API token: User Settings > Tokens > Create API token
# 6. Run: terraform login
#
# For CI/CD, set TF_API_TOKEN as GitHub Environment secret

terraform {
  cloud {
    organization = "bdp-project"  # Change to your org name

    workspaces {
      name = "bdp-mvp"
    }
  }
}

# Alternative: Use OVH Object Storage for state (if you prefer)
# Uncomment below and comment out the cloud block above
#
# terraform {
#   backend "s3" {
#     bucket                      = "bdp-terraform-state"
#     key                         = "mvp/terraform.tfstate"
#     region                      = "gra"
#     endpoint                    = "https://s3.gra.cloud.ovh.net"
#     skip_credentials_validation = true
#     skip_region_validation      = true
#     skip_metadata_api_check     = true
#     skip_requesting_account_id  = true
#     # Credentials via AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY env vars
#   }
# }
