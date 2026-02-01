# BDP Universal Installer (PowerShell)
#
# This script installs the BDP CLI from GitHub releases on Windows.
# It supports multiple channels (stable, canary, specific versions)
# and performs checksum verification for security.
#
# Usage:
#   iwr https://install.bdp.dev/install.ps1 -useb | iex
#   iwr https://install.bdp.dev/install.ps1 -useb | iex; Install-BDP -Channel canary
#   iwr https://install.bdp.dev/install.ps1 -useb | iex; Install-BDP -Version v0.1.0
#   iwr https://install.bdp.dev/install.ps1 -useb | iex; Install-BDP -Path C:\Tools

param(
    [string]$Channel = "stable",
    [string]$Version = "",
    [string]$Path = ""
)

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "datadir-lab/bdp"
$BinaryName = "bdp"
$GitHubApi = "https://api.github.com"
$GitHubDownload = "https://github.com"

# Default install path
if ([string]::IsNullOrEmpty($Path)) {
    if ($env:CARGO_HOME) {
        $Path = Join-Path $env:CARGO_HOME "bin"
    } else {
        $Path = Join-Path $env:USERPROFILE ".cargo\bin"
    }
}

# Helper functions
function Write-Info {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "Warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "Error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Get-Platform {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")

    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { return "aarch64-pc-windows-msvc" }
        default { Write-Error "Unsupported architecture: $arch" }
    }
}

function Invoke-GitHubApi {
    param([string]$Endpoint)

    $url = "$GitHubApi$Endpoint"

    try {
        $response = Invoke-RestMethod -Uri $url -Method Get -UseBasicParsing
        return $response
    } catch {
        Write-Error "Failed to query GitHub API: $url`n$($_.Exception.Message)"
    }
}

function Resolve-Version {
    param(
        [string]$Channel,
        [string]$SpecificVersion
    )

    Write-Info "Resolving version for channel: $Channel"

    switch ($Channel) {
        "stable" {
            # Get latest non-prerelease
            $release = Invoke-GitHubApi "/repos/$Repo/releases/latest"
            if (-not $release.tag_name) {
                Write-Error "Failed to resolve latest stable version"
            }
            return $release.tag_name
        }
        "canary" {
            # Get latest prerelease
            $releases = Invoke-GitHubApi "/repos/$Repo/releases"
            $canary = $releases | Where-Object { $_.prerelease -eq $true } | Select-Object -First 1

            if (-not $canary) {
                Write-Warn "No canary releases found, falling back to latest stable"
                $stable = $releases | Where-Object { $_.prerelease -eq $false } | Select-Object -First 1
                return $stable.tag_name
            }
            return $canary.tag_name
        }
        "specific" {
            if ([string]::IsNullOrEmpty($SpecificVersion)) {
                Write-Error "Version not specified for specific channel"
            }
            # Ensure version starts with 'v'
            if (-not $SpecificVersion.StartsWith("v")) {
                $SpecificVersion = "v$SpecificVersion"
            }
            return $SpecificVersion
        }
        default {
            Write-Error "Unknown channel: $Channel"
        }
    }
}

function Get-BinaryArchive {
    param(
        [string]$Platform,
        [string]$Version
    )

    $archiveName = "$BinaryName-$Platform.tar.gz"
    $downloadUrl = "$GitHubDownload/$Repo/releases/download/$Version/$archiveName"
    $checksumUrl = "$downloadUrl.sha256"
    $tempDir = Join-Path $env:TEMP "bdp-install-$(Get-Random)"

    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    Write-Info "Downloading $BinaryName $Version for $Platform..."

    $archivePath = Join-Path $tempDir $archiveName

    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
    } catch {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Error "Failed to download binary from $downloadUrl`n$($_.Exception.Message)"
    }

    # Download checksum
    $checksumPath = "$archivePath.sha256"
    try {
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing
    } catch {
        Write-Warn "Failed to download checksum file, skipping verification"
        return @{
            TempDir = $tempDir
            ArchivePath = $archivePath
            ChecksumPath = $null
        }
    }

    return @{
        TempDir = $tempDir
        ArchivePath = $archivePath
        ChecksumPath = $checksumPath
    }
}

