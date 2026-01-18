//! Test fixtures and data builders for BDP server tests
//!
//! This module provides reusable test data builders and fixture generators
//! for creating test entities with minimal boilerplate.

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Organization Fixtures
// ============================================================================

/// Builder for creating test organizations with fluent API
#[derive(Debug, Clone)]
pub struct OrganizationFixture {
    slug: String,
    name: String,
    website: Option<String>,
    description: Option<String>,
    logo_url: Option<String>,
    is_system: bool,
}

impl OrganizationFixture {
    /// Create a new organization fixture with required fields
    pub fn new(slug: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            name: name.into(),
            website: None,
            description: None,
            logo_url: None,
            is_system: false,
        }
    }

    /// Create a system organization (pre-configured data providers)
    pub fn system(slug: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(slug, name).with_system(true)
    }

    /// Set the website URL
    pub fn with_website(mut self, website: impl Into<String>) -> Self {
        self.website = Some(website.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the logo URL
    pub fn with_logo(mut self, logo_url: impl Into<String>) -> Self {
        self.logo_url = Some(logo_url.into());
        self
    }

    /// Set the system flag
    pub fn with_system(mut self, is_system: bool) -> Self {
        self.is_system = is_system;
        self
    }

    /// Create the organization in the database and return its ID
    pub async fn create(self, pool: &PgPool) -> sqlx::Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, website, description, logo_url, is_system)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            self.slug,
            self.name,
            self.website,
            self.description,
            self.logo_url,
            self.is_system
        )
        .fetch_one(pool)
        .await?;

        Ok(id)
    }
}

// ============================================================================
// Registry Entry Fixtures
// ============================================================================

/// Builder for creating test registry entries
#[derive(Debug, Clone)]
pub struct RegistryEntryFixture {
    organization_id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    entry_type: String,
}

impl RegistryEntryFixture {
    /// Create a new registry entry fixture
    pub fn new(organization_id: Uuid, slug: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            organization_id,
            slug: slug.into(),
            name: name.into(),
            description: None,
            entry_type: "data_source".to_string(),
        }
    }

    /// Create a data source entry
    pub fn data_source(
        organization_id: Uuid,
        slug: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self::new(organization_id, slug, name).with_type("data_source")
    }

    /// Create a tool entry
    pub fn tool(organization_id: Uuid, slug: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(organization_id, slug, name).with_type("tool")
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the entry type
    pub fn with_type(mut self, entry_type: impl Into<String>) -> Self {
        self.entry_type = entry_type.into();
        self
    }

    /// Create the registry entry in the database and return its ID
    pub async fn create(self, pool: &PgPool) -> sqlx::Result<Uuid> {
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

// ============================================================================
// Version Fixtures
// ============================================================================

/// Builder for creating test versions
#[derive(Debug, Clone)]
pub struct VersionFixture {
    entry_id: Uuid,
    version: String,
    external_version: Option<String>,
    release_date: Option<NaiveDate>,
    size_bytes: Option<i64>,
    additional_metadata: Option<serde_json::Value>,
}

impl VersionFixture {
    /// Create a new version fixture
    pub fn new(entry_id: Uuid, version: impl Into<String>) -> Self {
        Self {
            entry_id,
            version: version.into(),
            external_version: None,
            release_date: None,
            size_bytes: None,
            additional_metadata: None,
        }
    }

    /// Set the external version string
    pub fn with_external_version(mut self, external_version: impl Into<String>) -> Self {
        self.external_version = Some(external_version.into());
        self
    }

    /// Set the release date
    pub fn with_release_date(mut self, year: i32, month: u32, day: u32) -> Self {
        self.release_date = NaiveDate::from_ymd_opt(year, month, day);
        self
    }

    /// Set the total size in bytes
    pub fn with_size_bytes(mut self, size_bytes: i64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    /// Set additional metadata as JSON
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.additional_metadata = Some(metadata);
        self
    }

    /// Create the version in the database and return its ID
    pub async fn create(self, pool: &PgPool) -> sqlx::Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO versions (entry_id, version, external_version, release_date, size_bytes, additional_metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            self.entry_id,
            self.version,
            self.external_version,
            self.release_date,
            self.size_bytes,
            self.additional_metadata
        )
        .fetch_one(pool)
        .await?;

        Ok(id)
    }
}

// ============================================================================
// Dependency Fixtures
// ============================================================================

/// Builder for creating test dependencies
#[derive(Debug, Clone)]
pub struct DependencyFixture {
    version_id: Uuid,
    depends_on_entry_id: Uuid,
    depends_on_version: String,
    dependency_type: String,
}

impl DependencyFixture {
    /// Create a new dependency fixture
    pub fn new(
        version_id: Uuid,
        depends_on_entry_id: Uuid,
        depends_on_version: impl Into<String>,
    ) -> Self {
        Self {
            version_id,
            depends_on_entry_id,
            depends_on_version: depends_on_version.into(),
            dependency_type: "required".to_string(),
        }
    }

    /// Create a required dependency
    pub fn required(
        version_id: Uuid,
        depends_on_entry_id: Uuid,
        depends_on_version: impl Into<String>,
    ) -> Self {
        Self::new(version_id, depends_on_entry_id, depends_on_version).with_type("required")
    }

    /// Create an optional dependency
    pub fn optional(
        version_id: Uuid,
        depends_on_entry_id: Uuid,
        depends_on_version: impl Into<String>,
    ) -> Self {
        Self::new(version_id, depends_on_entry_id, depends_on_version).with_type("optional")
    }

    /// Set the dependency type
    pub fn with_type(mut self, dependency_type: impl Into<String>) -> Self {
        self.dependency_type = dependency_type.into();
        self
    }

    /// Create the dependency in the database and return its ID
    pub async fn create(self, pool: &PgPool) -> sqlx::Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version, dependency_type)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            self.version_id,
            self.depends_on_entry_id,
            self.depends_on_version,
            self.dependency_type
        )
        .fetch_one(pool)
        .await?;

        Ok(id)
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a complete test dataset with organization, entry, and version
pub async fn create_test_dataset(
    pool: &PgPool,
    org_slug: &str,
    entry_slug: &str,
    version: &str,
) -> sqlx::Result<(Uuid, Uuid, Uuid)> {
    let org_id = OrganizationFixture::new(org_slug, format!("{} Organization", org_slug))
        .create(pool)
        .await?;

    let entry_id =
        RegistryEntryFixture::data_source(org_id, entry_slug, format!("{} Dataset", entry_slug))
            .create(pool)
            .await?;

    let version_id = VersionFixture::new(entry_id, version).create(pool).await?;

    Ok((org_id, entry_id, version_id))
}

