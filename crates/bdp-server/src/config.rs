//! Configuration management

use serde::{Deserialize, Serialize};

// ============================================================================
// Server Configuration Constants
// ============================================================================

/// Default server host binding.
pub const DEFAULT_SERVER_HOST: &str = "127.0.0.1";

/// Default server port.
pub const DEFAULT_SERVER_PORT: u16 = 8000;

/// Default shutdown timeout in seconds.
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

/// Default database URL for local development.
pub const DEFAULT_DATABASE_URL: &str = "postgresql://localhost/bdp";

/// Default maximum database connections in the pool.
pub const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 10;

/// Default minimum database connections in the pool.
pub const DEFAULT_DATABASE_MIN_CONNECTIONS: u32 = 2;

/// Default database connection timeout in seconds.
pub const DEFAULT_DATABASE_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Default database idle timeout in seconds (10 minutes).
pub const DEFAULT_DATABASE_IDLE_TIMEOUT_SECS: u64 = 600;

/// Default CORS allowed origin for local development.
pub const DEFAULT_CORS_ALLOWED_ORIGIN: &str = "http://localhost:3000";

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
                host: std::env::var("BDP_HOST").unwrap_or_else(|_| DEFAULT_SERVER_HOST.to_string()),
                port: std::env::var("BDP_PORT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_SERVER_PORT),
                shutdown_timeout_secs: std::env::var("BDP_SHUTDOWN_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_SHUTDOWN_TIMEOUT_SECS),
            },
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string()),
                max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_DATABASE_MAX_CONNECTIONS),
                min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_DATABASE_MIN_CONNECTIONS),
                connect_timeout_secs: std::env::var("DATABASE_CONNECT_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_DATABASE_CONNECT_TIMEOUT_SECS),
                idle_timeout_secs: std::env::var("DATABASE_IDLE_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_DATABASE_IDLE_TIMEOUT_SECS),
            },
            cors: CorsConfig {
                allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| DEFAULT_CORS_ALLOWED_ORIGIN.to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                allow_credentials: std::env::var("CORS_ALLOW_CREDENTIALS")
                    .ok()
                    .and_then(|s| s.parse().ok())
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
                host: DEFAULT_SERVER_HOST.to_string(),
                port: DEFAULT_SERVER_PORT,
                shutdown_timeout_secs: DEFAULT_SHUTDOWN_TIMEOUT_SECS,
            },
            database: DatabaseConfig {
                url: DEFAULT_DATABASE_URL.to_string(),
                max_connections: DEFAULT_DATABASE_MAX_CONNECTIONS,
                min_connections: DEFAULT_DATABASE_MIN_CONNECTIONS,
                connect_timeout_secs: DEFAULT_DATABASE_CONNECT_TIMEOUT_SECS,
                idle_timeout_secs: DEFAULT_DATABASE_IDLE_TIMEOUT_SECS,
            },
            cors: CorsConfig {
                allowed_origins: vec![DEFAULT_CORS_ALLOWED_ORIGIN.to_string()],
                allow_credentials: true,
            },
        }
    }
}
