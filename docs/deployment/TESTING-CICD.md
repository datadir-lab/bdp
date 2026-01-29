# Complete CI/CD Testing & Monitoring Guide

End-to-end guide for testing the entire deployment pipeline and monitoring the production system.

## 🎯 Overview

This guide covers:
1. Environment variable setup
2. Testing infrastructure deployment
3. Testing application deployment
4. Monitoring and validation
5. Troubleshooting

---

## 📋 Prerequisites

Before testing, ensure all secrets are configured:

### Required GitHub Secrets (Production Environment)

```bash
# Check what's configured
gh secret list --env production
```

#### Infrastructure Secrets ✅
- `TF_API_TOKEN` - Terraform Cloud
- `OVH_APPLICATION_KEY` - OVH API
- `OVH_APPLICATION_SECRET` - OVH API
- `OVH_CONSUMER_KEY` - OVH API
- `OVH_PROJECT_ID` - OVH Project
- `OPENSTACK_USER_NAME` - OpenStack
- `OPENSTACK_PASSWORD` - OpenStack
- `SSH_PUBLIC_KEY` - Your SSH key
- `DEPLOY_SSH_PUBLIC_KEY` - CI/CD SSH key
- `DEPLOY_SSH_PRIVATE_KEY` - CI/CD SSH key

#### Application Secrets ⏳ (Set after Terraform deployment)

After running `terraform apply`, you need to set these from Terraform outputs:

```bash
cd infrastructure

# Get all outputs
terraform output -json > outputs.json

# Set each secret
gh secret set DATABASE_URL --env production --body "$(terraform output -raw database_uri):$(terraform output -raw database_password)"

gh secret set STORAGE_S3_ENDPOINT --env production --body "$(terraform output -raw s3_endpoint)"
gh secret set STORAGE_S3_REGION --env production --body "$(terraform output -raw s3_region)"
gh secret set STORAGE_S3_BUCKET --env production --body "$(terraform output -raw s3_bucket)"
gh secret set STORAGE_S3_ACCESS_KEY --env production --body "$(terraform output -raw s3_access_key)"
gh secret set STORAGE_S3_SECRET_KEY --env production --body "$(terraform output -raw s3_secret_key)"

gh secret set PRODUCTION_SERVER_IP --env production --body "$(terraform output -raw instance_ip)"

# Optional: Domain name
gh secret set PRODUCTION_DOMAIN --env production --body "your-domain.com"

# Optional: Ingestion settings (defaults work if not set)
gh secret set INGEST_ENABLED --env production --body "true"
gh secret set INGEST_SCHEDULE --env production --body "0 2 * * *"
```

---

## 🧪 Phase 1: Test Infrastructure Deployment

### Step 1: Verify Terraform Configuration Locally

```bash
cd infrastructure

# Initialize
terraform init

# Validate
terraform validate
# Expected: Success! The configuration is valid.

# Format check
terraform fmt -check

# Plan (will show what will be created)
terraform plan
```

### Step 2: Deploy Infrastructure via GitHub Actions

```
1. Go to: https://github.com/datadir-lab/bdp/actions/workflows/infrastructure.yml
2. Click "Run workflow"
3. Select action: "plan"
4. Wait for completion
5. Review the plan output
6. Run again with action: "apply"
7. Approve the deployment when prompted
8. Wait 5-10 minutes for resources to provision
```

### Step 3: Verify Infrastructure

```bash
cd infrastructure

# Get server IP
terraform output instance_ip
# Example: 51.178.12.34

# Get database connection
terraform output database_uri

# Get all outputs
terraform output
```

### Step 4: Set Application Secrets

```bash
# Run the commands from "Application Secrets" section above
# This populates DATABASE_URL, S3 credentials, etc.
```

### Step 5: Test SSH Access

```bash
# Test with personal key
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@$(cd infrastructure && terraform output -raw instance_ip)

# Check provisioning
cat /opt/bdp/provisioned.txt
# Should say: "BDP MVP server provisioned successfully"

# Verify Docker
docker --version
docker compose version

# Verify both SSH keys are installed
cat ~/.ssh/authorized_keys
# Should show both bdp-production and bdp-ci-deploy keys

# Exit
exit
```

---

## 🚀 Phase 2: Test Application Deployment

### Step 1: Trigger Deployment Workflow

```
1. Go to: https://github.com/datadir-lab/bdp/actions/workflows/deploy-production.yml
2. Click "Run workflow"
3. Configure options:
   - Deploy backend: ✅ Yes
   - Deploy frontend: ✅ Yes
   - Run migrations: ✅ Yes (first time only)
4. Click "Run workflow"
5. Approve the deployment when prompted
6. Wait for workflow to complete (~5-10 minutes)
```

