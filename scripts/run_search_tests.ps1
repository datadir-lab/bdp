# Run all search optimization tests and benchmarks
#
# Usage:
#   .\scripts\run_search_tests.ps1 [OPTIONS]
#
# Options:
#   -Unit          Run unit tests only
#   -Integration   Run integration tests only
#   -Load          Run load tests only
#   -Bench         Run benchmarks only
#   -Quick         Quick test run (smaller datasets)
#   -Full          Full test run (large datasets, slow)
#   -Help          Show this help message

param(
    [switch]$Unit,
    [switch]$Integration,
    [switch]$Load,
    [switch]$Bench,
    [switch]$Quick,
    [switch]$Full,
    [switch]$Help
)

function Write-Header {
    param([string]$Text)
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Blue
    Write-Host "  $Text" -ForegroundColor Blue
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Blue
    Write-Host ""
}

function Write-Success {
    param([string]$Text)
    Write-Host "✓ $Text" -ForegroundColor Green
}

function Write-Error-Message {
    param([string]$Text)
    Write-Host "✗ $Text" -ForegroundColor Red
}

function Write-Warning {
    param([string]$Text)
    Write-Host "⚠ $Text" -ForegroundColor Yellow
}

function Write-Info {
    param([string]$Text)
    Write-Host "→ $Text" -ForegroundColor Cyan
}

if ($Help) {
    Get-Content $PSCommandPath | Select-Object -First 15 | Select-Object -Skip 1
    exit 0
}

# Default options
$RunUnit = $false
$RunIntegration = $false
$RunLoad = $false
$RunBench = $false

if ($Full) {
    $RunUnit = $true
    $RunIntegration = $true
    $RunLoad = $true
    $RunBench = $true
} elseif (-not ($Unit -or $Integration -or $Load -or $Bench)) {
    # If no options specified, run unit and integration tests
    $RunUnit = $true
    $RunIntegration = $true
} else {
    $RunUnit = $Unit
    $RunIntegration = $Integration
    $RunLoad = $Load
    $RunBench = $Bench
}

Write-Header "Search Optimization Test Suite"

# Check environment
if (-not $env:DATABASE_URL) {
    Write-Error-Message "DATABASE_URL not set"
    Write-Host "  Please set DATABASE_URL environment variable"
    exit 1
}

Write-Success "DATABASE_URL configured"
Write-Host ""

# Run migrations if needed
Write-Info "Checking database migrations..."
if (Get-Command sqlx -ErrorAction SilentlyContinue) {
    $result = sqlx migrate run 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Message "Failed to run migrations"
        Write-Host $result
        exit 1
    }
    Write-Success "Migrations applied"
} else {
    Write-Warning "sqlx-cli not installed, skipping migration check"
}
Write-Host ""

$testsFailed = $false

# Unit tests
if ($RunUnit) {
    Write-Header "Running Unit Tests"

    cargo test --package bdp-server --lib features::search::queries --no-fail-fast
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Message "Unit tests failed"
        $testsFailed = $true
    } else {
        Write-Host ""
        Write-Success "Unit tests passed"
        Write-Host ""
    }
}

# Integration tests
if ($RunIntegration -and -not $testsFailed) {
    Write-Header "Running Integration Tests"

    cargo test --package bdp-server --test search_integration_tests --no-fail-fast -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Message "Integration tests failed"
        $testsFailed = $true
    } else {
        Write-Host ""
        Write-Success "Integration tests passed"
        Write-Host ""
    }
}

# Load tests
if ($RunLoad -and -not $testsFailed) {
    Write-Header "Running Load Tests"
    Write-Warning "Load tests may take several minutes..."
    Write-Host ""

    if ($Quick) {
        Write-Info "Running quick load test (concurrent searches only)"
        cargo test --package bdp-server --test search_load_tests test_concurrent_searches -- --ignored --nocapture --test-threads=1
    } else {
        Write-Info "Running all load tests"
        cargo test --package bdp-server --test search_load_tests -- --ignored --nocapture --test-threads=1
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error-Message "Load tests failed"
        $testsFailed = $true
    } else {
        Write-Host ""
        Write-Success "Load tests passed"
        Write-Host ""
    }
}

# Benchmarks
if ($RunBench -and -not $testsFailed) {
    Write-Header "Running Benchmarks"
    Write-Warning "Benchmarks may take 10-30 minutes..."
    Write-Host ""

    if ($Quick) {
        Write-Info "Running quick benchmarks (sample size: 10)"
        cargo bench --bench search_performance -- --sample-size 10
    } else {
        Write-Info "Running full benchmarks"
        cargo bench --bench search_performance
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error-Message "Benchmarks failed"
        $testsFailed = $true
    } else {
        Write-Host ""
        Write-Success "Benchmarks completed"
        Write-Host ""

        # Show benchmark report location
        $reportDir = "target\criterion"
        if (Test-Path $reportDir) {
            Write-Host "Benchmark reports available at:" -ForegroundColor Blue
            Write-Host "  file://$(Get-Location)\$reportDir\report\index.html"
            Write-Host ""
        }
    }
}

# Summary
Write-Header "Test Summary"

if ($RunUnit) {
    if ($testsFailed) {
        Write-Error-Message "Unit tests: FAILED"
    } else {
        Write-Success "Unit tests: PASSED"
    }
}

if ($RunIntegration) {
    if ($testsFailed) {
        Write-Error-Message "Integration tests: FAILED"
    } else {
        Write-Success "Integration tests: PASSED"
    }
}

if ($RunLoad) {
    if ($testsFailed) {
        Write-Error-Message "Load tests: FAILED"
    } else {
        Write-Success "Load tests: PASSED"
    }
}

if ($RunBench) {
    if ($testsFailed) {
        Write-Error-Message "Benchmarks: FAILED"
    } else {
        Write-Success "Benchmarks: COMPLETED"
    }
}

Write-Host ""

if ($testsFailed) {
    Write-Error-Message "Some tests failed"
    exit 1
} else {
    Write-Success "All tests passed successfully! 🎉"
    Write-Host ""
    exit 0
}
