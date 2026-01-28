# Infrastructure Security Guide

This document explains the security model for BDP's infrastructure management in an open source context.

## Security Model Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        GitHub Repository                             │
│                         (Public/Open Source)                         │
├─────────────────────────────────────────────────────────────────────┤
│  infrastructure/                                                     │
│  ├── *.tf files          ← Public (no secrets)                      │
│  ├── terraform.tfvars    ← .gitignored (never committed)            │
│  └── *.tfstate           ← Stored in Terraform Cloud (encrypted)    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  GitHub Environment: "production"                                    │
│  ├── Required Reviewers: [maintainer1, maintainer2]                 │
│  ├── Secrets: (encrypted, only accessible to environment)           │
│  │   ├── TF_API_TOKEN                                               │
│  │   ├── OVH_APPLICATION_KEY                                        │
│  │   ├── OVH_APPLICATION_SECRET                                     │
│  │   ├── OVH_CONSUMER_KEY                                           │
│  │   ├── OVH_PROJECT_ID                                             │
│  │   ├── OPENSTACK_USER_NAME                                        │
│  │   ├── OPENSTACK_PASSWORD                                         │
│  │   └── SSH_PUBLIC_KEY                                             │
│  └── Protection Rules:                                              │
│      ├── Fork PRs: Cannot access secrets                            │
│      ├── Plan: Runs automatically, no approval needed               │
│      └── Apply/Destroy: Requires maintainer approval                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Terraform Cloud (Free)                           │
│                                                                      │
│  Organization: bdp-project                                          │
│  Workspace: bdp-mvp                                                 │
│  ├── State: Encrypted at rest                                       │
│  ├── Access: Organization members only                              │
│  ├── Audit Log: All state changes logged                            │
│  └── Locking: Prevents concurrent modifications                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Why This Setup is Secure

### 1. Fork PRs Cannot Access Secrets

GitHub Environment secrets are **not available** to workflows triggered by fork PRs. The workflow explicitly checks for forks and blocks execution:

```yaml
security-check:
  steps:
    - name: Check if fork
      run: |
        if [ "${{ github.event.pull_request.head.repo.fork }}" == "true" ]; then
          echo "is_fork=true"  # Workflow will be blocked
        fi
```

### 2. State Never Touches Git

Terraform state contains sensitive information:
- Database passwords
- S3 access keys
- Instance IP addresses

By using Terraform Cloud, state is:
- Stored remotely (never in repo)
- Encrypted at rest (AES-256)
- Access controlled (org members only)
- Audit logged (who accessed when)

### 3. Manual Approval for Destructive Actions

| Action | Trigger | Approval Required |
|--------|---------|-------------------|
| `plan` | PR or manual | No |
| `apply` | Manual only | Yes (maintainer) |
| `destroy` | Manual only | Yes + type "destroy" |

### 4. Secrets in Environment, Not Repo

**Repository Secrets** (❌ Don't use for infra):
- Visible to all workflows
- Can be accessed by fork PRs in some cases

**Environment Secrets** (✅ Use this):
- Only accessible to workflows using that environment
- Can require approval before access
- Not visible to forks

## Setup Instructions

### 1. Create Terraform Cloud Account

1. Sign up at https://app.terraform.io/signup (free)
2. Create organization (e.g., `bdp-project`)
3. Create workspace `bdp-mvp`
4. Set Execution Mode to **Local** (in workspace settings)
5. Generate API token: User Settings → Tokens → Create API token

### 2. Configure GitHub Environment

1. Go to repo Settings → Environments
2. Create environment named `production`
3. Add **Required Reviewers** (your GitHub username)
4. Add these **Environment Secrets**:

| Secret | Description | Where to get |
|--------|-------------|--------------|
| `TF_API_TOKEN` | Terraform Cloud API token | app.terraform.io → User Settings → Tokens |
| `OVH_APPLICATION_KEY` | OVH API key | api.ovh.com/createToken |
| `OVH_APPLICATION_SECRET` | OVH API secret | api.ovh.com/createToken |
| `OVH_CONSUMER_KEY` | OVH consumer key | api.ovh.com/createToken |
| `OVH_PROJECT_ID` | Public Cloud project ID | OVH Control Panel |
| `OPENSTACK_USER_NAME` | OpenStack username | OVH → Public Cloud → Users |
| `OPENSTACK_PASSWORD` | OpenStack password | OVH → Public Cloud → Users |
| `SSH_PUBLIC_KEY` | Your SSH public key | `cat ~/.ssh/id_ed25519.pub` |

### 3. Configure OVH API Token Permissions

When creating OVH API token, set these permissions:

```
GET    /cloud/project/*
POST   /cloud/project/*
PUT    /cloud/project/*
DELETE /cloud/project/*
```

## Workflow Usage

### Plan (Preview Changes)

**Automatic on PR:**
```bash
# Create PR with infrastructure changes
git checkout -b infra/update-instance-size
# Edit infrastructure/*.tf
git commit -am "Increase instance size"
git push origin infra/update-instance-size
# Open PR - plan runs automatically
```

**Manual:**
1. Go to Actions → Infrastructure
2. Click "Run workflow"
3. Select action: `plan`
4. Click "Run workflow"

### Apply (Deploy Changes)

1. Go to Actions → Infrastructure
2. Click "Run workflow"
3. Select action: `apply`
4. Click "Run workflow"
5. **Wait for approval notification**
6. Approve in the pending deployment
7. Deployment proceeds

### Destroy (Remove All Resources)

1. Go to Actions → Infrastructure
2. Click "Run workflow"
3. Select action: `destroy`
4. Type `destroy` in confirmation field
5. Click "Run workflow"
6. **Wait for approval notification**
7. Approve (double-check you want this!)

## Local Development

For local testing (maintainers only):

```bash
cd infrastructure

# Login to Terraform Cloud
terraform login

# Copy and fill in variables
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your values

# Initialize
terraform init

# Plan
terraform plan

# Apply (creates resources)
terraform apply
```

## Incident Response

### If Secrets Are Exposed

1. **Immediately rotate** all exposed credentials:
   - OVH: Regenerate API token at api.ovh.com
   - Terraform Cloud: Regenerate API token
   - SSH: Generate new keypair

2. **Update GitHub Environment secrets** with new values

3. **Check Terraform Cloud audit log** for unauthorized access

4. **Review OVH activity logs** for suspicious activity

### If State Is Compromised

1. State in Terraform Cloud is encrypted, but if you suspect issues:
2. Rotate all credentials in the state (database password, S3 keys)
3. Run `terraform apply` to update resources with new credentials
4. Contact Terraform Cloud support if needed

## Security Checklist

- [ ] Terraform Cloud organization created
- [ ] Workspace set to Local execution mode
- [ ] GitHub Environment "production" created
- [ ] Required reviewers configured
- [ ] All secrets added to Environment (not repo)
- [ ] `.gitignore` includes `terraform.tfvars` and `*.tfstate*`
- [ ] Fork PR protection verified
- [ ] OVH API token has minimal required permissions
