# Scripts

Development, testing, deployment, ingestion utilities, and installation scripts.

**Structure:** `dev/` `test/` `deploy/` `ingest/` `output/`

## Installation Scripts

Universal installers for BDP CLI with channel support (stable, canary, specific versions).

### install.sh (Linux/macOS)

Smart installer for Unix-like systems:

```bash
# Stable release (default)
sh scripts/install.sh

# Canary release (pre-release)
sh scripts/install.sh --channel canary

# Specific version
sh scripts/install.sh --version v0.1.0

# Custom install path
sh scripts/install.sh --path /usr/local/bin
```

**Features:**
- Auto-detect platform (x86_64/ARM64, Linux/macOS)
- SHA256 checksum verification
- GitHub API integration for version resolution
- Fallback to wget if curl unavailable
- Idempotent (safe to re-run)
- Clear error messages and progress indicators

### install.ps1 (Windows)

PowerShell installer with same features:

```powershell
# Dot-source to use Install-BDP function
. .\scripts\install.ps1

# Stable release (default)
Install-BDP

# Canary release
Install-BDP -Channel canary

# Specific version
Install-BDP -Version v0.1.0

# Custom install path
Install-BDP -Path C:\Tools
```

**Features:**
- Auto-detect architecture (x86_64/ARM64)
- SHA256 checksum verification (Get-FileHash)
- Automatic PATH configuration
- Built-in tar support (Windows 10 1903+)

### Testing

CI testing in `.github/workflows/test-release.yml`:
- Tests on 4 OS variants (Ubuntu 22.04, macOS 13/14, Windows 2022)
- Tests stable and specific-version channels
- Tests custom install path
- Tests upgrade (re-install)
- Tests uninstall
- Release only publishes after all tests pass

### Deployment

Scripts live in repository (`scripts/` directory). To deploy to `install.bdp.dev`:

1. Copy scripts to OVH instance
2. Configure Caddy to serve from `/var/www/install/`
3. Set up DNS: `install.bdp.dev` → OVH instance IP

See deployment plan in project documentation.

## DB Diagram

```bash
./scripts/generate-db-diagram.sh
# Outputs: schema.sql, tables.txt, schema.dot, schema.png
# View: open scripts/output/schema_latest.png
# Online: https://dreampuf.github.io/GraphvizOnline/
```

**Requirements:** Docker, optional Graphviz

## Guidelines

Use bash strict mode, add usage docs, validate prereqs, handle errors, export env vars
