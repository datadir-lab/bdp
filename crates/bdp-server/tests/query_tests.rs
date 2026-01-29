//! Integration tests for SQL query endpoint
//!
//! These tests verify the `/api/v1/query` endpoint works correctly with
//! various SQL queries, validation, and error handling.

mod helpers;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use helpers::TestDb;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn test_query_simple_select() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();

    // Create test organization
    helpers::create_test_organization(&pool, "test-org", "Test Organization")
        .await
        .expect("Failed to create test organization");

    // Create test app
    let app = helpers::TestApp::new(pool).await;

    // Execute query
    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT id, slug, name FROM organizations LIMIT 10"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["data"]["columns"].is_array());
    assert!(json["data"]["rows"].is_array());

    let columns = json["data"]["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0], "id");
    assert_eq!(columns[1], "slug");
    assert_eq!(columns[2], "name");

    let rows = json["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn test_query_with_where_clause() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();

    // Create multiple organizations
    helpers::create_test_organization(&pool, "org-1", "Organization 1")
        .await
        .expect("Failed to create org 1");
    helpers::create_test_organization_full(
        &pool,
        "org-2",
        "Organization 2",
        None,
        None,
        true, // is_system
    )
    .await
    .expect("Failed to create org 2");

    let app = helpers::TestApp::new(pool).await;

    // Query only system organizations
    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT slug, name, is_system FROM organizations WHERE is_system = true"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);

    let rows = json["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);

    // Verify the row data
    let row = &rows[0].as_array().unwrap();
    assert_eq!(row[0], "org-2"); // slug
    assert_eq!(row[1], "Organization 2"); // name
    assert_eq!(row[2], true); // is_system
}

#[tokio::test]
async fn test_query_count_aggregate() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();

    // Create test organizations
    for i in 1..=5 {
        helpers::create_test_organization(&pool, &format!("org-{}", i), &format!("Org {}", i))
            .await
            .expect("Failed to create organization");
    }

    let app = helpers::TestApp::new(pool).await;

    // Query with COUNT
    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT COUNT(*) as total FROM organizations"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);

    let rows = json["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);

    let count = rows[0][0].as_i64().unwrap();
    assert_eq!(count, 5);
}

#[tokio::test]
async fn test_query_join() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();

    // Create organization and registry entry
    let org_id = helpers::create_test_organization(&pool, "test-org", "Test Organization")
        .await
        .expect("Failed to create organization");

    helpers::create_test_registry_entry(&pool, org_id, "test-entry", "Test Entry", "data_source")
        .await
        .expect("Failed to create registry entry");

    let app = helpers::TestApp::new(pool).await;

    // Query with JOIN
    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT o.name as org_name, r.name as entry_name FROM organizations o JOIN registry_entries r ON o.id = r.organization_id"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);

    let rows = json["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);

    let row = &rows[0].as_array().unwrap();
    assert_eq!(row[0], "Test Organization");
    assert_eq!(row[1], "Test Entry");
}

#[tokio::test]
async fn test_query_explain() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    // EXPLAIN query should be allowed
    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "EXPLAIN SELECT * FROM organizations"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_empty_result() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    // Query with no results
    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT * FROM organizations WHERE slug = 'nonexistent'"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["rows"].as_array().unwrap().len(), 0);
}

// ============================================================================
// Security Tests - Validate that dangerous operations are blocked
// ============================================================================

#[tokio::test]
async fn test_query_blocks_drop() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "DROP TABLE organizations"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["error"].to_string().contains("DROP"));
}

#[tokio::test]
async fn test_query_blocks_delete() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "DELETE FROM organizations WHERE id = '123'"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_query_blocks_update() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "UPDATE organizations SET name = 'hacked' WHERE id = '123'"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_query_blocks_insert() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "INSERT INTO organizations (slug, name) VALUES ('bad', 'Bad Org')"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_query_blocks_truncate() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "TRUNCATE TABLE organizations"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_query_blocks_alter() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "ALTER TABLE organizations ADD COLUMN evil TEXT"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_query_blocks_create() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();
    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "CREATE TABLE evil (id UUID)"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_query_with_special_characters() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();

    // Create organization with special characters
    sqlx::query!(
        r#"INSERT INTO organizations (slug, name, description) VALUES ($1, $2, $3)"#,
        "test-org",
        "Test's \"Organization\"",
        "Description with\nnewlines and\ttabs"
    )
    .execute(test_db.pool())
    .await
    .expect("Failed to create organization");

    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT name, description FROM organizations WHERE slug = 'test-org'"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);

    let rows = json["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);

    let row = &rows[0].as_array().unwrap();
    assert_eq!(row[0], "Test's \"Organization\"");
    assert!(row[1].as_str().unwrap().contains("newlines"));
}

#[tokio::test]
async fn test_query_with_null_values() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool_cloned();

    // Create organization with NULL fields
    sqlx::query!(
        r#"INSERT INTO organizations (slug, name, description, website) VALUES ($1, $2, $3, $4)"#,
        "test-org",
        "Test Organization",
        None::<String>,
        None::<String>
    )
    .execute(test_db.pool())
    .await
    .expect("Failed to create organization");

    let app = helpers::TestApp::new(pool).await;

    let request = Request::builder()
        .uri("/query")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sql": "SELECT slug, name, description, website FROM organizations WHERE slug = 'test-org'"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let rows = json["data"]["rows"].as_array().unwrap();
    let row = &rows[0].as_array().unwrap();

    assert_eq!(row[0], "test-org");
    assert_eq!(row[1], "Test Organization");
    assert_eq!(row[2], Value::Null);
    assert_eq!(row[3], Value::Null);
}