function Test-Checksum {
    param(
        [string]$ArchivePath,
        [string]$ChecksumPath
    )

    if ([string]::IsNullOrEmpty($ChecksumPath) -or -not (Test-Path $ChecksumPath)) {
        Write-Warn "Checksum file not found, skipping verification"
        return
    }

    Write-Info "Verifying checksum..."

    # Read expected checksum
    $expectedChecksum = (Get-Content $ChecksumPath -Raw).Split()[0].Trim()

    # Calculate actual checksum
    $actualChecksum = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()

    if ($expectedChecksum -ne $actualChecksum) {
        Write-Error "Checksum verification failed!`nExpected: $expectedChecksum`nActual:   $actualChecksum"
    }

    Write-Success "Checksum verified"
}

function Install-Binary {
    param(
        [string]$TempDir,
        [string]$ArchivePath,
        [string]$InstallPath
    )

    Write-Info "Extracting binary..."

    # PowerShell's built-in tar support (Windows 10 1903+)
    if (Get-Command tar -ErrorAction SilentlyContinue) {
        tar -xzf $ArchivePath -C $TempDir
    } else {
        # Fallback: Use .NET for extraction (requires .tar.gz to be extracted in two steps)
        Write-Error "tar command not found. Please install tar or use Windows 10 1903 or later."
    }

    # Create install directory if it doesn't exist
    if (-not (Test-Path $InstallPath)) {
        Write-Info "Creating install directory: $InstallPath"
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
    }

    # Find the binary in the extracted files
    $binaryPath = Get-ChildItem -Path $TempDir -Recurse -Filter "$BinaryName.exe" -File | Select-Object -First 1

    if (-not $binaryPath) {
        Write-Error "Binary not found in archive"
    }

    $targetPath = Join-Path $InstallPath "$BinaryName.exe"

    Write-Info "Installing to $targetPath"

    # Remove old binary if it exists
    if (Test-Path $targetPath) {
        Remove-Item -Path $targetPath -Force
    }

    Copy-Item -Path $binaryPath.FullName -Destination $targetPath -Force

    # Cleanup
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

    Write-Success "Installation complete!"
}

function Add-ToPath {
    param([string]$InstallPath)

    $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -notlike "*$InstallPath*") {
        Write-Info "Adding $InstallPath to PATH..."
        $newPath = "$InstallPath;$userPath"
        [System.Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$InstallPath;$env:Path"
        Write-Success "Added to PATH (restart your shell to use without full path)"
    }
}

function Test-Installation {
    param([string]$InstallPath)

    Write-Info "Verifying installation..."

    $binaryPath = Join-Path $InstallPath "$BinaryName.exe"

    if (-not (Test-Path $binaryPath)) {
        Write-Error "Binary not found at $binaryPath"
    }

    try {
        $output = & $binaryPath --version 2>&1
        Write-Success "Installed: $output"
    } catch {
        Write-Error "Installation verification failed: $($_.Exception.Message)"
    }
}

function Install-BDP {
    param(
        [string]$Channel = "stable",
        [string]$Version = "",
        [string]$Path = ""
    )

    # Use script-level variables if parameters not provided
    if ([string]::IsNullOrEmpty($Path)) {
        if ($env:CARGO_HOME) {
            $Path = Join-Path $env:CARGO_HOME "bin"
        } else {
            $Path = Join-Path $env:USERPROFILE ".cargo\bin"
        }
    }

    Write-Info "BDP Universal Installer"
    Write-Info "========================"

    $platform = Get-Platform
    Write-Info "Detected platform: $platform"

    # Determine channel based on version parameter
    $channelToUse = $Channel
    if (-not [string]::IsNullOrEmpty($Version)) {
        $channelToUse = "specific"
    }

    $resolvedVersion = Resolve-Version -Channel $channelToUse -SpecificVersion $Version
    Write-Info "Resolved version: $resolvedVersion"

    $download = Get-BinaryArchive -Platform $platform -Version $resolvedVersion

    if ($download.ChecksumPath) {
        Test-Checksum -ArchivePath $download.ArchivePath -ChecksumPath $download.ChecksumPath
    }

    Install-Binary -TempDir $download.TempDir -ArchivePath $download.ArchivePath -InstallPath $Path

    Add-ToPath -InstallPath $Path

    Test-Installation -InstallPath $Path

    Write-Host ""
    Write-Success "BDP $resolvedVersion installed successfully!"
    Write-Host ""
    Write-Info "Get started by running:"
    Write-Host "    $BinaryName --help"
    Write-Host ""
}

# If script is run directly (not dot-sourced), run installation
if ($MyInvocation.InvocationName -ne '.') {
    # Determine channel based on version parameter
    $channelToUse = $Channel
    if (-not [string]::IsNullOrEmpty($Version)) {
        $channelToUse = "specific"
    }

    Install-BDP -Channel $channelToUse -Version $Version -Path $Path
}
