#!/bin/bash
# Restic backup of /mnt/data to Hetzner Storage Box.
# Credentials sourced from /mnt/data/.secrets/env:
#   RESTIC_REPOSITORY, RESTIC_PASSWORD, STORAGE_BOX_HOST (for known_hosts)
set -euo pipefail

MOUNT_POINT="${MOUNT_POINT:-/mnt/data}"
SECRETS_FILE="$MOUNT_POINT/.secrets/env"
SSH_KEY="$MOUNT_POINT/.secrets/backup_ssh_key"
SSH_CONFIG="/root/.ssh/config"

if ! mountpoint -q "$MOUNT_POINT"; then
  echo "ERROR: $MOUNT_POINT is not mounted — aborting"; exit 1
fi
[ -f "$SECRETS_FILE" ] || { echo "ERROR: $SECRETS_FILE missing"; exit 1; }
source "$SECRETS_FILE"

# Install restic if absent (Ubuntu 24.04 ships it in apt)
command -v restic &>/dev/null || apt-get install -y -q restic

# SSH client config — use dedicated key, trust storage box on first connect
mkdir -p /root/.ssh
chmod 700 /root/.ssh
if ! grep -q "your-storagebox.de" "$SSH_CONFIG" 2>/dev/null; then
  cat >> "$SSH_CONFIG" <<'EOF'
Host *.your-storagebox.de
    IdentityFile /mnt/data/.secrets/backup_ssh_key
    StrictHostKeyChecking accept-new
    Port 23
    AddressFamily any
EOF
fi
# Pre-populate known_hosts to avoid interactive prompt on first run
ssh-keyscan -p 23 "$STORAGE_BOX_HOST" >> /root/.ssh/known_hosts 2>/dev/null || true

export RESTIC_REPOSITORY RESTIC_PASSWORD

# Auto-initialize repo on first run
restic snapshots &>/dev/null || restic init

# Backup (exclude the backup archive dir and sentinel file)
restic backup "$MOUNT_POINT" \
  --exclude "$MOUNT_POINT/.backups" \
  --exclude "$MOUNT_POINT/.initialized" \
  --tag "$(hostname)" \
  --compression max

# Forget + prune: 7 daily, 4 weekly, 3 monthly
restic forget \
  --keep-daily   7 \
  --keep-weekly  4 \
  --keep-monthly 3 \
  --prune

LAST_SNAP=$(restic snapshots --last --json 2>/dev/null | python3 -c 'import json,sys; s=json.load(sys.stdin); print(s[0]["time"][:19] if s else "none")' 2>/dev/null || echo "done")
echo "Restic backup complete: $LAST_SNAP"
