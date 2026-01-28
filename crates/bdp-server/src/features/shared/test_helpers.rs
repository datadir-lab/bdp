//! Test helpers and fixtures for database tests
//!
//! Provides utilities to reduce boilerplate in test setup.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::features::shared::test_helpers::*;
//!
//! #[sqlx::test]
//! async fn test_something(pool: PgPool) -> sqlx::Result<()> {
//!     // Create an organization
//!     let org = TestOrganization::new("test-org", "Test Org")
//!         .with_system(true)
//!         .insert(&pool)
//!         .await?;
//!
//!     // Create a data source
//!     let data_source = TestDataSource::new(&org, "test-source", "Test Source")
//!         .with_source_type("protein")
//!         .insert(&pool)
//!         .await?;
//!
//!     // ... test logic ...
//!     Ok(())
//! }
//! ```

use sqlx::PgPool;
use uuid::Uuid;

/// Builder for creating test organizations
#[derive(Debug, Clone)]
pub struct TestOrganization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub is_system: bool,
    pub website: Option<String>,
    pub description: Option<String>,
}

impl TestOrganization {
    /// Create a new test organization builder
    pub fn new(slug: &str, name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            slug: slug.to_string(),
            name: name.to_string(),
            is_system: false,
            website: None,
            description: None,
        }
    }

    /// Set the is_system flag
    pub fn with_system(mut self, is_system: bool) -> Self {
        self.is_system = is_system;
        self
    }

    /// Set the website
    pub fn with_website(mut self, website: &str) -> Self {
        self.website = Some(website.to_string());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Use a specific ID
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Insert the organization into the database
    pub async fn insert(self, pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system, website, description)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            self.id,
            self.slug,
            self.name,
            self.is_system,
            self.website,
            self.description
        )
        .execute(pool)
        .await?;

        Ok(self)
    }
}

/// Builder for creating test data sources
#[derive(Debug, Clone)]
pub struct TestDataSource {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub slug: String,
    pub name: String,
    pub source_type: String,
    pub description: Option<String>,
    pub external_id: Option<String>,
}

impl TestDataSource {
    /// Create a new test data source builder
    pub fn new(org: &TestOrganization, slug: &str, name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            organization_id: org.id,
            slug: slug.to_string(),
            name: name.to_string(),
            source_type: "protein".to_string(),
            description: None,
            external_id: None,
        }
    }

    /// Create a data source with a specific organization ID
    pub fn with_org_id(org_id: Uuid, slug: &str, name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            organization_id: org_id,
            slug: slug.to_string(),
            name: name.to_string(),
            source_type: "protein".to_string(),
            description: None,
            external_id: None,
        }
    }

    /// Set the source type
    pub fn with_source_type(mut self, source_type: &str) -> Self {
        self.source_type = source_type.to_string();
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the external ID
    pub fn with_external_id(mut self, external_id: &str) -> Self {
        self.external_id = Some(external_id.to_string());
        self
    }

    /// Use a specific ID
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Insert the data source into the database
    ///
    /// Creates both the registry_entry and data_sources records.
    pub async fn insert(self, pool: &PgPool) -> sqlx::Result<Self> {
        // Insert registry entry
        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, $5, 'data_source')
            RETURNING id
            "#,
            self.id,
            self.organization_id,
            self.slug,
            self.name,
            self.description
        )
        .fetch_one(pool)
        .await?;

        // Insert data source record
        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type, external_id)
            VALUES ($1, $2, $3)
            "#,
            entry_id,
            self.source_type,
            self.external_id
        )
        .execute(pool)
        .await?;

        Ok(Self {
            id: entry_id,
            ..self
        })
    }
}

/// Builder for creating test versions
#[derive(Debug, Clone)]
pub struct TestVersion {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub version: String,
    pub external_version: Option<String>,
    pub download_count: i64,
}

