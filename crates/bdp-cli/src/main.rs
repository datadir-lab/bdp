//! BDP CLI - Main entry point

use bdp_cli::{Cli, Commands, ConfigCommand, SourceCommand};
use bdp_common::logging::{init_logging, LogConfig, LogLevel, LogOutput};
use clap::Parser;
use std::process;
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
        // Normal mode: only errors to console, info+ to file
        LogConfig::builder()
            .level(LogLevel::Warn)
            .output(LogOutput::Console)
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
        } => {
            bdp_cli::commands::init::run(
                path.clone(),
                name.clone(),
                version.clone(),
                description.clone(),
                *force,
            )
            .await
        }

        Commands::Source { command } => match command {
            SourceCommand::Add { source } => {
                bdp_cli::commands::source::add(source.clone()).await
            }
            SourceCommand::Remove { source } => {
                bdp_cli::commands::source::remove(source.clone()).await
            }
            SourceCommand::List => bdp_cli::commands::source::list().await,
        },

        Commands::Pull { force } => {
            bdp_cli::commands::pull::run(cli.server_url.clone(), *force).await
        }

        Commands::Status => bdp_cli::commands::status::run().await,

        Commands::Audit { command } => bdp_cli::commands::audit::run(command).await,

        Commands::Clean { all } => bdp_cli::commands::clean::run(*all).await,

        Commands::Config { command } => match command {
            ConfigCommand::Get { key } => bdp_cli::commands::config::get(key.clone()).await,
            ConfigCommand::Set { key, value } => {
                bdp_cli::commands::config::set(key.clone(), value.clone()).await
            }
            ConfigCommand::Show => bdp_cli::commands::config::show().await,
        },

        Commands::Uninstall { yes, purge } => {
            bdp_cli::commands::uninstall::run(*yes, *purge).await
        }
    }
}
