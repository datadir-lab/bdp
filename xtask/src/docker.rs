use crate::utils::*;
/// Docker operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum DockerCommand {
    /// Start all services with Docker Compose
    Up,
    /// Stop all Docker Compose services
    Down,
    /// Run migrations in Docker container
    Migrate,
    /// View logs from all services
    Logs,
    /// View backend logs
    LogsBackend,
    /// Restart backend service
    RestartBackend,
    /// Full stack with migrations (recommended for first time)
    Setup,
}

pub fn handle(cmd: DockerCommand) -> Result<()> {
    match cmd {
        DockerCommand::Up => up(),
        DockerCommand::Down => down(),
        DockerCommand::Migrate => migrate(),
        DockerCommand::Logs => logs(),
        DockerCommand::LogsBackend => logs_backend(),
        DockerCommand::RestartBackend => restart_backend(),
        DockerCommand::Setup => setup(),
    }
}

fn up() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🐳 Starting all services..."
docker compose up -d
Write-Host "⏳ Waiting for services to be ready..."
Start-Sleep -Seconds 5
Write-Host "✓ Services started"
Write-Host "  🗄️  PostgreSQL:   localhost:5432"
Write-Host "  🚀 Backend API:   http://localhost:8000"
Write-Host "  📦 MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
Write-Host ""
Write-Host "💡 Run migrations: cargo xtask docker migrate"
Write-Host "💡 Start frontend: cargo xtask dev web (dev) or cargo xtask dev web-prod (production)"
"#,
            "Starting all services",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🐳 Starting all services..."
docker compose up -d
echo "⏳ Waiting for services to be ready..."
sleep 5
echo "✓ Services started"
echo "  🗄️  PostgreSQL:   localhost:5432"
echo "  🚀 Backend API:   http://localhost:8000"
echo "  📦 MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
echo ""
echo "💡 Run migrations: cargo xtask docker migrate"
echo "💡 Start frontend: cargo xtask dev web (dev) or cargo xtask dev web-prod (production)"
"#,
            "Starting all services",
        )
    }
}

fn down() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🛑 Stopping all services..."
docker compose down
Write-Host "✓ Services stopped"
"#,
            "Stopping all services",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🛑 Stopping all services..."
docker compose down
echo "✓ Services stopped"
"#,
            "Stopping all services",
        )
    }
}

fn migrate() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🔄 Running migrations in Docker..."
docker compose exec bdp-server sqlx migrate run
Write-Host "✓ Migrations complete"
"#,
            "Running migrations",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🔄 Running migrations in Docker..."
docker compose exec bdp-server sqlx migrate run
echo "✓ Migrations complete"
"#,
            "Running migrations",
        )
    }
}

fn logs() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f"], "View logs")
}

fn logs_backend() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f", "bdp-server"], "View backend logs")
}

fn restart_backend() -> Result<()> {
    info("🔄 Restarting backend...");
    run("docker", &["compose", "restart", "bdp-server"], "Restart backend")?;
    success("Backend restarted");
    Ok(())
}

fn setup() -> Result<()> {
    up()?;

    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "⏳ Waiting for database to be ready..."
Start-Sleep -Seconds 3
"#,
            "Waiting for database",
        )?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        sleep(3);
    }

    migrate()?;

    println!();
    success("Full stack ready!");
    println!("  🌐 Start frontend: cargo xtask dev web");
    println!("  🌐 Frontend URL:   http://localhost:3000");
    Ok(())
}
