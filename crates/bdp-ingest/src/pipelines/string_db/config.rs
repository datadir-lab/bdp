// crates/bdp-ingest/src/pipelines/string_db/config.rs

use uuid::Uuid;

pub const STRING_LINKS_URL: &str =
    "https://stringdb-downloads.org/download/protein.links.detailed.v12.0/9606.protein.links.detailed.v12.0.txt.gz";
pub const STRING_ALIASES_URL: &str =
    "https://stringdb-downloads.org/download/protein.aliases.v12.0/9606.protein.aliases.v12.0.txt.gz";

#[derive(Debug, Clone)]
pub struct StringConfig {
    pub species_id: u32,
    pub min_combined_score: u16,
    pub links_url: String,
    pub aliases_url: String,
    pub max_retries: u32,
    pub org_id: Uuid,
}

impl StringConfig {
    pub fn new(org_id: Uuid) -> Self {
        Self {
            species_id: 9606,
            min_combined_score: 400,
            links_url: STRING_LINKS_URL.to_string(),
            aliases_url: STRING_ALIASES_URL.to_string(),
            max_retries: 3,
            org_id,
        }
    }

    pub fn with_params(species_id: u32, min_combined_score: u16, org_id: Uuid) -> Self {
        Self {
            species_id,
            min_combined_score,
            links_url: STRING_LINKS_URL.to_string(),
            aliases_url: STRING_ALIASES_URL.to_string(),
            max_retries: 3,
            org_id,
        }
    }
}
