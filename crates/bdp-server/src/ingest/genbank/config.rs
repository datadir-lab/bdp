// GenBank/RefSeq FTP configuration

use super::models::{Division, SourceDatabase};

/// GenBank FTP configuration
#[derive(Debug, Clone)]
pub struct GenbankFtpConfig {
    /// FTP host
    pub host: String,

    /// FTP port
    pub port: u16,

    /// Base path for GenBank files
    pub genbank_path: String,

    /// Base path for RefSeq files
    pub refseq_path: String,

    /// Source database type
    pub source_database: SourceDatabase,

    /// Parse limit for testing (None = parse all)
    pub parse_limit: Option<usize>,

    /// Download timeout in seconds
    pub timeout_seconds: u64,

    /// Number of retries for failed downloads
    pub max_retries: u32,

    /// Batch size for database operations
    pub batch_size: usize,

    /// Concurrency for parallel processing
    pub concurrency: usize,
}

impl Default for GenbankFtpConfig {
    fn default() -> Self {
        Self {
            host: "ftp.ncbi.nlm.nih.gov".to_string(),
            port: 21,
            genbank_path: "/genbank".to_string(),
            refseq_path: "/refseq/release".to_string(),
            source_database: SourceDatabase::Genbank,
            parse_limit: None,
            timeout_seconds: 300,
            max_retries: 3,
            batch_size: 500,
            concurrency: 4,
        }
    }
}

impl GenbankFtpConfig {
    /// Create new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set to use GenBank database
    pub fn with_genbank(mut self) -> Self {
        self.source_database = SourceDatabase::Genbank;
        self
    }

    /// Set to use RefSeq database
    pub fn with_refseq(mut self) -> Self {
        self.source_database = SourceDatabase::Refseq;
        self
    }

    /// Set parse limit for testing
    pub fn with_parse_limit(mut self, limit: usize) -> Self {
        self.parse_limit = Some(limit);
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set concurrency level
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Get base path for current source database
    pub fn get_base_path(&self) -> &str {
        match self.source_database {
            SourceDatabase::Genbank => &self.genbank_path,
            SourceDatabase::Refseq => &self.refseq_path,
        }
    }

    /// Get release number file path
    pub fn get_release_number_path(&self) -> String {
        match self.source_database {
            SourceDatabase::Genbank => format!("{}/GB_Release_Number", self.genbank_path),
            SourceDatabase::Refseq => format!("{}/RELEASE_NUMBER", self.refseq_path),
        }
    }

    /// Get file pattern for division
    pub fn get_division_file_pattern(&self, division: &Division) -> String {
        format!("{}*.seq.gz", division.file_prefix())
    }

    /// List all available divisions for GenBank
    pub fn get_all_divisions() -> Vec<Division> {
        vec![
            Division::Phage,        // Smallest - good for testing
            Division::Viral,
            Division::Bacterial,
            Division::Plant,
            Division::Mammalian,
            Division::Primate,
            Division::Rodent,
            Division::Vertebrate,
            Division::Invertebrate,
            Division::Synthetic,
            Division::Unannotated,
            Division::Environmental,
            Division::Patent,
            Division::Est,
            Division::Sts,
            Division::Gss,
            Division::Htg,
            Division::Con,
        ]
    }

    /// Get primary divisions (most commonly used)
    pub fn get_primary_divisions() -> Vec<Division> {
        vec![
            Division::Phage,
            Division::Viral,
            Division::Bacterial,
            Division::Plant,
            Division::Mammalian,
            Division::Primate,
            Division::Rodent,
            Division::Vertebrate,
            Division::Invertebrate,
        ]
    }

    /// Get test division (smallest for quick testing)
    pub fn get_test_division() -> Division {
        Division::Phage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GenbankFtpConfig::default();
        assert_eq!(config.host, "ftp.ncbi.nlm.nih.gov");
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.concurrency, 4);
    }

    #[test]
    fn test_genbank_paths() {
        let config = GenbankFtpConfig::new().with_genbank();
        assert_eq!(config.get_base_path(), "/genbank");
        assert_eq!(config.get_release_number_path(), "/genbank/GB_Release_Number");
    }

    #[test]
    fn test_refseq_paths() {
        let config = GenbankFtpConfig::new().with_refseq();
        assert_eq!(config.get_base_path(), "/refseq/release");
        assert_eq!(config.get_release_number_path(), "/refseq/release/RELEASE_NUMBER");
    }

    #[test]
    fn test_division_file_pattern() {
        let config = GenbankFtpConfig::new();
        assert_eq!(config.get_division_file_pattern(&Division::Viral), "gbvrl*.seq.gz");
        assert_eq!(config.get_division_file_pattern(&Division::Bacterial), "gbbct*.seq.gz");
        assert_eq!(config.get_division_file_pattern(&Division::Phage), "gbphg*.seq.gz");
    }

    #[test]
    fn test_builder_pattern() {
        let config = GenbankFtpConfig::new()
            .with_parse_limit(1000)
            .with_batch_size(100)
            .with_concurrency(8)
            .with_timeout(600);

        assert_eq!(config.parse_limit, Some(1000));
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.concurrency, 8);
        assert_eq!(config.timeout_seconds, 600);
    }
}
