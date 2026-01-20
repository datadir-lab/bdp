//! Citation policy management for data sources
//!
//! This module provides functionality to set up citation policies and required citations
//! for each data source organization (GO, UniProt, NCBI, etc.)

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

/// Citation data for creating citation records
#[derive(Debug, Clone)]
pub struct CitationData {
    pub doi: Option<String>,
    pub pubmed_id: Option<String>,
    pub title: String,
    pub journal: Option<String>,
    pub publication_date: Option<chrono::NaiveDate>,
    pub volume: Option<String>,
    pub pages: Option<String>,
    pub authors: String,
    pub bibtex: Option<String>,
}

/// Citation policy configuration
#[derive(Debug, Clone)]
pub struct CitationPolicyConfig {
    pub organization_id: Uuid,
    pub policy_url: String,
    pub license_id: Option<Uuid>,
    pub requires_version_citation: bool,
    pub requires_accession_citation: bool,
    pub citation_instructions: String,
    pub required_citations: Vec<RequiredCitation>,
}

/// Required citation configuration
#[derive(Debug, Clone)]
pub struct RequiredCitation {
    pub citation: CitationData,
    pub requirement_type: CitationRequirementType,
    pub display_order: i32,
    pub context: Option<String>,
}

/// Citation requirement types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationRequirementType {
    Required,
    Recommended,
    Conditional,
}

impl CitationRequirementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Recommended => "recommended",
            Self::Conditional => "conditional",
        }
    }
}

/// Gene Ontology citation policy
pub fn gene_ontology_policy(organization_id: Uuid, license_id: Option<Uuid>) -> CitationPolicyConfig {
    CitationPolicyConfig {
        organization_id,
        policy_url: "https://geneontology.org/docs/go-citation-policy/".to_string(),
        license_id,
        requires_version_citation: true, // GO requires zenodo DOIs for releases
        requires_accession_citation: false,
        citation_instructions: "When publishing research using GO or its annotations, you must cite both foundational papers and include the GO release date and DOI.".to_string(),
        required_citations: vec![
            RequiredCitation {
                citation: CitationData {
                    doi: Some("10.1038/75556".to_string()),
                    pubmed_id: Some("10802651".to_string()),
                    title: "Gene ontology: tool for the unification of biology".to_string(),
                    journal: Some("Nature Genetics".to_string()),
                    publication_date: chrono::NaiveDate::from_ymd_opt(2000, 5, 1),
                    volume: Some("25".to_string()),
                    pages: Some("25-29".to_string()),
                    authors: "Ashburner, M., Ball, C. A., Blake, J. A., et al.".to_string(),
                    bibtex: None,
                },
                requirement_type: CitationRequirementType::Required,
                display_order: 1,
                context: Some("Original GO paper - always required".to_string()),
            },
            RequiredCitation {
                citation: CitationData {
                    doi: Some("10.1093/nar/gkaf1292".to_string()),
                    pubmed_id: None,
                    title: "The Gene Ontology knowledgebase in 2025".to_string(),
                    journal: Some("Nucleic Acids Research".to_string()),
                    publication_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 1),
                    volume: Some("53".to_string()),
                    pages: Some("D1".to_string()),
                    authors: "Gene Ontology Consortium".to_string(),
                    bibtex: None,
                },
                requirement_type: CitationRequirementType::Required,
                display_order: 2,
                context: Some("Most recent update paper - always required".to_string()),
            },
        ],
    }
}

/// UniProt citation policy
pub fn uniprot_policy(organization_id: Uuid, license_id: Option<Uuid>) -> CitationPolicyConfig {
    CitationPolicyConfig {
        organization_id,
        policy_url: "https://www.uniprot.org/help/publications".to_string(),
        license_id,
        requires_version_citation: false,
        requires_accession_citation: true, // UniProt requires accession numbers in citations
        citation_instructions: "When citing UniProt, use the most recent database paper and include accession numbers (e.g., UniProtKB P68369) in your text.".to_string(),
        required_citations: vec![
            RequiredCitation {
                citation: CitationData {
                    doi: Some("10.1093/nar/gkae1010".to_string()),
                    pubmed_id: Some("39552041".to_string()),
                    title: "UniProt: the Universal Protein Knowledgebase in 2025".to_string(),
                    journal: Some("Nucleic Acids Research".to_string()),
                    publication_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 6),
                    volume: Some("53".to_string()),
                    pages: Some("D609-D617".to_string()),
                    authors: "The UniProt Consortium".to_string(),
                    bibtex: None,
                },
                requirement_type: CitationRequirementType::Required,
                display_order: 1,
                context: Some("Primary database paper".to_string()),
            },
        ],
    }
}

