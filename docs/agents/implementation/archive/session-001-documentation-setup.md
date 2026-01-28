# Session 001: Documentation Setup

**Date**: 2026-01-16
**Agent**: Claude Sonnet 4.5
**Status**: Complete

## Summary

Set up the complete agent documentation structure for the BDP project rewrite.

## Key Decisions

### Technology Stack
- **Backend**: Rust + axum + PostgreSQL
- **CLI**: Rust + clap
- **Frontend**: Next.js 16 + Nextra
- **Deployment**: Single server architecture

### CLI Command Model

BDP manages bioinformatics **data sources** and **tools**, not traditional packages:

```bash
bdp init                              # Initialize project
bdp source add uniprot:P12345-fasta@1.0     # Add protein from UniProt
bdp source add ncbi:genome/GCA_000001405.29  # Add reference genome
bdp tool add samtools@1.18            # Add bioinformatics tool
bdp lock                              # Lock all dependencies
bdp sync                              # Download and verify sources
```

### Manifest Format (bdp.toml)

```toml
[project]
name = "my-analysis"
version = "0.1.0"
description = "Protein structure analysis pipeline"
authors = ["Your Name <you@example.com>"]
keywords = ["protein", "structure"]

[sources]
insulin = "uniprot:P01308-fasta@1.0"
hemoglobin = { provider = "pdb", identifier = "1A3N", version = "latest" }
human_genome = "ncbi:genome/GCA_000001405.29"

[tools]
samtools = "1.18.0"
bwa = "0.7.17"
blast = "2.14.0"
```

## Documentation Structure

```
docs/agents/
├── architecture.md          # System design, database schema, API design
├── stack.md                 # Technology choices and rationale
├── rust-backend.md          # Backend development patterns
├── cli-development.md       # CLI commands and structure
├── nextjs-frontend.md       # Next.js 16 frontend guide
└── implementation/          # Session-specific docs
    ├── .gitkeep
    └── session-001-documentation-setup.md  # This file
```

### Documentation Organization Rules

- `docs/agents/*.md` → Permanent agent reference documentation
- `docs/agents/implementation/` → Temporary session notes, summaries, implementation logs

## Files Created

1. **README.md** - Brief project overview
2. **AGENTS.md** - Entry point for all agent documentation
3. **docs/agents/architecture.md** - System architecture
4. **docs/agents/stack.md** - Technology stack breakdown
5. **docs/agents/rust-backend.md** - Rust backend guide
6. **docs/agents/cli-development.md** - CLI development guide
7. **docs/agents/nextjs-frontend.md** - Next.js 16 frontend guide
8. **docs/agents/implementation/.gitkeep** - Implementation docs directory

## Next Steps

1. Complete remaining agent docs:
   - `api-integration.md`
   - `testing.md`
   - `best-practices.md`
   - `deployment.md`

2. Begin implementation:
   - Set up Cargo workspace
   - Create crate scaffolding
   - PostgreSQL schema
   - Basic axum server skeleton

3. Source providers to implement:
   - UniProt (protein sequences)
   - NCBI (genomes, sequences)
   - PDB (protein structures)
   - Ensembl (genomes, annotations)

## References

- Next.js 16: https://nextjs.org/docs
- axum: https://docs.rs/axum/
- clap: https://docs.rs/clap/
- PostgreSQL: https://www.postgresql.org/docs/
