#!/bin/bash
# One-time bootstrap helper.
# Usage: bash infrastructure/hetzner/scripts/bootstrap.sh
# Or use: cargo xtask infra bootstrap
set -euo pipefail

ENV_FILE="infrastructure/hetzner/environments/prod/.env"
TF_DIR="infrastructure/hetzner/terraform"

echo "=== BDP Infrastructure Bootstrap ==="
echo ""

if [ ! -f "$ENV_FILE" ]; then
  echo "Creating .env from example..."
  cp "${ENV_FILE}.example" "$ENV_FILE"
  echo ""
  echo "Edit $ENV_FILE and fill in all required values, then run again."
  exit 0
fi

set -a; source "$ENV_FILE"; set +a

# Generate SSH key
SSH_KEY="${SSH_KEY_PATH:-$HOME/.ssh/bdp_prod_ed25519}"
SSH_KEY="${SSH_KEY/#\~/$HOME}"
if [ ! -f "$SSH_KEY" ]; then
  echo "Generating SSH key: $SSH_KEY"
  ssh-keygen -t ed25519 -C "bdp-prod" -f "$SSH_KEY" -N ""
  echo ""
  echo "Add to .env as: SSH_PUBLIC_KEY=$(cat ${SSH_KEY}.pub)"
  echo ""
fi

# Terraform init
echo "Initializing Terraform..."
cd "$TF_DIR"
terraform init

echo ""
echo "Bootstrap complete. Next: cargo xtask infra plan"
