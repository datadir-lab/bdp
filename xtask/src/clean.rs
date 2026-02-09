use crate::utils::*;
/// Cleanup operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum CleanCommand {
    /// Clean build artifacts
    Workspace,
    /// Deep clean (including dependencies)
    All,
    /// Stop all Docker services
    Stop,
    /// Stop all and remove volumes (deletes data)
    StopAll,
}

pub fn handle(cmd: CleanCommand) -> Result<()> {
    match cmd {
        CleanCommand::Workspace => workspace(),
        CleanCommand::All => all(),
        CleanCommand::Stop => stop(),
        CleanCommand::StopAll => stop_all(),
    }
}

fn workspace() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧹 Cleaning build artifacts..."
cargo clean
cd web; Remove-Item -Recurse -Force .next, node_modules/.cache -ErrorAction SilentlyContinue
Write-Host "✓ Cleaned"
"#,
            "Cleaning workspace",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧹 Cleaning build artifacts..."
cargo clean
cd web && rm -rf .next node_modules/.cache 2>/dev/null || true
echo "✓ Cleaned"
"#,
            "Cleaning workspace",
        )
    }
}

fn all() -> Result<()> {
    workspace()?;

    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧹 Deep cleaning..."
cd web; Remove-Item -Recurse -Force node_modules -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force target -ErrorAction SilentlyContinue
Write-Host "✓ Deep cleaned"
"#,
            "Deep cleaning",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧹 Deep cleaning..."
cd web && rm -rf node_modules 2>/dev/null || true
rm -rf target 2>/dev/null || true
echo "✓ Deep cleaned"
"#,
            "Deep cleaning",
        )
    }
}

fn stop() -> Result<()> {
    info("🛑 Stopping all services...");
    run("docker", &["compose", "down"], "Stop services")?;
    success("Services stopped");
    Ok(())
}

fn stop_all() -> Result<()> {
    info("🛑 Stopping all services and removing volumes...");
    run("docker", &["compose", "down", "-v"], "Stop services and remove volumes")?;
    success("Services stopped, volumes removed");
    Ok(())
}
