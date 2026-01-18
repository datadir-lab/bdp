# BDP Uninstall Script for Windows
# This script uninstalls the BDP CLI tool from your system

$ErrorActionPreference = "Stop"

Write-Host "BDP Uninstall Script" -ForegroundColor Green
Write-Host "====================" -ForegroundColor Green
Write-Host ""

# Determine install location
if ($env:CARGO_HOME) {
    $InstallDir = "$env:CARGO_HOME\bin"
} else {
    $InstallDir = "$env:USERPROFILE\.cargo\bin"
}

$BdpPath = Join-Path $InstallDir "bdp.exe"

# Check if BDP is installed
if (-not (Test-Path $BdpPath)) {
    Write-Host "BDP is not installed at $BdpPath" -ForegroundColor Yellow
    Write-Host "Nothing to uninstall."
    exit 0
}

# Confirm uninstallation
Write-Host "This will remove BDP from: $BdpPath" -ForegroundColor Yellow
$Confirm = Read-Host "Continue? (y/N)"

if ($Confirm -notmatch '^[Yy]') {
    Write-Host "Uninstallation cancelled."
    exit 0
}

# Remove the binary
Write-Host "Removing BDP binary..."
try {
    Remove-Item -Path $BdpPath -Force -ErrorAction Stop
    Write-Host "Binary removed successfully." -ForegroundColor Green
} catch {
    Write-Host "Error removing binary: $_" -ForegroundColor Red
    exit 1
}

# Remove cache directory (optional, ask user)
if ($env:LOCALAPPDATA) {
    $CacheDir = "$env:LOCALAPPDATA\bdp"
} else {
    $CacheDir = "$env:USERPROFILE\AppData\Local\bdp"
}

if (Test-Path $CacheDir) {
    Write-Host "Remove BDP cache directory? ($CacheDir)" -ForegroundColor Yellow -NoNewline
    $ConfirmCache = Read-Host " (y/N)"

    if ($ConfirmCache -match '^[Yy]') {
        Write-Host "Removing cache directory..."
        Remove-Item -Path $CacheDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "Cache directory removed." -ForegroundColor Green
    } else {
        Write-Host "Keeping cache directory."
    }
}

Write-Host ""
Write-Host "✓ BDP has been successfully uninstalled!" -ForegroundColor Green
Write-Host ""
Write-Host "To reinstall BDP in the future, visit:"
Write-Host "  https://github.com/datadir-lab/bdp"
