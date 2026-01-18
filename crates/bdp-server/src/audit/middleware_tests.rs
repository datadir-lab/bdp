use super::middleware::*;
use super::models::{AuditAction, AuditEntry, ResourceType};
use super::queries::query_audit_logs;
use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn test_create_handler(Json(payload): Json<serde_json::Value>) -> impl axum::response::IntoResponse {
    (StatusCode::CREATED, Json(json!({"id": Uuid::new_v4(), "data": payload})))
}

async fn test_update_handler(Json(_payload): Json<serde_json::Value>) -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(json!({"updated": true})))
}

async fn test_delete_handler() -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(json!({"deleted": true})))
}

async fn test_get_handler() -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(json!({"data": "test"})))
}

fn create_test_router(pool: PgPool) -> Router {
    Router::new()
        .route("/api/v1/organizations", post(test_create_handler))
        .route("/api/v1/organizations/:id", put(test_update_handler))
        .route("/api/v1/organizations/:id", delete(test_delete_handler))
        .route("/api/v1/organizations", get(test_get_handler))
        .route("/api/v1/sources", post(test_create_handler))
        .route("/api/v1/tools", post(test_create_handler))
        .layer(AuditLayer::new(pool))
}

#[sqlx::test]
async fn test_post_request_creates_audit_log(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/organizations")
                .header("content-type", "application/json")
                .header("x-user-id", Uuid::new_v4().to_string())
                .body(Body::from(r#"{"name":"Test Org","slug":"test-org"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let logs = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'create' AND resource_type = 'organization'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(logs.action, "create");
    assert_eq!(logs.resource_type, "organization");
    assert!(logs.changes.is_some());

    Ok(())
}

#[sqlx::test]
async fn test_put_request_creates_audit_log(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());
    let org_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/organizations/{}", org_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Updated Name"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let logs = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'update' AND resource_type = 'organization'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(logs.action, "update");
    assert_eq!(logs.resource_type, "organization");

    Ok(())
}

#[sqlx::test]
async fn test_delete_request_creates_audit_log(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());
    let org_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/organizations/{}", org_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let logs = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'delete' AND resource_type = 'organization'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(logs.action, "delete");
    assert_eq!(logs.resource_type, "organization");

    Ok(())
}

#[sqlx::test]
async fn test_get_request_not_audited(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let count_before = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/organizations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let count_after = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    assert_eq!(count_before, count_after, "GET requests should not create audit logs");

    Ok(())
}

#[sqlx::test]
async fn test_user_id_captured(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());
    let user_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/organizations")
                .header("content-type", "application/json")
                .header("x-user-id", user_id.to_string())
                .body(Body::from(r#"{"name":"Test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'create'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(log.user_id, Some(user_id));

    Ok(())
}

#[sqlx::test]
async fn test_user_agent_captured(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/organizations")
                .header("content-type", "application/json")
                .header("user-agent", "test-agent/1.0")
                .body(Body::from(r#"{"name":"Test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'create'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(log.user_agent, Some("test-agent/1.0".to_string()));

    Ok(())
}

#[sqlx::test]
async fn test_different_resource_types(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sources")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Test Source"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE resource_type = 'data_source'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(log.resource_type, "data_source");

    Ok(())
}

#[sqlx::test]
async fn test_request_body_captured_in_changes(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let request_body = json!({"name": "Test Org", "slug": "test-org", "description": "Test"});

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/organizations")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'create'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert!(log.changes.is_some());
    let changes = log.changes.unwrap();
    assert_eq!(changes["name"], "Test Org");
    assert_eq!(changes["slug"], "test-org");

    Ok(())
}

#[sqlx::test]
async fn test_metadata_includes_http_info(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/organizations")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'create'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert!(log.metadata.is_some());
    let metadata = log.metadata.unwrap();
    assert_eq!(metadata["method"], "POST");
    assert!(metadata["uri"].as_str().unwrap().contains("/organizations"));
    assert_eq!(metadata["status"], 201);

    Ok(())
}

#[sqlx::test]
async fn test_failed_requests_not_audited(pool: PgPool) -> sqlx::Result<()> {
    async fn failing_handler() -> impl axum::response::IntoResponse {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid request"})))
    }

    let app = Router::new()
        .route("/api/v1/organizations", post(failing_handler))
        .layer(AuditLayer::new(pool.clone()));

    let count_before = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/organizations")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"invalid":"data"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let count_after = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    assert_eq!(
        count_before, count_after,
        "Failed requests should not create audit logs"
    );

    Ok(())
}

#[sqlx::test]
async fn test_multiple_requests_create_multiple_logs(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());

    let count_before = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    for i in 0..3 {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/organizations")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"name":"Org {}"}}"#, i)))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let count_after = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    assert_eq!(count_after - count_before, 3, "Should have 3 new audit log entries");

    Ok(())
}

#[sqlx::test]
async fn test_uuid_in_path_captured_as_resource_id(pool: PgPool) -> sqlx::Result<()> {
    let app = create_test_router(pool.clone());
    let resource_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/organizations/{}", resource_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, user_id, action, resource_type, resource_id, changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE action = 'delete'
        ORDER BY timestamp DESC
        LIMIT 1
        "#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(log.resource_id, Some(resource_id));

    Ok(())
}
