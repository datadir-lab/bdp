//! NCBI Taxonomy data models

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A NCBI Taxonomy entry with lineage information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxonomyEntry {
    /// NCBI Taxonomy ID (e.g., 9606 for Homo sapiens)
    pub taxonomy_id: i32,
    /// Scientific name (e.g., "Homo sapiens")
    pub scientific_name: String,
    /// Common name (e.g., "human"), optional
    pub common_name: Option<String>,
    /// Taxonomic rank (e.g., "species", "genus", "family")
    pub rank: String,
    /// Full lineage as semicolon-separated string
    /// (e.g., "Eukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens")
    pub lineage: String,
}

impl TaxonomyEntry {
    /// Create a new TaxonomyEntry
    pub fn new(
        taxonomy_id: i32,
        scientific_name: String,
        common_name: Option<String>,
        rank: String,
        lineage: String,
    ) -> Self {
        Self {
            taxonomy_id,
            scientific_name,
            common_name,
            rank,
            lineage,
        }
    }

    /// Convert entry to JSON format
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize TaxonomyEntry to JSON")
    }

    /// Convert entry to TSV format (without header)
    pub fn to_tsv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.taxonomy_id,
            self.scientific_name,
            self.common_name.as_deref().unwrap_or(""),
            self.rank,
            self.lineage
        )
    }

    /// Get TSV header
    pub fn tsv_header() -> String {
        "taxonomy_id\tscientific_name\tcommon_name\trank\tlineage".to_string()
    }

    /// Parse lineage string into a vector
    pub fn lineage_vec(&self) -> Vec<String> {
        self.lineage
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Validate the entry for consistency
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.taxonomy_id > 0,
            "Taxonomy ID must be positive, got {}",
            self.taxonomy_id
        );
        anyhow::ensure!(!self.scientific_name.is_empty(), "Scientific name cannot be empty");
        anyhow::ensure!(!self.rank.is_empty(), "Rank cannot be empty");
        anyhow::ensure!(!self.lineage.is_empty(), "Lineage cannot be empty");

        Ok(())
    }
}

/// Represents a merged taxon (old ID merged into new ID)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergedTaxon {
    /// Old taxonomy ID (deprecated)
    pub old_taxonomy_id: i32,
    /// New taxonomy ID (current)
    pub new_taxonomy_id: i32,
}

impl MergedTaxon {
    /// Create a new MergedTaxon
    pub fn new(old_taxonomy_id: i32, new_taxonomy_id: i32) -> Self {
        Self {
            old_taxonomy_id,
            new_taxonomy_id,
        }
    }
}

/// Represents a deleted taxon
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletedTaxon {
    /// Deleted taxonomy ID
    pub taxonomy_id: i32,
}

impl DeletedTaxon {
    /// Create a new DeletedTaxon
    pub fn new(taxonomy_id: i32) -> Self {
        Self { taxonomy_id }
    }
}

/// Complete taxdump data parsed from NCBI FTP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxdumpData {
    /// All taxonomy entries
    pub entries: Vec<TaxonomyEntry>,
    /// Merged taxa (old ID → new ID)
    pub merged: Vec<MergedTaxon>,
    /// Deleted taxa
    pub deleted: Vec<DeletedTaxon>,
    /// External version (FTP timestamp, e.g., "2026-01-15")
    pub external_version: String,
}

impl TaxdumpData {
    /// Create a new TaxdumpData
    pub fn new(
        entries: Vec<TaxonomyEntry>,
        merged: Vec<MergedTaxon>,
        deleted: Vec<DeletedTaxon>,
        external_version: String,
    ) -> Self {
        Self {
            entries,
            merged,
            deleted,
            external_version,
        }
    }

    /// Get a taxonomy entry by ID
    pub fn get_entry(&self, taxonomy_id: i32) -> Option<&TaxonomyEntry> {
        self.entries.iter().find(|e| e.taxonomy_id == taxonomy_id)
    }

    /// Check if a taxonomy ID was merged
    pub fn is_merged(&self, taxonomy_id: i32) -> Option<i32> {
        self.merged
            .iter()
            .find(|m| m.old_taxonomy_id == taxonomy_id)
            .map(|m| m.new_taxonomy_id)
    }

    /// Check if a taxonomy ID was deleted
    pub fn is_deleted(&self, taxonomy_id: i32) -> bool {
        self.deleted.iter().any(|d| d.taxonomy_id == taxonomy_id)
    }

