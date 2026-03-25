// crates/bdp-ingest/tests/common.rs
//
// Shared test helpers for bdp-ingest E2E tests.
// Provides TestPostgres: spin up PostgreSQL container + apply migrations.

#![allow(dead_code)]

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use testcontainers::{core::IntoContainerPort, runners::AsyncRunner, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// PostgreSQL test container with migrations applied.
pub struct TestPostgres {
    // Keep container alive for the duration of the test
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

/// Create a test organization and return its UUID.
pub async fn create_test_org(pool: &PgPool, slug: &str) -> Result<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (slug, name, description, is_system)
         VALUES ($1, $2, $3, false)
         RETURNING id",
    )
    .bind(slug)
    .bind(format!("{} (test)", slug))
    .bind("E2E test organization")
    .fetch_one(pool)
    .await
    .context("create org")?;
    Ok(id)
}

/// Count rows in a table.
pub async fn count_rows(pool: &PgPool, table: &str) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
        .fetch_one(pool)
        .await
        .context("count rows")?;
    Ok(n)
}
