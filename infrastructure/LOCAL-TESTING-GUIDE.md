# Local Terraform Testing Guide

Complete guide for testing Terraform configuration locally before deploying.

## 🎯 Overview

**Current Setup**: Local state file (for testing)
**Next Step**: Migrate to Terraform Cloud (for production)

---

## ✅ Current Configuration Status

### **Files Ready**
- ✅ `backend.tf` - Configured for local state (Terraform Cloud commented out)
- ✅ `terraform.tfvars` - SSH keys configured, OVH credentials pending
- ✅ `variables.tf` - Fixed instance flavor comment (2 vCPU, 4GB RAM)
- ✅ All resource files - Cheapest options selected

### **SSH Keys Configured**
- ✅ Personal key: `ssh-ed25519 AAAAC3...Wz9z bdp-production`
- ✅ CI/CD key: `ssh-ed25519 AAAAC3...qNUH bdp-ci-deploy`

### **Terraform Cloud Info (for later)**
- Organization: `datadir`
- Workspace: `bdp`

---

## 💰 Cost Configuration (CHEAPEST OPTIONS)

| Resource | Selection | Monthly Cost | Why Cheapest |
|----------|-----------|--------------|--------------|
| Compute | `d2-2` (2 vCPU, 4GB RAM) | ~€5-12 | Discovery line - smallest instance |
| Database | `essential` + `db1-4` | ~€30 | Essential plan (no HA), smallest flavor |
| Storage | S3-compatible (pay-per-GB) | ~€1 | Pay only for what you use |
| **TOTAL** | | **~€36-43/month** | Optimal for MVP |

### Could It Be Cheaper?

**❌ Cheaper compute?** No - d2-2 is OVH's smallest/cheapest option
**❌ Cheaper database?** No - Essential + db1-4 is minimum
**❌ Skip managed DB?** Not recommended - self-hosted PostgreSQL adds complexity
**✅ OVH Grant?** YES! Apply for startup credits (12-24 months free)

---

## 🚫 What's Missing (BLOCKER)

You cannot deploy yet because you need **OVH credentials**:

```bash
# Required from OVH (after grant approval):
ovh_application_key     = "..."
ovh_application_secret  = "..."
ovh_consumer_key        = "..."
ovh_project_id          = "..."
openstack_user_name     = "..."
openstack_password      = "..."
```

**Next Step**: Complete Linear task BDP-8 (OVH Grant Application)

---

## 🧪 Local Dry-Run (Without OVH Credentials)

You can still test Terraform syntax and configuration:

### Step 1: Install Terraform (if not installed)

**Windows (PowerShell as Administrator)**:
```powershell
# Install via Chocolatey
choco install terraform

# OR download from: https://www.terraform.io/downloads
```

**Verify installation**:
```bash
terraform version
# Expected: Terraform v1.6.0 or higher
```

### Step 2: Initialize Terraform

```bash
cd infrastructure

# Initialize (downloads providers)
terraform init

# Expected output:
# Initializing the backend...
# Initializing provider plugins...
# - Finding ovh/ovh versions matching ">= 0.40.0"...
# - Finding terraform-provider-openstack/openstack versions matching "~> 1.49.0"...
# Terraform has been successfully initialized!
```

### Step 3: Validate Configuration

```bash
# Check syntax and configuration
terraform validate

# Expected output:
# Success! The configuration is valid.
```

### Step 4: Format Code

```bash
# Format all .tf files
terraform fmt

# Check formatting without changes
terraform fmt -check
```

### Step 5: Dry-Run (Plan) - WILL FAIL

```bash
# This will fail because OVH credentials are placeholders
terraform plan

# Expected error:
# Error: Invalid OVH Application Key
# This is NORMAL - you don't have credentials yet!
```

---

## ✅ What You Can Do Now (Without OVH)

### 1. Verify Configuration Files

```bash
cd infrastructure

# Check all files are present
ls -la

# Should see:
# main.tf, variables.tf, compute.tf, database.tf, storage.tf,
# outputs.tf, backend.tf, terraform.tfvars
```

### 2. Review Resource Definitions

```bash
# Check compute configuration
cat compute.tf

# Check database configuration
cat database.tf

# Check your variables
cat terraform.tfvars
```

### 3. Verify SSH Keys Are Set

```bash
# Check SSH keys in tfvars
grep "ssh_public_key" terraform.tfvars

# Should show your actual keys (ssh-ed25519 AAAA...)
```

### 4. Estimate Costs

```bash
# Review cost estimates
cat README.md | grep -A 10 "Monthly Cost"

# Your configuration:
# - d2-2: €5-12/month (CHEAPEST compute)
# - essential db1-4: €30/month (CHEAPEST database)
# - S3 storage: ~€1/month
# TOTAL: ~€36-43/month
```

---

