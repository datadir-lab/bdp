//! Integration tests for job routes
//!
//! These tests verify the public job status API endpoints.

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use serde_json::json;
    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::features::jobs::jobs_routes;

    /// Helper to create a test router
    fn create_test_router(pool: PgPool) -> Router {
        jobs_routes().with_state(pool)
    }

    #[sqlx::test]
    async fn test_list_jobs_endpoint(pool: PgPool) {
        let app = create_test_router(pool);

        let response = app
            .oneshot(Request::builder().uri("/jobs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should succeed even with empty database
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test]
    async fn test_list_jobs_with_filters(pool: PgPool) {
        let app = create_test_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jobs?status=Pending&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test]
    async fn test_get_job_not_found(pool: PgPool) {
        let app = create_test_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jobs/nonexistent-job-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 for non-existent job
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn test_list_sync_status_endpoint(pool: PgPool) {
        let app = create_test_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/sync-status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should succeed even with empty database
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test]
    async fn test_list_sync_status_with_filters(pool: PgPool) {
        let app = create_test_router(pool);
        let org_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/sync-status?organization_id={}", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test]
    async fn test_get_sync_status_not_found(pool: PgPool) {
        let app = create_test_router(pool);
        let org_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/sync-status/{}", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 for non-existent organization
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn test_get_sync_status_with_data(pool: PgPool) {
        // First, insert test data
        let org_id = Uuid::new_v4();

        // Create organization
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, 'test-org', 'Test Organization', true)
            "#,
            org_id
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create sync status
        sqlx::query!(
            r#"
            INSERT INTO organization_sync_status (organization_id, status, total_entries)
            VALUES ($1, 'idle', 1000)
            "#,
            org_id
        )
        .execute(&pool)
        .await
        .unwrap();

        // Test the endpoint
        let app = create_test_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/sync-status/{}", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        use axum::body::to_bytes;
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify data
        assert_eq!(json["organization_id"], org_id.to_string());
        assert_eq!(json["status"], "idle");
        assert_eq!(json["total_entries"], 1000);
    }
}
