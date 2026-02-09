use crate::utils::*;
/// Development operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum DevCommand {
    /// Start development (database + backend server)
    Server,
    /// Start frontend development server with hot reload
    Web,
    /// Build frontend (with Pagefind indexing)
    WebBuild,
    /// Build frontend with Pagefind indexing and start production server
    WebProd,
    /// Start all services (backend + frontend + database)
    All,
    /// Watch and rebuild on changes
    Watch,
    /// Format code
    Fmt,
    /// Lint code
    Lint,
    /// Fix linting issues
    Fix,
    /// Run security audit
    SecurityAudit,
}

pub fn handle(cmd: DevCommand) -> Result<()> {
    match cmd {
        DevCommand::Server => server(),
        DevCommand::Web => web(),
        DevCommand::WebBuild => web_build(),
        DevCommand::WebProd => web_prod(),
        DevCommand::All => all(),
        DevCommand::Watch => watch(),
        DevCommand::Fmt => fmt(),
        DevCommand::Lint => lint(),
        DevCommand::Fix => fix(),
        DevCommand::SecurityAudit => security_audit(),
    }
}

fn server() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::Up)?;

    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🚀 Starting backend server..."
cargo run --bin bdp-server
"#,
            "Starting backend server",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🚀 Starting backend server..."
cargo run --bin bdp-server
"#,
            "Starting backend server",
        )
    }
}

fn web() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🌐 Starting frontend (dev mode)..."
cd web; yarn dev
"#,
            "Starting frontend",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🌐 Starting frontend (dev mode)..."
cd web && yarn dev
"#,
            "Starting frontend",
        )
    }
}

pub fn web_build() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📚 Generating CLI documentation..."
cargo run --package xtask -- generate-cli-docs
Write-Host "🌐 Building frontend..."
cd web; $env:NEXT_PRIVATE_DISABLE_TURBO="1"; yarn build
Write-Host "📦 Copying static files to standalone..."
cd web; Copy-Item -Recurse -Force public .next/standalone/
cd web; Copy-Item -Recurse -Force .next/static .next/standalone/.next/
Write-Host "🔍 Indexing documentation with Pagefind..."
cd web; yarn pagefind
Write-Host "✓ Build complete with Pagefind index"
"#,
            "Building frontend",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
set -euo pipefail
echo "📚 Generating CLI documentation..."
cargo run --package xtask -- generate-cli-docs
echo "🌐 Building frontend..."
cd web
NEXT_PRIVATE_DISABLE_TURBO=1 yarn build
echo "📦 Copying static files to standalone..."
cp -r public .next/standalone/
cp -r .next/static .next/standalone/.next/
echo "🔍 Indexing documentation with Pagefind..."
yarn pagefind
echo "✓ Build complete with Pagefind index"
"#,
            "Building frontend",
        )
    }
}

fn web_prod() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📚 Generating CLI documentation..."
cargo run --package xtask -- generate-cli-docs
Write-Host "🌐 Building frontend..."
cd web; $env:NEXT_PRIVATE_DISABLE_TURBO="1"; yarn build
Write-Host "📦 Copying static files to standalone..."
cd web; Copy-Item -Recurse -Force public .next/standalone/
cd web; Copy-Item -Recurse -Force .next/static .next/standalone/.next/
Write-Host "🔍 Indexing documentation with Pagefind..."
cd web; yarn pagefind
Write-Host "✓ Build complete with Pagefind index"
Write-Host "🌐 Starting production server..."
cd web; yarn start
"#,
            "Building and starting production frontend",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
set -euo pipefail
echo "📚 Generating CLI documentation..."
cargo run --package xtask -- generate-cli-docs
echo "🌐 Building frontend..."
cd web
NEXT_PRIVATE_DISABLE_TURBO=1 yarn build
echo "📦 Copying static files to standalone..."
cp -r public .next/standalone/
cp -r .next/static .next/standalone/.next/
echo "🔍 Indexing documentation with Pagefind..."
yarn pagefind
echo "✓ Build complete with Pagefind index"
echo "🌐 Starting production server..."
yarn start
"#,
            "Building and starting production frontend",
        )
    }
}

fn all() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::Up)?;
    info("🚀 Starting all services...");
    println!("Backend: http://localhost:8000");
    println!("Frontend: http://localhost:3000");

    // Note: This launches in parallel, user should use separate terminals
    warning("Run 'cargo xtask dev server' and 'cargo xtask dev web' in separate terminals");
    Ok(())
}

fn watch() -> Result<()> {
    run_streaming("cargo", &["watch", "-x", "run --bin bdp-server"], "Watch and rebuild")
}

fn fmt() -> Result<()> {
    info("🎨 Formatting code...");
    run("cargo", &["fmt", "--all"], "Format Rust code")?;
    run_in_dir("web", "yarn", &["format"], "Format web code")?;
    success("Code formatted");
    Ok(())
}

fn lint() -> Result<()> {
    info("🔍 Linting code...");
    run(
        "cargo",
        &["clippy", "--workspace", "--bins", "--lib", "--", "-D", "warnings"],
        "Lint Rust code",
    )?;
    run_in_dir("web", "yarn", &["lint"], "Lint web code")?;
    success("Linting complete");
    Ok(())
}

fn fix() -> Result<()> {
    info("🔧 Fixing linting issues...");
    run(
        "cargo",
        &["clippy", "--fix", "--allow-dirty", "--allow-staged"],
        "Fix clippy issues",
    )?;
    run("cargo", &["fmt", "--all"], "Format code")?;
    success("Fixes applied");
    Ok(())
}

fn security_audit() -> Result<()> {
    info("🔒 Running security audit...");
    run("cargo", &["audit"], "Security audit")?;
    success("Security audit complete");
    Ok(())
}