impl TestVersion {
    /// Create a new test version builder
    pub fn new(data_source: &TestDataSource, version: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            entry_id: data_source.id,
            version: version.to_string(),
            external_version: None,
            download_count: 0,
        }
    }

    /// Create with a specific entry ID
    pub fn with_entry_id(entry_id: Uuid, version: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            entry_id,
            version: version.to_string(),
            external_version: None,
            download_count: 0,
        }
    }

    /// Set the external version
    pub fn with_external_version(mut self, external_version: &str) -> Self {
        self.external_version = Some(external_version.to_string());
        self
    }

    /// Set the download count
    pub fn with_download_count(mut self, count: i64) -> Self {
        self.download_count = count;
        self
    }

    /// Insert the version into the database
    pub async fn insert(self, pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query!(
            r#"
            INSERT INTO versions (id, entry_id, version, external_version, download_count)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            self.id,
            self.entry_id,
            self.version,
            self.external_version,
            self.download_count
        )
        .execute(pool)
        .await?;

        Ok(self)
    }
}

/// Builder for creating test tags
#[derive(Debug, Clone)]
pub struct TestTag {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
}

impl TestTag {
    /// Create a new test tag builder
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            category: None,
        }
    }

    /// Set the category
    pub fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }

    /// Insert the tag into the database
    pub async fn insert(self, pool: &PgPool) -> sqlx::Result<Self> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO tags (name, category)
            VALUES ($1, $2)
            RETURNING id
            "#,
            self.name,
            self.category
        )
        .fetch_one(pool)
        .await?;

        Ok(Self { id, ..self })
    }

    /// Link this tag to a data source
    pub async fn link_to_entry(&self, pool: &PgPool, entry_id: Uuid) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO entry_tags (entry_id, tag_id)
            VALUES ($1, $2)
            "#,
            entry_id,
            self.id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Quick helper to create an organization and return its ID
pub async fn create_test_org(pool: &PgPool, slug: &str, name: &str) -> sqlx::Result<Uuid> {
    let org = TestOrganization::new(slug, name).insert(pool).await?;
    Ok(org.id)
}

/// Quick helper to create a data source and return its ID
pub async fn create_test_data_source(
    pool: &PgPool,
    org_id: Uuid,
    slug: &str,
    name: &str,
    source_type: &str,
) -> sqlx::Result<Uuid> {
    let ds = TestDataSource::with_org_id(org_id, slug, name)
        .with_source_type(source_type)
        .insert(pool)
        .await?;
    Ok(ds.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organization_builder() {
        let org = TestOrganization::new("test-org", "Test Org")
            .with_system(true)
            .with_website("https://example.com")
            .with_description("Test description");

        assert_eq!(org.slug, "test-org");
        assert_eq!(org.name, "Test Org");
        assert!(org.is_system);
        assert_eq!(org.website, Some("https://example.com".to_string()));
        assert_eq!(org.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_data_source_builder() {
        let org = TestOrganization::new("test-org", "Test Org");
        let ds = TestDataSource::new(&org, "test-ds", "Test DS")
            .with_source_type("genome")
            .with_external_id("ABC123");

        assert_eq!(ds.slug, "test-ds");
        assert_eq!(ds.source_type, "genome");
        assert_eq!(ds.external_id, Some("ABC123".to_string()));
        assert_eq!(ds.organization_id, org.id);
    }

    #[test]
    fn test_version_builder() {
        let org = TestOrganization::new("test-org", "Test Org");
        let ds = TestDataSource::new(&org, "test-ds", "Test DS");
        let version = TestVersion::new(&ds, "1.0.0")
            .with_external_version("2024_01")
            .with_download_count(100);

        assert_eq!(version.version, "1.0.0");
        assert_eq!(version.external_version, Some("2024_01".to_string()));
        assert_eq!(version.download_count, 100);
    }

    #[test]
    fn test_tag_builder() {
        let tag = TestTag::new("human").with_category("organism");

        assert_eq!(tag.name, "human");
        assert_eq!(tag.category, Some("organism".to_string()));
    }
}
