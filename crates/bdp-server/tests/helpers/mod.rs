//! Test helpers for BDP server integration tests
//!
//! This module provides utilities for:
//! - Test database setup and management
//! - Migration runners
//! - Fixture loading
//! - Common test assertions
//! - Test data builders

pub mod fixtures;

use sqlx::{postgres::PgPoolOptions, PgPool, Postgres};
use std::sync::Arc;
use uuid::Uuid;

// Re-export fixtures for convenience
pub use fixtures::*;

/// Test database configuration
pub struct TestDb {
    pool: PgPool,
    database_name: String,
}

impl TestDb {
    /// Create a new test database with a unique name
    ///
    /// This will:
    /// 1. Connect to the PostgreSQL server
    /// 2. Create a new database with a unique name
    /// 3. Run all migrations
    /// 4. Return a pool connected to the test database
    ///
    /// # Example
    ///
    /// ```no_run
    /// use helpers::TestDb;
    ///
    /// #[tokio::test]
    /// async fn test_example() {
    ///     let test_db = TestDb::new().await;
    ///     let pool = test_db.pool();
    ///     // Use pool for testing
    /// }
    /// ```
    pub async fn new() -> Self {
        let base_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://bdp_test:test_password@localhost:5433/postgres".to_string()
        });

        // Generate unique database name
        let database_name = format!("test_db_{}", Uuid::new_v4().to_string().replace('-', "_"));

        // Connect to postgres database to create test database
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&base_url)
            .await
            .expect("Failed to connect to PostgreSQL");

        // Create test database
        sqlx::query(&format!("CREATE DATABASE {}", database_name))
            .execute(&pool)
            .await
            .expect("Failed to create test database");

        pool.close().await;

        // Connect to the new test database
        let test_db_url = base_url.replace("/postgres", &format!("/{}", database_name));
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&test_db_url)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        Self {
            pool,
            database_name,
        }
    }

    /// Get a reference to the database pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get a cloned database pool
    pub fn pool_cloned(&self) -> PgPool {
        self.pool.clone()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Note: Cleanup happens automatically when the pool is dropped
        // For explicit cleanup, users should call cleanup() manually
    }
}

/// Helper to load fixtures from SQL files
pub struct FixtureLoader {
    pool: PgPool,
}

impl FixtureLoader {
    /// Create a new fixture loader
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load a fixture from a SQL file
    ///
    /// Fixtures should be located in `tests/fixtures/`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use helpers::FixtureLoader;
    ///
    /// #[tokio::test]
    /// async fn test_with_fixtures() {
    ///     let test_db = TestDb::new().await;
    ///     let loader = FixtureLoader::new(test_db.pool_cloned());
    ///
    ///     loader.load("organizations").await.unwrap();
    ///     loader.load("registry_entries").await.unwrap();
    /// }
    /// ```
    pub async fn load(&self, fixture_name: &str) -> Result<(), sqlx::Error> {
        let fixture_path = format!("tests/fixtures/{}.sql", fixture_name);
        let sql = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|_| panic!("Failed to read fixture file: {}", fixture_path));

        sqlx::query(&sql).execute(&self.pool).await?;
        Ok(())
    }

    /// Load multiple fixtures in order
    pub async fn load_all(&self, fixtures: &[&str]) -> Result<(), sqlx::Error> {
        for fixture in fixtures {
            self.load(fixture).await?;
        }
        Ok(())
    }
}

/// Test data builders for common entities
pub mod builders {
    use super::*;

    /// Builder for test organizations
    pub struct OrganizationBuilder {
        slug: String,
        name: String,
        website: Option<String>,
        description: Option<String>,
        is_system: bool,
    }

    impl OrganizationBuilder {
        pub fn new(slug: impl Into<String>, name: impl Into<String>) -> Self {
            Self {
                slug: slug.into(),
                name: name.into(),
                website: None,
                description: None,
                is_system: false,
            }
        }

