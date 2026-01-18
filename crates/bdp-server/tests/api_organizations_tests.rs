//! Integration tests for organizations API endpoints

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bdp_server::api;
use serde_json::Value;
use tower::ServiceExt; // for `oneshot` and `ready`

mod helpers;
use helpers::{setup_test_app, setup_test_db};

#[tokio::test]
async fn test_list_organizations_empty() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

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

    assert_eq!(json["success"], true);
    assert!(json["data"].is_array());
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_organizations_with_pagination() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations?page=1&limit=10")
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

    assert_eq!(json["success"], true);
    assert!(json["meta"]["pagination"].is_object());
    assert_eq!(json["meta"]["pagination"]["page"], 1);
    assert_eq!(json["meta"]["pagination"]["per_page"], 10);
}

#[tokio::test]
async fn test_get_organization_not_found() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
#[ignore] // Requires database setup with test data
async fn test_get_organization_success() {
    let pool = setup_test_db().await;

    // Create a test organization
    bdp_server::db::organizations::create_organization(
        &pool,
        "test-org",
        "Test Organization",
        Some("A test organization"),
    )
    .await
    .unwrap();

    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations/test-org")
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

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["slug"], "test-org");
    assert_eq!(json["data"]["name"], "Test Organization");
}

#[tokio::test]
async fn test_pagination_validation() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    // Test with invalid page (should default to 1)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations?page=0&limit=20")
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

    // Page should be clamped to 1
    assert_eq!(json["meta"]["pagination"]["page"], 1);
}

#[tokio::test]
async fn test_limit_clamping() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    // Test with limit > 100 (should be clamped to 100)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations?page=1&limit=200")
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

    // Limit should be clamped to 100
    assert_eq!(json["meta"]["pagination"]["per_page"], 100);
}
