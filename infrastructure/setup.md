# Infrastructure Setup Guide

This guide walks you through setting up the BDP infrastructure on OVH Cloud.

## What's Included

### Terraform Configuration Files

| File | Purpose |
|------|---------|
| `main.tf` | Provider configuration (OVH + OpenStack) |
| `variables.tf` | Input variables for all resources |
| `compute.tf` | Single d2-2 instance with security groups |
| `database.tf` | Managed PostgreSQL (Essential plan) |
| `storage.tf` | S3 credentials for Object Storage |
| `outputs.tf` | Connection info and auto-generated .env |
| `backend.tf` | Terraform Cloud for secure state storage |

### Deployment Scripts

| File | Purpose |
|------|---------|
| `deploy/setup.sh` | Server provisioning (Docker, Caddy, tools) |
| `deploy/docker-compose.prod.yml` | Production compose file |
| `deploy/Caddyfile.example` | Reverse proxy configuration |

### Documentation

| File | Purpose |
|------|---------|
| `README.md` | Full documentation with architecture diagram |
| `SECURITY.md` | Security guide for open source projects |

## CI/CD Pipeline

The GitHub Actions workflow (`.github/workflows/infrastructure.yml`) provides:

- **Fork PR protection** - blocks access to secrets from forks
- **`plan`** - runs automatically on PRs, no approval needed
- **`apply`** - manual trigger, requires maintainer approval
- **`destroy`** - manual trigger, requires approval + typing "destroy"

## Security Model

- State stored in Terraform Cloud (encrypted, never in git)
- Secrets in GitHub Environment (not repo secrets)
- Fork PRs cannot access credentials
- Manual approval gates for destructive actions

## Estimated MVP Cost

| Resource | Specification | Monthly Cost |
|----------|---------------|--------------|
| Compute | d2-2 (2 vCPU, 4GB RAM) | ~12 EUR |
| Database | Essential PostgreSQL (2GB) | ~18 EUR |
| Storage | Object Storage (S3-compatible) | ~6 EUR |
| **Total** | | **~36 EUR** |

---

## Setup Instructions

### Step 1: Create Terraform Cloud Account

1. Sign up at https://app.terraform.io/signup (free)
2. Create organization: `bdp-project`
3. Create workspace: `bdp-mvp`
4. Set Execution Mode to **Local** (in workspace settings)
5. Generate API token: User Settings → Tokens → Create API token

### Step 2: Get OVH API Credentials

1. Go to https://api.ovh.com/createToken
2. Set permissions:
   ```
   GET    /cloud/project/*
   POST   /cloud/project/*
   PUT    /cloud/project/*
   DELETE /cloud/project/*
   ```
3. Save the following values:
   - Application Key
   - Application Secret
   - Consumer Key

4. Get your Project ID from OVH Control Panel → Public Cloud

### Step 3: Create OpenStack User

1. Go to OVH Control Panel → Public Cloud → Users & Roles
2. Create a new user with "Administrator" role
3. Save the username and password

### Step 4: Configure GitHub Environment

1. Go to repo Settings → Environments
2. Create environment named `production`
3. Add **Required Reviewers** (your GitHub username)
4. Add these **Environment Secrets**:

| Secret | Description |
|--------|-------------|
| `TF_API_TOKEN` | Terraform Cloud API token |
| `OVH_APPLICATION_KEY` | OVH API key |
| `OVH_APPLICATION_SECRET` | OVH API secret |
| `OVH_CONSUMER_KEY` | OVH consumer key |
| `OVH_PROJECT_ID` | Public Cloud project ID |
| `OPENSTACK_USER_NAME` | OpenStack username |
| `OPENSTACK_PASSWORD` | OpenStack password |
| `SSH_PUBLIC_KEY` | Your SSH public key (`cat ~/.ssh/id_ed25519.pub`) |

### Step 5: Deploy Infrastructure

**Option A: Via GitHub Actions (Recommended)**

1. Go to Actions → Infrastructure
2. Click "Run workflow"
3. Select action: `plan` (to preview changes)
4. Review the plan output
5. Run again with action: `apply`
6. Approve the deployment when prompted

**Option B: Local Deployment**

```bash
cd infrastructure

# Login to Terraform Cloud
terraform login

# Create your variables file
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your values

# Initialize
terraform init

# Preview changes
terraform plan

# Deploy
terraform apply
```

### Step 6: Configure the Server

After Terraform completes:

```bash
# Get the instance IP
terraform output instance_ip

# SSH into the server
ssh ubuntu@<instance_ip>

# Run the setup script
curl -sSL https://raw.githubusercontent.com/YOUR_ORG/bdp/main/infrastructure/deploy/setup.sh | sudo bash

# Copy your .env file
scp .env ubuntu@<instance_ip>:/opt/bdp/.env

# Copy docker-compose file
scp infrastructure/deploy/docker-compose.prod.yml ubuntu@<instance_ip>:/opt/bdp/

# Start the application
ssh ubuntu@<instance_ip> "sudo systemctl start bdp"
```

### Step 7: Configure DNS and SSL

1. Point your domain to the instance IP
2. SSH into the server and configure Caddy:
   ```bash
   sudo nano /etc/caddy/Caddyfile
   ```
3. Replace `your-domain.com` with your actual domain
4. Reload Caddy:
   ```bash
   sudo systemctl reload caddy
   ```

Caddy will automatically obtain and renew SSL certificates.

---

## Useful Commands

```bash
# Check infrastructure status
just infra-status

# View Terraform outputs
just infra-output

# Generate .env file from outputs
just infra-env

# SSH into the instance
just infra-ssh

# Destroy all resources (be careful!)
just infra-destroy
```

## Troubleshooting

### Terraform Cloud connection issues

```bash
# Re-authenticate
terraform logout
terraform login
```

### OVH API errors

- Verify your API token permissions
- Check that Project ID is correct
- Ensure OpenStack user has Administrator role

### Instance not accessible

- Check security groups allow SSH (port 22)
- Verify your SSH key is correct
- Check OVH firewall settings in control panel

### Database connection issues

- Verify IP whitelist includes instance IP
- Check database credentials in outputs
- Ensure database is in "running" state