        pub fn website(mut self, website: impl Into<String>) -> Self {
            self.website = Some(website.into());
            self
        }

        pub fn description(mut self, description: impl Into<String>) -> Self {
            self.description = Some(description.into());
            self
        }

        pub fn system(mut self) -> Self {
            self.is_system = true;
            self
        }

        /// Create the organization in the database and return its ID
        pub async fn create(self, pool: &PgPool) -> Result<Uuid, sqlx::Error> {
            let id = sqlx::query_scalar!(
                r#"
                INSERT INTO organizations (slug, name, website, description, is_system)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
                self.slug,
                self.name,
                self.website,
                self.description,
                self.is_system
            )
            .fetch_one(pool)
            .await?;

            Ok(id)
        }
    }

    /// Builder for test registry entries
    pub struct RegistryEntryBuilder {
        organization_id: Uuid,
        slug: String,
        name: String,
        description: Option<String>,
        entry_type: String,
    }

    impl RegistryEntryBuilder {
        pub fn new(
            organization_id: Uuid,
            slug: impl Into<String>,
            name: impl Into<String>,
        ) -> Self {
            Self {
                organization_id,
                slug: slug.into(),
                name: name.into(),
                description: None,
                entry_type: "data_source".to_string(),
            }
        }

        pub fn description(mut self, description: impl Into<String>) -> Self {
            self.description = Some(description.into());
            self
        }

        pub fn as_tool(mut self) -> Self {
            self.entry_type = "tool".to_string();
            self
        }

        pub fn as_data_source(mut self) -> Self {
            self.entry_type = "data_source".to_string();
            self
        }

        /// Create the registry entry in the database and return its ID
        pub async fn create(self, pool: &PgPool) -> Result<Uuid, sqlx::Error> {
            let id = sqlx::query_scalar!(
                r#"
                INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
                self.organization_id,
                self.slug,
                self.name,
                self.description,
                self.entry_type
            )
            .fetch_one(pool)
            .await?;

            Ok(id)
        }
    }
}

/// Common test assertions
pub mod assertions {
    use super::*;

    /// Assert that a table has a specific row count
    pub async fn assert_table_count(
        pool: &PgPool,
        table: &str,
        expected: i64,
    ) -> Result<(), sqlx::Error> {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(pool)
            .await?;

        assert_eq!(
            count, expected,
            "Expected {} rows in table '{}', found {}",
            expected, table, count
        );
        Ok(())
    }

    /// Assert that a record exists by ID
    pub async fn assert_exists_by_id(
        pool: &PgPool,
        table: &str,
        id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let exists: bool =
            sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1)", table))
                .bind(id)
                .fetch_one(pool)
                .await?;

        assert!(exists, "Record with id {} not found in table '{}'", id, table);
        Ok(())
    }

    /// Assert that a record does not exist by ID
    pub async fn assert_not_exists_by_id(
        pool: &PgPool,
        table: &str,
        id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let exists: bool =
            sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1)", table))
                .bind(id)
                .fetch_one(pool)
                .await?;

        assert!(
            !exists,
            "Record with id {} found in table '{}', but expected it to not exist",
            id, table
        );
        Ok(())
    }
}

/// Helpers for creating test data
pub async fn create_test_organization(
    pool: &PgPool,
    slug: &str,
    name: &str,
) -> Result<Uuid, sqlx::Error> {
    builders::OrganizationBuilder::new(slug, name)
        .create(pool)
        .await
}

/// Create a test organization with full details
pub async fn create_test_organization_full(
    pool: &PgPool,
    slug: &str,
    name: &str,
    website: Option<&str>,
    description: Option<&str>,
    is_system: bool,
) -> Result<Uuid, sqlx::Error> {
    let mut builder = builders::OrganizationBuilder::new(slug, name);

    if let Some(w) = website {
        builder = builder.website(w);
    }
    if let Some(d) = description {
        builder = builder.description(d);
    }
    if is_system {
        builder = builder.system();
    }

    builder.create(pool).await
}

