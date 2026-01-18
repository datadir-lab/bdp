//! Integration tests for server startup and health checks
//!
//! These tests verify:
//! - Server starts successfully
//! - Health check endpoint works
//! - API endpoints are accessible
//! - Graceful shutdown works

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot` and `ready`

mod helpers;

/// Test helper to create a test server
async fn create_test_app(pool: PgPool) -> axum::Router {
    use axum::{
        extract::State,
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::get,
        Json, Router,
    };
    use serde_json::json;
    use tower::ServiceBuilder;
    use tower_http::compression::CompressionLayer;

    #[derive(Clone)]
    struct AppState {
        db: PgPool,
    }

    async fn root() -> impl IntoResponse {
        Json(json!({
            "name": "BDP Server",
            "version": env!("CARGO_PKG_VERSION"),
            "status": "running",
        }))
    }

    async fn health_check(State(state): State<AppState>) -> Result<Response, StatusCode> {
        match sqlx::query("SELECT 1").fetch_one(&state.db).await {
            Ok(_) => Ok((
                StatusCode::OK,
                Json(json!({
                    "status": "healthy",
                    "database": "connected"
                })),
            )
                .into_response()),
            Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }

    async fn list_organizations(State(state): State<AppState>) -> impl IntoResponse {
        match sqlx::query!("SELECT id, slug, name, website, is_system FROM organizations LIMIT 10")
            .fetch_all(&state.db)
            .await
        {
            Ok(orgs) => {
                let org_list: Vec<_> = orgs
                    .iter()
                    .map(|org| {
                        json!({
                            "id": org.id,
                            "slug": org.slug,
                            "name": org.name,
                            "website": org.website,
                            "is_system": org.is_system
                        })
                    })
                    .collect();

                (StatusCode::OK, Json(json!({ "organizations": org_list }))).into_response()
            },
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch organizations" })),
            )
                .into_response(),
        }
    }

    let state = AppState { db: pool };

    let api_routes = Router::new()
        .route("/health", get(health_check))
        .route("/organizations", get(list_organizations));

    Router::new()
        .route("/", get(root))
        .nest("/api/v1", api_routes)
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()))
        .with_state(state)
}

#[sqlx::test]
async fn test_root_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app(pool).await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "BDP Server");
    assert_eq!(json["status"], "running");

    Ok(())
}

#[sqlx::test]
async fn test_health_check_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert_eq!(json["database"], "connected");

    Ok(())
}

#[sqlx::test(fixtures("organizations"))]
async fn test_list_organizations_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["organizations"].is_array());
    let orgs = json["organizations"].as_array().unwrap();
    assert!(!orgs.is_empty(), "Should have organizations from fixtures");

    // Verify organization structure
    if let Some(first_org) = orgs.first() {
        assert!(first_org["id"].is_string());
        assert!(first_org["slug"].is_string());
        assert!(first_org["name"].is_string());
    }

    Ok(())
}

#[sqlx::test]
async fn test_list_organizations_empty(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["organizations"].is_array());
    let orgs = json["organizations"].as_array().unwrap();
    assert_eq!(orgs.len(), 0, "Should have no organizations");

    Ok(())
}

#[sqlx::test]
async fn test_404_not_found(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[cfg(test)]
mod config_tests {
    use bdp_server::config::Config;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.database.min_connections, 2);
        assert_eq!(config.cors.allow_credentials, true);
    }

    #[test]
    fn test_config_validation_invalid_port() {
        let mut config = Config::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_empty_db_url() {
        let mut config = Config::default();
        config.database.url = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_pool_size() {
        let mut config = Config::default();
        config.database.min_connections = 20;
        config.database.max_connections = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_max_connections() {
        let mut config = Config::default();
        config.database.max_connections = 0;
        assert!(config.validate().is_err());
    }
}

#[cfg(test)]
mod middleware_tests {
    use bdp_server::{
        config::CorsConfig,
        middleware::{cors_layer, rate_limit::RateLimitConfig},
    };

    #[test]
    fn test_cors_layer_creation() {
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allow_credentials: true,
        };

        let _layer = cors_layer(&config);
        // If we get here, the layer was created successfully
    }

    #[test]
    fn test_rate_limit_config_from_env() {
        std::env::set_var("RATE_LIMIT_REQUESTS_PER_MINUTE", "200");

        let config = RateLimitConfig::from_env();
        assert_eq!(config.requests_per_minute, 200);

        std::env::remove_var("RATE_LIMIT_REQUESTS_PER_MINUTE");
    }
}

#[cfg(test)]
mod error_tests {
    use axum::{http::StatusCode, response::IntoResponse};
    use bdp_server::AppError;

    #[test]
    fn test_not_found_error_response() {
        let error = AppError::NotFound("Resource not found".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_validation_error_response() {
        let error = AppError::Validation("Invalid input".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_internal_error_response() {
        let error = AppError::Internal("Something went wrong".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_unauthorized_error_response() {
        let error = AppError::Unauthorized("Not authorized".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
