/// Load tests for search functionality
///
/// These tests simulate concurrent users performing searches
/// to verify scalability and identify bottlenecks.
///
/// Run with: cargo test --test search_load_tests -- --nocapture --test-threads=1

use bdp_server::{
    db::{create_pool, DbConfig},
    features::search::queries::{
        RefreshSearchIndexCommand, SearchSuggestionsQuery, UnifiedSearchQuery,
    },
};
use futures::future::join_all;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Helper to create test data
async fn create_load_test_data(pool: &PgPool, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating {} test entries for load testing...", count);

    // Create organization
    sqlx::query!(
        r#"
        INSERT INTO organizations (slug, name, description, is_system)
        VALUES ('load-org', 'Load Test Organization', 'For load testing', false)
        ON CONFLICT (slug) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    let org_id = sqlx::query_scalar!(r#"SELECT id FROM organizations WHERE slug = 'load-org'"#)
        .fetch_one(pool)
        .await?;

    // Create taxonomy
    let taxonomy_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, 'load-taxonomy', 'Load Test Taxonomy', 'data_source')
        ON CONFLICT (organization_id, slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
        org_id
    )
    .fetch_one(pool)
    .await?;

    sqlx::query!(
        r#"INSERT INTO data_sources (id, source_type) VALUES ($1, 'taxonomy') ON CONFLICT (id) DO NOTHING"#,
        taxonomy_id
    )
    .execute(pool)
    .await?;

    let taxonomy_ds_id = sqlx::query_scalar!(
        r#"
        INSERT INTO taxonomy_metadata (data_source_id, scientific_name, common_name, taxonomy_id, rank)
        VALUES ($1, 'Homo sapiens', 'Human', 9606, 'species')
        ON CONFLICT (data_source_id) DO UPDATE SET scientific_name = EXCLUDED.scientific_name
        RETURNING data_source_id
        "#,
        taxonomy_id
    )
    .fetch_one(pool)
    .await?;

    // Batch insert entries
    const BATCH_SIZE: usize = 500;
    for batch_start in (0..count).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(count);

        for i in batch_start..batch_end {
            let slug = format!("load-entry-{}", i);
            let name = format!("Load Test Entry {} Dataset", i);
            let desc = format!("Load test entry {} for concurrent search testing", i);

            let entry_id = sqlx::query_scalar!(
                r#"
                INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
                VALUES ($1, $2, $3, $4, 'data_source')
                ON CONFLICT (organization_id, slug) DO UPDATE SET name = EXCLUDED.name
                RETURNING id
                "#,
                org_id,
                slug,
                name,
                desc
            )
            .fetch_one(pool)
            .await?;

            sqlx::query!(
                r#"INSERT INTO data_sources (id, source_type) VALUES ($1, 'protein') ON CONFLICT (id) DO NOTHING"#,
                entry_id
            )
            .execute(pool)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO protein_metadata (data_source_id, uniprot_id, taxonomy_id, protein_name)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (data_source_id) DO UPDATE SET protein_name = EXCLUDED.protein_name
                "#,
                entry_id,
                format!("LOAD{:05}", i),
                taxonomy_ds_id,
                name
            )
            .execute(pool)
            .await?;

            let version_id = sqlx::query_scalar!(
                r#"
                INSERT INTO versions (entry_id, version, external_version, published_at, download_count)
                VALUES ($1, '1.0', 'v1.0', NOW(), $2)
                ON CONFLICT (entry_id, version) DO UPDATE SET download_count = EXCLUDED.download_count
                RETURNING id
                "#,
                entry_id,
                (i % 100) as i64
            )
            .fetch_one(pool)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO version_files (version_id, format, file_path, size_bytes, checksum)
                VALUES ($1, 'fasta', $2, 1024, 'load123')
                ON CONFLICT (version_id, format) DO NOTHING
                "#,
                version_id,
                format!("/data/{}.fasta", slug)
            )
            .execute(pool)
            .await?;
        }

        if batch_end % 5000 == 0 || batch_end == count {
            println!("  Created {} / {} entries", batch_end, count);
        }
    }

    // Refresh materialized view
    println!("Refreshing materialized view...");
    let start = Instant::now();
    let command = RefreshSearchIndexCommand { concurrent: false };
    bdp_server::features::search::queries::refresh_search_index::handle(pool.clone(), command)
        .await?;
    println!("MV refresh took {:?}", start.elapsed());

    Ok(())
}