/// Create a test registry entry
pub async fn create_test_registry_entry(
    pool: &PgPool,
    organization_id: Uuid,
    slug: &str,
    name: &str,
    entry_type: &str,
) -> Result<Uuid, sqlx::Error> {
    let mut builder = builders::RegistryEntryBuilder::new(organization_id, slug, name);

    if entry_type == "tool" {
        builder = builder.as_tool();
    }

    builder.create(pool).await
}

/// Create a test version
pub async fn create_test_version(
    pool: &PgPool,
    registry_entry_id: Uuid,
    version_string: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO versions (registry_entry_id, version, status)
        VALUES ($1, $2, 'published')
        RETURNING id
        "#,
        registry_entry_id,
        version_string
    )
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// Setup a test database connection pool (simpler version for API tests)
///
/// This uses an in-memory or test database. You should set
/// TEST_DATABASE_URL environment variable to point to a test database.
pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/bdp_test".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Setup a test application with routes
pub async fn setup_test_app(pool: PgPool) -> axum::Router {
    use axum::{routing::get, Router};
    use bdp_server::api;

    // Create router with just the API endpoints
    // Note: Since storage uses SQLite which isn't enabled in sqlx workspace config,
    // we'll just test the API routes with the database pool directly
    let api_v1 = Router::new()
        .route("/organizations", get(api::organizations::list_organizations))
        .route("/organizations/:slug", get(api::organizations::get_organization))
        .route("/sources", get(api::sources::list_sources))
        .route("/sources/:org/:name", get(api::sources::get_source))
        .route("/sources/:org/:name/:version", get(api::sources::get_source_version))
        .route("/sources/:org/:name/:version/dependencies", get(api::sources::get_dependencies))
        .route("/sources/:org/:name/:version/download", get(api::sources::download_file))
        .with_state(pool);

    Router::new().nest("/api/v1", api_v1)
}

/// Test application wrapper for integration tests
pub struct TestApp {
    pub pool: PgPool,
    pub router: axum::Router,
}

impl TestApp {
    /// Create a new test application with CQRS features
    pub async fn new(pool: PgPool) -> Self {
        use axum::{routing::get, Router};
        use bdp_server::{audit, features};
        use tower::ServiceBuilder;

        // Create feature state
        let feature_state = features::FeatureState { db: pool.clone() };

        // Feature routes (CQRS architecture)
        let feature_routes = features::router(feature_state);

        // Audit endpoint
        let audit_layer = audit::AuditLayer::new(pool.clone());

        // Build router with audit middleware
        let router = Router::new()
            .merge(feature_routes)
            .layer(ServiceBuilder::new().layer(audit_layer));

        Self { pool, router }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_organization_builder() {
        let test_db = TestDb::new().await;
        let pool = test_db.pool();

        let org_id = builders::OrganizationBuilder::new("test-org", "Test Organization")
            .website("https://example.com")
            .description("A test organization")
            .create(pool)
            .await
            .expect("Failed to create organization");

        // Verify organization was created
        let org = sqlx::query!(
            "SELECT slug, name, website, description FROM organizations WHERE id = $1",
            org_id
        )
        .fetch_one(pool)
        .await
        .expect("Failed to fetch organization");

        assert_eq!(org.slug, "test-org");
        assert_eq!(org.name, "Test Organization");
        assert_eq!(org.website.as_deref(), Some("https://example.com"));
        assert_eq!(org.description.as_deref(), Some("A test organization"));
    }

    #[tokio::test]
    async fn test_assert_table_count() {
        let test_db = TestDb::new().await;
        let pool = test_db.pool();

        // Initially should have 0 organizations
        assertions::assert_table_count(pool, "organizations", 0)
            .await
            .expect("Count assertion failed");

        // Create an organization
        create_test_organization(pool, "test", "Test")
            .await
            .expect("Failed to create organization");

        // Now should have 1
        assertions::assert_table_count(pool, "organizations", 1)
            .await
            .expect("Count assertion failed");
    }
}
