use crate::utils::*;
/// Version management & releases
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum ReleaseCommand {
    /// Bump patch version (0.1.0 → 0.1.1) and create git tag
    Patch,
    /// Bump minor version (0.1.0 → 0.2.0) and create git tag
    Minor,
    /// Bump major version (0.1.0 → 1.0.0) and create git tag
    Major,
    /// Dry run of patch release (preview changes)
    PatchDry,
    /// Dry run of minor release (preview changes)
    MinorDry,
    /// Manual version bump without git operations (for testing)
    Bump {
        /// Version number (e.g., 0.1.1)
        version: String,
    },
}

pub fn handle(cmd: ReleaseCommand) -> Result<()> {
    match cmd {
        ReleaseCommand::Patch => patch(),
        ReleaseCommand::Minor => minor(),
        ReleaseCommand::Major => major(),
        ReleaseCommand::PatchDry => patch_dry(),
        ReleaseCommand::MinorDry => minor_dry(),
        ReleaseCommand::Bump { version } => bump(&version),
    }
}

fn patch() -> Result<()> {
    info("📦 Bumping patch version...");
    run(
        "cargo",
        &["release", "patch", "--execute", "--no-publish"],
        "Bump patch version",
    )
}

fn minor() -> Result<()> {
    info("📦 Bumping minor version...");
    run(
        "cargo",
        &["release", "minor", "--execute", "--no-publish"],
        "Bump minor version",
    )
}

fn major() -> Result<()> {
    info("📦 Bumping major version...");
    run(
        "cargo",
        &["release", "major", "--execute", "--no-publish"],
        "Bump major version",
    )
}

fn patch_dry() -> Result<()> {
    info("🔍 Dry run of patch release...");
    run("cargo", &["release", "patch", "--no-publish"], "Dry run patch")
}

fn minor_dry() -> Result<()> {
    info("🔍 Dry run of minor release...");
    run("cargo", &["release", "minor", "--no-publish"], "Dry run minor")
}

fn bump(version: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        info(&format!("📦 Bumping version to {}...", version));

        // Read Cargo.toml
        let cargo_toml = std::fs::read_to_string("Cargo.toml")?;

        // Replace version line
        let new_cargo_toml = cargo_toml
            .lines()
            .map(|line| {
                if line.starts_with("version = ") && !line.contains("workspace") {
                    format!("version = \"{}\"", version)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write("Cargo.toml", new_cargo_toml)?;
        info("Updated Cargo.toml...");

        // Sync to package.json
        std::env::set_var("NEW_VERSION", version);
        run("node", &["scripts/sync-version.js"], "Sync version to package.json")?;

        success(&format!("Version bumped to {}", version));
        warning("Remember to commit and tag manually!");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            &format!(
                r#"
echo "📦 Bumping version to {}..."
echo "Updating Cargo.toml..."
sed -i 's/^version = ".*"/version = "{}"/' Cargo.toml
echo "Syncing to package.json..."
NEW_VERSION={} node scripts/sync-version.js
echo "✓ Version bumped to {}"
echo "⚠️  Remember to commit and tag manually!"
"#,
                version, version, version, version
            ),
            "Bump version",
        )
    }
}
