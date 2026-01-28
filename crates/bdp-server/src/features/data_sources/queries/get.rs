//! Get data source query
//!
//! Retrieves detailed information about a data source including versions,
//! organism information, protein metadata, and tags.

use chrono::{DateTime, NaiveDate, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Query to retrieve a data source by organization slug and data source slug
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::features::data_sources::queries::GetDataSourceQuery;
///
/// let query = GetDataSourceQuery {
///     organization_slug: "uniprot".to_string(),
///     slug: "P01308-fasta".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataSourceQuery {
    pub organization_slug: String,
    pub slug: String,
}

/// Response containing full data source details
///
/// Includes organization info, versions, organism data, protein metadata,
/// tags, and download statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataSourceResponse {
    pub id: Uuid,
    pub organization: OrganizationInfo,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organism: Option<OrganismInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_metadata: Option<ProteinMetadataInfo>,
    pub versions: Vec<VersionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub total_downloads: i64,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization information embedded in data source response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    /// Unique identifier
    pub id: Uuid,
    /// URL-safe slug
    pub slug: String,
    /// Display name
    pub name: String,
}

/// Organism/taxonomy information for a data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismInfo {
    /// Unique identifier of the organism data source
    pub id: Uuid,
    /// NCBI Taxonomy ID (e.g., 9606 for Homo sapiens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_taxonomy_id: Option<i32>,
    /// Scientific name (e.g., "Homo sapiens")
    pub scientific_name: String,
    /// Common name (e.g., "Human")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
    /// Taxonomic rank (e.g., "species", "genus")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
    /// Taxonomy data source organization slug (e.g., "ncbi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_organization_slug: Option<String>,
    /// Taxonomy data source slug (e.g., "9606")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_slug: Option<String>,
    /// Taxonomy data source version (e.g., "1.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_version: Option<String>,
}

/// Protein-specific metadata for protein data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinMetadataInfo {
    /// UniProt accession number (e.g., "P01308")
    pub accession: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_length: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mass_da: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_numbers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_existence: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organelle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_created: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_updated: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_updated: Option<NaiveDate>,
}

/// Version information for a data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Unique identifier
    pub id: Uuid,
    /// Semantic version string (e.g., "1.0.0")
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub download_count: i64,
    pub published_at: DateTime<Utc>,
}

