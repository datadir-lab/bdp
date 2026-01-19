# Test script for NCBI Taxonomy integration tests
#
# This script runs the NCBI taxonomy integration tests against a PostgreSQL database.
# It requires a running PostgreSQL instance with migrations applied.
#
# Usage:
#   .\test_ncbi_taxonomy.ps1 [-UnitOnly] [-Integration] [-All] [-NoCapture]
#
# Parameters:
#   -UnitOnly         Run only unit tests (parser, pipeline, version discovery)
#   -Integration      Run integration tests (requires database)
#   -All              Run all tests (default)
#   -NoCapture        Show test output (useful for debugging)
#
# Environment:
#   $env:DATABASE_URL   PostgreSQL connection string (default: postgresql://localhost/bdp_test)

param(
    [switch]$UnitOnly,
    [switch]$Integration,
    [switch]$All,
    [switch]$NoCapture
)

# Default to running unit tests only
$RunUnit = $true
$RunIntegration = $false

# Parse parameters
if ($Integration) {
    $RunUnit = $false
    $RunIntegration = $true
}

if ($All) {
    $RunUnit = $true
    $RunIntegration = $true
}

$NoCaptureFlag = ""
if ($NoCapture) {
    $NoCaptureFlag = "--nocapture"
}

# Colors
function Write-Header {
    param([string]$Message)
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host $Message -ForegroundColor Blue
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host ""
}

function Write-Success {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Cyan
}

function Write-Warning {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Yellow
}

Write-Header "NCBI Taxonomy Test Suite"

# Check if DATABASE_URL is set for integration tests
if ($RunIntegration) {
    if (-not $env:DATABASE_URL) {
        Write-Warning "Warning: DATABASE_URL not set, using default: postgresql://localhost/bdp_test"
        $env:DATABASE_URL = "postgresql://localhost/bdp_test"
    }
    Write-Info "Database: $env:DATABASE_URL"
    Write-Host ""
}

# Run unit tests
if ($RunUnit) {
    Write-Success "Running Unit Tests..."

    Write-Info "1. Parser tests (12 tests)"
    cargo test --test ncbi_taxonomy_parser_test $NoCaptureFlag
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Parser tests failed!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Info "2. Pipeline tests (2 tests)"
    cargo test --lib ncbi_taxonomy::pipeline::tests $NoCaptureFlag
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Pipeline tests failed!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Info "3. Version discovery tests"
    cargo test --lib ncbi_taxonomy::version_discovery::tests $NoCaptureFlag
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Version discovery tests failed!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Success "✓ Unit tests completed"
    Write-Host ""
}

# Run integration tests
if ($RunIntegration) {
    Write-Success "Running Integration Tests (8 tests)..."
    Write-Warning "Note: These require a running PostgreSQL database with migrations applied"
    Write-Host ""

    cargo test --test ncbi_taxonomy_integration_test -- --ignored $NoCaptureFlag
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Integration tests failed!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Success "✓ Integration tests completed"
    Write-Host ""
}

Write-Header "All tests completed successfully!"