struct LoadTestStats {
    successful: usize,
    failed: usize,
    total_duration: Duration,
    min_duration: Duration,
    max_duration: Duration,
    p50_duration: Duration,
    p95_duration: Duration,
    p99_duration: Duration,
}

impl LoadTestStats {
    fn from_durations(mut durations: Vec<Duration>, failed: usize) -> Self {
        durations.sort();
        let successful = durations.len();
        let total_duration: Duration = durations.iter().sum();

        let min_duration = durations.first().copied().unwrap_or_default();
        let max_duration = durations.last().copied().unwrap_or_default();
        let p50_duration = durations.get(successful / 2).copied().unwrap_or_default();
        let p95_duration = durations
            .get(successful * 95 / 100)
            .copied()
            .unwrap_or_default();
        let p99_duration = durations
            .get(successful * 99 / 100)
            .copied()
            .unwrap_or_default();

        Self {
            successful,
            failed,
            total_duration,
            min_duration,
            max_duration,
            p50_duration,
            p95_duration,
            p99_duration,
        }
    }

    fn print(&self, test_name: &str) {
        println!("\n=== {} Results ===", test_name);
        println!("Successful: {}", self.successful);
        println!("Failed: {}", self.failed);
        println!("Total time: {:?}", self.total_duration);
        if self.successful > 0 {
            println!(
                "Avg: {:?}",
                self.total_duration / self.successful as u32
            );
            println!("Min: {:?}", self.min_duration);
            println!("p50: {:?}", self.p50_duration);
            println!("p95: {:?}", self.p95_duration);
            println!("p99: {:?}", self.p99_duration);
            println!("Max: {:?}", self.max_duration);
        }
    }
}

