#!/bin/bash
# Standard Hetzner volume mount with safety checks.
# Idempotent — safe to call on every redeploy.
#
# Required env vars:
#   VOLUME_DEVICE  — e.g. /dev/disk/by-id/scsi-0HC_Volume_12345
#
# Optional env vars:
#   MOUNT_POINT    — default /mnt/data
set -euo pipefail

MOUNT_POINT="${MOUNT_POINT:-/mnt/data}"

echo "=== Volume Mount ==="
echo "  Device:      $VOLUME_DEVICE"
echo "  Mount point: $MOUNT_POINT"

# --------------------------------------------------------------------------
# Wait for device to appear (up to 30s)
# --------------------------------------------------------------------------
echo "  Waiting for device to appear..."
for i in $(seq 1 30); do
  if [ -e "$VOLUME_DEVICE" ]; then
    echo "  ✓ Device found after ${i}s"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "============================================================"
    echo "ERROR: Device not found after 30 seconds: $VOLUME_DEVICE"
    echo "============================================================"
    echo "Check that the Hetzner volume is attached to this server."
    exit 1
  fi
  sleep 1
done

# --------------------------------------------------------------------------
# Safety check: verify ext4 filesystem — NEVER auto-format
# --------------------------------------------------------------------------
echo "  Checking filesystem type..."
if blkid "$VOLUME_DEVICE" >/dev/null 2>&1; then
  if ! blkid "$VOLUME_DEVICE" | grep -q 'TYPE="ext4"'; then
    echo "============================================================"
    echo "ERROR: Volume has non-ext4 filesystem"
    echo "============================================================"
    echo "Current filesystem: $(blkid "$VOLUME_DEVICE")"
    echo ""
    echo "MANUAL ACTION REQUIRED:"
    echo "  To reformat (WARNING: destroys all data):"
    echo "    mkfs.ext4 -F $VOLUME_DEVICE"
    echo ""
    echo "This safety check prevents accidental data loss."
    echo "============================================================"
    exit 1
  fi
  echo "  ✓ Volume has ext4 filesystem"
else
  echo "============================================================"
  echo "ERROR: Volume is unformatted (new volume detected)"
  echo "============================================================"
  echo "Device: $VOLUME_DEVICE"
  echo ""
  echo "MANUAL ACTION REQUIRED:"
  echo "  To format for first use:"
  echo "    mkfs.ext4 $VOLUME_DEVICE"
  echo ""
  echo "WARNING: This destroys any data on the volume."
  echo "This safety check prevents accidental formatting."
  echo "============================================================"
  exit 1
fi

# --------------------------------------------------------------------------
# Add to /etc/fstab if not already present
# --------------------------------------------------------------------------
if ! grep -q " $MOUNT_POINT " /etc/fstab; then
  echo "  Adding volume to /etc/fstab..."
  echo "$VOLUME_DEVICE $MOUNT_POINT ext4 defaults,nofail 0 0" >> /etc/fstab
fi

# --------------------------------------------------------------------------
# Mount (skip if already mounted)
# --------------------------------------------------------------------------
mkdir -p "$MOUNT_POINT"

if mountpoint -q "$MOUNT_POINT"; then
  echo "  ✓ Volume already mounted at $MOUNT_POINT"
else
  echo "  Mounting volume..."
  mount -a 2>/dev/null || mount "$MOUNT_POINT"
  echo "  ✓ Volume mounted at $MOUNT_POINT"
fi

# --------------------------------------------------------------------------
# Set permissions
# --------------------------------------------------------------------------
chmod 755 "$MOUNT_POINT"
echo "  ✓ Permissions set (755) on $MOUNT_POINT"
echo "=== Volume Mount complete ==="
