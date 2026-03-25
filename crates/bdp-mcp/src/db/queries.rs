// crates/bdp-mcp/src/db/queries.rs
//
// Runtime DB queries for MCP tools — disease, phenotype, gene, pathway, compound.
// Uses sqlx::query() (NOT sqlx::query!() macros) to avoid offline-cache dependency.

use sqlx::{PgPool, Row};
use uuid::Uuid;

// ─── Disease ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DiseaseRow {
    pub id: Uuid,
    pub mondo_id: String,
    pub name: String,
    pub definition: Option<String>,
    pub omim_id: Option<String>,
    pub orphanet_id: Option<String>,
    pub mondo_release: String,
}

#[derive(Debug)]
pub struct DiseaseSynonymRow {
    pub scope: String,
    pub text: String,
}

#[derive(Debug)]
pub struct DiseaseXrefRow {
    pub source_db: String,
    pub source_id: String,
}

/// Fetch disease by internal UUID (for FTS resolve path).
pub async fn get_disease_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<DiseaseRow>> {
    let row = sqlx::query(
        "SELECT id, mondo_id, name, definition, omim_id, orphanet_id, mondo_release
         FROM disease_terms WHERE id = $1 AND is_obsolete = FALSE",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| DiseaseRow {
        id: r.get("id"),
        mondo_id: r.get("mondo_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        omim_id: r.get("omim_id"),
        orphanet_id: r.get("orphanet_id"),
        mondo_release: r.get("mondo_release"),
    }))
}

/// Fetch a disease term by MONDO ID string (e.g. "MONDO:0004975").
pub async fn get_disease(pool: &PgPool, mondo_id: &str) -> sqlx::Result<Option<DiseaseRow>> {
    let row = sqlx::query(
        "SELECT id, mondo_id, name, definition, omim_id, orphanet_id, mondo_release
         FROM disease_terms
         WHERE mondo_id = $1 AND is_obsolete = FALSE",
    )
    .bind(mondo_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| DiseaseRow {
        id: r.get("id"),
        mondo_id: r.get("mondo_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        omim_id: r.get("omim_id"),
        orphanet_id: r.get("orphanet_id"),
        mondo_release: r.get("mondo_release"),
    }))
}

pub async fn get_disease_synonyms(
    pool: &PgPool,
    disease_id: Uuid,
) -> sqlx::Result<Vec<DiseaseSynonymRow>> {
    let rows = sqlx::query("SELECT scope, text FROM disease_term_synonyms WHERE term_id = $1")
        .bind(disease_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .iter()
        .map(|r| DiseaseSynonymRow {
            scope: r.get("scope"),
            text: r.get("text"),
        })
        .collect())
}

pub async fn get_disease_xrefs(
    pool: &PgPool,
    disease_id: Uuid,
) -> sqlx::Result<Vec<DiseaseXrefRow>> {
    let rows =
        sqlx::query("SELECT source_db, source_id FROM disease_term_xrefs WHERE term_id = $1")
            .bind(disease_id)
            .fetch_all(pool)
            .await?;

    Ok(rows
        .iter()
        .map(|r| DiseaseXrefRow {
            source_db: r.get("source_db"),
            source_id: r.get("source_id"),
        })
        .collect())
}

#[derive(Debug)]
pub struct DiseasePhenotypeRow {
    pub hpo_id: String,
    pub hpo_name: String,
    pub frequency: Option<String>,
    pub onset: Option<String>,
    pub evidence: Option<String>,
    pub reference: Option<String>,
}

