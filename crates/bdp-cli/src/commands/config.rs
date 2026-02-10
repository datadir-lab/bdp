//! `bdp config` command implementation
//!
//! Manages CLI configuration.

use crate::{
    commands::output::Render,
    config::Config,
    error::{CliError, Result},
};

/// Output from the `bdp config` commands.
pub enum ConfigOutput {
    /// A single configuration value
    Value { value: String },
    /// Hint about setting configuration via environment variables
    SetHint {
        key: String,
        value: String,
        env_var: String,
    },
    /// Show all configuration
    Show {
        server_url: String,
        cache_dir: String,
        verbose: bool,
    },
}

impl Render for ConfigOutput {
    fn render(&self) {
        use colored::Colorize;

        match self {
            ConfigOutput::Value { value } => {
                println!("{}", value);
            },
            ConfigOutput::SetHint {
                key,
                value,
                env_var,
            } => {
                println!(
                    "{} Configuration is managed via environment variables:",
                    "\u{2139}".cyan()
                );
                println!("  BDP_SERVER_URL  - Server URL (default: {})", crate::BASE_SERVER_URL);
                println!("  BDP_CACHE_DIR   - Cache directory");
                println!();
                println!("To set {}:", key);
                println!("  export {}={}", env_var, value);
            },
            ConfigOutput::Show {
                server_url,
                cache_dir,
                verbose,
            } => {
                println!("{}", "BDP CLI Configuration:".cyan().bold());
                println!();
                println!("{:<15} {}", "server_url:", server_url);
                println!("{:<15} {}", "cache_dir:", cache_dir);
                println!("{:<15} {}", "verbose:", verbose);
                println!();
                println!("{}", "Environment Variables:".cyan());
                println!("  BDP_SERVER_URL  - Server URL");
                println!("  BDP_CACHE_DIR   - Cache directory");
            },
        }
    }
}

/// Get configuration value
pub async fn get(key: String) -> Result<ConfigOutput> {
    let config = Config::from_env()?;

    let value = match key.as_str() {
        "server_url" => config.server_url().to_string(),
        "cache_dir" => config.cache_dir().display().to_string(),
        "verbose" => config.is_verbose().to_string(),
        _ => {
            return Err(CliError::config(format!("Unknown config key: {}", key)));
        },
    };

    Ok(ConfigOutput::Value { value })
}

/// Set configuration value
pub async fn set(key: String, value: String) -> Result<ConfigOutput> {
    // For now, configuration is read from environment variables
    // In the future, could implement a config file

    let env_var = format_env_var(&key);

    Ok(ConfigOutput::SetHint {
        key,
        value,
        env_var,
    })
}

/// Show all configuration
pub async fn show() -> Result<ConfigOutput> {
    let config = Config::from_env()?;

    Ok(ConfigOutput::Show {
        server_url: config.server_url().to_string(),
        cache_dir: config.cache_dir().display().to_string(),
        verbose: config.is_verbose(),
    })
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