## 🚀 When You Get OVH Credentials

Once you receive OVH grant approval and credentials:

### Step 1: Update terraform.tfvars

```bash
cd infrastructure
nano terraform.tfvars  # or your preferred editor

# Replace all "REPLACE_WITH_..." placeholders with real values
```

### Step 2: Test Plan

```bash
# Run terraform plan
terraform plan

# Review the output carefully:
# - Check resource counts (1 instance, 1 database, 1 storage user)
# - Verify security groups look correct
# - Check estimated costs
```

### Step 3: Apply (Deploy)

```bash
# Deploy infrastructure (this creates real resources!)
terraform apply

# Review the plan again
# Type "yes" to confirm

# Wait 5-10 minutes for resources to provision
```

### Step 4: Get Outputs

```bash
# Get server IP
terraform output instance_ip

# Get database connection string
terraform output database_uri

# Get all sensitive outputs
terraform output -json > outputs.json

# Generate .env file for application
terraform output -raw env_file_content > ../production.env
```

### Step 5: Connect to Server

```bash
# SSH into your new server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@$(terraform output -raw instance_ip)

# Verify provisioning
cat /opt/bdp/provisioned.txt
# Should say: "BDP MVP server provisioned successfully"

# Check Docker
docker --version
docker compose version
```

---

## 🔄 Migration to Terraform Cloud (Later)

After testing locally, migrate to Terraform Cloud for production:

### Step 1: Setup Terraform Cloud

1. Create account: https://app.terraform.io/signup
2. Create organization: `datadir`
3. Create workspace: `bdp`
4. Set Execution Mode: `Local`
5. Generate API token

### Step 2: Update Configuration

```bash
cd infrastructure

# Edit backend.tf
# Uncomment the Terraform Cloud block (lines with organization = "datadir")
```

### Step 3: Migrate State

```bash
# Login to Terraform Cloud
terraform login

# Migrate local state to cloud
terraform init -migrate-state

# Confirm migration when prompted
```

### Step 4: Verify Migration

```bash
# Check state is in cloud
terraform state list

# Your local terraform.tfstate should now be empty/backup
```

---

## 📋 Pre-Deployment Checklist

Before running `terraform apply`:

- [ ] Terraform installed and working (`terraform version`)
- [ ] OVH credentials obtained (grant approved)
- [ ] `terraform.tfvars` updated with real OVH credentials
- [ ] SSH keys verified in `terraform.tfvars`
- [ ] `terraform init` completed successfully
- [ ] `terraform validate` passes
- [ ] `terraform plan` shows expected resources
- [ ] Cost estimate acceptable (~€36-43/month)
- [ ] Ready to deploy!

---

## 🛠️ Useful Commands

```bash
# Validate configuration
terraform validate

# Format code
terraform fmt

# Plan (dry-run)
terraform plan

# Plan and save to file
terraform plan -out=tfplan

# Apply saved plan
terraform apply tfplan

# Apply with auto-approve (careful!)
terraform apply -auto-approve

# Show current state
terraform show

# List resources
terraform state list

# Get outputs
terraform output

# Get specific output
terraform output instance_ip

# Destroy everything (careful!)
terraform destroy
```

---

## 🐛 Troubleshooting

### "Error: Invalid OVH credentials"

**Cause**: Placeholder values in `terraform.tfvars`

**Fix**: Wait for OVH grant approval, then update with real credentials

### "Terraform not found"

**Cause**: Terraform not installed or not in PATH

**Fix**:
```bash
# Windows - Install via Chocolatey
choco install terraform

# Or download from terraform.io/downloads
```

### "Error: Failed to load backend"

**Cause**: Trying to use Terraform Cloud without setup

**Fix**: Keep using local backend (current configuration is correct)

### "Error: Resource already exists"

**Cause**: Terraform state out of sync with real resources

**Fix**:
```bash
# Import existing resource
terraform import <resource_type>.<name> <id>

# Or destroy and recreate
terraform destroy
terraform apply
```

---

## 📚 Next Steps

1. ✅ **Current**: Local testing configuration ready
2. ⏳ **Blocked**: Waiting for OVH grant approval (Linear BDP-8)
3. 🔜 **Next**: Update `terraform.tfvars` with OVH credentials
4. 🚀 **Deploy**: Run `terraform apply` when ready
5. 🌐 **Migrate**: Move to Terraform Cloud for production

---

## 🔗 Resources

- [Terraform Documentation](https://www.terraform.io/docs)
- [OVH Cloud Provider](https://registry.terraform.io/providers/ovh/ovh/latest/docs)
- [OpenStack Provider](https://registry.terraform.io/providers/terraform-provider-openstack/openstack/latest/docs)
- [OVH API Console](https://api.ovh.com/console/)
- [Terraform Cloud](https://app.terraform.io)
