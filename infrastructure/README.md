# BDP Infrastructure - OVH Cloud

Terraform configuration for deploying BDP to OVH Cloud.

## Architecture (MVP)

```
┌─────────────────────────────────────────────────────────────┐
│                        Internet                              │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              OVH Public Cloud Instance                       │
│                    (d2-2: 1 vCPU, 2GB RAM)                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Caddy (reverse proxy, auto HTTPS)                  │   │
│  │    ├── :443 → Next.js (:3000)                       │   │
│  │    └── :443/api → Rust Backend (:8000)              │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌──────────────────┐  ┌───────────────────────────────┐   │
│  │  Next.js Frontend │  │  Rust Backend (axum)         │   │
│  │  (Docker)         │  │  (Docker)                    │   │
│  └──────────────────┘  └───────────────────────────────┘   │
└─────────────────────────┬───────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┐
          ▼                               ▼
┌──────────────────────┐    ┌─────────────────────────────────┐
│  OVH Managed         │    │  OVH Object Storage              │
│  PostgreSQL          │    │  (S3-compatible)                 │
│  (Essential Plan)    │    │                                  │
│  - 1 node            │    │  - Data files                    │
│  - 4GB RAM           │    │  - No egress fees                │
│  - Auto backups      │    │                                  │
└──────────────────────┘    └─────────────────────────────────┘
```

## Estimated Monthly Cost (MVP)

| Component | Service | Cost |
|-----------|---------|------|
| Compute | d2-2 (1 vCPU, 2GB RAM) | ~5 EUR |
| Database | PostgreSQL Essential db1-4 | ~30 EUR |
| Storage | Object Storage 100GB | ~1 EUR |
| Bandwidth | Included (no egress fees!) | 0 EUR |
| **Total** | | **~36 EUR/month** |

## Prerequisites

1. **OVH Account** with Public Cloud project
2. **API Credentials** from https://api.ovh.com/createToken/
3. **OpenStack User** created in OVH Control Panel
4. **SSH Key** for instance access
5. **Terraform** >= 1.0.0 installed

## Quick Start

### 1. Get OVH API Credentials

1. Go to https://api.ovh.com/createToken/
2. Set permissions:
   - GET, POST, PUT, DELETE on `/cloud/project/*`
3. Save Application Key, Application Secret, and Consumer Key

### 2. Get OpenStack Credentials

1. OVH Control Panel → Public Cloud → Users & Roles
2. Create User with "Administrator" role
3. Note the username and password

### 3. Configure Terraform

```bash
cd infrastructure

# Copy example variables
cp terraform.tfvars.example terraform.tfvars

# Edit with your values
nano terraform.tfvars  # or your preferred editor
```

### 4. Deploy

```bash
# Initialize Terraform
terraform init

# Preview changes
terraform plan

# Apply (creates resources)
terraform apply

# Get connection info
terraform output
terraform output -raw env_file_content > ../production.env
```

### 5. Connect to Instance

```bash
# SSH into the server
ssh ubuntu@$(terraform output -raw instance_ip)

# Check Docker is running
docker --version
docker compose version
```

### 6. Deploy Application

```bash
# On your local machine, copy files to server
scp -r ../docker ubuntu@$(terraform output -raw instance_ip):/opt/bdp/
scp ../docker-compose.yml ubuntu@$(terraform output -raw instance_ip):/opt/bdp/
scp ../production.env ubuntu@$(terraform output -raw instance_ip):/opt/bdp/.env

# SSH into server
ssh ubuntu@$(terraform output -raw instance_ip)

# Deploy
cd /opt/bdp
docker compose up -d
```

## Files

| File | Description |
|------|-------------|
| `main.tf` | Provider configuration |
| `variables.tf` | Input variables |
| `compute.tf` | Instance and security groups |
| `database.tf` | Managed PostgreSQL |
| `storage.tf` | S3 object storage credentials |
| `outputs.tf` | Connection information |
| `terraform.tfvars.example` | Example variable values |

## Useful Commands

```bash
# View all outputs
terraform output

# Get database password
terraform output -raw database_password

# Get S3 credentials
terraform output -raw s3_access_key
terraform output -raw s3_secret_key

# Generate .env file
terraform output -raw env_file_content > .env

# SSH command
terraform output -raw ssh_command

# Destroy everything (careful!)
terraform destroy
```

## Scaling Up (Post-MVP)

When ready to scale:

1. **More CPU/RAM**: Change `instance_flavor` to `d2-4` or `d2-8`
2. **Database HA**: Change `db_plan` to `business` (2 nodes)
3. **Load Balancer**: Uncomment load balancer resources (create `loadbalancer.tf`)
4. **Multiple Instances**: Create instance count variable
5. **Kubernetes**: Consider migrating to OVH Managed Kubernetes

## Security Notes

- Database only accessible from instance IP
- SSH key authentication only (no passwords)
- HTTPS via Caddy with auto Let's Encrypt
- Consider adding firewall rules for production

## Troubleshooting

### Cannot connect to database
1. Check IP restriction includes instance IP
2. Verify SSL mode is `require`
3. Check password in Terraform output

### Instance not accessible
1. Check security group rules
2. Verify SSH key is correct
3. Check instance status in OVH Control Panel

### S3 bucket not found
1. Create bucket manually via AWS CLI or OVH Control Panel
2. Bucket is created on first application use

## Support

- OVH Documentation: https://help.ovhcloud.com/
- Terraform OVH Provider: https://registry.terraform.io/providers/ovh/ovh/latest/docs
- OVH API Console: https://api.ovh.com/console/
