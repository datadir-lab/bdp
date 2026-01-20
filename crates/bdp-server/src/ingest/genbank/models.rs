// Data models for GenBank/RefSeq records

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Source database type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceDatabase {
    Genbank,
    Refseq,
}

impl SourceDatabase {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceDatabase::Genbank => "genbank",
            SourceDatabase::Refseq => "refseq",
        }
    }
}

/// GenBank division types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Division {
    Viral,      // gbvrl - viruses
    Bacterial,  // gbbct - bacteria
    Plant,      // gbpln - plants
    Mammalian,  // gbmam - other mammals
    Primate,    // gbpri - primates
    Rodent,     // gbrod - rodents
    Vertebrate, // gbvrt - other vertebrates
    Invertebrate, // gbinv - invertebrates
    Phage,      // gbphg - phages
    Synthetic,  // gbsyn - synthetic
    Unannotated, // gbuna - unannotated
    Environmental, // gbenv - environmental samples
    Patent,     // gbpat - patent sequences
    Est,        // gbest - expressed sequence tags
    Sts,        // gbsts - sequence tagged sites
    Gss,        // gbgss - genome survey sequences
    Htg,        // gbhtg - high throughput genomic
    Con,        // gbcon - constructed sequences
}

impl Division {
    pub fn as_str(&self) -> &'static str {
        match self {
            Division::Viral => "viral",
            Division::Bacterial => "bacterial",
            Division::Plant => "plant",
            Division::Mammalian => "mammalian",
            Division::Primate => "primate",
            Division::Rodent => "rodent",
            Division::Vertebrate => "vertebrate",
            Division::Invertebrate => "invertebrate",
            Division::Phage => "phage",
            Division::Synthetic => "synthetic",
            Division::Unannotated => "unannotated",
            Division::Environmental => "environmental",
            Division::Patent => "patent",
            Division::Est => "est",
            Division::Sts => "sts",
            Division::Gss => "gss",
            Division::Htg => "htg",
            Division::Con => "con",
        }
    }

    pub fn file_prefix(&self) -> &'static str {
        match self {
            Division::Viral => "gbvrl",
            Division::Bacterial => "gbbct",
            Division::Plant => "gbpln",
            Division::Mammalian => "gbmam",
            Division::Primate => "gbpri",
            Division::Rodent => "gbrod",
            Division::Vertebrate => "gbvrt",
            Division::Invertebrate => "gbinv",
            Division::Phage => "gbphg",
            Division::Synthetic => "gbsyn",
            Division::Unannotated => "gbuna",
            Division::Environmental => "gbenv",
            Division::Patent => "gbpat",
            Division::Est => "gbest",
            Division::Sts => "gbsts",
            Division::Gss => "gbgss",
            Division::Htg => "gbhtg",
            Division::Con => "gbcon",
        }
    }
}

/// Molecule topology
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Topology {
    Linear,
    Circular,
}

/// Source feature (organism information)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFeature {
    pub organism: Option<String>,
    pub mol_type: Option<String>,
    pub strain: Option<String>,
    pub isolate: Option<String>,
    pub db_xref: Vec<String>, // e.g., "taxon:511145"
    pub qualifiers: HashMap<String, String>,
}

/// CDS (Coding Sequence) feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdsFeature {
    pub location: String,       // e.g., "190..255" or "complement(1000..2000)"
    pub start: Option<i32>,
    pub end: Option<i32>,
    pub strand: Option<String>, // "+" or "-"
    pub locus_tag: Option<String>,
    pub gene: Option<String>,
    pub product: Option<String>,
    pub protein_id: Option<String>, // Links to UniProt
    pub translation: Option<String>,
    pub codon_start: Option<i32>,
    pub transl_table: Option<i32>,
    pub qualifiers: HashMap<String, String>,
}

/// Generic feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub feature_type: String, // "gene", "CDS", "rRNA", "tRNA", etc.
    pub location: String,
    pub qualifiers: HashMap<String, String>,
}

/// Complete GenBank record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenbankRecord {
    // LOCUS line
    pub locus_name: String,
    pub sequence_length: i32,
    pub molecule_type: String, // DNA, RNA, etc.
    pub topology: Option<Topology>,
    pub division_code: String, // BCT, VRL, PLN, etc.
    pub modification_date: Option<String>,

    // DEFINITION
    pub definition: String,

    // ACCESSION
    pub accession: String,

    // VERSION
    pub accession_version: String,
    pub version_number: Option<i32>, // From "NC_000913.3" -> 3

    // SOURCE/ORGANISM
    pub organism: Option<String>,
    pub taxonomy: Vec<String>, // Taxonomic lineage
    pub taxonomy_id: Option<i32>, // Extracted from db_xref="taxon:511145"

    // FEATURES
    pub source_feature: Option<SourceFeature>,
    pub cds_features: Vec<CdsFeature>,
    pub all_features: Vec<Feature>,

    // ORIGIN (sequence)
    pub sequence: String, // ACGT nucleotides
    pub sequence_hash: String, // SHA256 for deduplication
    pub gc_content: f32,

    // Metadata
    pub source_database: SourceDatabase,
    pub division: Option<Division>,
}

impl GenbankRecord {
    /// Extract taxonomy ID from db_xref fields
    pub fn extract_taxonomy_id(&self) -> Option<i32> {
        if let Some(ref source) = self.source_feature {
            for xref in &source.db_xref {
                if xref.starts_with("taxon:") {
                    if let Ok(id) = xref.strip_prefix("taxon:")?.parse::<i32>() {
                        return Some(id);
                    }
                }
            }
        }
        None
    }

    /// Extract gene name from first CDS feature
    pub fn extract_gene_name(&self) -> Option<String> {
        self.cds_features.first()?.gene.clone()
    }

    /// Extract locus tag from first CDS feature
    pub fn extract_locus_tag(&self) -> Option<String> {
        self.cds_features.first()?.locus_tag.clone()
    }

    /// Extract protein ID from first CDS feature
    pub fn extract_protein_id(&self) -> Option<String> {
        self.cds_features.first()?.protein_id.clone()
    }

    /// Extract product description from first CDS feature
    pub fn extract_product(&self) -> Option<String> {
        self.cds_features.first()?.product.clone()
    }

    /// Generate S3 key for this record
    pub fn generate_s3_key(&self, release: &str) -> String {
        let db = self.source_database.as_str();
        let div = self.division.as_ref()
            .map(|d| d.as_str())
            .unwrap_or("unknown");
        format!("{}/release-{}/{}/{}.fasta", db, release, div, self.accession_version)
    }

    /// Convert to FASTA format
    pub fn to_fasta(&self) -> String {
        format!(
            ">{} {}\n{}",
            self.accession_version,
            self.definition,
            self.format_sequence_lines()
        )
    }

    /// Format sequence with 60 characters per line (FASTA standard)
    fn format_sequence_lines(&self) -> String {
        self.sequence
            .chars()
            .collect::<Vec<char>>()
            .chunks(60)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n")
    }
}

/// Pipeline processing result
#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub data_source_id: Uuid,
    pub release: String,
    pub division: String,
    pub records_processed: usize,
    pub sequences_inserted: usize,
    pub mappings_created: usize,
    pub bytes_uploaded: u64,
    pub duration_seconds: f64,
}

/// Orchestrator summary result
#[derive(Debug, Clone)]
pub struct OrchestratorResult {
    pub release: String,
    pub divisions_processed: usize,
    pub total_records: usize,
    pub total_sequences: usize,
    pub total_mappings: usize,
    pub total_bytes: u64,
    pub duration_seconds: f64,
    pub division_results: Vec<PipelineResult>,
}
