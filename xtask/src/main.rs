//! Build automation tasks for BDP
//!
//! This tool provides comprehensive automation tasks for the BDP project, including:
//! - Database operations (setup, migrations, seeding)
//! - Development workflows (building, testing, running)
//! - CI/CD operations (linting, testing, checks)
//! - Documentation generation
//! - Docker and infrastructure management
//! - Release management

use clap::Parser;

// Import all task modules
mod build;
mod ci;
mod clean;
mod db;
mod dev;
mod docker;
mod docs;
mod e2e;
mod infra;
mod ingest;
mod minio;
mod release;
mod setup;
mod sqlx;
mod test;
mod util;
mod utils;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation tasks for BDP", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Database operations
    #[command(subcommand)]
    Db(db::DbCommand),

    /// Build tasks
    #[command(subcommand)]
    Build(build::BuildCommand),

    /// Development operations
    #[command(subcommand)]
    Dev(dev::DevCommand),

    /// Testing operations
    #[command(subcommand)]
    Test(test::TestCommand),

    /// Documentation operations
    #[command(subcommand)]
    Docs(docs::DocsCommand),

    /// CI/CD simulation
    #[command(subcommand)]
    Ci(ci::CiCommand),

    /// Cleanup operations
    #[command(subcommand)]
    Clean(clean::CleanCommand),

    /// Docker operations
    #[command(subcommand)]
    Docker(docker::DockerCommand),

    /// SQLx operations
    #[command(subcommand)]
    Sqlx(sqlx::SqlxCommand),

    /// MinIO operations
    #[command(subcommand)]
    Minio(minio::MinioCommand),

    /// Ingestion operations
    #[command(subcommand)]
    Ingest(ingest::IngestCommand),

    /// Setup operations
    #[command(subcommand)]
    Setup(setup::SetupCommand),

    /// Infrastructure operations
    #[command(subcommand)]
    Infra(infra::InfraCommand),

    /// E2E testing operations
    #[command(subcommand)]
    E2e(e2e::E2eCommand),

    /// Release management
    #[command(subcommand)]
    Release(release::ReleaseCommand),

    /// Utility operations
    #[command(subcommand)]
    Util(util::UtilCommand),

    // ===== Flat command aliases for backward compatibility =====
    /// Generate CLI documentation in MDX format (legacy command)
    GenerateCliDocs {
        /// Output directory for generated documentation
        #[arg(short, long, default_value = "web/app/[locale]/docs/content/en")]
        output_dir: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // Module commands
        Command::Db(cmd) => db::handle(cmd),
        Command::Build(cmd) => build::handle(cmd),
        Command::Dev(cmd) => dev::handle(cmd),
        Command::Test(cmd) => test::handle(cmd),
        Command::Docs(cmd) => docs::handle(cmd),
        Command::Ci(cmd) => ci::handle(cmd),
        Command::Clean(cmd) => clean::handle(cmd),
        Command::Docker(cmd) => docker::handle(cmd),
        Command::Sqlx(cmd) => sqlx::handle(cmd),
        Command::Minio(cmd) => minio::handle(cmd),
        Command::Ingest(cmd) => ingest::handle(cmd),
        Command::Setup(cmd) => setup::handle(cmd),
        Command::Infra(cmd) => infra::handle(cmd),
        Command::E2e(cmd) => e2e::handle(cmd),
        Command::Release(cmd) => release::handle(cmd),
        Command::Util(cmd) => util::handle(cmd),

        // Legacy command (kept for backward compatibility)
        Command::GenerateCliDocs { output_dir } => generate_cli_docs(&output_dir),
    }
}

// ===== Legacy generate-cli-docs implementation =====
// This function is kept for backward compatibility
// New code should use: cargo xtask docs cli

fn generate_cli_docs(output_dir: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::path::PathBuf;

    println!("Generating CLI documentation...");

    // Generate markdown from clap definitions
    let markdown = clap_markdown::help_markdown::<bdp_cli::Cli>();

    // Create MDX content with frontmatter and enhanced formatting
    let mdx_content = format!(
        r#"---
title: CLI Reference
description: Complete command reference for the BDP CLI
---

# BDP CLI Reference

This documentation is auto-generated from the CLI source code. Last updated: {}.

## Overview

BDP (Biological Dataset Package) is a command-line tool for managing biological datasets with version control, checksums, and audit trails.

## Installation

### From Source

```bash
git clone https://github.com/datadir-lab/bdp.git
cd bdp
cargo install --path crates/bdp-cli
```

### Using Cargo

```bash
cargo install bdp-cli
```

## Quick Start

```bash
# Initialize a new project
bdp init --name my-project

# Add data sources
bdp source add uniprot:P01308-fasta@1.0

# Download sources
bdp pull

# Check status
bdp status

# View audit trail
bdp audit list
```

## Commands

{}

## Environment Variables

- `BDP_SERVER_URL` - Backend server URL (default: `http://localhost:8000`)
- `RUST_LOG` - Logging level (e.g., `debug`, `info`, `warn`, `error`)

## Configuration

BDP uses a `bdp.yml` manifest file in your project directory. This file is created automatically when you run `bdp init`.

Example `bdp.yml`:

```yaml
name: my-project
version: 0.1.0
description: My biological data project
sources:
  - id: uniprot:P01308-fasta@1.0
    checksum: sha256:abc123...
```

## Audit Trail

BDP maintains a cryptographically-linked audit trail of all operations in `.bdp/audit.db`. This provides:

- Tamper-evident logging
- Regulatory compliance (FDA 21 CFR Part 11, NIH, EMA)
- Full traceability of data sources

Export audit trails for compliance:

```bash
# Export to FDA format
bdp audit export --format fda --output audit-report.pdf

# Export to JSON
bdp audit export --format json --output audit.json
```

## Examples

### Working with Multiple Sources

```bash
# Initialize project
bdp init --name multi-source-project

# Add multiple sources
bdp source add uniprot:P01308-fasta@1.0
bdp source add ncbi-taxonomy:taxdump@2024-01
bdp source add genbank:NC_000001.11@latest

# List all sources
bdp source list

# Pull all sources
bdp pull

# Check what's cached
bdp status
```

### Verifying Data Integrity

```bash
# Verify audit trail integrity
bdp audit verify

# Export audit trail with date filter
bdp audit export \
  --format fda \
  --from 2024-01-01 \
  --to 2024-12-31 \
  --project-name "My Project" \
  --project-version "1.0.0"
```

## Support

- GitHub Issues: https://github.com/datadir-lab/bdp/issues
- Documentation: https://bdp.datadir.io/docs

---

*This documentation is automatically generated from the CLI source code. To update, run `cargo xtask generate-cli-docs`.*
"#,
        chrono::Utc::now().format("%Y-%m-%d"),
        markdown
    );

    // Create output directory if it doesn't exist
    let output_path = PathBuf::from(output_dir);
    fs::create_dir_all(&output_path)?;

    // Write the MDX file
    let file_path = output_path.join("cli-reference.mdx");
    fs::write(&file_path, mdx_content)?;

    println!("✅ Generated CLI documentation at: {}", file_path.display());
    println!();
    println!("Next steps:");
    println!("  1. Review the generated documentation");
    println!("  2. Commit it to version control");
    println!("  3. Add a CI check to ensure docs stay in sync");

    Ok(())
}