#[tokio::test]
#[ignore] // Run explicitly: cargo test --test search_load_tests test_concurrent_searches -- --ignored --nocapture
async fn test_concurrent_searches() -> Result<(), Box<dyn std::error::Error>> {
    let db_config = DbConfig::from_env()?;
    let pool = create_pool(&db_config).await?;

    // Create test data if not exists
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM search_registry_entries_mv")
        .fetch_one(&pool)
        .await?;

    if count < 10000 {
        create_load_test_data(&pool, 10000).await?;
    }

    let pool = Arc::new(pool);
    let concurrent_users = 100;
    let queries_per_user = 10;

    println!(
        "\n🚀 Starting load test: {} concurrent users, {} queries each",
        concurrent_users, queries_per_user
    );

    let start = Instant::now();
    let mut durations = Vec::new();
    let mut failed = 0;

    let mut tasks = Vec::new();
    for user_id in 0..concurrent_users {
        let pool = Arc::clone(&pool);
        let task = tokio::spawn(async move {
            let mut user_durations = Vec::new();
            let mut user_failed = 0;

            for query_num in 0..queries_per_user {
                let query_start = Instant::now();

                let query = UnifiedSearchQuery {
                    query: format!("load test {}", (user_id * queries_per_user + query_num) % 100),
                    type_filter: if query_num % 3 == 0 {
                        Some(vec!["data_source".to_string()])
                    } else {
                        None
                    },
                    source_type_filter: if query_num % 5 == 0 {
                        Some(vec!["protein".to_string()])
                    } else {
                        None
                    },
                    organism: if query_num % 7 == 0 {
                        Some("human".to_string())
                    } else {
                        None
                    },
                    format: None,
                    page: Some(1),
                    per_page: Some(20),
                };

                match bdp_server::features::search::queries::unified_search::handle(
                    (*pool).clone(),
                    query,
                )
                .await
                {
                    Ok(_) => {
                        user_durations.push(query_start.elapsed());
                    }
                    Err(_) => {
                        user_failed += 1;
                    }
                }

                // Small delay to simulate user think time
                sleep(Duration::from_millis(10)).await;
            }

            (user_durations, user_failed)
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    for result in results {
        let (user_durations, user_failed) = result?;
        durations.extend(user_durations);
        failed += user_failed;
    }

    let total_elapsed = start.elapsed();
    let stats = LoadTestStats::from_durations(durations, failed);

    println!("\nTotal test duration: {:?}", total_elapsed);
    println!(
        "Throughput: {:.2} queries/sec",
        stats.successful as f64 / total_elapsed.as_secs_f64()
    );

    stats.print("Concurrent Search Load Test");

    // Assertions
    assert!(
        stats.failed < stats.successful / 100,
        "More than 1% of queries failed"
    );
    assert!(
        stats.p95_duration < Duration::from_millis(500),
        "p95 latency exceeded 500ms"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_concurrent_suggestions() -> Result<(), Box<dyn std::error::Error>> {
    let db_config = DbConfig::from_env()?;
    let pool = create_pool(&db_config).await?;

    // Ensure test data exists
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM search_registry_entries_mv")
        .fetch_one(&pool)
        .await?;

    if count < 1000 {
        create_load_test_data(&pool, 1000).await?;
    }

    let pool = Arc::new(pool);
    let concurrent_users = 50;
    let queries_per_user = 20;

    println!(
        "\n🚀 Starting suggestions load test: {} concurrent users, {} queries each",
        concurrent_users, queries_per_user
    );

    let start = Instant::now();
    let mut durations = Vec::new();
    let mut failed = 0;

    let mut tasks = Vec::new();
    for user_id in 0..concurrent_users {
        let pool = Arc::clone(&pool);
        let task = tokio::spawn(async move {
            let mut user_durations = Vec::new();
            let mut user_failed = 0;

            for query_num in 0..queries_per_user {
                let query_start = Instant::now();

                // Simulate typing - progressively longer queries
                let search_terms = ["lo", "loa", "load", "load t", "load te"];
                let search_term = search_terms[(user_id + query_num) % search_terms.len()];

                let query = SearchSuggestionsQuery {
                    q: search_term.to_string(),
                    limit: Some(10),
                    type_filter: if query_num % 3 == 0 {
                        Some(vec!["data_source".to_string()])
                    } else {
                        None
                    },
                    source_type_filter: None,
                };

                match bdp_server::features::search::queries::suggestions::handle(
                    (*pool).clone(),
                    query,
                )
                .await
                {
                    Ok(_) => {
                        user_durations.push(query_start.elapsed());
                    }
                    Err(_) => {
                        user_failed += 1;
                    }
                }

                // Very small delay for autocomplete
                sleep(Duration::from_millis(5)).await;
            }

            (user_durations, user_failed)
        });

        tasks.push(task);
    }

    let results = join_all(tasks).await;

    for result in results {
        let (user_durations, user_failed) = result?;
        durations.extend(user_durations);
        failed += user_failed;
    }

    let total_elapsed = start.elapsed();
    let stats = LoadTestStats::from_durations(durations, failed);

    println!("\nTotal test duration: {:?}", total_elapsed);
    println!(
        "Throughput: {:.2} queries/sec",
        stats.successful as f64 / total_elapsed.as_secs_f64()
    );

    stats.print("Concurrent Suggestions Load Test");

    // Assertions
    assert!(
        stats.failed < stats.successful / 100,
        "More than 1% of queries failed"
    );
    assert!(
        stats.p95_duration < Duration::from_millis(100),
        "p95 latency exceeded 100ms for suggestions"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_search_under_mv_refresh() -> Result<(), Box<dyn std::error::Error>> {
    let db_config = DbConfig::from_env()?;
    let pool = create_pool(&db_config).await?;

    // Ensure test data exists
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM search_registry_entries_mv")
        .fetch_one(&pool)
        .await?;

    if count < 1000 {
        create_load_test_data(&pool, 1000).await?;
    }

    let pool = Arc::new(pool);

    println!("\n🚀 Testing search performance during concurrent MV refresh");

    // Start background MV refresh
    let pool_refresh = Arc::clone(&pool);
    let refresh_handle = tokio::spawn(async move {
        println!("Starting concurrent MV refresh in background...");
        let start = Instant::now();
        let command = RefreshSearchIndexCommand { concurrent: true };
        bdp_server::features::search::queries::refresh_search_index::handle(
            (*pool_refresh).clone(),
            command,
        )
        .await
        .unwrap();
        println!("MV refresh completed in {:?}", start.elapsed());
    });

    // Give refresh a moment to start
    sleep(Duration::from_millis(500)).await;

    // Run searches while refresh is happening
    let concurrent_users = 20;
    let queries_per_user = 10;
    let mut durations = Vec::new();
    let mut failed = 0;

    let mut tasks = Vec::new();
    for user_id in 0..concurrent_users {
        let pool = Arc::clone(&pool);
        let task = tokio::spawn(async move {
            let mut user_durations = Vec::new();
            let mut user_failed = 0;

            for query_num in 0..queries_per_user {
                let query_start = Instant::now();

                let query = UnifiedSearchQuery {
                    query: format!("test {}", (user_id * queries_per_user + query_num) % 50),
                    type_filter: None,
                    source_type_filter: None,
                    organism: None,
                    format: None,
                    page: Some(1),
                    per_page: Some(20),
                };

                match bdp_server::features::search::queries::unified_search::handle(
                    (*pool).clone(),
                    query,
                )
                .await
                {
                    Ok(_) => {
                        user_durations.push(query_start.elapsed());
                    }
                    Err(_) => {
                        user_failed += 1;
                    }
                }

                sleep(Duration::from_millis(50)).await;
            }

            (user_durations, user_failed)
        });

        tasks.push(task);
    }

    let results = join_all(tasks).await;

    for result in results {
        let (user_durations, user_failed) = result?;
        durations.extend(user_durations);
        failed += user_failed;
    }

    // Wait for refresh to complete
    refresh_handle.await?;

    let stats = LoadTestStats::from_durations(durations, failed);
    stats.print("Search During MV Refresh");

    // Assertions - should still be fast even during refresh
    assert_eq!(stats.failed, 0, "No queries should fail during concurrent refresh");
    assert!(
        stats.p95_duration < Duration::from_secs(1),
        "p95 latency exceeded 1s during refresh"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_sustained_load() -> Result<(), Box<dyn std::error::Error>> {
    let db_config = DbConfig::from_env()?;
    let pool = create_pool(&db_config).await?;

    // Ensure test data exists
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM search_registry_entries_mv")
        .fetch_one(&pool)
        .await?;

    if count < 5000 {
        create_load_test_data(&pool, 5000).await?;
    }

    let pool = Arc::new(pool);
    let duration = Duration::from_secs(60); // 1 minute sustained load
    let concurrent_users = 50;

    println!(
        "\n🚀 Starting sustained load test: {} concurrent users for {:?}",
        concurrent_users, duration
    );

    let start = Instant::now();
    let end_time = start + duration;
    let mut all_durations = Vec::new();
    let mut all_failed = 0;

    let mut tasks = Vec::new();
    for user_id in 0..concurrent_users {
        let pool = Arc::clone(&pool);
        let task = tokio::spawn(async move {
            let mut user_durations = Vec::new();
            let mut user_failed = 0;
            let mut query_count = 0;

            while Instant::now() < end_time {
                let query_start = Instant::now();

                let query = UnifiedSearchQuery {
                    query: format!("load entry {}", (user_id + query_count) % 200),
                    type_filter: if query_count % 4 == 0 {
                        Some(vec!["data_source".to_string()])
                    } else {
                        None
                    },
                    source_type_filter: None,
                    organism: None,
                    format: None,
                    page: Some(1),
                    per_page: Some(20),
                };

                match bdp_server::features::search::queries::unified_search::handle(
                    (*pool).clone(),
                    query,
                )
                .await
                {
                    Ok(_) => {
                        user_durations.push(query_start.elapsed());
                    }
                    Err(_) => {
                        user_failed += 1;
                    }
                }

                query_count += 1;
                sleep(Duration::from_millis(100)).await; // 10 queries per second per user
            }

            (user_durations, user_failed)
        });

        tasks.push(task);
    }

    let results = join_all(tasks).await;

    for result in results {
        let (user_durations, user_failed) = result?;
        all_durations.extend(user_durations);
        all_failed += user_failed;
    }

    let total_elapsed = start.elapsed();
    let stats = LoadTestStats::from_durations(all_durations, all_failed);

    println!("\nSustained load test completed");
    println!("Total duration: {:?}", total_elapsed);
    println!(
        "Throughput: {:.2} queries/sec",
        stats.successful as f64 / total_elapsed.as_secs_f64()
    );

    stats.print("Sustained Load Test");

    // Assertions
    assert!(
        stats.failed < stats.successful / 50,
        "More than 2% of queries failed during sustained load"
    );
    assert!(
        stats.p99_duration < Duration::from_secs(1),
        "p99 latency exceeded 1s during sustained load"
    );

    Ok(())
}
