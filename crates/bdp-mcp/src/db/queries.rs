// crates/bdp-mcp/src/db/queries.rs
//
// Runtime DB queries for MCP tools — disease, phenotype, gene, pathway, compound.
// Uses sqlx::query() (NOT sqlx::query!() macros) to avoid offline-cache dependency.

use chrono::Datelike;
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

/// Fetch phenotype by internal UUID (for FTS resolve path).
pub async fn get_phenotype_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<PhenotypeRow>> {
    let row = sqlx::query(
        "SELECT id, hpo_id, name, definition, synonyms, alt_ids
         FROM hpo_term_metadata WHERE id = $1 AND is_obsolete = FALSE",
    )
    .bind(id)
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

/// Fetch a pathway by internal UUID (for FTS resolve path).
pub async fn get_pathway_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<PathwayRow>> {
    let row = sqlx::query(
        "SELECT reactome_id, name, species_name, is_top_level, reactome_release
         FROM pathway_terms WHERE id = $1",
    )
    .bind(id)
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

// ─── Gene–Disease Associations ────────────────────────────────────────────────

pub async fn get_gene_diseases(
    pool: &PgPool,
    gene_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT dt.mondo_id, dt.name, gda.score, gda.source_version
           FROM gene_disease_associations gda
           JOIN disease_terms dt ON dt.id = gda.disease_term_id
           WHERE gda.gene_id = $1
           ORDER BY gda.score DESC NULLS LAST
           LIMIT $2 OFFSET $3"#,
    )
    .bind(gene_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "mondo_id": r.try_get::<String, _>("mondo_id").unwrap_or_default(),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "score": r.try_get::<Option<f32>, _>("score").unwrap_or(None),
                "source_version": r.try_get::<Option<String>, _>("source_version").unwrap_or(None),
            })
        })
        .collect())
}

pub async fn get_disease_trials(
    pool: &PgPool,
    disease_term_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT ct.nct_id, ct.title, ct.status, ct.phase
           FROM trial_disease_links tdl
           JOIN clinical_trials ct ON ct.id = tdl.trial_id
           WHERE tdl.disease_term_id = $1
           LIMIT $2 OFFSET $3"#,
    )
    .bind(disease_term_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "nct_id": r.try_get::<String, _>("nct_id").unwrap_or_default(),
                "title": r.try_get::<Option<String>, _>("title").unwrap_or(None),
                "status": r.try_get::<Option<String>, _>("status").unwrap_or(None),
                "phase": r.try_get::<Option<String>, _>("phase").unwrap_or(None),
            })
        })
        .collect())
}

/// Resolve a ChEBI ID string to the compound_terms internal UUID.
pub async fn compound_uuid_by_chebi_id(
    pool: &PgPool,
    chebi_id: &str,
) -> sqlx::Result<Option<Uuid>> {
    let row =
        sqlx::query("SELECT id FROM compound_terms WHERE chebi_id = $1 AND is_obsolete = FALSE")
            .bind(chebi_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.get("id")))
}

pub async fn get_compound_targets(
    pool: &PgPool,
    compound_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT ds.external_id AS uniprot_acc, dta.activity_type, dta.activity_value, dta.activity_unit
           FROM drug_target_activities dta
           JOIN data_sources ds ON ds.id = dta.target_gene_id
           WHERE dta.compound_id = $1
           LIMIT $2 OFFSET $3"#,
    )
    .bind(compound_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "uniprot_acc": r.try_get::<Option<String>, _>("uniprot_acc").unwrap_or(None),
                "activity_type": r.try_get::<Option<String>, _>("activity_type").unwrap_or(None),
                "activity_value": r.try_get::<Option<f64>, _>("activity_value").unwrap_or(None),
                "activity_unit": r.try_get::<Option<String>, _>("activity_unit").unwrap_or(None),
            })
        })
        .collect())
}

pub async fn search_literature(
    pool: &PgPool,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT pmid, title, journal, pub_date
           FROM publications
           WHERE to_tsvector('english', COALESCE(title,'') || ' ' || COALESCE("abstract",'')) @@ plainto_tsquery('english', $1)
           ORDER BY pub_date DESC NULLS LAST
           LIMIT $2 OFFSET $3"#,
    )
    .bind(query)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "pmid": r.try_get::<i32, _>("pmid").unwrap_or(0),
                "title": r.try_get::<String, _>("title").unwrap_or_default(),
                "journal": r.try_get::<Option<String>, _>("journal").unwrap_or(None),
                "pub_year": r.try_get::<Option<chrono::NaiveDate>, _>("pub_date").ok().flatten().map(|d| d.year()),
            })
        })
        .collect())
}

pub async fn get_publication(
    pool: &PgPool,
    pmid: i32,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT pmid, title, "abstract", journal, pub_date FROM publications WHERE pmid = $1"#,
    )
    .bind(pmid)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        serde_json::json!({
            "pmid": r.try_get::<i32, _>("pmid").unwrap_or(0),
            "title": r.try_get::<String, _>("title").unwrap_or_default(),
            "abstract": r.try_get::<Option<String>, _>("abstract").unwrap_or(None),
            "journal": r.try_get::<Option<String>, _>("journal").unwrap_or(None),
            "pub_year": r.try_get::<Option<chrono::NaiveDate>, _>("pub_date").ok().flatten().map(|d| d.year()),
        })
    }))
}

pub async fn get_gene_interactions(
    pool: &PgPool,
    gene_uuid: Uuid,
    min_score: i16,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT
              CASE WHEN pi.protein_a_id = $1 THEN ds_b.external_id ELSE ds_a.external_id END AS partner,
              pi.combined_score, pi.score_experimental
           FROM protein_interactions pi
           JOIN data_sources ds_a ON ds_a.id = pi.protein_a_id
           JOIN data_sources ds_b ON ds_b.id = pi.protein_b_id
           WHERE (pi.protein_a_id = $1 OR pi.protein_b_id = $1)
             AND pi.combined_score >= $2
           ORDER BY pi.combined_score DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(gene_uuid)
    .bind(min_score)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "partner_uniprot": r.try_get::<Option<String>, _>("partner").unwrap_or(None),
                "combined_score": r.try_get::<i16, _>("combined_score").unwrap_or(0),
                "experimental_score": r.try_get::<Option<i16>, _>("score_experimental").unwrap_or(None),
            })
        })
        .collect())
}

/// Resolve a UniProt accession to the data_sources UUID.
pub async fn resolve_gene_uuid(
    pool: &PgPool,
    uniprot_acc: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id FROM data_sources WHERE external_id = $1 AND source_type = 'uniprot'",
    )
    .bind(uniprot_acc)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.get("id")))
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

/// Fetch compound by internal UUID (for FTS resolve path).
pub async fn get_compound_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<CompoundRow>> {
    let row = sqlx::query(
        "SELECT chebi_id, name, definition, formula, inchikey, smiles, mass_mono, charge
         FROM compound_terms WHERE id = $1 AND is_obsolete = FALSE",
    )
    .bind(id)
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
