use crate::utils::*;
/// Database operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum DbCommand {
    /// Start development database
    Up,
    /// Stop database
    Down,
    /// Start test database
    TestUp,
    /// Stop test database
    TestDown,
    /// Complete database setup (start + migrate)
    Setup,
    /// Run database migrations
    Migrate,
    /// Revert last migration
    MigrateRevert,
    /// Create new migration
    MigrateAdd {
        /// Migration name
        name: String,
    },
    /// Reset database (dangerous - drops all data)
    Reset,
    /// Seed development data
    Seed,
    /// Connect to database with psql
    Shell,
    /// Database logs
    Logs,
}

pub fn handle(cmd: DbCommand) -> Result<()> {
    match cmd {
        DbCommand::Up => up(),
        DbCommand::Down => down(),
        DbCommand::TestUp => test_up(),
        DbCommand::TestDown => test_down(),
        DbCommand::Setup => setup(),
        DbCommand::Migrate => migrate(),
        DbCommand::MigrateRevert => migrate_revert(),
        DbCommand::MigrateAdd { name } => migrate_add(&name),
        DbCommand::Reset => reset(),
        DbCommand::Seed => seed(),
        DbCommand::Shell => shell(),
        DbCommand::Logs => logs(),
    }
}

fn up() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🐘 Starting PostgreSQL..."
docker compose up -d postgres
Write-Host "⏳ Waiting for database..."
Start-Sleep -Seconds 3
Write-Host "✓ Database ready"
"#,
            "Starting PostgreSQL",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🐘 Starting PostgreSQL..."
docker compose up -d postgres
echo "⏳ Waiting for database..."
sleep 3
echo "✓ Database ready"
"#,
            "Starting PostgreSQL",
        )
    }
}

fn down() -> Result<()> {
    info("Stopping PostgreSQL...");
    run("docker", &["compose", "down", "postgres"], "Stop PostgreSQL")
}

fn test_up() -> Result<()> {
    info("🧪 Starting test database...");
    run("docker", &["compose", "up", "-d", "postgres-test"], "Start test database")?;
    info("⏳ Waiting for test database...");
    sleep(3);
    success("Test database ready");
    Ok(())
}

fn test_down() -> Result<()> {
    info("Stopping test database...");
    run("docker", &["compose", "down", "postgres-test"], "Stop test database")
}

fn setup() -> Result<()> {
    up()?;
    sleep(2);
    Ok(())
}

fn migrate() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🔄 Running migrations..."
sqlx migrate run
Write-Host "✓ Migrations complete"
"#,
            "Running migrations",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🔄 Running migrations..."
sqlx migrate run
echo "✓ Migrations complete"
"#,
            "Running migrations",
        )
    }
}

fn migrate_revert() -> Result<()> {
    info("⏪ Reverting last migration...");
    run("sqlx", &["migrate", "revert"], "Revert migration")?;
    success("Migration reverted");
    Ok(())
}

fn migrate_add(name: &str) -> Result<()> {
    info(&format!("📝 Creating migration: {}", name));
    run("sqlx", &["migrate", "add", name], "Create migration")?;
    success("Migration file created in migrations/");
    Ok(())
}

fn reset() -> Result<()> {
    warning("⚠️  WARNING: This will delete all data!");
    println!("Press Ctrl+C to cancel, Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    run("docker", &["compose", "down", "postgres", "-v"], "Drop database volumes")?;
    success("Database reset");
    setup()?;
    migrate()
}

fn seed() -> Result<()> {
    info("🌱 Seeding database...");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    run("psql", &[&database_url, "-f", "scripts/seed-data.sql"], "Seed database")?;
    success("Database seeded");
    Ok(())
}

fn shell() -> Result<()> {
    info("🐘 Connecting to database...");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    run_streaming("psql", &[&database_url], "Connect to database")
}

fn logs() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f", "postgres"], "Database logs")
}
