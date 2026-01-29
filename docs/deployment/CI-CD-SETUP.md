# CI/CD Deployment Setup

Complete guide to automated deployment with GitHub Actions.

## 🔑 SSH Keys Overview

You have **2 SSH key pairs** for different purposes:

### 1. Personal Key (`bdp-production`)
**Location**: `C:\Users\sebas\.ssh\bdp-production`

**Purpose**: Your manual SSH access to the server

**Used by**: You (from your computer)

**How to use**:
```bash
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
```

### 2. Deploy Key (`bdp-deploy`)
**Location**: `C:\Users\sebas\.ssh\bdp-deploy`

**Purpose**: CI/CD automated deployments

**Used by**: GitHub Actions workflows

**Stored in**: GitHub secret `DEPLOY_SSH_PRIVATE_KEY`

---

## 🔐 GitHub Secrets (Production Environment)

All secrets are stored in the `production` environment. Check status:

```bash
gh secret list --env production
```

### Current Secrets ✅

| Secret | Purpose | Status |
|--------|---------|--------|
| `TF_API_TOKEN` | Terraform Cloud authentication | ✅ Set |
| `OVH_APPLICATION_KEY` | OVH API access | ✅ Set |
| `OVH_APPLICATION_SECRET` | OVH API access | ✅ Set |
| `OVH_CONSUMER_KEY` | OVH API access | ✅ Set |
| `OVH_PROJECT_ID` | OVH Public Cloud project | ✅ Set |
| `OPENSTACK_USER_NAME` | OpenStack (OVH compute) | ✅ Set |
| `OPENSTACK_PASSWORD` | OpenStack (OVH compute) | ✅ Set |
| `SSH_PUBLIC_KEY` | Your personal SSH public key | ✅ Set |
| `DEPLOY_SSH_PUBLIC_KEY` | CI/CD SSH public key | ✅ Set |
| `DEPLOY_SSH_PRIVATE_KEY` | CI/CD SSH private key | ✅ Set |

### Missing Secrets (Add After Deployment)

After running `terraform apply`, you need to add:

```bash
# Get server IP from Terraform
cd infrastructure
terraform output -raw instance_ip

# Set the secret
gh secret set PRODUCTION_SERVER_IP --env production --body "<server-ip>"

# Optional: Set domain if you have one
gh secret set PRODUCTION_DOMAIN --env production --body "bdp.example.com"
```

---

## 🚀 Deployment Workflows

### 1. Infrastructure Deployment (`infrastructure.yml`)

**Provisions** OVH Cloud resources (compute, database, storage).

**Triggers**:
- Automatic `plan` on PRs to `infrastructure/` files
- Manual `apply` or `destroy` via workflow dispatch

**What it does**:
- Creates OVH compute instance
- Sets up PostgreSQL database
- Configures S3 storage
- Adds **both SSH keys** to the server

**How to use**:
```bash
# Via GitHub UI:
# 1. Go to Actions → Infrastructure
# 2. Click "Run workflow"
# 3. Select action: "apply"
# 4. Approve deployment
```

### 2. Production Deployment (`deploy-production.yml`)

**Deploys** application code to the provisioned server.

**Triggers**:
- Manual via workflow dispatch
- (Optional) Automatic on push to main (currently commented out)

**What it does**:
1. Builds Docker images (backend + frontend)
2. Pushes to GitHub Container Registry
3. SSHs into server using `DEPLOY_SSH_PRIVATE_KEY`
4. Pulls images and restarts services
5. Runs health checks

**How to use**:
```bash
# Via GitHub UI:
# 1. Go to Actions → Deploy to Production
# 2. Click "Run workflow"
# 3. Select options:
#    - Deploy backend: Yes/No
#    - Deploy frontend: Yes/No
#    - Run migrations: Yes/No
# 4. Approve deployment
```

---

## 📋 Complete Deployment Flow

### Step 1: Provision Infrastructure

```bash
# GitHub Actions → Infrastructure → Run workflow
# Action: apply
# Wait for approval → Approve
```

**Result**: OVH server created with both SSH keys installed

### Step 2: Get Server IP

```bash
cd infrastructure
terraform output -raw instance_ip
# Example output: 51.178.12.34

# Set as GitHub secret
gh secret set PRODUCTION_SERVER_IP --env production --body "51.178.12.34"
```

### Step 3: Initial Server Setup (One-Time)

SSH into server and prepare for deployments:

```bash
# Connect to server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>

# Verify Docker is installed
docker --version
docker compose version

# Create .env file
cd /opt/bdp
nano .env
# Add your environment variables (DATABASE_URL, S3 credentials, etc.)
```

### Step 4: Deploy Application

