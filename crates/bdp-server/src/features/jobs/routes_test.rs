//! Integration tests for job routes
//!
//! These tests verify the public job status API endpoints.
//! Uses `#[tokio::test(flavor = "multi_thread")]` instead of `#[sqlx::test]`
//! because the mediator crate requires a multi-threaded Tokio runtime.

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };

    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::features::jobs::jobs_routes;
    use crate::features::FeatureState;
    use crate::storage::config::StorageConfig;
    use crate::storage::Storage;

    /// Helper to connect to the test database.
    /// Uses compile-time DATABASE_URL since sqlx::test unsets it at runtime.
    async fn test_pool() -> PgPool {
        let url = env!("DATABASE_URL");
        PgPool::connect(url)
            .await
            .expect("Failed to connect to test database")
    }

    /// Helper to create a test router with FeatureState
    async fn create_test_router(pool: PgPool) -> Router {
        let storage_config = StorageConfig::for_minio("http://127.0.0.1:19999", "bdp-test");
        let storage = Storage::new(storage_config)
            .await
            .expect("Failed to create test storage");
        let mediator = crate::cqrs::build_mediator(pool, storage);
        let state = FeatureState { mediator };
        jobs_routes().with_state(state)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_jobs_endpoint() {
        let pool = test_pool().await;
        let app = create_test_router(pool).await;

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should succeed even with empty database
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_jobs_with_filters() {
        let pool = test_pool().await;
        let app = create_test_router(pool).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/?status=Pending&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_job_not_found() {
        let pool = test_pool().await;
        let app = create_test_router(pool).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nonexistent-job-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 for non-existent job
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_sync_status_endpoint() {
        let pool = test_pool().await;
        let storage_config = StorageConfig::for_minio("http://127.0.0.1:19999", "bdp-test");
        let storage = Storage::new(storage_config)
            .await
            .expect("Failed to create test storage");
        let mediator = crate::cqrs::build_mediator(pool, storage);
        let state = FeatureState { mediator };

        let app = crate::features::jobs::sync_status_routes().with_state(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should succeed even with empty database
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_sync_status_with_filters() {
        let pool = test_pool().await;
        let storage_config = StorageConfig::for_minio("http://127.0.0.1:19999", "bdp-test");
        let storage = Storage::new(storage_config)
            .await
            .expect("Failed to create test storage");
        let mediator = crate::cqrs::build_mediator(pool, storage);
        let state = FeatureState { mediator };
        let org_id = Uuid::new_v4();

        let app = crate::features::jobs::sync_status_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/?organization_id={}", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_sync_status_not_found() {
        let pool = test_pool().await;
        let storage_config = StorageConfig::for_minio("http://127.0.0.1:19999", "bdp-test");
        let storage = Storage::new(storage_config)
            .await
            .expect("Failed to create test storage");
        let mediator = crate::cqrs::build_mediator(pool, storage);
        let state = FeatureState { mediator };
        let org_id = Uuid::new_v4();

        let app = crate::features::jobs::sync_status_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/{}", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 for non-existent organization
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_sync_status_with_data() {
        let pool = test_pool().await;

        // Use a unique org_id to avoid conflicts with concurrent tests
        let org_id = Uuid::new_v4();

        // Create organization
        sqlx::query(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, 'Test Organization', true)",
        )
        .bind(org_id)
        .bind(format!("test-org-{}", &org_id.to_string()[..8]))
        .execute(&pool)
        .await
        .unwrap();

        // Create sync status
        sqlx::query(
            "INSERT INTO organization_sync_status (organization_id, status, total_entries) VALUES ($1, 'idle', 1000)",
        )
        .bind(org_id)
        .execute(&pool)
        .await
        .unwrap();

        // Test the endpoint
        let storage_config = StorageConfig::for_minio("http://127.0.0.1:19999", "bdp-test");
        let storage = Storage::new(storage_config)
            .await
            .expect("Failed to create test storage");
        let mediator = crate::cqrs::build_mediator(pool.clone(), storage);
        let state = FeatureState { mediator };

        let app = crate::features::jobs::sync_status_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/{}", org_id))
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

        // Verify data (wrapped in ApiResponse::success envelope)
        let data = &json["data"];
        assert_eq!(data["organization_id"], org_id.to_string());
        assert_eq!(data["status"], "idle");
        assert_eq!(data["total_entries"], 1000);

        // Cleanup
        let _ = sqlx::query("DELETE FROM organization_sync_status WHERE organization_id = $1")
            .bind(org_id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(&pool)
            .await;
    }
}
