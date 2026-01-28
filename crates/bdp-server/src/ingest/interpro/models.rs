// InterPro Data Models
//
// Rust structs representing InterPro entries, signatures, and protein matches.
// These models map directly to the database schema in migration 20260128000001.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Enums
// ============================================================================

/// InterPro entry type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum EntryType {
    #[serde(rename = "Family")]
    Family,
    #[serde(rename = "Domain")]
    Domain,
    #[serde(rename = "Repeat")]
    Repeat,
    #[serde(rename = "Site")]
    Site,
    #[serde(rename = "Homologous_superfamily")]
    HomologousSuperfamily,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::Family => write!(f, "Family"),
            EntryType::Domain => write!(f, "Domain"),
            EntryType::Repeat => write!(f, "Repeat"),
            EntryType::Site => write!(f, "Site"),
            EntryType::HomologousSuperfamily => write!(f, "Homologous_superfamily"),
        }
    }
}

impl std::str::FromStr for EntryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Family" => Ok(EntryType::Family),
            "Domain" => Ok(EntryType::Domain),
            "Repeat" => Ok(EntryType::Repeat),
            "Site" => Ok(EntryType::Site),
            "Homologous_superfamily" => Ok(EntryType::HomologousSuperfamily),
            _ => Err(format!("Invalid entry type: {}", s)),
        }
    }
}

/// Member database for protein signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureDatabase {
    Pfam,
    Smart,
    Prosite,
    Prints,
    Panther,
    ProDom,
    Hamap,
    Cdd,
    Pirsf,
    Sfld,
    Superfamily,
    TigrFams,
    Gene3D,
    Other,
}

impl std::fmt::Display for SignatureDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureDatabase::Pfam => write!(f, "Pfam"),
            SignatureDatabase::Smart => write!(f, "SMART"),
            SignatureDatabase::Prosite => write!(f, "PROSITE"),
            SignatureDatabase::Prints => write!(f, "PRINTS"),
            SignatureDatabase::Panther => write!(f, "PANTHER"),
            SignatureDatabase::ProDom => write!(f, "ProDom"),
            SignatureDatabase::Hamap => write!(f, "HAMAP"),
            SignatureDatabase::Cdd => write!(f, "CDD"),
            SignatureDatabase::Pirsf => write!(f, "PIRSF"),
            SignatureDatabase::Sfld => write!(f, "SFLD"),
            SignatureDatabase::Superfamily => write!(f, "SUPERFAMILY"),
            SignatureDatabase::TigrFams => write!(f, "TIGRFAMs"),
            SignatureDatabase::Gene3D => write!(f, "Gene3D"),
            SignatureDatabase::Other => write!(f, "Other"),
        }
    }
}

impl std::str::FromStr for SignatureDatabase {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PFAM" => Ok(SignatureDatabase::Pfam),
            "SMART" => Ok(SignatureDatabase::Smart),
            "PROSITE" => Ok(SignatureDatabase::Prosite),
            "PRINTS" => Ok(SignatureDatabase::Prints),
            "PANTHER" => Ok(SignatureDatabase::Panther),
            "PRODOM" => Ok(SignatureDatabase::ProDom),
            "HAMAP" => Ok(SignatureDatabase::Hamap),
            "CDD" => Ok(SignatureDatabase::Cdd),
            "PIRSF" => Ok(SignatureDatabase::Pirsf),
            "SFLD" => Ok(SignatureDatabase::Sfld),
            "SUPERFAMILY" => Ok(SignatureDatabase::Superfamily),
            "TIGRFAMS" => Ok(SignatureDatabase::TigrFams),
            "GENE3D" => Ok(SignatureDatabase::Gene3D),
            _ => Ok(SignatureDatabase::Other),
        }
    }
}

// ============================================================================
// Database Models (FromRow)
// ============================================================================

/// InterPro entry metadata (database row)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InterProEntryMetadata {
    pub id: Uuid,
    pub data_source_id: Uuid,
    pub interpro_id: String, // IPR000001
    pub entry_type: String,  // Stored as VARCHAR in DB
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub is_obsolete: bool,
    pub replacement_interpro_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InterProEntryMetadata {
    /// Parse entry_type string to enum
    pub fn entry_type_enum(&self) -> Result<EntryType, String> {
        self.entry_type.parse()
    }
}

/// Protein signature (Pfam, SMART, PROSITE, etc.)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProteinSignature {
    pub id: Uuid,
    pub database: String,  // 'Pfam', 'SMART', etc.
    pub accession: String, // 'PF00051', 'SM00130', etc.
    pub name: Option<String>,
    pub description: Option<String>,
    pub clan_accession: Option<String>, // Pfam-specific
    pub clan_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProteinSignature {
    /// Parse database string to enum
    pub fn database_enum(&self) -> Result<SignatureDatabase, String> {
        self.database.parse()
    }
}

/// Link between InterPro entry and member signature
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InterProMemberSignature {
    pub id: Uuid,
    pub interpro_data_source_id: Uuid,
    pub signature_id: Uuid,
    pub is_primary: bool,
    pub integration_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

/// InterPro to GO term mapping (version-specific FKs)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InterProGoMapping {
    pub id: Uuid,
    pub interpro_data_source_id: Uuid,
    pub interpro_version_id: Uuid,
    pub go_data_source_id: Uuid,
    pub go_version_id: Uuid,
    pub evidence_code: Option<String>, // 'IEA', etc.
    pub created_at: DateTime<Utc>,
}

