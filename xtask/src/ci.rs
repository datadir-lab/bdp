use crate::utils::*;
/// CI/CD simulation
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum CiCommand {
    /// Run all CI checks locally
    All,
    /// Run CI checks in offline mode (like GitHub Actions)
    Offline,
}

pub fn handle(cmd: CiCommand) -> Result<()> {
    match cmd {
        CiCommand::All => all(),
        CiCommand::Offline => offline(),
    }
}

fn all() -> Result<()> {
    crate::docs::handle(crate::docs::DocsCommand::CliCheck)?;
    crate::sqlx::handle(crate::sqlx::SqlxCommand::Check)?;
    crate::dev::handle(crate::dev::DevCommand::Lint)?;
    crate::test::handle(crate::test::TestCommand::All)?;
    success("All CI checks passed!");
    Ok(())
}

fn offline() -> Result<()> {
    info("🔍 Running CI checks (offline mode)...");

    std::env::set_var("SQLX_OFFLINE", "true");

    run("cargo", &["check", "--workspace", "--all-features"], "Check workspace")?;
    run(
        "cargo",
        &["clippy", "--workspace", "--all-features", "--", "-D", "warnings"],
        "Clippy",
    )?;
    run("cargo", &["test", "--workspace", "--lib"], "Unit tests")?;

    success("Offline CI checks passed!");
    Ok(())
}
