//! Rate limiting middleware using tower-governor

use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder, GovernorLayer,
};

// ============================================================================
// Rate Limiting Constants
// ============================================================================

/// Default rate limit in requests per minute.
/// Can be configured via RATE_LIMIT_REQUESTS_PER_MINUTE environment variable.
pub const DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE: u64 = 100;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute allowed
    pub requests_per_minute: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE,
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
                .unwrap_or(DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE),
        }
    }
}

/// Create rate limiting layer from configuration
pub fn rate_limit_layer(config: RateLimitConfig) -> impl Clone {
    // For 100 requests per minute:
    // - Replenishment period = 60,000ms / 100 = 600ms per request
    // - Burst size = 100 (allow up to 100 requests before rate limiting kicks in)
    let replenishment_ms = 60_000 / config.requests_per_minute;
    let burst_size = config.requests_per_minute.try_into().unwrap_or(DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE as u32);

    // Build governor configuration
    // If configuration is invalid (which should never happen with validated inputs),
    // panic during application startup with a descriptive message.
    let governor_conf = match GovernorConfigBuilder::default()
        .per_millisecond(replenishment_ms)
        .burst_size(burst_size)
        .finish()
    {
        Ok(config) => Arc::new(config),
        Err(e) => {
            // This is a fatal configuration error that should never happen in production
            // with validated inputs. Panic with a clear message during startup.
            panic!("Fatal: Invalid rate limit configuration (burst_size={}, replenishment_ms={}): {}",
                   burst_size, replenishment_ms, e);
        }
    };

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
        assert_eq!(config.requests_per_minute, DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE);
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
