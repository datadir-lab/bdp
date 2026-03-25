use uuid::Uuid;

pub const PUBMED_FTP_BASE: &str = "https://ftp.ncbi.nlm.nih.gov/pubmed/baseline/";

#[derive(Debug, Clone)]
pub struct PubmedConfig {
    pub ftp_base: String,
    pub open_access_only: bool,
    pub worker_count: usize,
    pub batch_size: usize,
    pub max_retries: u32,
    pub parse_limit: Option<usize>,
    pub org_id: Uuid,
}

impl PubmedConfig {
    pub fn new(org_id: Uuid) -> Self {
        Self {
            ftp_base: PUBMED_FTP_BASE.to_string(),
            open_access_only: true,
            worker_count: 4,
            batch_size: 1000,
            max_retries: 3,
            parse_limit: None,
            org_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = PubmedConfig::new(org_id);
        assert!(cfg.open_access_only);
        assert_eq!(cfg.worker_count, 4);
        assert!(cfg.parse_limit.is_none());
    }
}
