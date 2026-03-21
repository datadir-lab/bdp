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
