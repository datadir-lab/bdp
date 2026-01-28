//! Integration tests for middleware
//!
//! These tests verify:
//! - CORS headers are correctly set
//! - Rate limiting works as expected
//! - Middleware stack functions properly

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use tower_http::compression::CompressionLayer;

use bdp_server::{config::CorsConfig, middleware};

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

/// Test helper to create a test server with CORS middleware
async fn create_test_app_with_cors(pool: PgPool, cors_config: CorsConfig) -> Router {
    async fn health() -> impl IntoResponse {
        Json(json!({ "status": "ok" }))
    }

    let state = AppState { db: pool };

    Router::new()
        .route("/health", get(health))
        .layer(middleware::cors_layer(&cors_config))
        .with_state(state)
}

/// Test helper to create a test server with rate limiting
async fn create_test_app_with_rate_limit(pool: PgPool) -> Router {
    async fn health() -> impl IntoResponse {
        Json(json!({ "status": "ok" }))
    }

    let state = AppState { db: pool };
    let rate_limit_config = middleware::rate_limit::RateLimitConfig {
        requests_per_minute: 5, // Very low limit for testing
    };

    Router::new()
        .route("/health", get(health))
        .layer(middleware::rate_limit::rate_limit_layer(rate_limit_config))
        .with_state(state)
}

#[sqlx::test]
async fn test_cors_headers_with_specific_origin(pool: PgPool) -> sqlx::Result<()> {
    let cors_config = CorsConfig {
        allowed_origins: vec!["http://localhost:3000".to_string()],
        allow_credentials: true,
    };

    let app = create_test_app_with_cors(pool, cors_config).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check CORS headers
    let headers = response.headers();
    assert!(headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    assert_eq!(
        headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
        "http://localhost:3000"
    );

    Ok(())
}

#[sqlx::test]
async fn test_cors_preflight_request(pool: PgPool) -> sqlx::Result<()> {
    let cors_config = CorsConfig {
        allowed_origins: vec!["http://localhost:3000".to_string()],
        allow_credentials: true,
    };

    let app = create_test_app_with_cors(pool, cors_config).await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check CORS preflight headers
    let headers = response.headers();
    assert!(headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    assert!(headers.contains_key(header::ACCESS_CONTROL_ALLOW_METHODS));
    assert!(headers.contains_key(header::ACCESS_CONTROL_MAX_AGE));

    // Verify max age is set to 3600 seconds
    let max_age = headers.get(header::ACCESS_CONTROL_MAX_AGE).unwrap();
    assert_eq!(max_age, "3600");

    Ok(())
}

#[sqlx::test]
async fn test_cors_allows_custom_headers(pool: PgPool) -> sqlx::Result<()> {
    let cors_config = CorsConfig {
        allowed_origins: vec!["http://localhost:3000".to_string()],
        allow_credentials: true,
    };

    let app = create_test_app_with_cors(pool, cors_config).await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "content-type, authorization, x-user-id",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check that custom headers are allowed
    let headers = response.headers();
    assert!(headers.contains_key(header::ACCESS_CONTROL_ALLOW_HEADERS));

    Ok(())
}

#[sqlx::test]
async fn test_cors_wildcard_origin(pool: PgPool) -> sqlx::Result<()> {
    let cors_config = CorsConfig {
        allowed_origins: vec!["*".to_string()],
        allow_credentials: false,
    };

    let app = create_test_app_with_cors(pool, cors_config).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(header::ORIGIN, "https://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check CORS headers
    let headers = response.headers();
    assert!(headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));

    Ok(())
}

#[sqlx::test]
async fn test_rate_limiting_allows_requests_under_limit(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app_with_rate_limit(pool).await;

    // First request should succeed
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("X-Forwarded-For", "192.168.1.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

#[sqlx::test]
async fn test_rate_limiting_blocks_excessive_requests(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app_with_rate_limit(pool).await;

    // Make multiple requests from the same IP
    for i in 0..6 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("X-Forwarded-For", "192.168.1.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        if i < 5 {
            // First 5 requests should succeed
            assert_eq!(response.status(), StatusCode::OK, "Request {} should succeed", i + 1);
        } else {
            // 6th request should be rate limited
            assert_eq!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "Request {} should be rate limited",
                i + 1
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod config_tests {
    use bdp_server::middleware::rate_limit::RateLimitConfig;

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
}
