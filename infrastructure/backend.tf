# =============================================================================
# Terraform Backend Configuration
# =============================================================================
#
# 🧪 PHASE 1: Local Testing (Current Configuration)
# - State file stored locally: terraform.tfstate
# - Fast iteration, no dependencies
# - Use this for initial testing and dry-runs
#
# 🚀 PHASE 2: Production (Uncomment after testing)
# - Migrate to Terraform Cloud for secure remote state
# - Required for CI/CD (GitHub Actions)
# - See migration instructions below

# =============================================================================
# LOCAL BACKEND (Testing) - Currently Active
# =============================================================================
# State file will be saved in infrastructure/terraform.tfstate
# ⚠️ DO NOT commit terraform.tfstate to git!

# terraform {
#   # No backend block = local state file
# }

# =============================================================================
# TERRAFORM CLOUD (Production) - Uncomment when ready
# =============================================================================
# Setup Instructions:
# 1. Create free Terraform Cloud account: https://app.terraform.io/signup
# 2. Create organization (e.g., "datadir-bdp" or your GitHub username)
# 3. Create workspace named "bdp-mvp" (CLI-driven workflow)
# 4. In workspace settings, set Execution Mode to "Local"
# 5. Generate API token: User Settings > Tokens > Create API token
# 6. Run: terraform login
# 7. Uncomment the block below
# 8. Run: terraform init -migrate-state
#
# For CI/CD, set TF_API_TOKEN as GitHub Environment secret

# terraform {
#   cloud {
#     organization = "datadir"  # Your Terraform Cloud organization
#
#     workspaces {
#       name = "bdp"  # Your workspace name
#     }
#   }
# }

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
