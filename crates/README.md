# BDP Crates

**bdp-server:** API server (REST, CQRS, storage, DB)
**bdp-cli:** CLI tool (commands, cache, audit)
**bdp-ingest:** Data ingestion (UniProt, NCBI, Ensembl)
**bdp-common:** Shared types and utilities

```bash
cargo build --workspace         # Build all
cargo build -p bdp-server       # Build specific
cargo test --workspace          # Test all
```
