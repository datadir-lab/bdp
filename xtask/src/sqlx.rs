use crate::utils::*;
/// SQLx management
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum SqlxCommand {
    /// Generate SQLx offline metadata
    Prepare,
    /// Verify SQLx metadata is up to date
    Check,
    /// Clean SQLx metadata
    Clean,
}

pub fn handle(cmd: SqlxCommand) -> Result<()> {
    match cmd {
        SqlxCommand::Prepare => prepare(),
        SqlxCommand::Check => check(),
        SqlxCommand::Clean => clean(),
    }
}

fn prepare() -> Result<()> {
    info("📦 Generating SQLx metadata...");
    run(
        "cargo",
        &["sqlx", "prepare", "--workspace", "--", "--all-targets"],
        "Generate metadata",
    )?;
    success("Metadata generated in .sqlx/");
    info("ℹ️  Commit .sqlx/ files to git for offline builds");
    Ok(())
}

fn check() -> Result<()> {
    info("🔍 Verifying SQLx metadata...");
    run(
        "cargo",
        &["sqlx", "prepare", "--check", "--workspace", "--", "--bins", "--lib"],
        "Verify metadata",
    )?;
    success("SQLx metadata is current");
    Ok(())
}

fn clean() -> Result<()> {
    info("🧹 Cleaning SQLx metadata...");
    if path_exists(".sqlx") {
        std::fs::remove_dir_all(".sqlx")?;
    }
    success("SQLx metadata cleaned");
    Ok(())
}
