//! Middleware for the BDP server
//!
//! This module provides middleware for:
//! - CORS (Cross-Origin Resource Sharing)
//! - Request logging with tracing
//! - Rate limiting
//! - Audit logging

use axum::http::{header, HeaderName, Method, StatusCode};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::config::CorsConfig;

pub mod audit;
pub mod rate_limit;

/// Create CORS layer from configuration
pub fn cors_layer(config: &CorsConfig) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::CONTENT_LANGUAGE,
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("x-user-id"),
        ])
        .max_age(Duration::from_secs(3600));

    // Configure origins
    if config.allowed_origins.is_empty() || config.allowed_origins.contains(&"*".to_string()) {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<_> = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }

    // Configure credentials
    if config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    cors
}

/// Create tracing/logging layer
pub fn tracing_layer(
) -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Micros),
        )
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_with_specific_origins() {
        let config = CorsConfig {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "https://example.com".to_string(),
            ],
            allow_credentials: true,
        };

        let _layer = cors_layer(&config);
        // Layer is created successfully
    }

    #[test]
    fn test_cors_layer_with_wildcard() {
        let config = CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allow_credentials: false,
        };

        let _layer = cors_layer(&config);
        // Layer is created successfully
    }

    #[test]
    fn test_cors_layer_with_empty_origins() {
        let config = CorsConfig {
            allowed_origins: vec![],
            allow_credentials: false,
        };

        let _layer = cors_layer(&config);
        // Layer is created successfully
    }

}