```bash
# GitHub Actions → Deploy to Production → Run workflow
# Options:
#   - Deploy backend: ✅
#   - Deploy frontend: ✅
#   - Run migrations: ✅ (first time only)
# Wait for approval → Approve
```

### Step 5: Verify Deployment

```bash
# Check application status
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
cd /opt/bdp
docker compose ps
docker compose logs -f
```

---

## 🔄 Regular Deployment Cycle

After initial setup, deploying updates is simple:

1. **Push code to main** (or create PR and merge)
2. **Go to Actions → Deploy to Production**
3. **Run workflow** with desired options
4. **Approve** when prompted
5. **Done!** Application updates automatically

---

## 🛡️ Security Model

### How SSH Keys Work

```
┌─────────────────────────────────────────────────────────────┐
│                       OVH Cloud Server                       │
│  /home/ubuntu/.ssh/authorized_keys contains:                 │
│  1. ssh-ed25519 AAAA...xyz bdp-production (your key)        │
│  2. ssh-ed25519 AAAA...abc bdp-ci-deploy (CI/CD key)        │
└─────────────────────────────────────────────────────────────┘
                        ↑                    ↑
                        │                    │
        ┌───────────────┘                    └──────────────────┐
        │                                                        │
┌───────┴────────┐                                   ┌──────────┴─────────┐
│  Your Computer │                                   │  GitHub Actions     │
│  Private key:  │                                   │  Private key:       │
│  bdp-production│                                   │  DEPLOY_SSH_PRIVATE │
│                │                                   │  (secret)           │
│  Used for:     │                                   │                     │
│  Manual access │                                   │  Used for:          │
│                │                                   │  Automated deploys  │
└────────────────┘                                   └─────────────────────┘
```

### Why This is Secure

1. **Separation of Concerns**: Human access vs automation access
2. **Revocation**: Can revoke CI/CD key without affecting your access
3. **Audit Trail**: All deployments logged in GitHub Actions
4. **Secrets Encryption**: GitHub encrypts all secrets at rest
5. **Environment Protection**: Manual approval required for production

### Private Key Storage

| Key | Stored Where | Encrypted? | Access |
|-----|--------------|------------|--------|
| Personal private key | Your computer | No (protected by file permissions) | Only you |
| Deploy private key (copy 1) | Your computer | No (backup) | Only you |
| Deploy private key (copy 2) | GitHub secrets | Yes (AES-256) | GitHub Actions only |

---

## 🧪 Testing the Setup

### Test Manual SSH Access

```bash
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
```

Expected: Successfully connects

### Test CI/CD SSH Access

```bash
# Simulate GitHub Actions SSH
ssh -i C:\Users\sebas\.ssh\bdp-deploy ubuntu@<server-ip>
```

Expected: Successfully connects (both keys work)

### Test Deployment Workflow

```bash
# Trigger a test deployment
# GitHub Actions → Deploy to Production
# Deploy backend: No
# Deploy frontend: No
# Run migrations: No
```

Expected: Workflow runs, connects to server, completes successfully

---

## 🔧 Troubleshooting

### "Permission denied (publickey)"

**Cause**: SSH key not on server or wrong key used

**Fix**:
```bash
# Check which keys are on the server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
cat ~/.ssh/authorized_keys
```

Should show both `bdp-production` and `bdp-ci-deploy` keys.

### "Host key verification failed"

**Cause**: Server's host key not in known_hosts

**Fix**: The deployment workflow handles this automatically with `ssh-keyscan`. For manual connections:
```bash
ssh-keyscan -H <server-ip> >> ~/.ssh/known_hosts
```

### Docker images not pulling

**Cause**: Not logged into GitHub Container Registry

**Fix**: The deployment workflow handles this automatically. For manual deployment:
```bash
echo "${{ secrets.GITHUB_TOKEN }}" | docker login ghcr.io -u USERNAME --password-stdin
```

---

## 📚 Additional Resources

- [GitHub Actions Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [GitHub Container Registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)
- [SSH Key Management](https://www.ssh.com/academy/ssh/keygen)
- [Docker Compose](https://docs.docker.com/compose/)

---

## 🎯 Quick Reference

### View Secrets
```bash
gh secret list --env production
```

### Set Secret
```bash
gh secret set SECRET_NAME --env production --body "value"
```

### Get Server IP
```bash
cd infrastructure && terraform output -raw instance_ip
```

### Connect to Server (Personal)
```bash
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
```

### Connect to Server (CI/CD key)
```bash
ssh -i C:\Users\sebas\.ssh\bdp-deploy ubuntu@<server-ip>
```

### View Application Logs
```bash
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
cd /opt/bdp
docker compose logs -f
```

### Restart Services
```bash
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
cd /opt/bdp
docker compose restart
```
