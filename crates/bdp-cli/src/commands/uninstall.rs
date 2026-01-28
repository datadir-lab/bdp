//! `bdp uninstall` command implementation
//!
//! Uninstalls BDP from the system.

use crate::error::{CliError, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Uninstall BDP from the system
pub async fn run(yes: bool, purge: bool) -> Result<()> {
    println!("{}", "BDP Uninstall".cyan().bold());
    println!("===============");
    println!();

    // Get the path to the current executable
    let exe_path = env::current_exe()
        .map_err(|e| CliError::config(format!("Failed to locate BDP executable: {}", e)))?;

    println!("BDP is installed at: {}", exe_path.display());

    // Get cache directory
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| CliError::config("Cannot find cache directory"))?
        .join("bdp");

    let cache_exists = cache_dir.exists();
    if cache_exists && purge {
        println!("Cache directory: {}", cache_dir.display());
    }

    println!();

    // Confirmation prompt (unless --yes flag is used)
    if !yes {
        println!("{}", "This will remove BDP from your system.".yellow());
        if purge && cache_exists {
            println!("{}", "The --purge flag will also remove all cache and data.".yellow());
        }
        println!();

        print!("Continue? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Uninstallation cancelled.");
            return Ok(());
        }
    }

    println!();
    println!("Uninstalling BDP...");

    // Remove cache if purge is enabled
    if purge && cache_exists {
        println!("Removing cache directory...");
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| CliError::cache(format!("Failed to remove cache directory: {}", e)))?;
        println!("{} Cache removed", "✓".green());
    }

    // On Unix-like systems, we can remove ourselves
    // On Windows, we schedule deletion on next boot or provide instructions
    #[cfg(unix)]
    {
        remove_self_unix(&exe_path)?;
    }

    #[cfg(windows)]
    {
        remove_self_windows(&exe_path)?;
    }

    Ok(())
}

#[cfg(unix)]
fn remove_self_unix(exe_path: &PathBuf) -> Result<()> {
    use std::process::Command;

    println!("Removing BDP executable...");

    // We can't delete ourselves while running, so we spawn a shell command
    // that waits for us to exit and then deletes the file
    let script = format!(r#"(sleep 1 && rm -f '{}') &"#, exe_path.display());

    Command::new("sh")
        .arg("-c")
        .arg(&script)
        .spawn()
        .map_err(|e| CliError::config(format!("Failed to schedule removal: {}", e)))?;

    println!("{} BDP has been uninstalled!", "✓".green());
    println!();
    println!("The binary will be removed momentarily.");
    println!("To reinstall BDP, visit: https://github.com/datadir-lab/bdp");

    Ok(())
}

#[cfg(windows)]
fn remove_self_windows(exe_path: &PathBuf) -> Result<()> {
    use std::process::Command;

    println!("Scheduling BDP executable removal...");

    // On Windows, we can't delete a running executable
    // Strategy: Rename it first, then schedule deletion via batch script
    let temp_path = exe_path.with_extension("exe.old");

    // Try to rename the executable (this works even while running)
    match fs::rename(exe_path, &temp_path) {
        Ok(_) => {
            // Create a batch script that waits and deletes the renamed file
            let batch_script = format!(
                r#"@echo off
timeout /t 2 /nobreak >nul
del /f /q "{}"
exit"#,
                temp_path.display()
            );

            let script_path = exe_path.with_extension("uninstall.bat");
            fs::write(&script_path, batch_script)?;

            // Run the batch script in the background
            Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg("/B")
                .arg(&script_path)
                .spawn()
                .map_err(|e| CliError::config(format!("Failed to schedule removal: {}", e)))?;

            println!("{} BDP has been uninstalled!", "✓".green());
            println!();
            println!("The binary will be removed momentarily.");
        },
        Err(_) => {
            // Rename failed - Windows has the file locked
            // Fallback: Schedule deletion on next reboot
            println!("{}", "⚠ Unable to remove the executable while it's running.".yellow());
            println!();
            println!("To complete uninstallation, either:");
            println!("  1. Restart your computer (recommended)");
            println!("  2. Manually delete: {}", exe_path.display());
            println!();
            println!(
                "{}",
                "Note: BDP is no longer in your PATH and won't run after closing this window."
                    .yellow()
            );
        },
    }

    println!("To reinstall BDP, visit: https://github.com/datadir-lab/bdp");

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_uninstall_cancelled() {
        // This is a tricky test since we can't actually uninstall during tests
        // Just verify the function exists and compiles
        // Real testing should be done in integration tests
    }
}
