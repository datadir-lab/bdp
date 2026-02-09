/// Utility functions for running shell commands and common operations
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::{Command, Stdio};

/// Execute a shell command with nice error messages
pub fn run(cmd: &str, args: &[&str], description: &str) -> Result<()> {
    println!("{} {}", "📦".bold(), description);

    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {:?}", cmd, args))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {:?}", cmd, args);
    }

    println!("{} {}", "✓".green().bold(), description);
    Ok(())
}

/// Execute and capture output
pub fn run_output(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {:?}", cmd, args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {} {:?}\n{}", cmd, args, stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute command and show output in real-time
pub fn run_streaming(cmd: &str, args: &[&str], description: &str) -> Result<()> {
    println!("{} {}", "📦".bold(), description);

    let status = Command::new(cmd)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute: {} {:?}", cmd, args))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {:?}", cmd, args);
    }

    println!("{} {}", "✓".green().bold(), description);
    Ok(())
}

/// Run in specific directory
pub fn run_in_dir(dir: &str, cmd: &str, args: &[&str], description: &str) -> Result<()> {
    println!("{} {}", "📦".bold(), description);

    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| format!("Failed to execute in {}: {} {:?}", dir, cmd, args))?;

    if !status.success() {
        anyhow::bail!("Command failed in {}: {} {:?}", dir, cmd, args);
    }

    println!("{} {}", "✓".green().bold(), description);
    Ok(())
}

/// Run in directory with streaming output
pub fn run_in_dir_streaming(dir: &str, cmd: &str, args: &[&str], description: &str) -> Result<()> {
    println!("{} {}", "📦".bold(), description);

    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute in {}: {} {:?}", dir, cmd, args))?;

    if !status.success() {
        anyhow::bail!("Command failed in {}: {} {:?}", dir, cmd, args);
    }

    println!("{} {}", "✓".green().bold(), description);
    Ok(())
}

/// Check if command exists
#[allow(dead_code)]
pub fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Sleep for seconds (cross-platform)
pub fn sleep(seconds: u64) {
    std::thread::sleep(std::time::Duration::from_secs(seconds));
}

/// Check if path exists
pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Check if file exists
#[allow(dead_code)]
pub fn file_exists(path: &str) -> bool {
    Path::new(path).is_file()
}

/// Check if directory exists
#[allow(dead_code)]
pub fn dir_exists(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Print info message
pub fn info(msg: &str) {
    println!("{} {}", "ℹ️".blue().bold(), msg);
}

/// Print success message
pub fn success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// Print warning message
pub fn warning(msg: &str) {
    println!("{} {}", "⚠️".yellow().bold(), msg);
}

/// Print error message
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

/// Print section header
#[allow(dead_code)]
pub fn section(title: &str) {
    println!("\n{}", title.bold().underline());
}

/// Run PowerShell command (Windows-specific)
#[cfg(target_os = "windows")]
pub fn run_powershell(script: &str, description: &str) -> Result<()> {
    println!("{} {}", "📦".bold(), description);

    let status = Command::new("powershell")
        .args(["-NoLogo", "-Command", script])
        .status()
        .with_context(|| format!("Failed to execute PowerShell: {}", script))?;

    if !status.success() {
        anyhow::bail!("PowerShell command failed: {}", script);
    }

    println!("{} {}", "✓".green().bold(), description);
    Ok(())
}

/// Run bash command (Unix-specific)
#[cfg(not(target_os = "windows"))]
pub fn run_bash(script: &str, description: &str) -> Result<()> {
    println!("{} {}", "📦".bold(), description);

    let status = Command::new("bash")
        .args(["-c", script])
        .status()
        .with_context(|| format!("Failed to execute bash: {}", script))?;

    if !status.success() {
        anyhow::bail!("Bash command failed: {}", script);
    }

    println!("{} {}", "✓".green().bold(), description);
    Ok(())
}
