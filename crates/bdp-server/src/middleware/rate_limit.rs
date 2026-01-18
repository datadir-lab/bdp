//! Rate limiting middleware using tower-governor

use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder, GovernorLayer,
};

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute allowed
    pub requests_per_minute: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
        }
    }
}

impl RateLimitConfig {
    /// Create from environment variables
    pub fn from_env() -> Self {
        Self {
            requests_per_minute: std::env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
        }
    }
}

/// Create rate limiting layer from configuration
pub fn rate_limit_layer(config: RateLimitConfig) -> impl Clone {
    // For 100 requests per minute:
    // - Replenishment period = 60,000ms / 100 = 600ms per request
    // - Burst size = 100 (allow up to 100 requests before rate limiting kicks in)
    let replenishment_ms = 60_000 / config.requests_per_minute;
    let burst_size = config.requests_per_minute.try_into().unwrap_or(100);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(replenishment_ms)
            .burst_size(burst_size)
            .finish()
            .unwrap(),
    );

    GovernorLayer {
        config: governor_conf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 100);
    }

    #[test]
    fn test_rate_limit_config_from_env() {
        std::env::set_var("RATE_LIMIT_REQUESTS_PER_MINUTE", "50");

        let config = RateLimitConfig::from_env();
        assert_eq!(config.requests_per_minute, 50);

        std::env::remove_var("RATE_LIMIT_REQUESTS_PER_MINUTE");
    }

    #[test]
    fn test_rate_limit_layer_creation() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
        };
        let _layer = rate_limit_layer(config);
        // Layer is created successfully
    }
}