/// Protein to InterPro match (version-specific FKs)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProteinInterProMatch {
    pub id: Uuid,
    pub interpro_data_source_id: Uuid,
    pub interpro_version_id: Uuid,
    pub protein_data_source_id: Uuid,
    pub protein_version_id: Uuid,
    pub uniprot_accession: String, // Denormalized for fast lookup
    pub signature_id: Uuid,
    pub start_position: i32,
    pub end_position: i32,
    pub e_value: Option<f64>,
    pub score: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// External reference from InterPro entry
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InterProExternalReference {
    pub id: Uuid,
    pub interpro_data_source_id: Uuid,
    pub database: String, // 'PDB', 'CATH', 'Wikipedia', etc.
    pub database_id: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Cached statistics for InterPro entry
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InterProEntryStats {
    pub interpro_data_source_id: Uuid,
    pub protein_count: i32,
    pub species_count: i32,
    pub signature_count: i32,
    pub last_updated: DateTime<Utc>,
}

// ============================================================================
// Parsed Data Structs (from FTP files)
// ============================================================================

/// Parsed InterPro entry from entry.list file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterProEntry {
    pub interpro_id: String,
    pub entry_type: EntryType,
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
}

impl Default for InterProEntry {
    fn default() -> Self {
        Self {
            interpro_id: String::new(),
            entry_type: EntryType::Family,
            name: String::new(),
            short_name: None,
            description: None,
        }
    }
}

/// Parsed protein match from protein2ipr.dat.gz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinMatch {
    pub uniprot_accession: String,
    pub interpro_id: String,
    pub interpro_name: String,
    pub signature_database: SignatureDatabase,
    pub signature_accession: String,
    pub signature_name: Option<String>,
    pub start_position: i32,
    pub end_position: i32,
    pub e_value: Option<f64>,
    pub score: Option<f64>,
}

impl Default for ProteinMatch {
    fn default() -> Self {
        Self {
            uniprot_accession: String::new(),
            interpro_id: String::new(),
            interpro_name: String::new(),
            signature_database: SignatureDatabase::Other,
            signature_accession: String::new(),
            signature_name: None,
            start_position: 0,
            end_position: 0,
            e_value: None,
            score: None,
        }
    }
}

/// Complete InterPro metadata bundle for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterProMetadata {
    pub entry: InterProEntry,
    pub member_signatures: Vec<MemberSignatureData>,
    pub go_mappings: Vec<GoMappingData>,
    pub external_references: Vec<ExternalReferenceData>,
}

impl Default for InterProMetadata {
    fn default() -> Self {
        Self {
            entry: InterProEntry::default(),
            member_signatures: Vec::new(),
            go_mappings: Vec::new(),
            external_references: Vec::new(),
        }
    }
}

/// Member signature data for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberSignatureData {
    pub database: SignatureDatabase,
    pub accession: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_primary: bool,
}

/// GO mapping data for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoMappingData {
    pub go_id: String,         // GO:0005515
    pub evidence_code: String, // IEA, IDA, etc.
}

/// External reference data for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalReferenceData {
    pub database: String,
    pub database_id: String,
    pub description: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_type_to_string() {
        assert_eq!(EntryType::Family.to_string(), "Family");
        assert_eq!(EntryType::Domain.to_string(), "Domain");
        assert_eq!(EntryType::HomologousSuperfamily.to_string(), "Homologous_superfamily");
    }

    #[test]
    fn test_entry_type_from_string() {
        assert_eq!("Family".parse::<EntryType>().unwrap(), EntryType::Family);
        assert_eq!("Domain".parse::<EntryType>().unwrap(), EntryType::Domain);
        assert_eq!(
            "Homologous_superfamily".parse::<EntryType>().unwrap(),
            EntryType::HomologousSuperfamily
        );
        assert!("InvalidType".parse::<EntryType>().is_err());
    }

    #[test]
    fn test_signature_database_to_string() {
        assert_eq!(SignatureDatabase::Pfam.to_string(), "Pfam");
        assert_eq!(SignatureDatabase::Smart.to_string(), "SMART");
        assert_eq!(SignatureDatabase::Prosite.to_string(), "PROSITE");
    }

    #[test]
    fn test_signature_database_from_string() {
        assert_eq!("PFAM".parse::<SignatureDatabase>().unwrap(), SignatureDatabase::Pfam);
        assert_eq!("pfam".parse::<SignatureDatabase>().unwrap(), SignatureDatabase::Pfam);
        assert_eq!("SMART".parse::<SignatureDatabase>().unwrap(), SignatureDatabase::Smart);
        assert_eq!("Unknown".parse::<SignatureDatabase>().unwrap(), SignatureDatabase::Other);
    }

    #[test]
    fn test_interpro_entry_default() {
        let entry = InterProEntry::default();
        assert_eq!(entry.interpro_id, "");
        assert_eq!(entry.entry_type, EntryType::Family);
    }

    #[test]
    fn test_protein_match_default() {
        let match_data = ProteinMatch::default();
        assert_eq!(match_data.start_position, 0);
        assert_eq!(match_data.end_position, 0);
        assert_eq!(match_data.signature_database, SignatureDatabase::Other);
    }
}
