//! Integration tests for sources API endpoints

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

mod helpers;
use helpers::{setup_test_app, setup_test_db};

#[tokio::test]
async fn test_list_sources_empty() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources")
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
}

#[tokio::test]
async fn test_list_sources_with_filters() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources?org=uniprot&type=protein&organism=human&page=1&limit=20")
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
}

#[tokio::test]
async fn test_get_source_not_found() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources/uniprot/P99999")
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
async fn test_get_source_version_not_found() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources/uniprot/P01308/1.5")
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
async fn test_get_dependencies_empty() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources/uniprot/all/1.0/dependencies?format=fasta&page=1&limit=1000")
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
    assert_eq!(json["data"]["dependency_count"], 0);
    assert!(json["data"]["dependencies"].is_array());
}

#[tokio::test]
async fn test_download_file_not_found() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources/uniprot/P01308/1.5/download?format=fasta")
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
async fn test_sources_pagination_limits() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    // Test with limit clamping
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources?page=1&limit=200")
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

#[tokio::test]
async fn test_dependencies_pagination_limits() {
    let pool = setup_test_db().await;
    let app = setup_test_app(pool).await;

    // Test dependencies with high limit (should clamp to 1000)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources/test/test/1.0/dependencies?limit=5000")
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

    // Limit should be clamped to 1000
    assert_eq!(json["meta"]["pagination"]["per_page"], 1000);
}
