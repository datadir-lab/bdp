//! UniProt data models

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A UniProt protein entry with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UniProtEntry {
    // === Core Fields ===
    /// Primary accession number (e.g., "P12345")
    pub accession: String,
    /// Entry name / ID (e.g., "ALBU_HUMAN")
    pub entry_name: String,
    /// Protein name (RecName: Full)
    pub protein_name: String,
    /// Gene name (optional)
    pub gene_name: Option<String>,
    /// Organism scientific name
    pub organism_name: String,
    /// NCBI Taxonomy ID
    pub taxonomy_id: i32,
    /// Taxonomic lineage from OC line (e.g., ["Viruses", "Riboviria", ...])
    pub taxonomy_lineage: Vec<String>,
    /// Protein sequence (amino acids)
    pub sequence: String,
    /// Sequence length (number of amino acids)
    pub sequence_length: i32,
    /// Molecular mass in Daltons
    pub mass_da: i64,
    /// Release date (last updated)
    pub release_date: NaiveDate,

    // === Extended Metadata (Phase 2) ===
    /// Alternative protein names (AltName, SubName)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternative_names: Vec<String>,
    /// EC numbers (enzyme classification)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ec_numbers: Vec<String>,
    /// Protein features (domains, sites, modifications, variants)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<ProteinFeature>,
    /// Database cross-references (PDB, GO, InterPro, KEGG, Pfam, RefSeq)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_references: Vec<CrossReference>,
    /// Comments (function, location, disease, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
    /// Protein existence level (1-5)
    pub protein_existence: Option<i32>,
    /// Keywords for functional classification
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// Organelle origin (mitochondrion, plastid, plasmid)
    pub organelle: Option<String>,
    /// Host organisms (for viruses)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub organism_hosts: Vec<String>,
}

/// Protein feature from FT line (domain, site, modification, variant)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProteinFeature {
    /// Feature type (e.g., "DOMAIN", "BINDING", "MOD_RES", "VARIANT")
    pub feature_type: String,
    /// Start position in sequence (1-based)
    pub start_pos: Option<i32>,
    /// End position in sequence (1-based)
    pub end_pos: Option<i32>,
    /// Feature description
    pub description: String,
}

/// Database cross-reference from DR line
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossReference {
    /// Database name (e.g., "PDB", "GO", "InterPro", "KEGG", "Pfam", "RefSeq")
    pub database: String,
    /// Database ID
    pub database_id: String,
    /// Additional metadata (varies by database)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<String>,
}

/// Comment from CC line (function, location, disease, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    /// Comment topic (e.g., "FUNCTION", "SUBCELLULAR LOCATION", "DISEASE")
    pub topic: String,
    /// Comment text
    pub text: String,
}

impl UniProtEntry {
    /// Convert entry to FASTA format
    ///
    /// # Format
    /// ```text
    /// >sp|{accession}|{entry_name} {protein_name} OS={organism_name} OX={taxonomy_id} GN={gene_name}
    /// {sequence wrapped at 60 chars}
    /// ```
    pub fn to_fasta(&self) -> String {
        let mut header = format!(
            ">sp|{}|{} {} OS={} OX={}",
            self.accession, self.entry_name, self.protein_name, self.organism_name, self.taxonomy_id
        );

        if let Some(ref gene_name) = self.gene_name {
            header.push_str(&format!(" GN={}", gene_name));
        }

        // Wrap sequence at 60 characters per line
        let wrapped_sequence = self
            .sequence
            .chars()
            .collect::<Vec<_>>()
            .chunks(60)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        format!("{}\n{}\n", header, wrapped_sequence)
    }

    /// Convert entry to JSON format
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize UniProtEntry to JSON")
    }

    /// Calculate SHA-256 checksum of the sequence
    pub fn sequence_checksum(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.sequence.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate the entry for consistency
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.accession.is_empty(), "Accession cannot be empty");
        anyhow::ensure!(!self.entry_name.is_empty(), "Entry name cannot be empty");
        anyhow::ensure!(!self.protein_name.is_empty(), "Protein name cannot be empty");
        anyhow::ensure!(!self.organism_name.is_empty(), "Organism name cannot be empty");
        anyhow::ensure!(self.taxonomy_id > 0, "Taxonomy ID must be positive");
        anyhow::ensure!(!self.sequence.is_empty(), "Sequence cannot be empty");
        anyhow::ensure!(
            self.sequence_length as usize == self.sequence.len(),
            "Sequence length mismatch: expected {}, got {}",
            self.sequence_length,
            self.sequence.len()
        );
        anyhow::ensure!(self.mass_da > 0, "Mass must be positive");

        Ok(())
    }
}

/// UniProt release information parsed from relnotes.txt
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseInfo {
    /// Release version (e.g., "2024_01")
    pub external_version: String,
    /// Release date
    pub release_date: NaiveDate,
    /// Number of SwissProt entries in this release
    pub swissprot_count: u64,
    /// License information
    pub license: Option<LicenseInfo>,
}

impl ReleaseInfo {
    /// Create a new ReleaseInfo
    pub fn new(external_version: String, release_date: NaiveDate, swissprot_count: u64) -> Self {
        Self {
            external_version,
            release_date,
            swissprot_count,
            license: Some(LicenseInfo::default()),
        }
    }

    /// Create ReleaseInfo without license
    pub fn without_license(external_version: String, release_date: NaiveDate, swissprot_count: u64) -> Self {
        Self {
            external_version,
            release_date,
            swissprot_count,
            license: None,
        }
    }
}