### Step 2: Monitor Deployment

Watch the workflow in real-time:

```
GitHub Actions > Deploy to Production > Latest run
```

Check each step:
- ✅ Build backend image
- ✅ Build frontend image
- ✅ Push to GitHub Container Registry
- ✅ Generate .env file
- ✅ Copy files to server
- ✅ Deploy application
- ✅ Health check
- ✅ Deployment summary

### Step 3: Verify Deployment on Server

```bash
# SSH into server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>

# Check running containers
cd /opt/bdp
docker compose ps

# Expected output:
# NAME        IMAGE                              STATUS
# bdp-server  ghcr.io/datadir-lab/bdp-server:latest   Up (healthy)
# bdp-web     ghcr.io/datadir-lab/bdp-web:latest      Up

# Check logs
docker compose logs -f

# Check individual service logs
docker compose logs bdp-server
docker compose logs bdp-web

# Verify .env file exists
ls -la .env
cat .env  # Verify variables are set correctly
```

---

## 🔍 Phase 3: Validation & Testing

### Test 1: Backend Health Check

```bash
# From server
curl http://localhost:8000/health

# From your computer (if ports are open)
curl http://<server-ip>:8000/health

# Expected response:
# {"status":"healthy","database":"connected","storage":"available"}
```

### Test 2: Frontend Access

```bash
# From server
curl http://localhost:3000

# From your computer
curl http://<server-ip>:3000

# Expected: HTML response from Next.js
```

### Test 3: Database Connection

```bash
# SSH into server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>

# Run database migration status
cd /opt/bdp
docker compose exec bdp-server sqlx migrate info

# Expected: List of applied migrations
```

### Test 4: S3 Storage

```bash
# Check S3 connectivity from server
docker compose exec bdp-server /bin/sh -c "echo 'test' > /tmp/test.txt && \
  # Try to upload to S3 (if application has upload endpoint)"

# Or check environment variables
docker compose exec bdp-server env | grep STORAGE
```

### Test 5: Ingestion Jobs (if enabled)

```bash
# Check ingestion configuration
docker compose exec bdp-server env | grep INGEST

# Manually trigger ingestion (if endpoint exists)
curl -X POST http://localhost:8000/api/admin/ingest/trigger

# Check ingestion logs
docker compose logs bdp-server | grep -i ingest
```

---

## 📊 Phase 4: Monitoring

### Real-Time Monitoring

```bash
# SSH into server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>

# Monitor all container logs
cd /opt/bdp
docker compose logs -f

# Monitor specific service
docker compose logs -f bdp-server

# Monitor with timestamps
docker compose logs -f --timestamps

# Monitor last 100 lines
docker compose logs --tail=100 -f
```

### Resource Monitoring

```bash
# Check system resources
htop

# Check Docker container stats
docker stats

# Check disk usage
df -h

# Check memory
free -h

# Check Docker disk usage
docker system df
```

### Application Metrics

```bash
# Check container health
docker compose ps

# Check container details
docker inspect bdp-server

# Check restart count
docker inspect bdp-server | grep RestartCount

# Check when container started
docker inspect bdp-server | grep StartedAt
```

### Log Analysis

```bash
# Search for errors
docker compose logs | grep -i error

# Search for warnings
docker compose logs | grep -i warn

# Count errors per service
docker compose logs bdp-server | grep -i error | wc -l

# Get last 1000 lines
docker compose logs --tail=1000 > logs.txt
```

---

## 🔄 Phase 5: Testing Updates

### Test Configuration Update

```bash
# Update a GitHub secret
gh secret set INGEST_ENABLED --env production --body "true"

# Redeploy
# GitHub Actions > Deploy to Production > Run workflow
# Deploy backend: Yes
# Deploy frontend: No
# Run migrations: No

# Verify change on server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
docker compose exec bdp-server env | grep INGEST_ENABLED
# Should show: INGEST_ENABLED=true
```

### Test Code Update

```bash
# Make a code change locally
# Commit and push to main

# Trigger deployment
# GitHub Actions > Deploy to Production > Run workflow

# Verify new image version
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
docker images | grep bdp-server
# Check image ID and created time
```

### Test Rollback

```bash
# SSH into server
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip>
cd /opt/bdp

# Check current image
docker images | grep bdp-server

# Roll back to previous image
docker compose down
docker compose up -d --pull never  # Use existing/older image

# Or pull specific version
# Edit docker-compose.yml to use specific tag
# image: ghcr.io/datadir-lab/bdp-server:sha-abc123
docker compose up -d
```

---

## 🐛 Troubleshooting

### Issue: Deployment workflow fails at "Build backend image"

**Symptoms**: Docker build fails