/// Fetch phenotype annotations for a disease.
/// Bridges through disease_terms.omim_id / disease_terms.orphanet_id to disease_phenotype_annotations.
pub async fn get_disease_phenotypes(
    pool: &PgPool,
    mondo_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<DiseasePhenotypeRow>> {
    let rows = sqlx::query(
        r#"
        SELECT dpa.hpo_id, h.name AS hpo_name,
               dpa.frequency, dpa.onset, dpa.evidence, dpa.reference
        FROM disease_terms dt
        JOIN disease_phenotype_annotations dpa ON (
            (dt.omim_id IS NOT NULL     AND dpa.disease_db = 'OMIM'  AND dpa.disease_id = dt.omim_id)
            OR
            (dt.orphanet_id IS NOT NULL AND dpa.disease_db = 'ORPHA' AND dpa.disease_id = dt.orphanet_id)
        )
        JOIN hpo_term_metadata h ON h.hpo_id = dpa.hpo_id
        WHERE dt.mondo_id = $1
        ORDER BY dpa.hpo_id
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(mondo_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| DiseasePhenotypeRow {
            hpo_id: r.get("hpo_id"),
            hpo_name: r.get("hpo_name"),
            frequency: r.get("frequency"),
            onset: r.get("onset"),
            evidence: r.get("evidence"),
            reference: r.get("reference"),
        })
        .collect())
}

// ─── Phenotype ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PhenotypeRow {
    pub id: Uuid,
    pub hpo_id: String,
    pub name: String,
    pub definition: Option<String>,
    pub synonyms_json: Option<serde_json::Value>,
    pub alt_ids_json: Option<serde_json::Value>,
}

pub async fn get_phenotype(pool: &PgPool, hpo_id: &str) -> sqlx::Result<Option<PhenotypeRow>> {
    let row = sqlx::query(
        "SELECT id, hpo_id, name, definition, synonyms, alt_ids
         FROM hpo_term_metadata
         WHERE hpo_id = $1 AND is_obsolete = FALSE",
    )
    .bind(hpo_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PhenotypeRow {
        id: r.get("id"),
        hpo_id: r.get("hpo_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        synonyms_json: r.get("synonyms"),
        alt_ids_json: r.get("alt_ids"),
    }))
}

#[derive(Debug)]
pub struct PhenotypesDiseaseRow {
    pub mondo_id: String,
    pub name: String,
    pub definition: Option<String>,
}

/// Reverse bridge: find diseases annotated with a given HPO term.
pub async fn get_phenotype_diseases(
    pool: &PgPool,
    hpo_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<PhenotypesDiseaseRow>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT dt.mondo_id, dt.name, dt.definition
        FROM disease_phenotype_annotations dpa
        JOIN disease_terms dt ON (
            (dpa.disease_db = 'OMIM'  AND dt.omim_id     = dpa.disease_id)
            OR
            (dpa.disease_db = 'ORPHA' AND dt.orphanet_id = dpa.disease_id)
        )
        WHERE dpa.hpo_id = $1
          AND dt.is_obsolete = FALSE
        ORDER BY dt.name
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(hpo_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| PhenotypesDiseaseRow {
            mondo_id: r.get("mondo_id"),
            name: r.get("name"),
            definition: r.get("definition"),
        })
        .collect())
}

// ─── Gene ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GeneRow {
    pub uniprot_acc: String,
    pub entry_name: Option<String>,
    pub gene_name: Option<String>,
    pub organism: Option<String>,
    pub ncbi_taxon_id: Option<i64>,
    pub sequence_length: Option<i32>,
}

/// Fetch gene by UniProt accession.
pub async fn get_gene_by_uniprot(pool: &PgPool, accession: &str) -> sqlx::Result<Option<GeneRow>> {
    let row = sqlx::query(
        r#"
        SELECT pm.accession AS uniprot_acc, pm.entry_name, pm.gene_name,
               tm.scientific_name AS organism, tm.taxonomy_id AS ncbi_taxon_id,
               pm.sequence_length
        FROM protein_metadata pm
        LEFT JOIN taxonomy_metadata tm ON tm.data_source_id = pm.taxonomy_id
        WHERE pm.accession = $1
        "#,
    )
    .bind(accession)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| GeneRow {
        uniprot_acc: r.get("uniprot_acc"),
        entry_name: r.get("entry_name"),
        gene_name: r.get("gene_name"),
        organism: r.get("organism"),
        ncbi_taxon_id: r.get("ncbi_taxon_id"),
        sequence_length: r.get("sequence_length"),
    }))
}

#[derive(Debug)]
pub struct GenePathwayRow {
    pub reactome_id: String,
    pub name: String,
    pub species_name: String,
    pub is_top_level: bool,
}