/// NCBI RefSeq citation policy
pub fn ncbi_refseq_policy(organization_id: Uuid, license_id: Option<Uuid>) -> CitationPolicyConfig {
    CitationPolicyConfig {
        organization_id,
        policy_url: "https://www.ncbi.nlm.nih.gov/refseq/publications/".to_string(),
        license_id,
        requires_version_citation: true, // RefSeq requires release numbers and accession.version
        requires_accession_citation: true, // Must include version number (e.g., NM_000014.6)
        citation_instructions: "When citing RefSeq, include both the accession and version number (e.g., NM_000014.6), and cite the RefSeq FTP release number when working with release datasets.".to_string(),
        required_citations: vec![
            RequiredCitation {
                citation: CitationData {
                    doi: Some("10.1093/nar/gkae1099".to_string()),
                    pubmed_id: None,
                    title: "NCBI RefSeq: reference sequence standards through 25 years of curation and annotation".to_string(),
                    journal: Some("Nucleic Acids Research".to_string()),
                    publication_date: chrono::NaiveDate::from_ymd_opt(2025, 1, 6),
                    volume: Some("53".to_string()),
                    pages: Some("D243-D257".to_string()),
                    authors: "Goldfarb, T., Shumway, M., Astashyn, A., et al.".to_string(),
                    bibtex: None,
                },
                requirement_type: CitationRequirementType::Required,
                display_order: 1,
                context: Some("Primary RefSeq paper".to_string()),
            },
        ],
    }
}

/// NCBI Taxonomy citation policy (shares NCBI organization)
pub fn ncbi_taxonomy_policy(organization_id: Uuid, license_id: Option<Uuid>) -> CitationPolicyConfig {
    CitationPolicyConfig {
        organization_id,
        policy_url: "https://support.nlm.nih.gov/knowledgebase/article/KA-03391/en-us".to_string(),
        license_id,
        requires_version_citation: false,
        requires_accession_citation: false,
        citation_instructions: "When citing NCBI Taxonomy, cite the NCBI resource paper.".to_string(),
        required_citations: vec![
            RequiredCitation {
                citation: CitationData {
                    doi: Some("10.1093/nar/gkac1052".to_string()),
                    pubmed_id: Some("36420893".to_string()),
                    title: "Database resources of the National Center for Biotechnology Information".to_string(),
                    journal: Some("Nucleic Acids Research".to_string()),
                    publication_date: chrono::NaiveDate::from_ymd_opt(2023, 1, 6),
                    volume: Some("51".to_string()),
                    pages: Some("D29-D38".to_string()),
                    authors: "Sayers, E. W., Bolton, E. E., Brister, J. R., et al.".to_string(),
                    bibtex: None,
                },
                requirement_type: CitationRequirementType::Required,
                display_order: 1,
                context: Some("NCBI resources paper".to_string()),
            },
        ],
    }
}

/// Set up citation policy for an organization
pub async fn setup_citation_policy(
    db: &PgPool,
    config: &CitationPolicyConfig,
) -> Result<Uuid> {
    let mut tx = db.begin().await.context("Failed to begin transaction")?;

    let policy_id = setup_citation_policy_tx(&mut tx, config).await?;

    tx.commit().await.context("Failed to commit transaction")?;

    Ok(policy_id)
}

