use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use thiserror::Error;

/// Database operation errors with contextual information
#[derive(Error, Debug)]
pub enum DbError {
    /// SQL query or connection error
    #[error("Database query failed: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Database configuration is invalid or missing
    #[error("Database configuration error: {0}. Check DATABASE_URL and connection settings.")]
    Config(String),

    /// Requested record does not exist
    #[error("{0}")]
    NotFound(String),

    /// Record already exists (unique constraint violation)
    #[error("{0}")]
    Duplicate(String),
}

impl DbError {
    /// Create a not found error with resource context
    pub fn not_found(resource_type: &str, identifier: &str) -> Self {
        Self::NotFound(format!("{} '{}' not found in database", resource_type, identifier))
    }

    /// Create a duplicate error with resource context
    pub fn duplicate(resource_type: &str, identifier: &str) -> Self {
        Self::Duplicate(format!("{} '{}' already exists", resource_type, identifier))
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }
}

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: Option<u64>,
    pub max_lifetime_secs: Option<u64>,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://postgres:postgres@localhost:5432/bdp".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout_secs: 30,
            idle_timeout_secs: Some(600),
            max_lifetime_secs: Some(1800),
        }
    }
}

impl DbConfig {
    pub fn from_env() -> DbResult<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| DbError::Config("DATABASE_URL not set".to_string()))?;

        let max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20);

        let min_connections = std::env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let connect_timeout_secs = std::env::var("DB_CONNECT_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let idle_timeout_secs = std::env::var("DB_IDLE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok());

        let max_lifetime_secs = std::env::var("DB_MAX_LIFETIME")
            .ok()
            .and_then(|s| s.parse().ok());

        Ok(Self {
            url,
            max_connections,
            min_connections,
            connect_timeout_secs,
            idle_timeout_secs,
            max_lifetime_secs,
        })
    }
}

pub async fn create_pool(config: &DbConfig) -> DbResult<PgPool> {
    let mut options = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.connect_timeout_secs));

    if let Some(idle_timeout) = config.idle_timeout_secs {
        options = options.idle_timeout(Duration::from_secs(idle_timeout));
    }

    if let Some(max_lifetime) = config.max_lifetime_secs {
        options = options.max_lifetime(Duration::from_secs(max_lifetime));
    }

    let pool = options.connect(&config.url).await?;

    tracing::info!(
        max_connections = config.max_connections,
        min_connections = config.min_connections,
        "Database connection pool created"
    );

    Ok(pool)
}

pub async fn health_check(pool: &PgPool) -> DbResult<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(DbError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DbConfig::default();
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connect_timeout_secs, 30);
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("DATABASE_URL", "postgresql://localhost/test");
        std::env::set_var("DB_MAX_CONNECTIONS", "15");

        let config = DbConfig::from_env().unwrap();
        assert_eq!(config.max_connections, 15);
        assert!(config.url.contains("localhost/test"));

        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("DB_MAX_CONNECTIONS");
    }

    #[test]
    fn test_config_from_env_missing_url() {
        std::env::remove_var("DATABASE_URL");
        let result = DbConfig::from_env();
        assert!(result.is_err());
    }
}
