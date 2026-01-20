# Install

## Quick

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
```

**Windows:**
```powershell
irm https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1 | iex
```

## Methods

**Pre-built binaries:** Download from [releases](https://github.com/datadir-lab/bdp/releases)
**Cargo:** `cargo install bdp-cli`
**Source:** `git clone && cargo install --path crates/bdp-cli`

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
