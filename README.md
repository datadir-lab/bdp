# BDP - Bioinformatics Dependencies Platform

Version-controlled registry for biological data sources. Think npm/cargo for bioinformatics data.

**Features:** UniProt, NCBI Taxonomy & GenBank/RefSeq ingestion, version control, lockfiles, audit trails, batch operations, parallel processing, S3 storage

## Quick Start

**CLI:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
bdp init && bdp source add "uniprot:P01308-fasta@1.0" && bdp pull
```

**Development:**
```bash
git clone https://github.com/datadir-lab/bdp.git && cd bdp
docker-compose up -d
# API: http://localhost:8000 | MinIO: http://localhost:9001
```

## Status

| Component | Status |
|-----------|--------|
| CLI, Backend, Ingestion, Audit | ✅ Complete |
| Export Formats, Post-Pull Hooks, Web | 🚧 Planned |

## Architecture

CLI/Server (Rust) → PostgreSQL (registry + coordination) → MinIO/S3 (storage)

## Data Sources

BDP supports automated ingestion from major biological databases with optimized batch operations and parallel processing:

### UniProt
- **Status:** ✅ Production Ready
- **Archives:** Quarterly releases (20+ years of history)
- **Performance:** Batch operations with 300-500x query reduction
- **Features:** Protein sequences, metadata, Swiss-Prot/TrEMBL support

### NCBI Taxonomy
- **Status:** ✅ Production Ready
- **Archives:** Monthly releases (86 versions, 2018-present)
- **Performance:** Batch operations (666x query reduction) + parallel processing (4x speedup)
- **Speed:** Full historical catchup (86 versions, 215M taxa) in ~3 hours (was 28 days)
- **Features:** Complete taxonomy tree, merged/deleted taxa tracking, version history

### GenBank/RefSeq
- **Status:** ✅ Implementation Complete (Ready for Testing)
- **Archives:** Bimonthly GenBank releases, quarterly RefSeq releases
- **Performance:** Batch operations (2,500x query reduction) + parallel processing (4x speedup)
- **Storage:** S3 for sequences (FASTA), PostgreSQL for metadata
- **Features:** Nucleotide sequences, protein mappings, 18 divisions, GC content, deduplication
- **Divisions:** Viral, Bacterial, Phage, Plant, Mammalian, Primate, Rodent, and more

**Documentation:**
- UniProt: See `crates/bdp-server/README.md`
- NCBI Taxonomy: See `NCBI_TAXONOMY_*.md` files in root directory
  - `NCBI_TAXONOMY_QUICK_REFERENCE.md` - Quick start guide
  - `NCBI_TAXONOMY_TESTING_GUIDE.md` - Testing instructions
  - `NCBI_TAXONOMY_IMPLEMENTATION_SUMMARY.md` - Complete overview
- GenBank/RefSeq: See `GENBANK_*.md` files in root directory
  - `GENBANK_REFSEQ_DESIGN.md` - Design document
  - `GENBANK_REFSEQ_IMPLEMENTATION_PLAN.md` - Implementation plan
  - `GENBANK_IMPLEMENTATION_SUMMARY.md` - Complete implementation summary

## Audit

Local SQLite trail (`.bdp/bdp.db`) with hash-chain integrity. Tracks all operations. Editable by design for research documentation.

```bash
bdp audit              # Verify checksums
bdp audit verify       # Verify chain integrity
```

Export formats (planned): FDA, NIH, EMA, DAS

## Development

**Prerequisites:** Docker, Rust 1.70+, sqlx-cli, just

```bash
git clone https://github.com/datadir-lab/bdp.git && cd bdp
cp .env.example .env
docker-compose up -d
curl http://localhost:8000/health
```

**Services:**
- API: http://localhost:8000
- MinIO: http://localhost:9001 (minioadmin/minioadmin)
- Postgres: localhost:5432 (bdp/bdp_dev_password)

## Workflows

**Backend:**
```bash
cargo watch -x 'run --bin bdp-server'  # Hot reload
cargo test                              # Run tests
cargo build --release                   # Production build
```

**Database:**
```bash
sqlx migrate add <name>      # New migration
sqlx migrate run             # Apply migrations
cargo sqlx prepare           # Offline mode
```

**CLI Testing:**
```bash
# Quick setup and test
just test-cli-setup         # Create test directory
just test-cli init          # Test CLI init command
just test-cli-full          # Run full workflow test

# Manual testing in different workspace
cd /path/to/test-workspace
cargo run --bin bdp -- init --name my-project
cargo run --bin bdp -- source add "uniprot:P01308-fasta@1.0"
cargo run --bin bdp -- source list
cargo run --bin bdp -- pull

# Install CLI locally for testing
just cli-install            # Installs bdp CLI globally
bdp --help                  # Test installed CLI
```

**Testing in Different Workspaces:**
1. **Create isolated test directory** (outside project):
   ```bash
   mkdir ~/bdp-test-workspace && cd ~/bdp-test-workspace
   ```

2. **Use development CLI** (from project):
   ```bash
   cargo run --bin bdp -- init --name test-project
   ```

3. **Or use installed CLI**:
   ```bash
   cargo install --path crates/bdp-cli
   bdp init --name test-project
   ```

4. **Verify isolation**:
   - Check `.bdp/` directory created in workspace
   - Check `bdp.toml` manifest
   - Check `.bdp/bdp.db` SQLite audit log

**Just commands:**
```bash
just --list         # Show all commands
just dev            # Run with hot reload
just test           # Run tests
just docker-up      # Start services
```

## Stack

**Backend:** Rust (axum, SQLx, mediator), PostgreSQL 16, MinIO/S3
**CLI:** Rust (clap, reqwest), SQLite
**Infrastructure:** Docker, GitHub Actions, cargo-dist

## Production

```bash
cargo build --release --bin bdp-server
docker build -f docker/Dockerfile.server -t bdp-server:latest .
```

**Checklist:** Strong JWT_SECRET, production DB URL, S3/IAM setup, SSL/TLS, RUST_LOG=info, monitoring

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Use tracing, write tests, run fmt/clippy, read [AGENTS.md](./AGENTS.md).

## Docs

**General:** [INSTALL.md](./INSTALL.md) | [CONTRIBUTING.md](./CONTRIBUTING.md) | [AGENTS.md](./AGENTS.md) | [docs/](./docs/)

**Data Sources:**
- UniProt: [crates/bdp-server/README.md](./crates/bdp-server/README.md)
- NCBI Taxonomy: [Quick Reference](./NCBI_TAXONOMY_QUICK_REFERENCE.md) | [Testing Guide](./NCBI_TAXONOMY_TESTING_GUIDE.md) | [Implementation Summary](./NCBI_TAXONOMY_IMPLEMENTATION_SUMMARY.md)

## License & Support

[LICENSE](./LICENSE) | [Issues](https://github.com/datadir-lab/bdp/issues) | support@datadir.dev
