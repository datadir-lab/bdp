# =============================================================================
# Compute Instance - Single MVP Server
# Runs: Caddy (reverse proxy), Next.js frontend, Rust backend
# =============================================================================

# SSH Key for instance access
resource "openstack_compute_keypair_v2" "bdp_key" {
  name       = var.ssh_key_name
  public_key = var.ssh_public_key
}

# Use existing default security group (OVH free tier has quota limits)
# Security group rules quota is 0, so we use default security group AS-IS
# You'll need to manually add SSH (22), HTTP (80), HTTPS (443) rules in OVH panel
data "openstack_networking_secgroup_v2" "default" {
  name = "default"
}

# Compute Instance
resource "openstack_compute_instance_v2" "bdp_server" {
  name            = var.instance_name
  image_name      = var.instance_image
  flavor_name     = var.instance_flavor
  key_pair        = openstack_compute_keypair_v2.bdp_key.name
  security_groups = [data.openstack_networking_secgroup_v2.default.name]

  network {
    name = "Ext-Net" # OVH public network
  }

  metadata = {
    environment = var.environment
    managed_by  = "terraform"
    project     = "bdp"
  }

  user_data = <<-EOF
    #!/bin/bash
    set -e

    # Add CI/CD deploy key to authorized_keys (if provided)
    ${var.deploy_ssh_public_key != "" ? "echo '${var.deploy_ssh_public_key}' >> /home/ubuntu/.ssh/authorized_keys" : "# No deploy key provided"}

    # Update system
    apt-get update && apt-get upgrade -y

    # Install Docker
    curl -fsSL https://get.docker.com | sh
    usermod -aG docker ubuntu

    # Install Docker Compose
    apt-get install -y docker-compose-plugin

    # Install Caddy
    apt-get install -y debian-keyring debian-archive-keyring apt-transport-https
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
    apt-get update && apt-get install -y caddy

    # Create app directory
    mkdir -p /opt/bdp
    chown ubuntu:ubuntu /opt/bdp

    # Install useful tools
    apt-get install -y htop curl wget git jq unzip

    echo "BDP MVP server provisioned successfully" > /opt/bdp/provisioned.txt
  EOF
}

# Floating IP (optional, for stable public IP)
# Uncomment if you need a static IP that survives instance recreation
# resource "openstack_networking_floatingip_v2" "bdp_ip" {
#   pool = "Ext-Net"
# }
#
# resource "openstack_compute_floatingip_associate_v2" "bdp_ip_assoc" {
#   floating_ip = openstack_networking_floatingip_v2.bdp_ip.address
#   instance_id = openstack_compute_instance_v2.bdp_server.id
# }
