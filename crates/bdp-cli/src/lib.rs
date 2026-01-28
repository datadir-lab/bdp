//! BDP CLI Library
#![deny(clippy::unwrap_used, clippy::expect_used)]
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
pub use lockfile::Lockfile;
pub use manifest::Manifest;

use clap::{Parser, Subcommand};

/// BDP - Biological Dataset Package Manager
#[derive(Parser, Debug)]
#[command(name = "bdp")]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Server URL
    #[arg(
        long,
        env = "BDP_SERVER_URL",
        default_value = "http://localhost:8000",
        global = true
    )]
    pub server_url: String,

    /// Generate markdown documentation (hidden)
    #[arg(long, hide = true)]
    pub markdown_help: bool,
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

        /// Clean only search cache
        #[arg(long)]
        search_cache: bool,
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

    /// Search for data sources and tools in the registry
    Search {
        /// Search query (multiple words will be joined)
        #[arg(required = true)]
        query: Vec<String>,

        /// Filter by entry type (can be repeated)
        #[arg(short = 't', long = "type")]
        entry_type: Vec<String>,

        /// Filter by source type (can be repeated)
        #[arg(short = 's', long = "source-type")]
        source_type: Vec<String>,

        /// Output format
        #[arg(short = 'f', long = "format", default_value = "interactive")]
        format: String,

        /// Force non-interactive mode
        #[arg(long = "no-interactive")]
        no_interactive: bool,

        /// Number of results per page (1-100)
        #[arg(short = 'l', long = "limit", default_value = "10")]
        limit: i32,

        /// Page number (for non-interactive pagination)
        #[arg(short = 'p', long = "page", default_value = "1")]
        page: i32,
    },

    /// Advanced SQL-like querying of data sources and metadata
    Query {
        /// Entity to query (protein, gene, genome, tools, orgs, etc.) or use --sql for raw SQL
        entity: Option<String>,

        /// Select specific fields (comma-separated)
        #[arg(long)]
        select: Option<String>,

        /// Filter results (can be repeated, AND combined)
        /// Simple: --where organism=human
        /// Complex: --where "organism='human' AND downloads>1000"
        #[arg(short = 'w', long = "where")]
        where_clause: Vec<String>,

        /// Sort results by field[:asc|desc]
        #[arg(long)]
        order_by: Option<String>,

        /// Limit number of results (default: 1000)
        #[arg(short = 'l', long, default_value = "1000")]
        limit: i64,

        /// Skip first N results
        #[arg(long)]
        offset: Option<i64>,

        /// Group results by field
        #[arg(long)]
        group_by: Option<String>,

        /// Aggregation expression (COUNT(*), SUM(field), etc.)
        #[arg(long)]
        aggregate: Option<String>,

        /// Filter grouped results
        #[arg(long)]
        having: Option<String>,

        /// Join with another entity/table
        #[arg(long)]
        join: Option<String>,

        /// Join condition
        #[arg(long)]
        on: Option<String>,

        /// Execute raw SQL query directly
        #[arg(long, conflicts_with_all = &["entity", "select", "where_clause", "order_by", "group_by", "aggregate", "having", "join", "on"])]
        sql: Option<String>,

        /// Output format
        #[arg(short = 'f', long = "format")]
        format: Option<String>,

        /// Write output to file instead of stdout
        #[arg(short = 'o', long = "output")]
        output: Option<String>,

        /// Omit header row (for CSV/TSV)
        #[arg(long)]
        no_header: bool,

        /// Show query execution plan
        #[arg(long)]
        explain: bool,

        /// Show generated SQL without executing
        #[arg(long)]
        dry_run: bool,
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
