use chrono::NaiveDate;
use std::path::PathBuf;
use uuid::Uuid;

pub const AACT_BASE_URL: &str = "https://aact.ctti-clinicaltrials.org";
pub const CT_API_BASE: &str = "https://clinicaltrials.gov/api/v2";

#[derive(Debug, Clone)]
pub struct ClinicalTrialsConfig {
    pub aact_dump_path: Option<PathBuf>,
    pub from_date: Option<NaiveDate>,
    pub api_page_size: usize,
    pub max_retries: u32,
    pub org_id: Uuid,
}

impl ClinicalTrialsConfig {
    pub fn new(org_id: Uuid) -> Self {
        Self {
            aact_dump_path: None,
            from_date: None,
            api_page_size: 100,
            max_retries: 3,
            org_id,
        }
    }

    pub fn with_dump(mut self, path: PathBuf) -> Self {
        self.aact_dump_path = Some(path);
        self
    }

    pub fn with_from_date(mut self, date: NaiveDate) -> Self {
        self.from_date = Some(date);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = ClinicalTrialsConfig::new(org_id);
        assert!(cfg.aact_dump_path.is_none());
        assert_eq!(cfg.api_page_size, 100);
    }
}