/// Errors that can occur when getting a data source
#[derive(Debug, thiserror::Error)]
pub enum GetDataSourceError {
    /// Organization slug was empty
    #[error("Organization slug is required. Provide the organization name in the URL (e.g., /sources/uniprot/protein-name).")]
    OrganizationSlugRequired,
    /// Data source slug was empty
    #[error("Data source slug is required. Provide the data source name in the URL (e.g., /sources/uniprot/protein-name).")]
    SlugRequired,
    /// Data source was not found
    #[error("Data source '{0}/{1}' not found. Verify the organization and data source names, or use the search endpoint to find available sources.")]
    NotFound(String, String),
    /// A database error occurred
    #[error("Failed to retrieve data source: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<GetDataSourceResponse, GetDataSourceError>> for GetDataSourceQuery {}

impl crate::cqrs::middleware::Query for GetDataSourceQuery {}

impl GetDataSourceQuery {
    /// Validates the query parameters
    ///
    /// # Errors
    ///
    /// - `OrganizationSlugRequired` - Organization slug is empty
    /// - `SlugRequired` - Data source slug is empty
    pub fn validate(&self) -> Result<(), GetDataSourceError> {
        if self.organization_slug.is_empty() {
            return Err(GetDataSourceError::OrganizationSlugRequired);
        }
        if self.slug.is_empty() {
            return Err(GetDataSourceError::SlugRequired);
        }
        Ok(())
    }
}

/// Handles the get data source query
///
/// Retrieves complete data source information including:
/// - Basic info (name, description, type)
/// - All versions with download counts
/// - Organism/taxonomy information (if applicable)
/// - Protein metadata (for protein data sources)
/// - Associated tags
///
/// The lookup is case-insensitive for both organization and data source slugs.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `query` - Query containing organization and data source slugs
///
/// # Returns
///
/// Returns complete data source details on success.
///
/// # Errors
///
/// - `OrganizationSlugRequired` - Organization slug is empty
/// - `SlugRequired` - Data source slug is empty
/// - `NotFound` - No matching data source exists
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetDataSourceQuery,
) -> Result<GetDataSourceResponse, GetDataSourceError> {
    query.validate()?;

    let result = sqlx::query_as!(
        DataSourceRecord,
        r#"
        SELECT
            re.id,
            re.organization_id,
            o.slug as organization_slug,
            o.name as organization_name,
            re.slug,
            re.name,
            re.description,
            ds.source_type,
            ds.external_id,
            COALESCE(pm.taxonomy_id, CASE WHEN ds.source_type = 'organism' THEN ds.id ELSE NULL END) as organism_id,
            COALESCE(om_ref.taxonomy_id, om_direct.taxonomy_id) as ncbi_taxonomy_id,
            COALESCE(om_ref.scientific_name, om_direct.scientific_name) as scientific_name,
            COALESCE(om_ref.common_name, om_direct.common_name) as common_name,
            pm.accession as "protein_accession?",
            pm.entry_name as "protein_entry_name?",
            pm.protein_name as "protein_name?",
            pm.gene_name as "gene_name?",
            pm.sequence_length as "sequence_length?",
            pm.mass_da as "mass_da?",
            pm.sequence_checksum as "sequence_checksum?",
            pm.alternative_names as "alternative_names?",
            pm.ec_numbers as "ec_numbers?",
            pm.protein_existence as "protein_existence?",
            pm.keywords as "keywords?",
            pm.organelle as "organelle?",
            pm.entry_created as "entry_created?: chrono::NaiveDate",
            pm.sequence_updated as "sequence_updated?: chrono::NaiveDate",
            pm.annotation_updated as "annotation_updated?: chrono::NaiveDate",
            COALESCE(om_ref.rank, om_direct.rank) as "organism_rank?",
            tax_org.slug as "taxonomy_organization_slug?",
            tax_re.slug as "taxonomy_slug?",
            tax_v.version_string as "taxonomy_version?",
            re.created_at as "created_at!",
            re.updated_at as "updated_at!"
        FROM registry_entries re
        JOIN data_sources ds ON re.id = ds.id
        JOIN organizations o ON re.organization_id = o.id
        LEFT JOIN protein_metadata pm ON ds.id = pm.data_source_id
        LEFT JOIN taxonomy_metadata om_ref ON pm.taxonomy_id = om_ref.data_source_id
        LEFT JOIN taxonomy_metadata om_direct ON ds.id = om_direct.data_source_id AND ds.source_type = 'organism'
        LEFT JOIN registry_entries tax_re ON pm.taxonomy_id = tax_re.id
        LEFT JOIN organizations tax_org ON tax_re.organization_id = tax_org.id
        LEFT JOIN LATERAL (
            SELECT version_string
            FROM versions
            WHERE entry_id = pm.taxonomy_id
            ORDER BY published_at DESC
            LIMIT 1
        ) tax_v ON pm.taxonomy_id IS NOT NULL
        WHERE LOWER(o.slug) = LOWER($1) AND LOWER(re.slug) = LOWER($2)
        "#,
        query.organization_slug,
        query.slug
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Database error retrieving data source {}/{}: {:?}",
            query.organization_slug,
            query.slug,
            e
        );
        GetDataSourceError::Database(e)
    })?;

    // PERFORMANCE: Extract slugs before consuming result to avoid unnecessary clones
    let result = match result {
        Some(r) => r,
        None => {
            return Err(GetDataSourceError::NotFound(query.organization_slug, query.slug));
        },
    };

    let versions = sqlx::query_as!(
        VersionRecord,
        r#"
        SELECT id, version, external_version, release_date, size_bytes,
               download_count as "download_count!", published_at as "published_at!"
        FROM versions
        WHERE entry_id = $1
        ORDER BY published_at DESC
        "#,
        result.id
    )
    .fetch_all(&pool)
    .await?;

    let total_downloads: i64 = versions.iter().map(|v| v.download_count).sum();

    let latest_version = versions.first().map(|v| v.version.clone());

    let tags = sqlx::query_scalar!(
        r#"
        SELECT t.name
        FROM tags t
        JOIN entry_tags et ON t.id = et.tag_id
        WHERE et.entry_id = $1
        ORDER BY t.name
        "#,
        result.id
    )
    .fetch_all(&pool)
    .await?;

    // PERFORMANCE: Move fields out of result instead of cloning
    // Construct optional nested structs by consuming result fields
    let organism = if let Some(id) = result.organism_id {
        Some(OrganismInfo {
            id,
            ncbi_taxonomy_id: result.ncbi_taxonomy_id,
            scientific_name: result.scientific_name.unwrap_or_default(),
            common_name: result.common_name,
            rank: result.organism_rank,
            taxonomy_organization_slug: result.taxonomy_organization_slug,
            taxonomy_slug: result.taxonomy_slug,
            taxonomy_version: result.taxonomy_version,
        })
    } else {
        None
    };

    let protein_metadata = if let Some(accession) = result.protein_accession {
        Some(ProteinMetadataInfo {
            accession,
            entry_name: result.protein_entry_name,
            protein_name: result.protein_name,
            gene_name: result.gene_name,
            sequence_length: result.sequence_length,
            mass_da: result.mass_da,
            sequence_checksum: result.sequence_checksum,
            alternative_names: result.alternative_names,
            ec_numbers: result.ec_numbers,
            protein_existence: result.protein_existence,
            keywords: result.keywords,
            organelle: result.organelle,
            entry_created: result.entry_created,
            sequence_updated: result.sequence_updated,
            annotation_updated: result.annotation_updated,
        })
    } else {
        None
    };

    Ok(GetDataSourceResponse {
        id: result.id,
        organization: OrganizationInfo {
            id: result.organization_id,
            slug: result.organization_slug,
            name: result.organization_name,
        },
        slug: result.slug,
        name: result.name,
        description: result.description,
        source_type: result.source_type,
        external_id: result.external_id,
        organism,
        protein_metadata,
        versions: versions
            .into_iter()
            .map(|v| VersionInfo {
                id: v.id,
                version: v.version,
                external_version: v.external_version,
                release_date: v.release_date,
                size_bytes: v.size_bytes,
                download_count: v.download_count,
                published_at: v.published_at,
            })
            .collect(),
        latest_version,
        total_downloads,
        tags,
        created_at: result.created_at,
        updated_at: result.updated_at,
    })
}

