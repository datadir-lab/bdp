//! Integration tests for search functionality.
//!
//! These tests verify the unified search endpoint works correctly with
//! various query parameters and filters.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bdp_server::db::{create_pool, DbConfig};
use serde_json::Value;
use tower::ServiceExt;

/// Helper to create test database pool
async fn create_test_pool() -> sqlx::PgPool {
    let config = DbConfig::from_env().expect("DATABASE_URL must be set for tests");
    create_pool(&config)
        .await
        .expect("Failed to create test pool")
}

#[tokio::test]
#[ignore] // Run with `cargo test --ignored` when database is available
async fn test_search_basic_query() {
    let pool = create_test_pool().await;

    // This test assumes there's sample data in the database
    // In a real scenario, you'd seed test data first

    let result = bdp_server::db::search::unified_search(
        &pool,
        "insulin",
        &bdp_server::db::search::SearchFilters::default(),
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_search_with_type_filter() {
    let pool = create_test_pool().await;

    let filters = bdp_server::db::search::SearchFilters {
        entry_type: Some(vec!["data_source".to_string()]),
        organism: None,
        format: None,
    };

    let result = bdp_server::db::search::unified_search(
        &pool,
        "protein",
        &filters,
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());

    // Verify all results are data sources
    if let Ok(results) = result {
        for r in results {
            assert_eq!(r.entry_type, "data_source");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_search_with_organism_filter() {
    let pool = create_test_pool().await;

    let filters = bdp_server::db::search::SearchFilters {
        entry_type: None,
        organism: Some("human".to_string()),
        format: None,
    };

    let result = bdp_server::db::search::unified_search(
        &pool,
        "protein",
        &filters,
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_search_with_format_filter() {
    let pool = create_test_pool().await;

    let filters = bdp_server::db::search::SearchFilters {
        entry_type: None,
        organism: None,
        format: Some("fasta".to_string()),
    };

    let result = bdp_server::db::search::unified_search(
        &pool,
        "protein",
        &filters,
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());

    // Verify all results have fasta format
    if let Ok(results) = result {
        for r in results {
            assert!(r.available_formats.contains(&"fasta".to_string()));
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_search_with_all_filters() {
    let pool = create_test_pool().await;

    let filters = bdp_server::db::search::SearchFilters {
        entry_type: Some(vec!["data_source".to_string()]),
        organism: Some("human".to_string()),
        format: Some("fasta".to_string()),
    };

    let result = bdp_server::db::search::unified_search(
        &pool,
        "insulin",
        &filters,
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());

    if let Ok(results) = result {
        for r in results {
            assert_eq!(r.entry_type, "data_source");
            assert!(r.available_formats.contains(&"fasta".to_string()));
            if let Some(org) = r.organism {
                assert!(
                    org.scientific_name.to_lowercase().contains("homo sapiens")
                        || org
                            .common_name
                            .as_ref()
                            .map(|n| n.to_lowercase().contains("human"))
                            .unwrap_or(false)
                );
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_search_pagination() {
    let pool = create_test_pool().await;

    // Test first page
    let page1 = bdp_server::db::search::unified_search(
        &pool,
        "protein",
        &bdp_server::db::search::SearchFilters::default(),
        bdp_common::types::Pagination::new(10, 0),
    )
    .await
    .unwrap();

    // Test second page
    let page2 = bdp_server::db::search::unified_search(
        &pool,
        "protein",
        &bdp_server::db::search::SearchFilters::default(),
        bdp_common::types::Pagination::new(10, 10),
    )
    .await
    .unwrap();

    // Pages should be different (assuming more than 10 results)
    if !page1.is_empty() && !page2.is_empty() {
        assert_ne!(page1[0].id, page2[0].id);
    }
}

#[tokio::test]
#[ignore]
async fn test_search_count() {
    let pool = create_test_pool().await;

    let count = bdp_server::db::search::count_search_results(
        &pool,
        "protein",
        &bdp_server::db::search::SearchFilters::default(),
    )
    .await;

    assert!(count.is_ok());
    assert!(count.unwrap() >= 0);
}

#[tokio::test]
#[ignore]
async fn test_search_empty_query() {
    let pool = create_test_pool().await;

    // Empty query should still work (might return all results)
    let result = bdp_server::db::search::unified_search(
        &pool,
        "",
        &bdp_server::db::search::SearchFilters::default(),
        bdp_common::types::Pagination::default(),
    )
    .await;

    // This might return an error or empty results depending on implementation
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_search_no_results() {
    let pool = create_test_pool().await;

    let result = bdp_server::db::search::unified_search(
        &pool,
        "nonexistentquerystring12345",
        &bdp_server::db::search::SearchFilters::default(),
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
#[ignore]
async fn test_search_multiple_entry_types() {
    let pool = create_test_pool().await;

    let filters = bdp_server::db::search::SearchFilters {
        entry_type: Some(vec!["data_source".to_string(), "tool".to_string()]),
        organism: None,
        format: None,
    };

    let result = bdp_server::db::search::unified_search(
        &pool,
        "blast",
        &filters,
        bdp_common::types::Pagination::default(),
    )
    .await;

    assert!(result.is_ok());
}

#[test]
fn test_search_filters_serialization() {
    use bdp_server::db::search::SearchFilters;
    use serde_json;

    let filters = SearchFilters {
        entry_type: Some(vec!["data_source".to_string()]),
        organism: Some("human".to_string()),
        format: Some("fasta".to_string()),
    };

    let json = serde_json::to_string(&filters).unwrap();
    let deserialized: SearchFilters = serde_json::from_str(&json).unwrap();

    assert_eq!(filters.entry_type.unwrap(), deserialized.entry_type.unwrap());
    assert_eq!(filters.organism.unwrap(), deserialized.organism.unwrap());
    assert_eq!(filters.format.unwrap(), deserialized.format.unwrap());
}
