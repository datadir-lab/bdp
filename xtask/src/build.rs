use crate::utils::*;
/// Build tasks
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum BuildCommand {
    /// Build all Rust crates
    Workspace,
    /// Build release version
    Release,
    /// Build all (backend + frontend)
    All,
    /// Build Docker images
    Docker,
}

pub fn handle(cmd: BuildCommand) -> Result<()> {
    match cmd {
        BuildCommand::Workspace => workspace(),
        BuildCommand::Release => release(),
        BuildCommand::All => all(),
        BuildCommand::Docker => docker(),
    }
}

fn workspace() -> Result<()> {
    info("🔨 Building Rust workspace...");
    run("cargo", &["build", "--workspace"], "Build workspace")
}

fn release() -> Result<()> {
    info("🔨 Building release version...");
    run("cargo", &["build", "--workspace", "--release"], "Build release")
}

fn all() -> Result<()> {
    workspace()?;
    crate::dev::web_build()?;
    success("All builds complete");
    Ok(())
}

fn docker() -> Result<()> {
    info("🐳 Building Docker images...");
    run(
        "docker",
        &["build", "-f", "docker/Dockerfile.server", "-t", "bdp-server:latest", "."],
        "Build server image",
    )?;
    run(
        "docker",
        &["build", "-f", "docker/Dockerfile.cli", "-t", "bdp-cli:latest", "."],
        "Build CLI image",
    )?;
    run(
        "docker",
        &["build", "-f", "docker/Dockerfile.ingest", "-t", "bdp-ingest:latest", "."],
        "Build ingest image",
    )?;
    run(
        "docker",
        &["build", "-f", "docker/Dockerfile.web", "-t", "bdp-web:latest", "."],
        "Build web image",
    )?;
    success("Docker images built");
    Ok(())
}
