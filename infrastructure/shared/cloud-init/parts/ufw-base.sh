#!/bin/bash
# UFW firewall baseline for BDP production server.
# Allows: SSH (22), HTTP (80), HTTPS (443), Dokploy UI (3000 — restrict after setup)
set -euo pipefail

echo "=== UFW Firewall Setup ==="

ufw --force reset
ufw default deny incoming
ufw default allow outgoing

ufw allow 22/tcp   comment "SSH"
ufw allow 80/tcp   comment "HTTP (ACME challenge)"
ufw allow 443/tcp  comment "HTTPS (Traefik)"
ufw allow 3000/tcp comment "Dokploy UI (restrict to your IP post-setup)"

ufw --force enable
echo "  UFW status:"
ufw status verbose
echo "=== UFW setup complete ==="
