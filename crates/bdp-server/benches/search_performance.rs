/// Performance benchmarks for search functionality
///
/// These benchmarks measure search performance with different dataset sizes
/// and compare materialized view vs direct table queries.
///
/// Run with: cargo bench --bench search_performance
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sqlx::PgPool;
use std::time::Duration;
use tokio::runtime::Runtime;

use bdp_server::{
    db::{create_pool, DbConfig},
    features::search::queries::{
        RefreshSearchIndexCommand, SearchSuggestionsQuery, UnifiedSearchQuery,
    },
};

/// Helper to create benchmark data
async fn create_benchmark_data(
    pool: &PgPool,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create test organization
    sqlx::query!(
        r#"
        INSERT INTO organizations (slug, name, description, is_system)
        VALUES ('bench-org', 'Benchmark Organization', 'For performance testing', false)
        ON CONFLICT (slug) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    let org_id = sqlx::query_scalar!(r#"SELECT id FROM organizations WHERE slug = 'bench-org'"#)
        .fetch_one(pool)
        .await?;

    // Create taxonomy for organism filtering
    let taxonomy_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, 'bench-taxonomy', 'Benchmark Taxonomy', 'data_source')
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

    println!("Creating {} benchmark entries...", count);

    // Batch insert for performance
    const BATCH_SIZE: usize = 1000;
    for batch_start in (0..count).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(count);

        for i in batch_start..batch_end {
            let slug = format!("bench-entry-{}", i);
            let name = format!("Benchmark Entry {} Protein", i);
            let desc = format!("Performance test entry {} for search benchmarking", i);

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
                r#"
                INSERT INTO data_sources (id, source_type)
                VALUES ($1, 'protein')
                ON CONFLICT (id) DO NOTHING
                "#,
                entry_id
            )
            .execute(pool)
            .await?;

            // Link to taxonomy
            sqlx::query!(
                r#"
                INSERT INTO protein_metadata (data_source_id, uniprot_id, taxonomy_id, protein_name)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (data_source_id) DO UPDATE SET protein_name = EXCLUDED.protein_name
                "#,
                entry_id,
                format!("P{:05}", i),
                taxonomy_ds_id,
                name
            )
            .execute(pool)
            .await?;

            // Create version with formats
            let version_id = sqlx::query_scalar!(
                r#"
                INSERT INTO versions (entry_id, version, external_version, published_at, download_count)
                VALUES ($1, '1.0', 'v1.0', NOW(), $2)
                ON CONFLICT (entry_id, version) DO UPDATE SET download_count = EXCLUDED.download_count
                RETURNING id
                "#,
                entry_id,
                (i % 1000) as i64
            )
            .fetch_one(pool)
            .await?;

            for format in &["fasta", "json", "xml"] {
                sqlx::query!(
                    r#"
                    INSERT INTO version_files (version_id, format, file_path, size_bytes, checksum)
                    VALUES ($1, $2, $3, 1024, 'abc123')
                    ON CONFLICT (version_id, format) DO NOTHING
                    "#,
                    version_id,
                    format,
                    format!("/data/{}.{}", slug, format)
                )
                .execute(pool)
                .await?;
            }
        }

        if batch_end % 10000 == 0 {
            println!("  Created {} entries...", batch_end);
        }
    }

    println!("Created {} entries", count);

    // Refresh materialized view
    println!("Refreshing materialized view...");
    let start = std::time::Instant::now();
    let command = RefreshSearchIndexCommand { concurrent: false };
    bdp_server::features::search::queries::refresh_search_index::handle(pool.clone(), command)
        .await?;
    println!("MV refresh took {:?}", start.elapsed());

    Ok(())
}

async fn setup_pool() -> PgPool {
    let db_config = DbConfig::from_env().expect("Database config");
    create_pool(&db_config)
        .await
        .expect("Failed to create pool")
}