    /// Get statistics about the taxdump
    pub fn stats(&self) -> TaxdumpStats {
        TaxdumpStats {
            total_entries: self.entries.len(),
            merged_count: self.merged.len(),
            deleted_count: self.deleted.len(),
            external_version: self.external_version.clone(),
        }
    }
}

/// Statistics about a taxdump
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxdumpStats {
    /// Total number of taxonomy entries
    pub total_entries: usize,
    /// Number of merged taxa
    pub merged_count: usize,
    /// Number of deleted taxa
    pub deleted_count: usize,
    /// External version (FTP timestamp)
    pub external_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> TaxonomyEntry {
        TaxonomyEntry {
            taxonomy_id: 9606,
            scientific_name: "Homo sapiens".to_string(),
            common_name: Some("human".to_string()),
            rank: "species".to_string(),
            lineage: "Eukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens"
                .to_string(),
        }
    }

    #[test]
    fn test_to_json() {
        let entry = sample_entry();
        let json = entry.to_json().unwrap();

        assert!(json.contains("\"taxonomy_id\": 9606"));
        assert!(json.contains("\"scientific_name\": \"Homo sapiens\""));
        assert!(json.contains("\"common_name\": \"human\""));
        assert!(json.contains("\"rank\": \"species\""));
    }

    #[test]
    fn test_to_tsv() {
        let entry = sample_entry();
        let tsv = entry.to_tsv();

        assert_eq!(
            tsv,
            "9606\tHomo sapiens\thuman\tspecies\tEukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens"
        );
    }

    #[test]
    fn test_to_tsv_without_common_name() {
        let mut entry = sample_entry();
        entry.common_name = None;
        let tsv = entry.to_tsv();

        assert_eq!(
            tsv,
            "9606\tHomo sapiens\t\tspecies\tEukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens"
        );
    }

    #[test]
    fn test_tsv_header() {
        let header = TaxonomyEntry::tsv_header();
        assert_eq!(header, "taxonomy_id\tscientific_name\tcommon_name\trank\tlineage");
    }

    #[test]
    fn test_lineage_vec() {
        let entry = sample_entry();
        let lineage = entry.lineage_vec();

        assert_eq!(lineage.len(), 8);
        assert_eq!(lineage[0], "Eukaryota");
        assert_eq!(lineage[7], "Homo sapiens");
    }

    #[test]
    fn test_validate_success() {
        let entry = sample_entry();
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_taxonomy_id() {
        let mut entry = sample_entry();
        entry.taxonomy_id = -1;
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_validate_empty_scientific_name() {
        let mut entry = sample_entry();
        entry.scientific_name = String::new();
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_merged_taxon() {
        let merged = MergedTaxon::new(123, 456);
        assert_eq!(merged.old_taxonomy_id, 123);
        assert_eq!(merged.new_taxonomy_id, 456);
    }

    #[test]
    fn test_deleted_taxon() {
        let deleted = DeletedTaxon::new(789);
        assert_eq!(deleted.taxonomy_id, 789);
    }

    #[test]
    fn test_taxdump_data_get_entry() {
        let entry = sample_entry();
        let data = TaxdumpData::new(vec![entry.clone()], vec![], vec![], "2026-01-15".to_string());

        assert_eq!(data.get_entry(9606), Some(&entry));
        assert_eq!(data.get_entry(9999), None);
    }

    #[test]
    fn test_taxdump_data_is_merged() {
        let merged = MergedTaxon::new(123, 456);
        let data = TaxdumpData::new(vec![], vec![merged], vec![], "2026-01-15".to_string());

        assert_eq!(data.is_merged(123), Some(456));
        assert_eq!(data.is_merged(456), None);
    }

    #[test]
    fn test_taxdump_data_is_deleted() {
        let deleted = DeletedTaxon::new(789);
        let data = TaxdumpData::new(vec![], vec![], vec![deleted], "2026-01-15".to_string());

        assert!(data.is_deleted(789));
        assert!(!data.is_deleted(123));
    }

    #[test]
    fn test_taxdump_stats() {
        let entry = sample_entry();
        let merged = MergedTaxon::new(123, 456);
        let deleted = DeletedTaxon::new(789);
        let data =
            TaxdumpData::new(vec![entry], vec![merged], vec![deleted], "2026-01-15".to_string());

        let stats = data.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.merged_count, 1);
        assert_eq!(stats.deleted_count, 1);
        assert_eq!(stats.external_version, "2026-01-15");
    }
}