pub async fn get_gene_pathways(
    pool: &PgPool,
    uniprot_acc: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<GenePathwayRow>> {
    let rows = sqlx::query(
        r#"
        SELECT pt.reactome_id, pt.name, pt.species_name, pt.is_top_level
        FROM protein_pathway_associations ppa
        JOIN pathway_terms pt ON pt.id = ppa.pathway_id
        WHERE ppa.uniprot_acc = $1
        ORDER BY pt.name
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(uniprot_acc)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| GenePathwayRow {
            reactome_id: r.get("reactome_id"),
            name: r.get("name"),
            species_name: r.get("species_name"),
            is_top_level: r.get("is_top_level"),
        })
        .collect())
}

// ─── Pathway ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PathwayRow {
    pub reactome_id: String,
    pub name: String,
    pub species_name: String,
    pub is_top_level: bool,
    pub reactome_release: String,
}

pub async fn get_pathway(pool: &PgPool, reactome_id: &str) -> sqlx::Result<Option<PathwayRow>> {
    let row = sqlx::query(
        "SELECT reactome_id, name, species_name, is_top_level, reactome_release
         FROM pathway_terms WHERE reactome_id = $1",
    )
    .bind(reactome_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PathwayRow {
        reactome_id: r.get("reactome_id"),
        name: r.get("name"),
        species_name: r.get("species_name"),
        is_top_level: r.get("is_top_level"),
        reactome_release: r.get("reactome_release"),
    }))
}

#[derive(Debug)]
pub struct PathwayProteinRow {
    pub uniprot_acc: String,
    pub evidence_type: Option<String>,
}

pub async fn get_pathway_proteins(
    pool: &PgPool,
    reactome_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<PathwayProteinRow>> {
    let rows = sqlx::query(
        r#"
        SELECT ppa.uniprot_acc, ppa.evidence_type
        FROM protein_pathway_associations ppa
        JOIN pathway_terms pt ON pt.id = ppa.pathway_id
        WHERE pt.reactome_id = $1
        ORDER BY ppa.uniprot_acc
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(reactome_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| PathwayProteinRow {
            uniprot_acc: r.get("uniprot_acc"),
            evidence_type: r.get("evidence_type"),
        })
        .collect())
}

// ─── Compound ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CompoundRow {
    pub chebi_id: String,
    pub name: String,
    pub definition: Option<String>,
    pub formula: Option<String>,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub mass_mono: Option<f64>,
    pub charge: Option<i32>,
}

pub async fn get_compound(pool: &PgPool, chebi_id: &str) -> sqlx::Result<Option<CompoundRow>> {
    let row = sqlx::query(
        "SELECT chebi_id, name, definition, formula, inchikey, smiles, mass_mono, charge
         FROM compound_terms WHERE chebi_id = $1 AND is_obsolete = FALSE",
    )
    .bind(chebi_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| CompoundRow {
        chebi_id: r.get("chebi_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        formula: r.get("formula"),
        inchikey: r.get("inchikey"),
        smiles: r.get("smiles"),
        mass_mono: r.get("mass_mono"),
        charge: r.get("charge"),
    }))
}

#[derive(Debug)]
pub struct CompoundRoleRow {
    pub chebi_id: String,
    pub name: String,
    pub relationship_type: String,
}

pub async fn get_compound_roles(
    pool: &PgPool,
    chebi_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<CompoundRoleRow>> {
    let rows = sqlx::query(
        r#"
        SELECT cr.object_chebi_id AS chebi_id, ct.name, cr.relationship_type
        FROM compound_relationships cr
        JOIN compound_terms ct ON ct.chebi_id = cr.object_chebi_id
        WHERE cr.subject_chebi_id = $1
          AND cr.relationship_type = 'has_role'
          AND ct.is_obsolete = FALSE
        ORDER BY ct.name
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(chebi_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| CompoundRoleRow {
            chebi_id: r.get("chebi_id"),
            name: r.get("name"),
            relationship_type: r.get("relationship_type"),
        })
        .collect())
}