/// Create a test organization with multiple entries
pub async fn create_organization_with_entries(
    pool: &PgPool,
    org_slug: &str,
    entry_count: usize,
) -> sqlx::Result<(Uuid, Vec<Uuid>)> {
    let org_id = OrganizationFixture::new(org_slug, format!("{} Organization", org_slug))
        .create(pool)
        .await?;

    let mut entry_ids = Vec::with_capacity(entry_count);
    for i in 0..entry_count {
        let entry_id = RegistryEntryFixture::data_source(
            org_id,
            format!("{}-entry-{}", org_slug, i),
            format!("Entry {}", i),
        )
        .create(pool)
        .await?;
        entry_ids.push(entry_id);
    }

    Ok((org_id, entry_ids))
}

/// Create a test entry with multiple versions
pub async fn create_entry_with_versions(
    pool: &PgPool,
    org_id: Uuid,
    entry_slug: &str,
    versions: &[&str],
) -> sqlx::Result<(Uuid, Vec<Uuid>)> {
    let entry_id =
        RegistryEntryFixture::data_source(org_id, entry_slug, format!("{} Dataset", entry_slug))
            .create(pool)
            .await?;

    let mut version_ids = Vec::with_capacity(versions.len());
    for version in versions {
        let version_id = VersionFixture::new(entry_id, *version).create(pool).await?;
        version_ids.push(version_id);
    }

    Ok((entry_id, version_ids))
}

// ============================================================================
// Seed Data Functions
// ============================================================================

/// Seed the database with common test organizations
pub async fn seed_organizations(pool: &PgPool) -> sqlx::Result<()> {
    OrganizationFixture::system("uniprot", "UniProt")
        .with_website("https://www.uniprot.org")
        .with_description("Universal Protein Resource")
        .create(pool)
        .await?;

    OrganizationFixture::system("ncbi", "NCBI")
        .with_website("https://www.ncbi.nlm.nih.gov")
        .with_description("National Center for Biotechnology Information")
        .create(pool)
        .await?;

    OrganizationFixture::system("ensembl", "Ensembl")
        .with_website("https://www.ensembl.org")
        .with_description("Ensembl Genome Browser")
        .create(pool)
        .await?;

    Ok(())
}

