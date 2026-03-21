#!/bin/bash
# Dokploy admin user bootstrap.
# Run once after first boot, sourcing credentials from /mnt/data/.secrets/env
set -euo pipefail

SECRETS_FILE="/mnt/data/.secrets/env"
[ -f "$SECRETS_FILE" ] && source "$SECRETS_FILE"

DOMAIN="${DOMAIN:-bdp.dev}"
ADMIN_EMAIL="${DOKPLOY_ADMIN_EMAIL:-admin@${DOMAIN}}"
ADMIN_PASSWORD="${DOKPLOY_ADMIN_PASSWORD:?DOKPLOY_ADMIN_PASSWORD is required}"

echo "=== Dokploy Admin Setup ==="
echo "Domain:      $DOMAIN"
echo "Admin email: $ADMIN_EMAIL"

# Wait for Dokploy to be ready (up to 5 minutes)
echo "Waiting for Dokploy..."
for i in $(seq 1 60); do
  if curl -sf http://localhost:3000 >/dev/null 2>&1; then
    echo "  Dokploy ready after ${i}x5s"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: Dokploy not ready after 5 minutes"
    exit 1
  fi
  sleep 5
done

# Check if admin already exists
USER_COUNT=$(docker exec "$(docker ps -q --filter 'name=dokploy-postgres')" \
  psql -U dokploy -d dokploy -t -c 'SELECT COUNT(*) FROM "user";' 2>/dev/null | tr -d ' ' || echo "0")

if [ "$USER_COUNT" = "0" ]; then
  echo "  Creating admin user..."

  PASSWORD_HASH=$(docker run --rm -w /tmp \
    -e PASSWORD="$ADMIN_PASSWORD" \
    node:lts-alpine sh -c "
      npm install bcryptjs >/dev/null 2>&1 && \
      node -e \"
        const bcrypt = require('bcryptjs');
        console.log(bcrypt.hashSync(process.env.PASSWORD, 10));
      \"
    ")

  USER_ID=$(cat /proc/sys/kernel/random/uuid)
  ACCOUNT_ID=$(cat /proc/sys/kernel/random/uuid)

  docker exec "$(docker ps -q --filter 'name=dokploy-postgres')" \
    psql -U dokploy -d dokploy -c "
      INSERT INTO \"user\" (id, email, email_verified, role, \"createdAt\", \"isRegistered\", \"expirationDate\", updated_at)
      VALUES ('$USER_ID', '$ADMIN_EMAIL', true, 'admin', NOW(), true, '', NOW())
      ON CONFLICT (email) DO NOTHING;

      INSERT INTO \"account\" (id, account_id, provider_id, user_id, password, created_at, updated_at)
      VALUES ('$ACCOUNT_ID', '$USER_ID', 'credential', '$USER_ID', '$PASSWORD_HASH', NOW(), NOW())
      ON CONFLICT DO NOTHING;
    " && echo "  Admin created: $ADMIN_EMAIL" || echo "  Admin already exists"
else
  echo "  Admin user already exists (count: $USER_COUNT)"
fi

echo "=== Dokploy setup complete ==="
echo ""
echo "  URL:      https://dokploy.$DOMAIN"
echo "  Email:    $ADMIN_EMAIL"
echo "  Password: (from .secrets)"