/// UniProt license information
///
/// As of 2016, UniProt data is distributed under the
/// Creative Commons Attribution 4.0 International (CC BY 4.0) license.
///
/// See: https://www.uniprot.org/help/license
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LicenseInfo {
    /// License name
    pub name: String,
    /// License identifier (SPDX)
    pub identifier: String,
    /// License URL
    pub url: String,
    /// Attribution requirement
    pub attribution_required: bool,
    /// Commercial use allowed
    pub commercial_use: bool,
    /// Modification allowed
    pub modification_allowed: bool,
    /// Citation text
    pub citation: Option<String>,
}

impl Default for LicenseInfo {
    fn default() -> Self {
        Self {
            name: "Creative Commons Attribution 4.0 International".to_string(),
            identifier: "CC-BY-4.0".to_string(),
            url: "https://creativecommons.org/licenses/by/4.0/".to_string(),
            attribution_required: true,
            commercial_use: true,
            modification_allowed: true,
            citation: Some(
                "UniProt Consortium. UniProt: the Universal Protein Knowledgebase. \
                 Nucleic Acids Research. https://www.uniprot.org/".to_string()
            ),
        }
    }
}

impl LicenseInfo {
    /// Create a custom license
    pub fn custom(name: String, identifier: String, url: String) -> Self {
        Self {
            name,
            identifier,
            url,
            ..Self::default()
        }
    }

    /// Get the SPDX license identifier
    pub fn spdx(&self) -> &str {
        &self.identifier
    }

    /// Get citation text for this data source
    pub fn citation_text(&self) -> String {
        self.citation.clone().unwrap_or_else(|| {
            format!(
                "This work is licensed under {}. See: {}",
                self.name, self.url
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> UniProtEntry {
        UniProtEntry {
            accession: "P12345".to_string(),
            entry_name: "TEST_HUMAN".to_string(),
            protein_name: "Test protein".to_string(),
            gene_name: Some("TEST".to_string()),
            organism_name: "Homo sapiens".to_string(),
            taxonomy_id: 9606,
            taxonomy_lineage: vec!["Eukaryota".to_string(), "Metazoa".to_string()],
            sequence: "MKTIIALSYIFCLVFADYKDDDDK".to_string(),
            sequence_length: 25,
            mass_da: 2897,
            release_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            alternative_names: vec![],
            ec_numbers: vec![],
            features: vec![],
            cross_references: vec![],
            comments: vec![],
            protein_existence: None,
            keywords: vec![],
            organelle: None,
            organism_hosts: vec![],
        }
    }

    #[test]
    fn test_to_fasta_with_gene_name() {
        let entry = sample_entry();
        let fasta = entry.to_fasta();

        assert!(fasta.starts_with(">sp|P12345|TEST_HUMAN"));
        assert!(fasta.contains("Test protein"));
        assert!(fasta.contains("OS=Homo sapiens"));
        assert!(fasta.contains("OX=9606"));
        assert!(fasta.contains("GN=TEST"));
        assert!(fasta.contains("MKTIIALSYIFCLVFADYKDDDDK"));
    }

    #[test]
    fn test_to_fasta_without_gene_name() {
        let mut entry = sample_entry();
        entry.gene_name = None;
        let fasta = entry.to_fasta();

        assert!(fasta.starts_with(">sp|P12345|TEST_HUMAN"));
        assert!(!fasta.contains("GN="));
    }

    #[test]
    fn test_to_fasta_wraps_long_sequences() {
        let mut entry = sample_entry();
        entry.sequence = "A".repeat(120);
        entry.sequence_length = 120;
        let fasta = entry.to_fasta();

        let lines: Vec<&str> = fasta.lines().collect();
        assert_eq!(lines.len(), 3); // Header + 2 lines of sequence
        assert_eq!(lines[1].len(), 60);
        assert_eq!(lines[2].len(), 60);
    }

    #[test]
    fn test_to_json() {
        let entry = sample_entry();
        let json = entry.to_json().unwrap();

        assert!(json.contains("\"accession\": \"P12345\""));
        assert!(json.contains("\"entry_name\": \"TEST_HUMAN\""));
        assert!(json.contains("\"taxonomy_id\": 9606"));
    }

    #[test]
    fn test_sequence_checksum() {
        let entry = sample_entry();
        let checksum = entry.sequence_checksum();

        // SHA256 should produce a 64-character hex string
        assert_eq!(checksum.len(), 64);

        // Same sequence should produce same checksum
        let entry2 = sample_entry();
        assert_eq!(checksum, entry2.sequence_checksum());

        // Different sequence should produce different checksum
        let mut entry3 = sample_entry();
        entry3.sequence = "DIFFERENT".to_string();
        assert_ne!(checksum, entry3.sequence_checksum());
    }

    #[test]
    fn test_validate_success() {
        let entry = sample_entry();
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_accession() {
        let mut entry = sample_entry();
        entry.accession = String::new();
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_taxonomy_id() {
        let mut entry = sample_entry();
        entry.taxonomy_id = -1;
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_validate_sequence_length_mismatch() {
        let mut entry = sample_entry();
        entry.sequence_length = 100; // Doesn't match actual sequence length
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_release_info_creation() {
        let info = ReleaseInfo::new(
            "2024_01".to_string(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            571609,
        );

        assert_eq!(info.external_version, "2024_01");
        assert_eq!(info.swissprot_count, 571609);
    }
}
