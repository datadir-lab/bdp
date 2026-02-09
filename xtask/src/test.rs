use crate::utils::*;
/// Testing operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum TestCommand {
    /// Run all tests
    All,
    /// Run tests with output
    Verbose,
    /// Run integration tests only
    Integration,
    /// Run unit tests only
    Unit,
    /// Run specific test
    One {
        /// Test name
        test: String,
    },
    /// Test with coverage
    Coverage,
    /// Reset and run tests
    Fresh,
    /// Set up test directory for CLI testing
    CliSetup,
    /// Clean CLI test directory
    CliClean,
    /// Run CLI command in test directory
    Cli {
        /// Command to run
        cmd: String,
    },
    /// Full CLI test workflow
    CliFull,
}

pub fn handle(cmd: TestCommand) -> Result<()> {
    match cmd {
        TestCommand::All => all(),
        TestCommand::Verbose => verbose(),
        TestCommand::Integration => integration(),
        TestCommand::Unit => unit(),
        TestCommand::One { test } => one(&test),
        TestCommand::Coverage => coverage(),
        TestCommand::Fresh => fresh(),
        TestCommand::CliSetup => cli_setup(),
        TestCommand::CliClean => cli_clean(),
        TestCommand::Cli { cmd } => cli(&cmd),
        TestCommand::CliFull => cli_full(),
    }
}

fn all() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::TestUp)?;

    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧪 Running tests..."
$env:TEST_DATABASE_URL = if ($env:TEST_DATABASE_URL) { $env:TEST_DATABASE_URL } else { "postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" }
$env:DATABASE_URL = if ($env:DATABASE_URL) { $env:DATABASE_URL } else { "postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" }
cargo test --workspace --all-features
Write-Host "✓ Tests complete"
"#,
            "Running tests",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧪 Running tests..."
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgresql://bdp:bdp_test_password@localhost:5433/bdp_test}"
export DATABASE_URL="${DATABASE_URL:-postgresql://bdp:bdp_test_password@localhost:5433/bdp_test}"
cargo test --workspace --all-features
echo "✓ Tests complete"
"#,
            "Running tests",
        )
    }
}

fn verbose() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::TestUp)?;

    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧪 Running tests (verbose)..."
$env:TEST_DATABASE_URL = if ($env:TEST_DATABASE_URL) { $env:TEST_DATABASE_URL } else { "postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" }
$env:DATABASE_URL = if ($env:DATABASE_URL) { $env:DATABASE_URL } else { "postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" }
cargo test --workspace --all-features -- --nocapture
"#,
            "Running tests (verbose)",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧪 Running tests (verbose)..."
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgresql://bdp:bdp_test_password@localhost:5433/bdp_test}"
export DATABASE_URL="${DATABASE_URL:-postgresql://bdp:bdp_test_password@localhost:5433/bdp_test}"
cargo test --workspace --all-features -- --nocapture
"#,
            "Running tests (verbose)",
        )
    }
}

fn integration() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::TestUp)?;

    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🧪 Running integration tests..."
$env:TEST_DATABASE_URL = if ($env:TEST_DATABASE_URL) { $env:TEST_DATABASE_URL } else { "postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" }
$env:DATABASE_URL = if ($env:DATABASE_URL) { $env:DATABASE_URL } else { "postgresql://bdp:bdp_test_password@localhost:5433/bdp_test" }
cargo test --test '*' --all-features
"#,
            "Running integration tests",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🧪 Running integration tests..."
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgresql://bdp:bdp_test_password@localhost:5433/bdp_test}"
export DATABASE_URL="${DATABASE_URL:-postgresql://bdp:bdp_test_password@localhost:5433/bdp_test}"
cargo test --test '*' --all-features
"#,
            "Running integration tests",
        )
    }
}

fn unit() -> Result<()> {
    info("🧪 Running unit tests...");
    run("cargo", &["test", "--workspace", "--lib", "--all-features"], "Run unit tests")
}

fn one(test: &str) -> Result<()> {
    info(&format!("🧪 Running test: {}", test));
    run_streaming("cargo", &["test", test, "--", "--nocapture"], "Run specific test")
}

fn coverage() -> Result<()> {
    info("🧪 Running tests with coverage...");
    run(
        "cargo",
        &[
            "tarpaulin",
            "--workspace",
            "--all-features",
            "--out",
            "Html",
            "--output-dir",
            "coverage",
        ],
        "Run coverage",
    )
}

fn fresh() -> Result<()> {
    crate::db::handle(crate::db::DbCommand::TestDown)?;
    crate::db::handle(crate::db::DbCommand::TestUp)?;
    all()?;
    success("Fresh tests complete");
    Ok(())
}

fn cli_setup() -> Result<()> {
    info("📁 Setting up CLI test directory...");
    std::fs::create_dir_all("D:/dev/datadir/bdp-example")?;
    success("Test directory ready at D:/dev/datadir/bdp-example");
    Ok(())
}

fn cli_clean() -> Result<()> {
    info("🧹 Cleaning CLI test directory...");
    if path_exists("D:/dev/datadir/bdp-example") {
        std::fs::remove_dir_all("D:/dev/datadir/bdp-example")?;
        std::fs::create_dir_all("D:/dev/datadir/bdp-example")?;
    }
    success("Test directory cleaned");
    Ok(())
}

fn cli(cmd: &str) -> Result<()> {
    info(&format!("🔧 Running: bdp {}", cmd));
    let cmd_args: Vec<&str> = cmd.split_whitespace().collect();

    // Combine cargo run args with bdp command args
    let mut all_args = vec!["run", "--bin", "bdp", "--"];
    all_args.extend(cmd_args);

    run_in_dir_streaming("D:/dev/datadir/bdp-example", "cargo", &all_args, "Run CLI command")
}

fn cli_full() -> Result<()> {
    cli_setup()?;

    info("🧪 Running full CLI test workflow...");

    println!("\n1. Initialize project...");
    run_in_dir_streaming(
        "D:/dev/datadir/bdp-example",
        "cargo",
        &["run", "--bin", "bdp", "--", "init", "--name", "test-project"],
        "Initialize project",
    )?;

    println!("\n2. Add sources...");
    run_in_dir_streaming(
        "D:/dev/datadir/bdp-example",
        "cargo",
        &["run", "--bin", "bdp", "--", "source", "add", "uniprot:P01308-fasta@1.0"],
        "Add source",
    )?;

    println!("\n3. List sources...");
    run_in_dir_streaming(
        "D:/dev/datadir/bdp-example",
        "cargo",
        &["run", "--bin", "bdp", "--", "source", "list"],
        "List sources",
    )?;

    success("CLI test workflow complete");
    Ok(())
}