/// Set up citation policy within a transaction
pub async fn setup_citation_policy_tx(
    tx: &mut Transaction<'_, Postgres>,
    config: &CitationPolicyConfig,
) -> Result<Uuid> {
    info!(
        "Setting up citation policy for organization {}",
        config.organization_id
    );

    // 1. Create or update citation policy
    let policy_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO citation_policies (
            organization_id,
            policy_url,
            license_id,
            requires_version_citation,
            requires_accession_citation,
            citation_instructions
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (organization_id)
        DO UPDATE SET
            policy_url = EXCLUDED.policy_url,
            license_id = EXCLUDED.license_id,
            requires_version_citation = EXCLUDED.requires_version_citation,
            requires_accession_citation = EXCLUDED.requires_accession_citation,
            citation_instructions = EXCLUDED.citation_instructions,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(config.organization_id)
    .bind(&config.policy_url)
    .bind(config.license_id)
    .bind(config.requires_version_citation)
    .bind(config.requires_accession_citation)
    .bind(&config.citation_instructions)
    .fetch_one(&mut **tx)
    .await
    .context("Failed to create citation policy")?;

    // 2. Create citations and link them to the policy
    for required_citation in &config.required_citations {
        let citation_id = create_citation_tx(
            tx,
            &required_citation.citation,
            config.organization_id,
        )
        .await?;

        // Link citation to policy
        sqlx::query(
            r#"
            INSERT INTO policy_required_citations (
                policy_id,
                citation_id,
                requirement_type,
                display_order,
                context
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (policy_id, display_order)
            DO UPDATE SET
                citation_id = EXCLUDED.citation_id,
                requirement_type = EXCLUDED.requirement_type,
                context = EXCLUDED.context
            "#,
        )
        .bind(policy_id)
        .bind(citation_id)
        .bind(required_citation.requirement_type.as_str())
        .bind(required_citation.display_order)
        .bind(&required_citation.context)
        .execute(&mut **tx)
        .await
        .context("Failed to link citation to policy")?;
    }

    info!(
        "Citation policy created with {} required citations",
        config.required_citations.len()
    );

    Ok(policy_id)
}

/// Create a citation record
async fn create_citation_tx(
    tx: &mut Transaction<'_, Postgres>,
    citation: &CitationData,
    organization_id: Uuid,
) -> Result<Uuid> {
    // First, we need to find or create a dummy version for the organization
    // Citations are linked to versions, so we need a version_id
    // For organization-level citations, we create a special "policy" version

    let version_id: Uuid = sqlx::query_scalar(
        r#"
        WITH entry AS (
            INSERT INTO registry_entries (
                organization_id,
                slug,
                name,
                entry_type
            )
            VALUES ($1, $2 || '-citations', $2 || ' Citations', 'data_source')
            ON CONFLICT (slug)
            DO UPDATE SET slug = EXCLUDED.slug
            RETURNING id
        )
        INSERT INTO versions (
            entry_id,
            version,
            external_version
        )
        SELECT id, 'policy', 'policy' FROM entry
        ON CONFLICT (entry_id, version)
        DO UPDATE SET version = EXCLUDED.version
        RETURNING id
        "#,
    )
    .bind(organization_id)
    .bind(format!("{}", organization_id))
    .fetch_one(&mut **tx)
    .await
    .context("Failed to create version for citation")?;

    let citation_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO citations (
            version_id,
            citation_type,
            doi,
            pubmed_id,
            title,
            journal,
            publication_date,
            volume,
            pages,
            authors,
            bibtex
        )
        VALUES ($1, 'primary', $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (version_id, doi) WHERE doi IS NOT NULL
        DO UPDATE SET
            title = EXCLUDED.title,
            journal = EXCLUDED.journal,
            publication_date = EXCLUDED.publication_date,
            volume = EXCLUDED.volume,
            pages = EXCLUDED.pages,
            authors = EXCLUDED.authors,
            bibtex = EXCLUDED.bibtex
        RETURNING id
        "#,
    )
    .bind(version_id)
    .bind(&citation.doi)
    .bind(&citation.pubmed_id)
    .bind(&citation.title)
    .bind(&citation.journal)
    .bind(citation.publication_date)
    .bind(&citation.volume)
    .bind(&citation.pages)
    .bind(&citation.authors)
    .bind(&citation.bibtex)
    .fetch_one(&mut **tx)
    .await
    .context("Failed to create citation")?;

    Ok(citation_id)
}

/// Add version-specific citation (e.g., GO zenodo DOI)
pub async fn add_version_citation(
    db: &PgPool,
    version_id: Uuid,
    citation: &CitationData,
) -> Result<Uuid> {
    let citation_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO citations (
            version_id,
            citation_type,
            doi,
            pubmed_id,
            title,
            journal,
            publication_date,
            volume,
            pages,
            authors,
            bibtex
        )
        VALUES ($1, 'version', $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (version_id, doi) WHERE doi IS NOT NULL
        DO UPDATE SET
            title = EXCLUDED.title
        RETURNING id
        "#,
    )
    .bind(version_id)
    .bind(&citation.doi)
    .bind(&citation.pubmed_id)
    .bind(&citation.title)
    .bind(&citation.journal)
    .bind(citation.publication_date)
    .bind(&citation.volume)
    .bind(&citation.pages)
    .bind(&citation.authors)
    .bind(&citation.bibtex)
    .fetch_one(db)
    .await
    .context("Failed to add version citation")?;

    Ok(citation_id)
}
