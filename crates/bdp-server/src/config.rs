//! Configuration management

use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cors: CorsConfig,
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout_secs: u64,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
}

impl Config {
    /// Load configuration from environment and defaults
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let config = Config {
            server: ServerConfig {
                host: std::env::var("BDP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: std::env::var("BDP_PORT")
                    .unwrap_or_else(|_| "8000".to_string())
                    .parse()?,
                shutdown_timeout_secs: std::env::var("BDP_SHUTDOWN_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?,
            },
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://localhost/bdp".to_string()),
                max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
                min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()?,
                connect_timeout_secs: std::env::var("DATABASE_CONNECT_TIMEOUT")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
                idle_timeout_secs: std::env::var("DATABASE_IDLE_TIMEOUT")
                    .unwrap_or_else(|_| "600".to_string())
                    .parse()?,
            },
            cors: CorsConfig {
                allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                allow_credentials: std::env::var("CORS_ALLOW_CREDENTIALS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
        };

        config.validate()?;

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate port
        if self.server.port == 0 {
            anyhow::bail!("Server port must be greater than 0");
        }

        // Validate database URL
        if self.database.url.is_empty() {
            anyhow::bail!("Database URL cannot be empty");
        }

        // Validate connection pool settings
        if self.database.max_connections == 0 {
            anyhow::bail!("Database max_connections must be greater than 0");
        }

        if self.database.min_connections > self.database.max_connections {
            anyhow::bail!(
                "Database min_connections ({}) cannot be greater than max_connections ({})",
                self.database.min_connections,
                self.database.max_connections
            );
        }

        // Validate CORS origins
        if self.cors.allowed_origins.is_empty() {
            tracing::warn!("No CORS origins configured - all origins will be allowed");
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8000,
                shutdown_timeout_secs: 30,
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/bdp".to_string(),
                max_connections: 10,
                min_connections: 2,
                connect_timeout_secs: 10,
                idle_timeout_secs: 600,
            },
            cors: CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
                allow_credentials: true,
            },
        }
    }
}
