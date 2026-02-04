# Installation Guide

## Quick Install (Recommended)

**Linux/macOS:**
```bash
curl -sSfL https://bdp.dev/install.sh | sh
```

**Windows:**
```powershell
irm https://bdp.dev/install.ps1 | iex
```

### Direct GitHub Installer

You can also install directly from GitHub releases without the redirect:

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

**Windows:**
```powershell
irm https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1 | iex
```

## Installation Methods

### 1. Pre-built Binaries

Download from [releases](https://github.com/datadir-lab/bdp/releases):

- **Linux x86_64**: `bdp-x86_64-unknown-linux-gnu.tar.gz`
- **Linux ARM64**: `bdp-aarch64-unknown-linux-gnu.tar.gz`
- **macOS x86_64**: `bdp-x86_64-apple-darwin.tar.gz`
- **macOS ARM64**: `bdp-aarch64-apple-darwin.tar.gz`
- **Windows**: `bdp-x86_64-pc-windows-msvc.zip`

Extract and copy the binary to a directory in your PATH.

### 2. From Source

**Using Cargo:**
```bash
cargo install bdp-cli
```

**From Repository:**
```bash
git clone https://github.com/datadir-lab/bdp.git
cd bdp
cargo install --path crates/bdp-cli
```

### 3. Development Build

```bash
git clone https://github.com/datadir-lab/bdp.git
cd bdp
cargo build --release
# Binary at: target/release/bdp
```

## Channels Explained

### Stable Channel
- Latest stable release (non-prerelease)
- Recommended for production use
- Thoroughly tested on all platforms

### Canary Channel
- Latest pre-release version
- Early access to new features
- May contain bugs or breaking changes
- Great for testing and feedback

### Specific Version
- Install any tagged release (e.g., v0.1.0)
- Useful for:
  - Pinning to a known working version
  - Testing specific versions
  - Rollback scenarios

## Verify

```bash
bdp --version && bdp --help
```

## Uninstall

```bash
bdp uninstall --purge -y
```

## Troubleshooting

**Command not found:** Add `~/.cargo/bin` to PATH
**SSL errors:** Install ca-certificates
**Windows:** Set execution policy `RemoteSigned`
