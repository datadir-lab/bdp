# BDP Installation Guide

This guide provides instructions for installing the BDP CLI tool on various platforms.

## Quick Install

### Linux and macOS

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1 | iex
```

---

## Installation Methods

### 1. Pre-built Binaries (Recommended)

Pre-built binaries are available for the following platforms:

- **Linux**: x86_64, ARM64
- **macOS**: x86_64 (Intel), ARM64 (Apple Silicon)
- **Windows**: x86_64

#### Download from GitHub Releases

1. Visit the [releases page](https://github.com/datadir-lab/bdp/releases)
2. Download the appropriate archive for your platform
3. Extract the binary and place it in your PATH

**Example for Linux/macOS:**
```bash
# Download the latest release
wget https://github.com/datadir-lab/bdp/releases/latest/download/bdp-x86_64-unknown-linux-gnu.tar.gz

# Extract
tar -xzf bdp-x86_64-unknown-linux-gnu.tar.gz

# Move to PATH
sudo mv bdp /usr/local/bin/

# Verify installation
bdp --version
```

**Example for Windows:**
```powershell
# Download the latest release
Invoke-WebRequest -Uri "https://github.com/datadir-lab/bdp/releases/latest/download/bdp-x86_64-pc-windows-msvc.zip" -OutFile "bdp.zip"

# Extract
Expand-Archive -Path bdp.zip -DestinationPath .

# Move to a directory in your PATH (e.g., C:\Program Files\bdp)
# Or add the current directory to your PATH
```

### 2. Install from Source

If you have Rust installed, you can build from source:

```bash
# Clone the repository
git clone https://github.com/datadir-lab/bdp.git
cd bdp

# Build and install
cargo install --path crates/bdp-cli
```

### 3. Using Cargo

Install directly from crates.io (once published):

```bash
cargo install bdp-cli
```

---

## Verifying Installation

After installation, verify that BDP is working correctly:

```bash
bdp --version
bdp --help
```

---

## Upgrading

### Using the installer script

The installer script automatically handles upgrades. Simply run the installation command again:

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

**Windows:**
```powershell
irm https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1 | iex
```

### Manual upgrade

1. Download the latest release from GitHub
2. Replace the existing binary with the new one
3. Verify the version: `bdp --version`

### Using Cargo

```bash
cargo install --force bdp-cli
```

---

## Uninstalling

### Using the built-in uninstall command (Recommended)

BDP includes a self-uninstall command:

```bash
# Uninstall with confirmation prompt
bdp uninstall

# Uninstall without confirmation
bdp uninstall -y

# Uninstall and remove all cache/configuration
bdp uninstall --purge -y
```

The uninstall command will:
- Remove the BDP binary from your system
- Optionally remove cache and configuration files (with `--purge`)
- Provide a confirmation prompt (skip with `-y`)

**How it works:**
- **Unix/Linux/macOS**: Spawns a background process to delete the binary
- **Windows**: Renames the executable and schedules deletion (works around file locking)

### Alternative: Using standalone uninstall scripts

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/datadir-lab/bdp/main/scripts/uninstall.sh | sh
```

**Windows:**
```powershell
irm https://raw.githubusercontent.com/datadir-lab/bdp/main/scripts/uninstall.ps1 | iex
```

### Manual uninstall

If the built-in uninstall command isn't working:

**Linux/macOS:**
```bash
# Remove the binary
rm ~/.cargo/bin/bdp

# Optionally remove the cache
rm -rf ~/.cache/bdp
```

**Windows:**
```powershell
# Remove the binary
Remove-Item "$env:USERPROFILE\.cargo\bin\bdp.exe"

# Optionally remove the cache
Remove-Item -Recurse "$env:LOCALAPPDATA\bdp"
```

---

## Post-Installation

### Initial Setup

After installing BDP, initialize a new project:

```bash
# Create a new directory
mkdir my-bio-project
cd my-bio-project

# Initialize BDP
bdp init --name "my-bio-project"
```

### Configuration

Configure the BDP server URL (if using a custom server):

```bash
bdp config set server-url https://your-bdp-server.com
```

Or set the environment variable:

```bash
export BDP_SERVER_URL=https://your-bdp-server.com
```

---

## Troubleshooting

### Command not found

If you get "command not found" after installation, ensure that the installation directory is in your PATH:

**Linux/macOS:**
```bash
export PATH="$HOME/.cargo/bin:$PATH"

# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

**Windows:**
Add `%USERPROFILE%\.cargo\bin` to your PATH environment variable.

### Permission denied

**Linux/macOS:**
Ensure the binary is executable:
```bash
chmod +x ~/.cargo/bin/bdp
```

### SSL/TLS errors

If you encounter SSL errors during installation:

**Linux:**
```bash
# Update ca-certificates
sudo apt-get update && sudo apt-get install ca-certificates
```

**macOS:**
```bash
# Update certificates
brew install ca-certificates
```

---

## Platform-Specific Notes

### Linux

- **Debian/Ubuntu**: Requires `libssl-dev` for HTTPS support
- **RHEL/CentOS**: Requires `openssl-devel`

### macOS

- On Apple Silicon Macs, use the ARM64 (aarch64) version for better performance
- Intel Macs should use the x86_64 version

### Windows

- PowerShell execution policy must allow running scripts
- Run `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser` if needed

---

## Security Notes

The installer scripts use HTTPS and verify checksums to ensure integrity. For maximum security:

1. **Verify the script** before piping to shell:
   ```bash
   # Download and inspect
   curl -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh > installer.sh
   less installer.sh

   # Run after review
   sh installer.sh
   ```

2. **Verify release signatures** (when available)

3. **Use official releases** from the GitHub releases page

---

## Getting Help

- **Documentation**: https://github.com/datadir-lab/bdp/blob/main/README.md
- **Issues**: https://github.com/datadir-lab/bdp/issues
- **Discussions**: https://github.com/datadir-lab/bdp/discussions

---

## Next Steps

After successful installation:

1. Read the [Quick Start Guide](QUICK_START.md)
2. Learn about [BDP concepts](README.md#concepts)
3. Explore [example projects](examples/)
