//! BDP CLI - Main entry point

use std::process;

use bdp_cli::commands::output::Render;
use bdp_cli::{CacheCommand, Cli, Commands, ConfigCommand, GenerateCommand, SourceCommand};
use bdp_common::logging::{init_logging, LogConfig, LogLevel, LogOutput};
use clap::Parser;
use tracing::error;

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Handle markdown help generation
    if cli.markdown_help {
        println!("{}", clap_markdown::help_markdown::<Cli>());
        return;
    }

    // Ensure a command is provided
    if cli.command.is_none() {
        eprintln!("Error: A subcommand is required");
        eprintln!();
        eprintln!("For more information, try '--help'.");
        process::exit(2);
    }

    // Initialize logging based on verbose flag and environment
    let log_config = if cli.verbose {
        // Verbose mode: log to console with debug level
        LogConfig::builder()
            .level(LogLevel::Debug)
            .output(LogOutput::Console)
            .log_file_prefix("bdp-cli".to_string())
            .build()
    } else {
        // Normal mode: disable console logging (users see println! output only)
        // All logs go to file only to avoid polluting user output
        LogConfig::builder()
            .level(LogLevel::Info)
            .output(LogOutput::File)
            .log_file_prefix("bdp-cli".to_string())
            .build()
    };

    // Merge with environment variables (they take precedence)
    let log_config = LogConfig::from_env().unwrap_or(log_config);

    // Initialize logging (ignore errors as CLI should work without logging)
    let _ = init_logging(&log_config);

    // Execute command
    let result = execute_command(&cli).await;

    // Handle result
    if let Err(e) = result {
        error!(error = %e, "Command failed");
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Execute the CLI command
async fn execute_command(cli: &Cli) -> bdp_cli::Result<()> {
    // Command is guaranteed to exist at this point (checked in main)
    let Some(ref command) = cli.command else {
        unreachable!("Command should have been validated in main");
    };

    match command {
        Commands::Init {
            path,
            name,
            version,
            description,
            force,
        } => bdp_cli::commands::init::run(
            path.clone(),
            name.clone(),
            version.clone(),
            description.clone(),
            *force,
        )
        .await
        .map(|output| output.render()),

        Commands::Source { command } => match command {
            SourceCommand::Add { source } => bdp_cli::commands::source::add(source.clone())
                .await
                .map(|output| output.render()),
            SourceCommand::Remove { source } => bdp_cli::commands::source::remove(source.clone())
                .await
                .map(|output| output.render()),
            SourceCommand::List => bdp_cli::commands::source::list()
                .await
                .map(|output| output.render()),
        },

        Commands::Pull { force } => bdp_cli::commands::pull::run(cli.server_url.clone(), *force)
            .await
            .map(|output| output.render()),

        Commands::Cache { command } => match command {
            CacheCommand::Set { path } => bdp_cli::commands::cache_cmd::set(path.clone())
                .await
                .map(|output| output.render()),
            CacheCommand::Show => bdp_cli::commands::cache_cmd::show()
                .await
                .map(|output| output.render()),
            CacheCommand::Reset => bdp_cli::commands::cache_cmd::reset()
                .await
                .map(|output| output.render()),
        },

        Commands::Generate { command } => match command {
            GenerateCommand::Python => bdp_cli::commands::generate::python()
                .await
                .map(|output| output.render()),
            GenerateCommand::Snakemake => bdp_cli::commands::generate::snakemake()
                .await
                .map(|output| output.render()),
            GenerateCommand::Nextflow => bdp_cli::commands::generate::nextflow()
                .await
                .map(|output| output.render()),
        },

        Commands::Status => bdp_cli::commands::status::run()
            .await
            .map(|output| output.render()),

        Commands::Audit { command } => bdp_cli::commands::audit::run(command)
            .await
            .map(|output| output.render()),

        Commands::Clean {
            all,
            search_cache,
            yes,
        } => bdp_cli::commands::clean::run(*all, *search_cache, *yes)
            .await
            .map(|output| output.render()),

        Commands::Config { command } => match command {
            // NOTE: Clone is necessary because we're matching on &command (borrowed)
            ConfigCommand::Get { key } => bdp_cli::commands::config::get(key.clone())
                .await
                .map(|output| output.render()),
            ConfigCommand::Set { key, value } => {
                bdp_cli::commands::config::set(key.clone(), value.clone())
                    .await
                    .map(|output| output.render())
            },
            ConfigCommand::Show => bdp_cli::commands::config::show()
                .await
                .map(|output| output.render()),
        },

        Commands::Uninstall { yes, purge } => bdp_cli::commands::uninstall::run(*yes, *purge).await,

        Commands::Search {
            query,
            org,
            entry_type,
            source_type,
            format,
            no_interactive,
            limit,
            page,
        } => {
            bdp_cli::commands::search::run(
                query.clone(),
                org.clone(),
                entry_type.clone(),
                source_type.clone(),
                format.clone(),
                *no_interactive,
                *limit,
                *page,
                cli.server_url.clone(),
            )
            .await
        },

        Commands::Query {
            entity,
            select,
            where_clause,
            order_by,
            limit,
            offset,
            group_by,
            aggregate,
            having,
            join,
            on,
            sql,
            format,
            output,
            no_header,
            explain,
            dry_run,
        } => {
            bdp_cli::commands::query::run(
                entity.clone(),
                select.clone(),
                where_clause.clone(),
                order_by.clone(),
                *limit,
                *offset,
                group_by.clone(),
                aggregate.clone(),
                having.clone(),
                join.clone(),
                on.clone(),
                sql.clone(),
                format.clone(),
                output.clone(),
                *no_header,
                *explain,
                *dry_run,
                cli.server_url.clone(),
            )
            .await
        },
    }
}
