#!/bin/bash
# BDP Production Setup Script
# Run on the OVH instance after Terraform provisioning
#
# Usage: curl -sSL https://raw.githubusercontent.com/YOUR_ORG/bdp/main/infrastructure/deploy/setup.sh | bash

set -e

echo "=== BDP Production Setup ==="
echo ""

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then
    echo "Please run with sudo: sudo bash setup.sh"
    exit 1
fi

# Variables
APP_DIR="/opt/bdp"
APP_USER="ubuntu"

echo "1. Creating application directory..."
mkdir -p $APP_DIR
chown $APP_USER:$APP_USER $APP_DIR

echo "2. Installing Docker (if not present)..."
if ! command -v docker &> /dev/null; then
    curl -fsSL https://get.docker.com | sh
    usermod -aG docker $APP_USER
fi

echo "3. Installing Docker Compose plugin..."
apt-get update
apt-get install -y docker-compose-plugin

echo "4. Installing Caddy..."
apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
apt-get update
apt-get install -y caddy

echo "5. Installing useful tools..."
apt-get install -y htop curl wget git jq unzip postgresql-client

echo "6. Creating systemd service for BDP..."
cat > /etc/systemd/system/bdp.service << 'EOF'
[Unit]
Description=BDP Application
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/opt/bdp
ExecStart=/usr/bin/docker compose -f docker-compose.prod.yml up -d
ExecStop=/usr/bin/docker compose -f docker-compose.prod.yml down
User=ubuntu
Group=docker

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable bdp.service

echo "7. Configuring firewall..."
ufw allow 22/tcp   # SSH
ufw allow 80/tcp   # HTTP
ufw allow 443/tcp  # HTTPS
ufw --force enable

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Copy your .env file to $APP_DIR/.env"
echo "2. Copy docker-compose.prod.yml to $APP_DIR/"
echo "3. Configure Caddy: sudo nano /etc/caddy/Caddyfile"
echo "4. Start the application: sudo systemctl start bdp"
echo "5. Check status: sudo systemctl status bdp"
echo ""
echo "Useful commands:"
echo "  - View logs: docker compose -f $APP_DIR/docker-compose.prod.yml logs -f"
echo "  - Restart: sudo systemctl restart bdp"
echo "  - Check Caddy: sudo systemctl status caddy"
echo ""