#[derive(Debug)]
struct DataSourceRecord {
    id: Uuid,
    organization_id: Uuid,
    organization_slug: String,
    organization_name: String,
    slug: String,
    name: String,
    description: Option<String>,
    source_type: String,
    external_id: Option<String>,
    organism_id: Option<Uuid>,
    ncbi_taxonomy_id: Option<i32>,
    scientific_name: Option<String>,
    common_name: Option<String>,
    protein_accession: Option<String>,
    protein_entry_name: Option<String>,
    protein_name: Option<String>,
    gene_name: Option<String>,
    sequence_length: Option<i32>,
    mass_da: Option<i64>,
    sequence_checksum: Option<String>,
    alternative_names: Option<Vec<String>>,
    ec_numbers: Option<Vec<String>>,
    protein_existence: Option<i32>,
    keywords: Option<Vec<String>>,
    organelle: Option<String>,
    entry_created: Option<NaiveDate>,
    sequence_updated: Option<NaiveDate>,
    annotation_updated: Option<NaiveDate>,
    organism_rank: Option<String>,
    taxonomy_organization_slug: Option<String>,
    taxonomy_slug: Option<String>,
    taxonomy_version: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
struct VersionRecord {
    id: Uuid,
    version: String,
    external_version: Option<String>,
    release_date: Option<NaiveDate>,
    size_bytes: Option<i64>,
    download_count: i64,
    published_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "test-protein".to_string(),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_organization_slug() {
        let query = GetDataSourceQuery {
            organization_slug: "".to_string(),
            slug: "test-protein".to_string(),
        };
        assert!(matches!(query.validate(), Err(GetDataSourceError::OrganizationSlugRequired)));
    }

    #[test]
    fn test_validation_empty_slug() {
        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "".to_string(),
        };
        assert!(matches!(query.validate(), Err(GetDataSourceError::SlugRequired)));
    }

    #[sqlx::test]
    async fn test_handle_gets_data_source(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein",
            Some("Test description")
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type, external_id) VALUES ($1, $2, $3)",
            entry_id,
            "protein",
            Some("P12345")
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO versions (entry_id, version, external_version) VALUES ($1, $2, $3)",
            entry_id,
            "1.0",
            Some("2025_01")
        )
        .execute(&pool)
        .await?;

        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "test-protein".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-protein");
        assert_eq!(response.name, "Test Protein");
        assert_eq!(response.source_type, "protein");
        assert_eq!(response.external_id, Some("P12345".to_string()));
        assert_eq!(response.versions.len(), 1);
        assert_eq!(response.versions[0].version, "1.0");
        assert_eq!(response.latest_version, Some("1.0".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = GetDataSourceQuery {
            organization_slug: "nonexistent".to_string(),
            slug: "nonexistent".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(result, Err(GetDataSourceError::NotFound(_, _))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_with_tags(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let tag_id = sqlx::query_scalar!(
            "INSERT INTO tags (name, category) VALUES ($1, $2) RETURNING id",
            "human",
            Some("organism")
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!("INSERT INTO entry_tags (entry_id, tag_id) VALUES ($1, $2)", entry_id, tag_id)
            .execute(&pool)
            .await?;

        let query = GetDataSourceQuery {
            organization_slug: "test-org".to_string(),
            slug: "test-protein".to_string(),
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.tags.len(), 1);
        assert_eq!(response.tags[0], "human");
        Ok(())
    }
}
