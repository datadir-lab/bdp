# PowerShell script to reset Docker database and restart ingestion
# Windows version of reset-docker-db.sh

$ErrorActionPreference = "Stop"

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "BDP Docker Database Reset & Ingestion Restart" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Change to project root
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location (Join-Path $scriptPath "..")
$projectRoot = Get-Location

Write-Host "[1/7] Stopping all containers..." -ForegroundColor Yellow
docker-compose down

Write-Host ""
Write-Host "[2/7] Removing database volumes..." -ForegroundColor Yellow
docker volume rm bdp_postgres_data 2>$null
if ($LASTEXITCODE -ne 0) { Write-Host "Volume bdp_postgres_data doesn't exist (OK)" }
docker volume rm bdp_postgres_test_data 2>$null
if ($LASTEXITCODE -ne 0) { Write-Host "Volume bdp_postgres_test_data doesn't exist (OK)" }

Write-Host ""
Write-Host "[3/7] Starting PostgreSQL and MinIO..." -ForegroundColor Yellow
docker-compose up -d postgres minio minio-init

Write-Host ""
Write-Host "[4/7] Waiting for PostgreSQL to be ready..." -ForegroundColor Yellow
Write-Host "This may take 10-15 seconds..."
Start-Sleep -Seconds 5

# Wait for PostgreSQL to be healthy
$maxAttempts = 30
for ($i = 1; $i -le $maxAttempts; $i++) {
    $ready = docker exec bdp-postgres pg_isready -U bdp -d bdp 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ PostgreSQL is ready" -ForegroundColor Green
        break
    }

    if ($i -eq $maxAttempts) {
        Write-Host "✗ PostgreSQL failed to start" -ForegroundColor Red
        exit 1
    }

    Write-Host "." -NoNewline
    Start-Sleep -Seconds 1
}

Write-Host ""
Write-Host "[5/7] Running database migrations..." -ForegroundColor Yellow
Write-Host "Using DATABASE_URL from .env.docker..."

# Load environment variables from .env.docker
if (Test-Path ".env.docker") {
    Get-Content ".env.docker" | ForEach-Object {
        if ($_ -match '^\s*([^#][^=]+)=(.*)$') {
            $name = $matches[1].Trim()
            $value = $matches[2].Trim()
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
} else {
    Write-Host "Error: .env.docker file not found" -ForegroundColor Red
    exit 1
}

# Set DATABASE_URL for local machine connection
$env:POSTGRES_USER = if ($env:POSTGRES_USER) { $env:POSTGRES_USER } else { "bdp" }
$env:POSTGRES_PASSWORD = if ($env:POSTGRES_PASSWORD) { $env:POSTGRES_PASSWORD } else { "bdp_dev_password" }
$env:POSTGRES_PORT = if ($env:POSTGRES_PORT) { $env:POSTGRES_PORT } else { "5432" }
$env:POSTGRES_DB = if ($env:POSTGRES_DB) { $env:POSTGRES_DB } else { "bdp" }

$env:DATABASE_URL = "postgresql://$($env:POSTGRES_USER):$($env:POSTGRES_PASSWORD)@localhost:$($env:POSTGRES_PORT)/$($env:POSTGRES_DB)"

Write-Host "Running: sqlx migrate run"
sqlx migrate run --source migrations

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Migrations completed successfully" -ForegroundColor Green
} else {
    Write-Host "✗ Migration failed" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "[6/7] Rebuilding bdp-server Docker image..." -ForegroundColor Yellow
docker-compose build bdp-server

Write-Host ""
Write-Host "[7/7] Starting bdp-server with ingestion enabled..." -ForegroundColor Yellow
docker-compose --env-file .env.docker up -d bdp-server

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "✓ Database reset and ingestion restart complete!" -ForegroundColor Green
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Monitor logs:"
Write-Host "     docker-compose logs -f bdp-server"
Write-Host ""
Write-Host "  2. Check database:"
Write-Host "     docker exec -it bdp-postgres psql -U bdp -d bdp"
Write-Host ""
Write-Host "  3. Trigger ingestion (if not auto-started):"
Write-Host "     curl -X POST http://localhost:8000/api/v1/ingest/uniprot/trigger"
Write-Host ""
Write-Host "Expected improvements:"
Write-Host "  - TAR cache: Single decompression (2280x faster)"
Write-Host "  - Batch operations: 300-500x query reduction"
Write-Host "  - Worker count: 16 workers (4x throughput)"
Write-Host "  - Batch size: 5000 entries per batch (5x efficiency)"
Write-Host ""
Write-Host "Monitor for:"
Write-Host "  - ERROR level messages (root cause of storage failures)"
Write-Host "  - 'Cache hit' logs (TAR decompression cache)"
Write-Host "  - 'Organism cache hit' logs"
Write-Host "  - 'Batch inserting' logs"
Write-Host "  - 'Creating organism bundle' logs"
Write-Host ""
