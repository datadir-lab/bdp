// crates/bdp-mcp/tests/common.rs
//
// Shared test helpers for bdp-mcp integration tests.
// Exact pattern from crates/bdp-ingest/tests/common.rs.

#![allow(dead_code)]

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use testcontainers::{core::IntoContainerPort, runners::AsyncRunner, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

pub struct TestPostgres {
    _container: testcontainers::ContainerAsync<Postgres>,
    pub pool: PgPool,
}

impl TestPostgres {
    pub async fn start() -> Result<Self> {
        let container = Postgres::default()
            .with_tag("16-alpine")
            .start()
            .await
            .context("start postgres container")?;

        let host = container.get_host().await.context("get host")?;
        let port = container
            .get_host_port_ipv4(5432.tcp())
            .await
            .context("get port")?;

        let url = format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&url)
            .await
            .context("connect to postgres")?;

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .context("run migrations")?;

        Ok(Self {
            _container: container,
            pool,
        })
    }
}

pub async fn create_test_org(pool: &PgPool, slug: &str) -> Result<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (slug, name, description, is_system)
         VALUES ($1, $2, $3, false) RETURNING id",
    )
    .bind(slug)
    .bind(format!("{slug} (test)"))
    .bind("MCP test organization")
    .fetch_one(pool)
    .await
    .context("create org")?;
    Ok(id)
}

pub async fn count_rows(pool: &PgPool, table: &str) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .context("count rows")?;
    Ok(n)
}

/// Seed a minimal disease_term for testing.
/// Inserts: registry_entry + data_source + disease_term (Alzheimer, MONDO:0004975).
pub async fn seed_disease(pool: &PgPool, org_id: Uuid) -> Result<Uuid> {
    // Insert registry_entry for the data source
    let re_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO registry_entries (id, organization_id, entry_type, slug, name)
         VALUES ($1, $2, 'data_source', 'mondo-test', 'MONDO Test')",
    )
    .bind(re_id)
    .bind(org_id)
    .execute(pool)
    .await
    .context("insert registry_entry")?;

    sqlx::query(
        "INSERT INTO data_sources (id, source_type, external_id)
         VALUES ($1, 'disease', 'mondo')",
    )
    .bind(re_id)
    .execute(pool)
    .await
    .context("insert data_source")?;

    let disease_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO disease_terms
         (id, data_source_id, mondo_id, mondo_accession, name, definition, is_obsolete, omim_id, mondo_release)
         VALUES ($1, $2, 'MONDO:0004975', 4975, 'Alzheimer disease',
                 'A progressive brain disorder', FALSE, '104300', '2026-01')",
    )
    .bind(disease_id)
    .bind(re_id)
    .execute(pool)
    .await
    .context("insert disease_term")?;

    Ok(disease_id)
}
