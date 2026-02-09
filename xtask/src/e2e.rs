use crate::utils::*;
/// E2E testing
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum E2eCommand {
    /// Run E2E tests in CI mode (fast, uses committed fixtures)
    Ci,
    /// Run E2E tests in Real mode (uses downloaded data)
    Real,
    /// Download real UniProt test data (idempotent, cached)
    DownloadData,
    /// Run E2E tests with full observability output
    Debug,
    /// Clean E2E test data (removes downloaded data, keeps CI fixtures)
    Clean,
    /// Show E2E test data info
    Info,
}

pub fn handle(cmd: E2eCommand) -> Result<()> {
    match cmd {
        E2eCommand::Ci => ci(),
        E2eCommand::Real => real(),
        E2eCommand::DownloadData => download_data(),
        E2eCommand::Debug => debug(),
        E2eCommand::Clean => clean(),
        E2eCommand::Info => info_cmd(),
    }
}

fn ci() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧪 Running E2E tests (CI mode)..."
$env:BDP_E2E_MODE = "ci"
cargo test --test e2e -- --test-threads=1 --nocapture
"#,
            "Run E2E tests (CI mode)",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧪 Running E2E tests (CI mode)..."
export BDP_E2E_MODE=ci
cargo test --test e2e -- --test-threads=1 --nocapture
"#,
            "Run E2E tests (CI mode)",
        )
    }
}

fn real() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧪 Running E2E tests (Real mode with downloaded data)..."
$env:BDP_E2E_MODE = "real"
cargo test --test e2e -- --test-threads=1 --nocapture
"#,
            "Run E2E tests (Real mode)",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧪 Running E2E tests (Real mode with downloaded data)..."
export BDP_E2E_MODE=real
cargo test --test e2e -- --test-threads=1 --nocapture
"#,
            "Run E2E tests (Real mode)",
        )
    }
}

fn download_data() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📥 Downloading real UniProt test data..."
cargo run --bin download-test-data
"#,
            "Download test data",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📥 Downloading real UniProt test data..."
cargo run --bin download-test-data
"#,
            "Download test data",
        )
    }
}

fn debug() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🔍 Running E2E tests (debug mode)..."
$env:BDP_E2E_MODE = "ci"
$env:RUST_LOG = "debug,bdp_server=trace"
cargo test --test e2e -- --test-threads=1 --nocapture
"#,
            "Run E2E tests (debug mode)",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🔍 Running E2E tests (debug mode)..."
export BDP_E2E_MODE=ci
export RUST_LOG="debug,bdp_server=trace"
cargo test --test e2e -- --test-threads=1 --nocapture
"#,
            "Run E2E tests (debug mode)",
        )
    }
}

fn clean() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧹 Cleaning E2E test data..."
if (Test-Path "tests/fixtures/real") {
    Get-ChildItem "tests/fixtures/real" -Exclude ".gitkeep" | Remove-Item -Recurse -Force
}
Write-Host "✓ E2E test data cleaned"
"#,
            "Clean E2E test data",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧹 Cleaning E2E test data..."
if [ -d "tests/fixtures/real" ]; then
    find tests/fixtures/real -mindepth 1 -not -name ".gitkeep" -delete
fi
echo "✓ E2E test data cleaned"
"#,
            "Clean E2E test data",
        )
    }
}

fn info_cmd() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📊 E2E Test Data Information"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
Write-Host "CI Mode:"
if (Test-Path "tests/fixtures/uniprot_ci_sample.dat") {
    $size = (Get-Item "tests/fixtures/uniprot_ci_sample.dat").Length
    Write-Host "  ✓ CI sample:     $([math]::Round($size/1KB, 1)) KB"
} else {
    Write-Host "  ✗ CI sample not found"
}
Write-Host ""
Write-Host "Real Mode:"
if (Test-Path "tests/fixtures/real") {
    $files = Get-ChildItem "tests/fixtures/real" -Filter "*.dat*"
    if ($files.Count -gt 0) {
        foreach ($f in $files) {
            $size = $f.Length
            Write-Host "  ✓ $($f.Name):  $([math]::Round($size/1MB, 1)) MB"
        }
    } else {
        Write-Host "  ⚠ No real data downloaded (run: cargo xtask e2e download-data)"
    }
} else {
    Write-Host "  ✗ Real data directory not found"
}
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show E2E test data info",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📊 E2E Test Data Information"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "CI Mode:"
if [ -f "tests/fixtures/uniprot_ci_sample.dat" ]; then
    size=$(stat -f%z "tests/fixtures/uniprot_ci_sample.dat" 2>/dev/null || stat -c%s "tests/fixtures/uniprot_ci_sample.dat")
    size_kb=$((size / 1024))
    echo "  ✓ CI sample:     ${size_kb} KB"
else
    echo "  ✗ CI sample not found"
fi
echo ""
echo "Real Mode:"
if [ -d "tests/fixtures/real" ]; then
    files=$(find tests/fixtures/real -name "*.dat*" 2>/dev/null)
    if [ -n "$files" ]; then
        echo "$files" | while read f; do
            size=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
            size_mb=$((size / 1024 / 1024))
            echo "  ✓ $(basename $f):  ${size_mb} MB"
        done
    else
        echo "  ⚠ No real data downloaded (run: cargo xtask e2e download-data)"
    fi
else
    echo "  ✗ Real data directory not found"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show E2E test data info",
        )
    }
}
