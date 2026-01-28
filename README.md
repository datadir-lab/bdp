# BDP - Bioinformatics Dependencies Platform

Version-controlled registry for biological data sources. Think npm/cargo for bioinformatics data.

## Quick Start

```bash
# Install CLI
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh

# Use it
bdp init && bdp source add "uniprot:P01308-fasta@1.0" && bdp pull
```

## Features

- **Version Control** - Lock down exact data versions for reproducibility
- **Audit Trail** - Regulatory-compliant audit logs (FDA 21 CFR Part 11, NIH DMS, EMA ALCOA++)
- **Data Sources** - UniProt, NCBI Taxonomy, GenBank/RefSeq, Gene Ontology
- **Lockfiles** - Reproducible dependency resolution
- **S3 Storage** - Scalable object storage for large datasets

## Status

| Component | Status |
|-----------|--------|
| CLI, Backend, Ingestion, Audit | ✅ Complete |
| Web Interface | ✅ Complete |
| Production Data | 🚧 In Progress |

## Documentation

- **[Installation Guide](./docs/INSTALL.md)** - Get started
- **[Quick Start](./docs/QUICK_START.md)** - 5-minute tutorial
- **[Full Documentation](./docs/INDEX.md)** - Complete documentation index
- **[Web Docs](https://bdp.datadir.dev/docs)** - Online documentation

## Development

```bash
# Clone and setup
git clone https://github.com/datadir-lab/bdp.git && cd bdp
docker-compose up -d

# Services
# API: http://localhost:8000
# MinIO: http://localhost:9001
# PostgreSQL: localhost:5432
```

See **[Development Setup](./docs/SETUP.md)** for details.

## Architecture

- **CLI/Server**: Rust (axum, SQLx, mediator CQRS pattern)
- **Database**: PostgreSQL 16
- **Storage**: MinIO/S3
- **Frontend**: Next.js 16
- **Infrastructure**: Terraform + OVH Cloud

## Contributing

See **[CONTRIBUTING.md](./CONTRIBUTING.md)** for guidelines.

## License

[LICENSE](./LICENSE)

## Contact

**Email**: sebastian.stupak@pm.me  
**Issues**: https://github.com/datadir-lab/bdp/issues
