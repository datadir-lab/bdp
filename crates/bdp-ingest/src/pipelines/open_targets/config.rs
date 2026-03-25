use uuid::Uuid;

pub const OPEN_TARGETS_BASE: &str = "https://ftp.ebi.ac.uk/pub/databases/opentargets/platform";

#[derive(Debug, Clone)]
pub struct OpenTargetsConfig {
    pub release: String,
    pub base_url: String,
    pub max_retries: u32,
    pub parse_limit: Option<usize>,
    pub min_score: f32,
    pub org_id: Uuid,
}

impl OpenTargetsConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        let release = release.into();
        Self {
            base_url: format!("{}/{}/output", OPEN_TARGETS_BASE, release),
            release,
            max_retries: 3,
            parse_limit: None,
            min_score: 0.0,
            org_id,
        }
    }

    pub fn associations_url(&self) -> String {
        format!("{}/association_overall_direct/", self.base_url)
    }

    pub fn targets_url(&self) -> String {
        format!("{}/targets/", self.base_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = OpenTargetsConfig::new("25.03", org_id);
        assert_eq!(cfg.release, "25.03");
        assert_eq!(cfg.max_retries, 3);
        assert!(cfg.parse_limit.is_none());
        assert!(cfg.associations_url().contains("25.03"));
    }
}
