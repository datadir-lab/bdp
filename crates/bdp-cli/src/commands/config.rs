//! `bdp config` command implementation
//!
//! Manages CLI configuration.

use crate::config::Config;
use crate::error::{CliError, Result};
use colored::Colorize;

/// Get configuration value
pub async fn get(key: String) -> Result<()> {
    let config = Config::from_env()?;

    match key.as_str() {
        "server_url" => println!("{}", config.server_url()),
        "cache_dir" => println!("{}", config.cache_dir().display()),
        "verbose" => println!("{}", config.is_verbose()),
        _ => {
            return Err(CliError::config(format!("Unknown config key: {}", key)));
        },
    }

    Ok(())
}

/// Set configuration value
pub async fn set(key: String, value: String) -> Result<()> {
    // For now, configuration is read from environment variables
    // In the future, could implement a config file

    println!("{} Configuration is managed via environment variables:", "ℹ".cyan());
    println!("  BDP_SERVER_URL  - Server URL (default: http://localhost:8000)");
    println!("  BDP_CACHE_DIR   - Cache directory");
    println!();
    println!("To set {}:", key);
    println!("  export {}={}", format_env_var(&key), value);

    Ok(())
}

/// Show all configuration
pub async fn show() -> Result<()> {
    let config = Config::from_env()?;

    println!("{}", "BDP CLI Configuration:".cyan().bold());
    println!();
    println!("{:<15} {}", "server_url:", config.server_url());
    println!("{:<15} {}", "cache_dir:", config.cache_dir().display());
    println!("{:<15} {}", "verbose:", config.is_verbose());
    println!();
    println!("{}", "Environment Variables:".cyan());
    println!("  BDP_SERVER_URL  - Server URL");
    println!("  BDP_CACHE_DIR   - Cache directory");

    Ok(())
}

/// Format config key as environment variable name
fn format_env_var(key: &str) -> String {
    format!("BDP_{}", key.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_get() {
        let result = get("server_url".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_show() {
        let result = show().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_format_env_var() {
        assert_eq!(format_env_var("server_url"), "BDP_SERVER_URL");
        assert_eq!(format_env_var("cache_dir"), "BDP_CACHE_DIR");
    }
}