**Diagnosis**:
```bash
# Check GitHub Actions logs
# Look for Rust compilation errors
```

**Fix**:
- Fix compilation errors in code
- Check Dockerfile syntax
- Ensure all dependencies are in Cargo.toml

### Issue: Deployment workflow fails at "Deploy application"

**Symptoms**: SSH connection fails or Docker commands fail

**Diagnosis**:
```bash
# Check SSH key
ssh -i C:\Users\sebas\.ssh\bdp-deploy ubuntu@<server-ip>

# Check if Docker is running on server
docker ps
```

**Fix**:
- Verify DEPLOY_SSH_PRIVATE_KEY secret is correct
- Verify PRODUCTION_SERVER_IP is correct
- Check server firewall rules

### Issue: Health check fails

**Symptoms**: Backend container not healthy

**Diagnosis**:
```bash
# SSH into server
docker compose logs bdp-server

# Check health check
docker compose exec bdp-server curl http://localhost:8000/health
```

**Fix**:
- Check DATABASE_URL is correct
- Check S3 credentials are correct
- Check application logs for errors

### Issue: Database connection fails

**Symptoms**: "Connection refused" or "Authentication failed"

**Diagnosis**:
```bash
# Check DATABASE_URL secret
# Check if database allows connections from server IP
```

**Fix**:
- Verify DATABASE_URL format
- Check OVH database firewall rules
- Check database is running in OVH panel

### Issue: S3 upload fails

**Symptoms**: "Access denied" or "Bucket not found"

**Diagnosis**:
```bash
# Check S3 environment variables
docker compose exec bdp-server env | grep STORAGE
```

**Fix**:
- Verify S3 credentials
- Create bucket manually if needed:
  ```bash
  aws s3 mb s3://bdp-data --endpoint-url=https://s3.gra.cloud.ovh.net
  ```

### Issue: Frontend not loading

**Symptoms**: Blank page or connection refused

**Diagnosis**:
```bash
# Check frontend logs
docker compose logs bdp-web

# Check if frontend can reach backend
docker compose exec bdp-web curl http://bdp-server:8000/health
```

**Fix**:
- Check NEXT_PUBLIC_API_URL is correct
- Verify Docker network connectivity
- Check frontend build logs for errors

---

## 📈 Performance Monitoring

### Key Metrics to Watch

1. **Container Health**: All containers should be "Up (healthy)"
2. **Memory Usage**: Should stay below 80% of available RAM
3. **Disk Usage**: Should not exceed 80% capacity
4. **Response Time**: API should respond < 500ms
5. **Error Rate**: Should be < 1% of requests

### Monitoring Commands

```bash
# Check all metrics at once
ssh -i C:\Users\sebas\.ssh\bdp-production ubuntu@<server-ip> << 'EOF'
  echo "=== Container Status ==="
  docker compose ps

  echo -e "\n=== Resource Usage ==="
  docker stats --no-stream

  echo -e "\n=== Disk Usage ==="
  df -h /

  echo -e "\n=== Recent Errors ==="
  docker compose logs --tail=100 | grep -i error | tail -10
EOF
```

---

## ✅ Complete Test Checklist

### Infrastructure Deployment
- [ ] Terraform plan successful
- [ ] Terraform apply successful (via GitHub Actions)
- [ ] Server accessible via SSH (personal key)
- [ ] Server accessible via SSH (CI/CD key)
- [ ] Docker installed and running
- [ ] Both SSH keys in authorized_keys

### Application Deployment
- [ ] Backend image builds successfully
- [ ] Frontend image builds successfully
- [ ] Images pushed to GHCR
- [ ] .env file generated correctly
- [ ] Files copied to server
- [ ] Containers started successfully
- [ ] Health checks passing

### Validation
- [ ] Backend health endpoint returns 200
- [ ] Frontend loads correctly
- [ ] Database connection works
- [ ] S3 connection works
- [ ] Logs show no critical errors

### Monitoring
- [ ] Can view logs in real-time
- [ ] Can check container stats
- [ ] Can access server metrics
- [ ] Error tracking working

---

## 🎯 Next Steps

After successful testing:

1. **Set up monitoring alerts** (UptimeRobot, Sentry, etc.)
2. **Configure automatic backups** (database, S3)
3. **Set up DNS and SSL** (Caddy, Let's Encrypt)
4. **Enable ingestion jobs** (if needed)
5. **Plan scaling strategy** (bigger instance, load balancer, etc.)

---

## 📚 Additional Resources

- [Docker Compose CLI](https://docs.docker.com/compose/reference/)
- [GitHub Actions Debugging](https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows)
- [OVH Cloud Documentation](https://help.ovhcloud.com/)
- [Terraform Debugging](https://www.terraform.io/docs/internals/debugging.html)
