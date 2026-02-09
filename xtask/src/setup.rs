use crate::utils::*;
/// Setup & installation
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum SetupCommand {
    /// Complete first-time setup (quick start)
    All,
    /// Install all dependencies
    InstallDeps,
    /// Setup environment file
    EnvSetup,
    /// Verify setup is correct
    Verify,
}

pub fn handle(cmd: SetupCommand) -> Result<()> {
    match cmd {
        SetupCommand::All => all(),
        SetupCommand::InstallDeps => install_deps(),
        SetupCommand::EnvSetup => env_setup(),
        SetupCommand::Verify => verify(),
    }
}

fn all() -> Result<()> {
    install_deps()?;
    env_setup()?;
    crate::db::handle(crate::db::DbCommand::Setup)?;
    crate::db::handle(crate::db::DbCommand::Migrate)?;
    success("Setup complete! Run 'cargo xtask dev server' to start development");
    Ok(())
}

fn install_deps() -> Result<()> {
    info("📦 Installing dependencies...");
    run("cargo", &["install", "sqlx-cli", "--features", "postgres"], "Install sqlx-cli")?;
    run_in_dir("web", "yarn", &["install"], "Install web dependencies")?;
    Ok(())
}

fn env_setup() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
if (!(Test-Path .env)) {
    Copy-Item .env.example .env
    Write-Host "✓ Created .env from .env.example"
} else {
    Write-Host "⚠ .env already exists, skipping"
}
"#,
            "Setup environment file",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
if [ ! -f .env ]; then
    cp .env.example .env
    echo "✓ Created .env from .env.example"
else
    echo "⚠ .env already exists, skipping"
fi
"#,
            "Setup environment file",
        )
    }
}

fn verify() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🔍 Verifying setup..."
Write-Host ""
Write-Host "📋 Required Files:"
if (Test-Path .env.example) { Write-Host "  ✓ .env.example" } else { Write-Host "  ✗ .env.example" }
if (Test-Path Cargo.toml) { Write-Host "  ✓ Cargo.toml" } else { Write-Host "  ✗ Cargo.toml" }
if (Test-Path docker-compose.yml) { Write-Host "  ✓ docker-compose.yml" } else { Write-Host "  ✗ docker-compose.yml" }
Write-Host ""
Write-Host "🐳 Docker:"
try { docker --version | Out-Null; Write-Host "  ✓ Docker installed" } catch { Write-Host "  ✗ Docker not found" }
try { docker compose version | Out-Null; Write-Host "  ✓ Docker Compose installed" } catch { Write-Host "  ✗ Docker Compose not found" }
Write-Host ""
Write-Host "🦀 Rust Toolchain:"
$rustc = rustc --version; Write-Host "  ✓ $rustc"
$cargo = cargo --version; Write-Host "  ✓ $cargo"
Write-Host ""
Write-Host "⚡ SQLx CLI:"
try { $sqlx = sqlx --version; Write-Host "  ✓ $sqlx" } catch { Write-Host "  ✗ sqlx-cli not installed (run: cargo install sqlx-cli --features postgres)" }
Write-Host ""
Write-Host "📦 Node.js:"
$node = node --version; Write-Host "  ✓ Node $node"
$npm = npm --version; Write-Host "  ✓ npm $npm"
Write-Host ""
Write-Host "✓ Verification complete"
"#,
            "Verify setup",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🔍 Verifying setup..."
echo ""
echo "📋 Required Files:"
test -f .env.example && echo "  ✓ .env.example" || echo "  ✗ .env.example"
test -f Cargo.toml && echo "  ✓ Cargo.toml" || echo "  ✗ Cargo.toml"
test -f docker-compose.yml && echo "  ✓ docker-compose.yml" || echo "  ✗ docker-compose.yml"
echo ""
echo "🐳 Docker:"
docker --version > /dev/null 2>&1 && echo "  ✓ Docker installed" || echo "  ✗ Docker not found"
docker compose version > /dev/null 2>&1 && echo "  ✓ Docker Compose installed" || echo "  ✗ Docker Compose not found"
echo ""
echo "🦀 Rust Toolchain:"
rustc --version 2>&1 | head -n1 | sed 's/^/  ✓ /'
cargo --version 2>&1 | sed 's/^/  ✓ /'
echo ""
echo "⚡ SQLx CLI:"
sqlx --version 2>&1 | sed 's/^/  ✓ /' || echo "  ✗ sqlx-cli not installed (run: cargo install sqlx-cli --features postgres)"
echo ""
echo "📦 Node.js:"
node --version 2>&1 | sed 's/^/  ✓ Node /'
npm --version 2>&1 | sed 's/^/  ✓ npm /'
echo ""
echo "✓ Verification complete"
"#,
            "Verify setup",
        )
    }
}
