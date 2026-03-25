use std::path::PathBuf;
use uuid::Uuid;

pub const CHEMBL_FTP_BASE: &str =
    "https://ftp.ebi.ac.uk/pub/databases/chembl/ChEMBLdb/releases/chembl_36";

#[derive(Debug, Clone)]
pub struct ChemblConfig {
    pub sqlite_path: PathBuf,
    pub uniprot_mapping_path: Option<PathBuf>,
    pub source_version: String,
    pub batch_size: usize,
    pub org_id: Uuid,
}

impl ChemblConfig {
    pub fn new(sqlite_path: PathBuf, org_id: Uuid) -> Self {
        Self {
            sqlite_path,
            uniprot_mapping_path: None,
            source_version: "chembl_36".to_string(),
            batch_size: 500,
            org_id,
        }
    }
}