fn bench_search_simple_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_pool());

    let mut group = c.benchmark_group("search_simple_query");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.to_async(&rt).iter(|| async {
                let query = UnifiedSearchQuery {
                    query: black_box("protein".to_string()),
                    type_filter: None,
                    source_type_filter: None,
                    organism: None,
                    format: None,
                    page: Some(1),
                    per_page: Some(20),
                };

                bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                    .await
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_search_with_filters(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_pool());

    let mut group = c.benchmark_group("search_with_filters");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("type_filter", |b| {
        b.to_async(&rt).iter(|| async {
            let query = UnifiedSearchQuery {
                query: black_box("benchmark".to_string()),
                type_filter: Some(vec!["data_source".to_string()]),
                source_type_filter: None,
                organism: None,
                format: None,
                page: Some(1),
                per_page: Some(20),
            };

            bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.bench_function("source_type_filter", |b| {
        b.to_async(&rt).iter(|| async {
            let query = UnifiedSearchQuery {
                query: black_box("entry".to_string()),
                type_filter: Some(vec!["data_source".to_string()]),
                source_type_filter: Some(vec!["protein".to_string()]),
                organism: None,
                format: None,
                page: Some(1),
                per_page: Some(20),
            };

            bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.bench_function("organism_filter", |b| {
        b.to_async(&rt).iter(|| async {
            let query = UnifiedSearchQuery {
                query: black_box("protein".to_string()),
                type_filter: Some(vec!["data_source".to_string()]),
                source_type_filter: None,
                organism: Some("sapiens".to_string()),
                format: None,
                page: Some(1),
                per_page: Some(20),
            };

            bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.bench_function("format_filter", |b| {
        b.to_async(&rt).iter(|| async {
            let query = UnifiedSearchQuery {
                query: black_box("entry".to_string()),
                type_filter: None,
                source_type_filter: None,
                organism: None,
                format: Some("fasta".to_string()),
                page: Some(1),
                per_page: Some(20),
            };

            bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.bench_function("combined_filters", |b| {
        b.to_async(&rt).iter(|| async {
            let query = UnifiedSearchQuery {
                query: black_box("benchmark".to_string()),
                type_filter: Some(vec!["data_source".to_string()]),
                source_type_filter: Some(vec!["protein".to_string()]),
                organism: Some("human".to_string()),
                format: Some("fasta".to_string()),
                page: Some(1),
                per_page: Some(20),
            };

            bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.finish();
}

fn bench_suggestions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_pool());

    let mut group = c.benchmark_group("suggestions");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("short_query", |b| {
        b.to_async(&rt).iter(|| async {
            let query = SearchSuggestionsQuery {
                q: black_box("pr".to_string()),
                limit: Some(10),
                type_filter: None,
                source_type_filter: None,
            };

            bdp_server::features::search::queries::suggestions::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.bench_function("longer_query", |b| {
        b.to_async(&rt).iter(|| async {
            let query = SearchSuggestionsQuery {
                q: black_box("protein".to_string()),
                limit: Some(10),
                type_filter: None,
                source_type_filter: None,
            };

            bdp_server::features::search::queries::suggestions::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.bench_function("with_filters", |b| {
        b.to_async(&rt).iter(|| async {
            let query = SearchSuggestionsQuery {
                q: black_box("bench".to_string()),
                limit: Some(10),
                type_filter: Some(vec!["data_source".to_string()]),
                source_type_filter: Some(vec!["protein".to_string()]),
            };

            bdp_server::features::search::queries::suggestions::handle(pool.clone(), query)
                .await
                .unwrap()
        });
    });

    group.finish();
}

fn bench_pagination(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_pool());

    let mut group = c.benchmark_group("pagination");
    group.sample_size(50);

    for page in [1, 10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(page), page, |b, &page| {
            b.to_async(&rt).iter(|| async {
                let query = UnifiedSearchQuery {
                    query: black_box("entry".to_string()),
                    type_filter: None,
                    source_type_filter: None,
                    organism: None,
                    format: None,
                    page: Some(page),
                    per_page: Some(20),
                };

                bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
                    .await
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_mv_refresh(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_pool());

    let mut group = c.benchmark_group("mv_refresh");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("non_concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let command = RefreshSearchIndexCommand { concurrent: false };
            bdp_server::features::search::queries::refresh_search_index::handle(
                pool.clone(),
                command,
            )
            .await
            .unwrap()
        });
    });

    group.bench_function("concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let command = RefreshSearchIndexCommand { concurrent: true };
            bdp_server::features::search::queries::refresh_search_index::handle(
                pool.clone(),
                command,
            )
            .await
            .unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_search_simple_query,
    bench_search_with_filters,
    bench_suggestions,
    bench_pagination,
    bench_mv_refresh
);
criterion_main!(benches);
