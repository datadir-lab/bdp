use crate::utils::*;
/// Documentation operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum DocsCommand {
    /// Build Cargo documentation
    Cargo,
    /// Serve frontend docs
    Web,
    /// Generate CLI reference documentation (MDX format)
    Cli,
    /// Generate CLI documentation using hidden flag (alternative method)
    CliRaw,
    /// Check if CLI docs are up to date (for CI)
    CliCheck,
}

pub fn handle(cmd: DocsCommand) -> Result<()> {
    match cmd {
        DocsCommand::Cargo => cargo(),
        DocsCommand::Web => web(),
        DocsCommand::Cli => cli(),
        DocsCommand::CliRaw => cli_raw(),
        DocsCommand::CliCheck => cli_check(),
    }
}

fn cargo() -> Result<()> {
    info("📚 Building documentation...");
    run("cargo", &["doc", "--workspace", "--no-deps", "--open"], "Build docs")?;
    success("Documentation ready");
    Ok(())
}

fn web() -> Result<()> {
    info("📚 Starting documentation server...");
    crate::dev::handle(crate::dev::DevCommand::Web)
}

fn cli() -> Result<()> {
    info("📚 Generating CLI reference documentation...");
    run(
        "cargo",
        &["run", "--package", "xtask", "--", "generate-cli-docs"],
        "Generate CLI docs",
    )?;
    success("CLI docs generated at: web/app/[locale]/docs/content/en/cli-reference.mdx");
    Ok(())
}

fn cli_raw() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        info("📚 Generating raw markdown from CLI...");
        let output = run_output("cargo", &["run", "--bin", "bdp", "--", "--markdown-help"])?;
        std::fs::write("web/app/[locale]/docs/content/en/cli-reference-raw.md", output)?;
        success("Raw CLI docs generated");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📚 Generating raw markdown from CLI..."
cargo run --bin bdp -- --markdown-help > web/app/[locale]/docs/content/en/cli-reference-raw.md
echo "✓ Raw CLI docs generated"
"#,
            "Generate raw CLI docs",
        )
    }
}

fn cli_check() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        info("🔍 Checking if CLI docs are up to date...");
        let temp_dir = std::env::temp_dir().join("bdp-cli-docs-check");
        std::fs::create_dir_all(&temp_dir)?;

        run(
            "cargo",
            &[
                "run",
                "--package",
                "xtask",
                "--",
                "generate-cli-docs",
                "--output-dir",
                temp_dir.to_str().unwrap(),
            ],
            "Generate temp docs",
        )?;

        let existing =
            std::fs::read_to_string("web/app/[locale]/docs/content/en/cli-reference.mdx")?;
        let generated = std::fs::read_to_string(temp_dir.join("cli-reference.mdx"))?;

        if existing == generated {
            success("CLI docs are up to date");
            Ok(())
        } else {
            error("CLI docs are outdated - run 'cargo xtask docs cli' to update");
            anyhow::bail!("CLI docs are outdated");
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
set -euo pipefail
echo "🔍 Checking if CLI docs are up to date..."
temp_dir=$(mktemp -d)
trap "rm -rf $temp_dir" EXIT
cargo run --package xtask -- generate-cli-docs --output-dir "$temp_dir"
if diff -q "web/app/[locale]/docs/content/en/cli-reference.mdx" "$temp_dir/cli-reference.mdx" > /dev/null 2>&1; then
    echo "✓ CLI docs are up to date"
else
    echo "✗ CLI docs are outdated - run 'cargo xtask docs cli' to update"
    exit 1
fi
"#,
            "Check CLI docs",
        )
    }
}
