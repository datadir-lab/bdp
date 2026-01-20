# Rebuild BDP Backend Server (PowerShell version)
# This script prepares SQLx cache, rebuilds the Docker image, and restarts the server

$ErrorActionPreference = "Stop"

Write-Host "=========================================="
Write-Host "BDP Backend Rebuild Script"
Write-Host "=========================================="
Write-Host ""

# Get the root directory
$ROOT_DIR = Split-Path -Parent $PSScriptRoot
Set-Location $ROOT_DIR

# Load environment variables
if (Test-Path .env.docker) {
    Write-Host "Loading environment from .env.docker..."
    Get-Content .env.docker | ForEach-Object {
        if ($_ -match '^([^#][^=]+)=(.*)$') {
            $name = $matches[1].Trim()
            $value = $matches[2].Trim()
            Set-Item -Path "env:$name" -Value $value
        }
    }
}

# Set defaults if not set
if (-not $env:POSTGRES_USER) { $env:POSTGRES_USER = "bdp" }
if (-not $env:POSTGRES_PASSWORD) { $env:POSTGRES_PASSWORD = "bdp_dev_password" }
if (-not $env:POSTGRES_PORT) { $env:POSTGRES_PORT = "5432" }
if (-not $env:POSTGRES_DB) { $env:POSTGRES_DB = "bdp" }

# Step 1: Check if database is running
Write-Host "Step 1: Checking database..."
$dbRunning = docker ps --filter "name=bdp-postgres" --format "{{.Names}}" | Select-String "bdp-postgres"
if (-not $dbRunning) {
    Write-Host "Error: PostgreSQL container is not running!"
    Write-Host "Please start it with: docker-compose up -d postgres"
    exit 1
}
Write-Host "✓ Database is running"
Write-Host ""

# Step 2: Prepare SQLx offline cache
Write-Host "Step 2: Preparing SQLx offline cache..."
$env:DATABASE_URL = "postgresql://$($env:POSTGRES_USER):$($env:POSTGRES_PASSWORD)@localhost:$($env:POSTGRES_PORT)/$($env:POSTGRES_DB)"

# Check if sqlx-cli is installed
$sqlxInstalled = cargo install --list | Select-String "sqlx-cli"
if (-not $sqlxInstalled) {
    Write-Host "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
}

# Prepare the query cache
Write-Host "Generating query cache..."
Set-Location $ROOT_DIR
cargo sqlx prepare --workspace -- --all-targets

Write-Host "✓ SQLx cache prepared"
Write-Host ""

# Step 3: Stop the current server
Write-Host "Step 3: Stopping current server..."
docker-compose stop bdp-server 2>$null
Write-Host "✓ Server stopped"
Write-Host ""

# Step 4: Rebuild the Docker image
Write-Host "Step 4: Building new Docker image..."
docker-compose build --no-cache bdp-server
Write-Host "✓ Docker image built"
Write-Host ""

# Step 5: Start the server
Write-Host "Step 5: Starting server..."
docker-compose up -d bdp-server
Write-Host "✓ Server started"
Write-Host ""

# Step 6: Wait for health check
Write-Host "Step 6: Waiting for server to be healthy..."
$maxAttempts = 30
$attempt = 0

while ($attempt -lt $maxAttempts) {
    try {
        $response = Invoke-WebRequest -Uri "http://localhost:8000/health" -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue
        if ($response.StatusCode -eq 200) {
            Write-Host "✓ Server is healthy!"
            break
        }
    }
    catch {
        # Continue waiting
    }

    $attempt++
    if ($attempt -eq $maxAttempts) {
        Write-Host "Warning: Server health check timeout"
        Write-Host "Check logs with: docker-compose logs bdp-server"
        exit 1
    }

    Write-Host "Waiting for server... ($attempt/$maxAttempts)"
    Start-Sleep -Seconds 2
}
Write-Host ""

# Step 7: Show logs
Write-Host "=========================================="
Write-Host "Server Status:"
Write-Host "=========================================="
docker-compose ps bdp-server
Write-Host ""

Write-Host "Recent logs:"
docker-compose logs --tail=20 bdp-server
Write-Host ""

Write-Host "=========================================="
Write-Host "✓ Backend rebuild complete!"
Write-Host "=========================================="
Write-Host ""
Write-Host "Useful commands:"
Write-Host "  - View logs: docker-compose logs -f bdp-server"
Write-Host "  - Check status: docker-compose ps"
Write-Host "  - Test API: curl http://localhost:8000/health"
Write-Host ""
