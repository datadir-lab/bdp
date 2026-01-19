//! BDP CLI Library
//!
//! Command-line interface for managing biological datasets with BDP.
//!
//! # Overview
//!
//! The BDP CLI provides a package manager-like experience for biological datasets:
//!
//! - **Project Management**: Initialize and configure BDP projects (`bdp init`)
//! - **Source Management**: Add/remove data sources (`bdp source add/remove/list`)
//! - **Dataset Installation**: Download and verify datasets (`bdp pull`)
//! - **Status Checking**: View cached datasets (`bdp status`)
//! - **Integrity Auditing**: Verify checksums (`bdp audit`)
//! - **Cache Management**: Clean unused cache (`bdp clean`)
//! - **Configuration**: Manage CLI settings (`bdp config`)

pub mod api;
pub mod audit;
pub mod cache;
pub mod checksum;
pub mod commands;
pub mod config;
pub mod error;
pub mod gitignore;
pub mod lockfile;
pub mod manifest;
pub mod progress;

// Re-export commonly used types
pub use error::{CliError, Result};
pub use manifest::Manifest;
pub use lockfile::Lockfile;

use clap::{Parser, Subcommand};

/// BDP - Biological Dataset Package Manager
#[derive(Parser, Debug)]
#[command(name = "bdp")]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Server URL
    #[arg(long, env = "BDP_SERVER_URL", default_value = "http://localhost:8000", global = true)]
    pub server_url: String,
}

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new BDP project
    Init {
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Project name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Project version
        #[arg(short = 'V', long, default_value = "0.1.0")]
        version: String,

        /// Project description
        #[arg(short, long)]
        description: Option<String>,

        /// Force overwrite if bdp.yml exists
        #[arg(short, long)]
        force: bool,
    },

    /// Manage data sources
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },

    /// Download and cache sources from manifest
    Pull {
        /// Force re-download even if cached
        #[arg(short, long)]
        force: bool,
    },

    /// Show status of cached sources
    Status,

    /// Audit trail management
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },

    /// Clean cache
    Clean {
        /// Clean all cached files
        #[arg(short, long)]
        all: bool,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Uninstall BDP from your system
    Uninstall {
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,

        /// Also remove cache and configuration files
        #[arg(long)]
        purge: bool,
    },
}

/// Source management subcommands
#[derive(Subcommand, Debug)]
pub enum SourceCommand {
    /// Add a source to the manifest
    Add {
        /// Source specification (e.g., "uniprot:P01308-fasta@1.0")
        source: String,
    },

    /// Remove a source from the manifest
    Remove {
        /// Source specification
        source: String,
    },

    /// List sources in the manifest
    List,
}

/// Configuration subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Show all configuration
    Show,
}

/// Audit trail subcommands
#[derive(Subcommand, Debug)]
pub enum AuditCommand {
    /// List audit events
    List {
        /// Limit number of events to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Show events from specific source
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Verify audit trail integrity
    Verify,

    /// Export audit trail to regulatory format
    Export {
        /// Export format (fda, nih, ema, das, json)
        #[arg(short, long, default_value = "fda")]
        format: String,

        /// Output file path (optional, defaults to audit-{format}.{ext})
        #[arg(short, long)]
        output: Option<String>,

        /// Filter events from date (ISO 8601)
        #[arg(long)]
        from: Option<String>,

        /// Filter events to date (ISO 8601)
        #[arg(long)]
        to: Option<String>,

        /// Project name for report
        #[arg(short = 'n', long)]
        project_name: Option<String>,

        /// Project version for report
        #[arg(short = 'v', long)]
        project_version: Option<String>,
    },
}
