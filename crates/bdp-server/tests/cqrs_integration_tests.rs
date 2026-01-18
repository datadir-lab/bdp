//! Integration tests for CQRS architecture and audit logging
//!
//! These tests verify:
//! - Commands (write operations) create audit log entries
//! - Queries (read operations) do not create audit log entries
//! - Audit trail can be retrieved and queried

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

mod helpers;

use helpers::TestApp;

#[sqlx::test]
async fn test_command_creates_audit_entry(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    // Get initial audit log count
    let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    // Execute a command (POST - create organization)
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/organizations")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "slug": "test-org",
                "name": "Test Organization",
                "description": "Test description"
            })
            .to_string(),
        ))?;

    let response = app.router.clone().oneshot(request).await?;

    // Command should succeed
    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify audit log was created
    let final_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    assert_eq!(final_count, initial_count + 1, "Audit log should be created for command");

    // Verify audit log content
    let audit_entry: (String, String) = sqlx::query_as(
        "SELECT action, resource_type FROM audit_log ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(audit_entry.0, "create");
    assert_eq!(audit_entry.1, "organization");

    Ok(())
}

#[sqlx::test]
async fn test_query_does_not_create_audit_entry(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    // Get initial audit log count
    let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    // Execute a query (GET - list organizations)
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/organizations")
        .body(Body::empty())?;

    let response = app.router.clone().oneshot(request).await?;

    // Query should succeed
    assert!(response.status().is_success());

    // Verify no audit log was created
    let final_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    assert_eq!(final_count, initial_count, "Audit log should NOT be created for query");

    Ok(())
}

#[sqlx::test]
async fn test_update_command_creates_audit_entry(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    // First create an organization
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/organizations")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "slug": "update-test-org",
                "name": "Update Test Org",
            })
            .to_string(),
        ))?;

    app.router.clone().oneshot(create_request).await?;

    // Get audit count before update
    let before_update_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    // Execute update command
    let update_request = Request::builder()
        .method("PUT")
        .uri("/api/v1/organizations/update-test-org")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Updated Organization Name",
            })
            .to_string(),
        ))?;

    let response = app.router.clone().oneshot(update_request).await?;

    // Update should succeed
    assert_eq!(response.status(), StatusCode::OK);

    // Verify audit log was created for update
    let after_update_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    assert_eq!(
        after_update_count,
        before_update_count + 1,
        "Audit log should be created for update command"
    );

    // Verify audit log action
    let audit_action: String =
        sqlx::query_scalar("SELECT action FROM audit_log ORDER BY timestamp DESC LIMIT 1")
            .fetch_one(&pool)
            .await?;

    assert_eq!(audit_action, "update");

    Ok(())
}

#[sqlx::test]
async fn test_delete_command_creates_audit_entry(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    // First create an organization
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/organizations")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "slug": "delete-test-org",
                "name": "Delete Test Org",
            })
            .to_string(),
        ))?;

    app.router.clone().oneshot(create_request).await?;

    // Get audit count before delete
    let before_delete_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    // Execute delete command
    let delete_request = Request::builder()
        .method("DELETE")
        .uri("/api/v1/organizations/delete-test-org")
        .body(Body::empty())?;

    let response = app.router.clone().oneshot(delete_request).await?;

    // Delete should succeed
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify audit log was created for delete
    let after_delete_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await?;

    assert_eq!(
        after_delete_count,
        before_delete_count + 1,
        "Audit log should be created for delete command"
    );

    // Verify audit log action
    let audit_action: String =
        sqlx::query_scalar("SELECT action FROM audit_log ORDER BY timestamp DESC LIMIT 1")
            .fetch_one(&pool)
            .await?;

    assert_eq!(audit_action, "delete");

    Ok(())
}

#[sqlx::test]
async fn test_audit_trail_retrieval(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    // Create multiple organizations to generate audit entries
    for i in 0..3 {
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/organizations")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "slug": format!("audit-test-org-{}", i),
                    "name": format!("Audit Test Org {}", i),
                })
                .to_string(),
            ))?;

        app.router.clone().oneshot(request).await?;
    }

    // Query audit logs endpoint
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit?limit=10")
        .body(Body::empty())?;

    let response = app.router.clone().oneshot(request).await?;

    // Audit query should succeed
    assert_eq!(response.status(), StatusCode::OK);

    // Parse response
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // Verify we have audit entries
    let data = json["data"].as_array().expect("data should be an array");
    assert!(data.len() >= 3, "Should have at least 3 audit entries");

    Ok(())
}

#[sqlx::test]
async fn test_audit_query_filtering(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    // Create organizations
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/organizations")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "slug": "filter-test-org",
                "name": "Filter Test Org",
            })
            .to_string(),
        ))?;

    app.router.clone().oneshot(create_request).await?;

    // Query audit logs filtered by resource_type
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit?resource_type=organization")
        .body(Body::empty())?;

    let response = app.router.clone().oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let json: Value = serde_json::from_slice(&body)?;

    let data = json["data"].as_array().expect("data should be an array");

    // All entries should be for organizations
    for entry in data {
        assert_eq!(entry["resource_type"].as_str().unwrap(), "organization");
    }

    Ok(())
}

#[sqlx::test]
async fn test_audit_changes_recorded(pool: PgPool) -> anyhow::Result<()> {
    let app = TestApp::new(pool.clone()).await;

    let org_data = json!({
        "slug": "changes-test-org",
        "name": "Changes Test Org",
        "description": "This is a test"
    });

    // Create organization
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/organizations")
        .header("content-type", "application/json")
        .body(Body::from(org_data.to_string()))?;

    app.router.clone().oneshot(request).await?;

    // Get the audit entry
    let audit_entry: (Option<Value>,) =
        sqlx::query_as("SELECT changes FROM audit_log ORDER BY timestamp DESC LIMIT 1")
            .fetch_one(&pool)
            .await?;

    // Verify changes were recorded
    assert!(audit_entry.0.is_some(), "Changes should be recorded");

    let changes = audit_entry.0.unwrap();
    assert_eq!(changes["slug"].as_str().unwrap(), "changes-test-org");
    assert_eq!(changes["name"].as_str().unwrap(), "Changes Test Org");

    Ok(())
}
