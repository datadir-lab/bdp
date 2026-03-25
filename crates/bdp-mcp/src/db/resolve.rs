use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Canonical ID type detected from the input string.
#[derive(Debug)]
pub enum CanonicalId<'a> {
    Mondo(&'a str),
    Hpo(&'a str),
    Chebi(&'a str),
    Reactome(&'a str),
    UniProt(&'a str),
}

/// Clamp input to 500 chars to prevent FTS query amplification.
pub fn cap_input(input: &str) -> &str {
    let end = input
        .char_indices()
        .nth(500)
        .map(|(i, _)| i)
        .unwrap_or(input.len());
    &input[..end]
}

/// Detect if the input string is a recognized canonical ID pattern.
pub fn detect_id_type(input: &str) -> Option<CanonicalId<'_>> {
    // MONDO:0000000
    if input.starts_with("MONDO:") && input[6..].chars().all(|c| c.is_ascii_digit()) {
        return Some(CanonicalId::Mondo(input));
    }
    // HP:0000000
    if input.starts_with("HP:") && input[3..].chars().all(|c| c.is_ascii_digit()) {
        return Some(CanonicalId::Hpo(input));
    }
    // CHEBI:00000
    if input.starts_with("CHEBI:") && input[6..].chars().all(|c| c.is_ascii_digit()) {
        return Some(CanonicalId::Chebi(input));
    }
    // R-HSA-000000 or R-MMU-000000 etc.
    if input.starts_with("R-") && input.contains('-') {
        return Some(CanonicalId::Reactome(input));
    }
    // UniProt accession: [A-Z][0-9][A-Z0-9]{3}[0-9] (6 chars) or 10 chars
    let bytes = input.as_bytes();
    if (bytes.len() == 6 || bytes.len() == 10)
        && bytes[0].is_ascii_uppercase()
        && bytes[1].is_ascii_digit()
    {
        return Some(CanonicalId::UniProt(input));
    }
    None
}

/// Fuzzy resolution result for FTS name searches.
#[derive(Debug)]
pub struct FtsMatch {
    pub id: Uuid,
    pub name: String,
}

/// Find a disease by MONDO canonical ID.
pub async fn disease_by_mondo_id(pool: &PgPool, mondo_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM disease_terms WHERE mondo_id = $1 AND is_obsolete = FALSE")
        .bind(mondo_id)
        .fetch_optional(pool)
        .await
}

/// Find up to 5 diseases by FTS name match.
/// Uses sqlx::query() runtime — NOT sqlx::query!() macro.
pub async fn diseases_by_name(pool: &PgPool, name: &str) -> sqlx::Result<Vec<FtsMatch>> {
    let name = cap_input(name);
    let rows = sqlx::query(
        "SELECT id, name FROM disease_terms,
         plainto_tsquery('english', $1) q
         WHERE to_tsvector('english', name) @@ q AND is_obsolete = FALSE
         ORDER BY ts_rank(to_tsvector('english', name), q) DESC LIMIT 5",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| FtsMatch {
            id: r.get("id"),
            name: r.get("name"),
        })
        .collect())
}

/// Find HPO term by HP: canonical ID.
pub async fn phenotype_by_hpo_id(pool: &PgPool, hpo_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM hpo_term_metadata WHERE hpo_id = $1 AND is_obsolete = FALSE")
        .bind(hpo_id)
        .fetch_optional(pool)
        .await
}

/// Find compound by CHEBI: canonical ID.
pub async fn compound_by_chebi_id(pool: &PgPool, chebi_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM compound_terms WHERE chebi_id = $1 AND is_obsolete = FALSE")
        .bind(chebi_id)
        .fetch_optional(pool)
        .await
}

/// Find pathway by R-HSA-... canonical ID.
pub async fn pathway_by_reactome_id(
    pool: &PgPool,
    reactome_id: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM pathway_terms WHERE reactome_id = $1")
        .bind(reactome_id)
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mondo_id() {
        assert!(matches!(detect_id_type("MONDO:0004975"), Some(CanonicalId::Mondo(_))));
    }

    #[test]
    fn test_detect_hpo_id() {
        assert!(matches!(detect_id_type("HP:0001234"), Some(CanonicalId::Hpo(_))));
    }

    #[test]
    fn test_detect_chebi_id() {
        assert!(matches!(detect_id_type("CHEBI:15422"), Some(CanonicalId::Chebi(_))));
    }

    #[test]
    fn test_detect_reactome_id() {
        assert!(matches!(detect_id_type("R-HSA-109581"), Some(CanonicalId::Reactome(_))));
    }

    #[test]
    fn test_uniprot_accession() {
        assert!(matches!(detect_id_type("P38398"), Some(CanonicalId::UniProt(_))));
    }

    #[test]
    fn test_free_text_returns_none() {
        assert!(detect_id_type("Alzheimer disease").is_none());
        assert!(detect_id_type("BRCA1").is_none());
    }

    #[test]
    fn test_input_length_cap() {
        let long = "a".repeat(501);
        assert!(cap_input(&long).len() <= 500);
    }
}