/// Seed the database with common test registry entries
pub async fn seed_registry_entries(pool: &PgPool) -> sqlx::Result<()> {
    let uniprot_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'uniprot'")
        .fetch_one(pool)
        .await?;

    let ncbi_id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = 'ncbi'")
        .fetch_one(pool)
        .await?;

    RegistryEntryFixture::data_source(uniprot_id, "swissprot-human", "Swiss-Prot Human Proteins")
        .with_description("Manually annotated and reviewed human proteins from UniProt/Swiss-Prot")
        .create(pool)
        .await?;

    RegistryEntryFixture::data_source(ncbi_id, "refseq-human", "RefSeq Human Sequences")
        .with_description("NCBI Reference Sequence Database - Human sequences")
        .create(pool)
        .await?;

    RegistryEntryFixture::tool(ncbi_id, "blast", "BLAST")
        .with_description("Basic Local Alignment Search Tool")
        .create(pool)
        .await?;

    Ok(())
}

/// Seed a complete test dataset with dependencies
pub async fn seed_complete_dataset(pool: &PgPool) -> sqlx::Result<()> {
    seed_organizations(pool).await?;
    seed_registry_entries(pool).await?;

    let swissprot_id =
        sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'swissprot-human'")
            .fetch_one(pool)
            .await?;

    let blast_id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = 'blast'")
        .fetch_one(pool)
        .await?;

    let version_id = VersionFixture::new(swissprot_id, "1.0")
        .with_external_version("2024_01")
        .with_release_date(2024, 1, 15)
        .with_size_bytes(1024 * 1024 * 1024)
        .create(pool)
        .await?;

    DependencyFixture::required(version_id, blast_id, "2.14.0")
        .create(pool)
        .await?;

    Ok(())
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that an organization exists by slug
pub async fn assert_organization_exists(pool: &PgPool, slug: &str) -> sqlx::Result<Uuid> {
    let id = sqlx::query_scalar!("SELECT id FROM organizations WHERE slug = $1", slug)
        .fetch_one(pool)
        .await?;
    Ok(id)
}

/// Assert that a registry entry exists by slug
pub async fn assert_registry_entry_exists(pool: &PgPool, slug: &str) -> sqlx::Result<Uuid> {
    let id = sqlx::query_scalar!("SELECT id FROM registry_entries WHERE slug = $1", slug)
        .fetch_one(pool)
        .await?;
    Ok(id)
}

/// Assert that a version exists
pub async fn assert_version_exists(
    pool: &PgPool,
    entry_id: Uuid,
    version: &str,
) -> sqlx::Result<Uuid> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM versions WHERE entry_id = $1 AND version = $2",
        entry_id,
        version
    )
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Assert the count of records in a table
pub async fn assert_count(pool: &PgPool, table: &str, expected: i64) -> sqlx::Result<()> {
    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
        .fetch_one(pool)
        .await?;

    assert_eq!(
        count, expected,
        "Expected {} records in table '{}', found {}",
        expected, table, count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::TestDb;

    #[tokio::test]
    async fn test_organization_fixture() {
        let test_db = TestDb::new().await;
        let pool = test_db.pool();

        let org_id = OrganizationFixture::new("test-org", "Test Organization")
            .with_website("https://example.com")
            .with_description("A test organization")
            .create(pool)
            .await
            .expect("Failed to create organization");

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
    async fn test_create_test_dataset() {
        let test_db = TestDb::new().await;
        let pool = test_db.pool();

        let (org_id, entry_id, version_id) =
            create_test_dataset(pool, "test-org", "test-entry", "1.0")
                .await
                .expect("Failed to create test dataset");

        assert!(sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)",
            org_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(false));
        assert!(sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM registry_entries WHERE id = $1)",
            entry_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(false));
        assert!(sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM versions WHERE id = $1)",
            version_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(false));
    }

    #[tokio::test]
    async fn test_seed_organizations() {
        let test_db = TestDb::new().await;
        let pool = test_db.pool();

        seed_organizations(pool)
            .await
            .expect("Failed to seed organizations");

        assert_count(pool, "organizations", 3)
            .await
            .expect("Expected 3 organizations");

        assert_organization_exists(pool, "uniprot")
            .await
            .expect("UniProt should exist");
        assert_organization_exists(pool, "ncbi")
            .await
            .expect("NCBI should exist");
        assert_organization_exists(pool, "ensembl")
            .await
            .expect("Ensembl should exist");
    }
}
