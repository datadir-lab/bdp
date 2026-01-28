// GenBank flat file parser
//
// Parses GenBank/RefSeq flat file format into GenbankRecord structs.
// Format documentation: https://www.ncbi.nlm.nih.gov/Sitemap/samplerecord.html

use super::models::{
    CdsFeature, Division, Feature, GenbankRecord, SourceDatabase, SourceFeature, Topology,
};
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

pub struct GenbankParser {
    source_database: SourceDatabase,
}

impl GenbankParser {
    pub fn new(source_database: SourceDatabase) -> Self {
        Self { source_database }
    }

    /// Parse all records from a reader
    pub fn parse_all<R: Read>(&self, reader: R) -> Result<Vec<GenbankRecord>> {
        let buf_reader = BufReader::new(reader);
        let mut records = Vec::new();
        let mut current_lines = Vec::new();

        for line in buf_reader.lines() {
            let line = line.context("Failed to read line")?;

            // Record delimiter
            if line.starts_with("//") {
                if !current_lines.is_empty() {
                    match self.parse_record(&current_lines) {
                        Ok(record) => records.push(record),
                        Err(e) => {
                            tracing::warn!("Failed to parse record: {}", e);
                        },
                    }
                    current_lines.clear();
                }
            } else {
                current_lines.push(line);
            }
        }

        Ok(records)
    }

    /// Parse all records with a limit (for testing)
    pub fn parse_with_limit<R: Read>(&self, reader: R, limit: usize) -> Result<Vec<GenbankRecord>> {
        let buf_reader = BufReader::new(reader);
        let mut records = Vec::new();
        let mut current_lines = Vec::new();

        for line in buf_reader.lines() {
            if records.len() >= limit {
                break;
            }

            let line = line.context("Failed to read line")?;

            if line.starts_with("//") {
                if !current_lines.is_empty() {
                    match self.parse_record(&current_lines) {
                        Ok(record) => records.push(record),
                        Err(e) => {
                            tracing::warn!("Failed to parse record: {}", e);
                        },
                    }
                    current_lines.clear();
                }
            } else {
                current_lines.push(line);
            }
        }

        Ok(records)
    }

    /// Parse a single record from lines
    fn parse_record(&self, lines: &[String]) -> Result<GenbankRecord> {
        let mut record = GenbankRecord {
            locus_name: String::new(),
            sequence_length: 0,
            molecule_type: String::new(),
            topology: None,
            division_code: String::new(),
            modification_date: None,
            definition: String::new(),
            accession: String::new(),
            accession_version: String::new(),
            version_number: None,
            organism: None,
            taxonomy: Vec::new(),
            taxonomy_id: None,
            source_feature: None,
            cds_features: Vec::new(),
            all_features: Vec::new(),
            sequence: String::new(),
            sequence_hash: String::new(),
            gc_content: 0.0,
            source_database: self.source_database,
            division: None,
        };

        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];

            if line.starts_with("LOCUS") {
                self.parse_locus(line, &mut record)?;
            } else if line.starts_with("DEFINITION") {
                i = self.parse_definition(lines, i, &mut record)?;
            } else if line.starts_with("ACCESSION") {
                self.parse_accession(line, &mut record)?;
            } else if line.starts_with("VERSION") {
                self.parse_version(line, &mut record)?;
            } else if line.starts_with("  ORGANISM") {
                i = self.parse_organism(lines, i, &mut record)?;
            } else if line.starts_with("FEATURES") {
                i = self.parse_features(lines, i, &mut record)?;
            } else if line.starts_with("ORIGIN") {
                i = self.parse_origin(lines, i, &mut record)?;
            }

            i += 1;
        }

        // Post-processing
        record.taxonomy_id = record.extract_taxonomy_id();
        record.gc_content = Self::calculate_gc_content(&record.sequence);
        record.sequence_hash = Self::calculate_hash(&record.sequence);
        record.division = Self::infer_division(&record.division_code);

        Ok(record)
    }

    /// Parse LOCUS line
    /// Format: LOCUS       NC_000913            4641652 bp    DNA     circular BCT 01-JAN-2026
    fn parse_locus(&self, line: &str, record: &mut GenbankRecord) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            return Err(anyhow!("Invalid LOCUS line: {}", line));
        }

        record.locus_name = parts[1].to_string();
        record.sequence_length = parts[2].parse().context("Invalid sequence length")?;
        record.molecule_type = parts[4].to_string();

        // Topology (linear or circular)
        if parts.len() > 5 {
            match parts[5].to_lowercase().as_str() {
                "circular" => record.topology = Some(Topology::Circular),
                "linear" => record.topology = Some(Topology::Linear),
                _ => {},
            }
        }

        // Division code
        if parts.len() > 6 {
            record.division_code = parts[6].to_string();
        }

        // Date
        if parts.len() > 7 {
            record.modification_date = Some(parts[7].to_string());
        }

        Ok(())
    }

    /// Parse DEFINITION (can span multiple lines)
    fn parse_definition(
        &self,
        lines: &[String],
        start: usize,
        record: &mut GenbankRecord,
    ) -> Result<usize> {
        let mut def_parts = Vec::new();
        let mut i = start;

        // First line
        if let Some(first) = lines[i].strip_prefix("DEFINITION  ") {
            def_parts.push(first.trim().to_string());
        }

        // Continuation lines (start with spaces)
        i += 1;
        while i < lines.len() {
            let line = &lines[i];
            if line.starts_with("            ") {
                def_parts.push(line.trim().to_string());
                i += 1;
            } else {
                break;
            }
        }

        record.definition = def_parts.join(" ");
        Ok(i - 1)
    }

    /// Parse ACCESSION line
    fn parse_accession(&self, line: &str, record: &mut GenbankRecord) -> Result<()> {
        if let Some(acc) = line.strip_prefix("ACCESSION   ") {
            record.accession = acc.trim().to_string();
        }
        Ok(())
    }

    /// Parse VERSION line
    /// Format: VERSION     NC_000913.3  GI:556503834
    fn parse_version(&self, line: &str, record: &mut GenbankRecord) -> Result<()> {
        if let Some(version_part) = line.strip_prefix("VERSION     ") {
            let parts: Vec<&str> = version_part.split_whitespace().collect();
            if !parts.is_empty() {
                record.accession_version = parts[0].to_string();

                // Extract version number from "NC_000913.3" -> 3
                if let Some(dot_pos) = parts[0].rfind('.') {
                    if let Ok(ver) = parts[0][dot_pos + 1..].parse::<i32>() {
                        record.version_number = Some(ver);
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse ORGANISM section (includes taxonomy lineage)
    fn parse_organism(
        &self,
        lines: &[String],
        start: usize,
        record: &mut GenbankRecord,
    ) -> Result<usize> {
        let mut i = start;

        // Organism name
        if let Some(org) = lines[i].strip_prefix("  ORGANISM  ") {
            record.organism = Some(org.trim().to_string());
        }

        // Taxonomy lineage (continuation lines)
        i += 1;
        let mut taxonomy_parts = Vec::new();
        while i < lines.len() {
            let line = &lines[i];
            if line.starts_with("            ") {
                let tax_line = line.trim().trim_end_matches(';').trim_end_matches('.');
                for taxon in tax_line.split(';') {
                    taxonomy_parts.push(taxon.trim().to_string());
                }
                i += 1;
            } else {
                break;
            }
        }

        record.taxonomy = taxonomy_parts;
        Ok(i - 1)
    }

    /// Parse FEATURES section
    fn parse_features(
        &self,
        lines: &[String],
        start: usize,
        record: &mut GenbankRecord,
    ) -> Result<usize> {
        let mut i = start + 1; // Skip "FEATURES             Location/Qualifiers"

        while i < lines.len() {
            let line = &lines[i];

            // End of features section
            if !line.starts_with("     ") && !line.starts_with(' ') {
                break;
            }

            // New feature starts at column 5 (not indented beyond that)
            if line.starts_with("     ") && !line.starts_with("                     ") {
                let (feature_type, location) = Self::parse_feature_header(line)?;
                let (qualifiers, end_i) = self.parse_feature_qualifiers(lines, i + 1)?;

                // Create feature
                let feature = Feature {
                    feature_type: feature_type.clone(),
                    location: location.clone(),
                    qualifiers: qualifiers.clone(),
                };

                // Handle specific feature types
                match feature_type.as_str() {
                    "source" => {
                        record.source_feature = Some(self.create_source_feature(&qualifiers));
                    },
                    "CDS" => {
                        record
                            .cds_features
                            .push(self.create_cds_feature(&location, &qualifiers));
                    },
                    _ => {},
                }

                record.all_features.push(feature);
                i = end_i;
            }

            i += 1;
        }

        Ok(i - 1)
    }

    /// Parse feature header line
    /// Format: "     source          1..4641652"
    fn parse_feature_header(line: &str) -> Result<(String, String)> {
        let trimmed = line.trim_start();
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();

        if parts.len() < 2 {
            return Err(anyhow!("Invalid feature header: {}", line));
        }

        let feature_type = parts[0].to_string();
        let location = parts[1].trim().to_string();

        Ok((feature_type, location))
    }

    /// Parse feature qualifiers
    fn parse_feature_qualifiers(
        &self,
        lines: &[String],
        start: usize,
    ) -> Result<(HashMap<String, String>, usize)> {
        let mut qualifiers = HashMap::new();
        let mut i = start;
        let mut current_key: Option<String> = None;
        let mut current_value = String::new();

        while i < lines.len() {
            let line = &lines[i];

            // Qualifier line starts with /
            if line.trim_start().starts_with('/') {
                // Save previous qualifier
                if let Some(key) = current_key.take() {
                    qualifiers.insert(key, current_value.trim_matches('"').to_string());
                    current_value.clear();
                }

                // New qualifier
                let trimmed = line.trim_start().trim_start_matches('/');
                if let Some(eq_pos) = trimmed.find('=') {
                    current_key = Some(trimmed[..eq_pos].to_string());
                    current_value = trimmed[eq_pos + 1..].to_string();
                } else {
                    // Boolean qualifier (no value)
                    qualifiers.insert(trimmed.to_string(), "true".to_string());
                }
            } else if line.starts_with("                     ") && current_key.is_some() {
                // Continuation of previous qualifier value
                current_value.push(' ');
                current_value.push_str(line.trim());
            } else if !line.starts_with("     ") || line.trim().is_empty() {
                // End of qualifiers
                break;
            } else {
                // Next feature starting
                break;
            }

            i += 1;
        }

        // Save last qualifier
        if let Some(key) = current_key {
            qualifiers.insert(key, current_value.trim_matches('"').to_string());
        }

        Ok((qualifiers, i - 1))
    }

    /// Create SourceFeature from qualifiers
    fn create_source_feature(&self, qualifiers: &HashMap<String, String>) -> SourceFeature {
        let mut db_xref = Vec::new();
        if let Some(xref_str) = qualifiers.get("db_xref") {
            db_xref.push(xref_str.clone());
        }

        SourceFeature {
            organism: qualifiers.get("organism").cloned(),
            mol_type: qualifiers.get("mol_type").cloned(),
            strain: qualifiers.get("strain").cloned(),
            isolate: qualifiers.get("isolate").cloned(),
            db_xref,
            qualifiers: qualifiers.clone(),
        }
    }

    /// Create CdsFeature from qualifiers
    fn create_cds_feature(
        &self,
        location: &str,
        qualifiers: &HashMap<String, String>,
    ) -> CdsFeature {
        let (start, end, strand) = Self::parse_location(location);

        CdsFeature {
            location: location.to_string(),
            start,
            end,
            strand,
            locus_tag: qualifiers.get("locus_tag").cloned(),
            gene: qualifiers.get("gene").cloned(),
            product: qualifiers.get("product").cloned(),
            protein_id: qualifiers.get("protein_id").cloned(),
            translation: qualifiers.get("translation").cloned(),
            codon_start: qualifiers.get("codon_start").and_then(|s| s.parse().ok()),
            transl_table: qualifiers.get("transl_table").and_then(|s| s.parse().ok()),
            qualifiers: qualifiers.clone(),
        }
    }

    /// Parse location string to extract start, end, strand
    /// Examples: "190..255", "complement(1000..2000)", "join(1..100,200..300)"
    fn parse_location(location: &str) -> (Option<i32>, Option<i32>, Option<String>) {
        let mut strand = Some("+".to_string());
        let mut loc = location;

        // Handle complement
        if loc.starts_with("complement(") {
            strand = Some("-".to_string());
            loc = loc.trim_start_matches("complement(").trim_end_matches(')');
        }

        // Handle join (take first range for simplicity)
        if loc.starts_with("join(") {
            loc = loc.trim_start_matches("join(").trim_end_matches(')');
            if let Some(comma_pos) = loc.find(',') {
                loc = &loc[..comma_pos];
            }
        }

        // Parse range "start..end"
        if let Some(dot_pos) = loc.find("..") {
            let start_str = loc[..dot_pos]
                .trim_start_matches('<')
                .trim_start_matches('>');
            let end_str = loc[dot_pos + 2..]
                .trim_start_matches('<')
                .trim_start_matches('>');

            let start = start_str.parse().ok();
            let end = end_str.parse().ok();

            (start, end, strand)
        } else {
            (None, None, strand)
        }
    }

    /// Parse ORIGIN section (sequence data)
    fn parse_origin(
        &self,
        lines: &[String],
        start: usize,
        record: &mut GenbankRecord,
    ) -> Result<usize> {
        let mut i = start + 1; // Skip "ORIGIN" line
        let mut sequence = String::new();

        while i < lines.len() {
            let line = &lines[i];

            // Sequence lines start with a number
            if line.trim().chars().next().map_or(false, |c| c.is_numeric()) {
                // Remove line numbers and spaces
                let seq_part: String = line
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .map(|c| c.to_ascii_uppercase())
                    .collect();
                sequence.push_str(&seq_part);
            } else {
                break;
            }

            i += 1;
        }

        record.sequence = sequence;
        Ok(i - 1)
    }

    /// Calculate GC content percentage
    pub fn calculate_gc_content(sequence: &str) -> f32 {
        if sequence.is_empty() {
            return 0.0;
        }

        let gc_count = sequence.chars().filter(|c| *c == 'G' || *c == 'C').count();

        (gc_count as f32 / sequence.len() as f32) * 100.0
    }

    /// Calculate SHA256 hash of sequence
    pub fn calculate_hash(sequence: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(sequence.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Infer Division enum from division code
    fn infer_division(code: &str) -> Option<Division> {
        match code {
            "VRL" => Some(Division::Viral),
            "BCT" => Some(Division::Bacterial),
            "PLN" => Some(Division::Plant),
            "MAM" => Some(Division::Mammalian),
            "PRI" => Some(Division::Primate),
            "ROD" => Some(Division::Rodent),
            "VRT" => Some(Division::Vertebrate),
            "INV" => Some(Division::Invertebrate),
            "PHG" => Some(Division::Phage),
            "SYN" => Some(Division::Synthetic),
            "UNA" => Some(Division::Unannotated),
            "ENV" => Some(Division::Environmental),
            "PAT" => Some(Division::Patent),
            "EST" => Some(Division::Est),
            "STS" => Some(Division::Sts),
            "GSS" => Some(Division::Gss),
            "HTG" => Some(Division::Htg),
            "CON" => Some(Division::Con),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_location() {
        let (start, end, strand) = GenbankParser::parse_location("190..255");
        assert_eq!(start, Some(190));
        assert_eq!(end, Some(255));
        assert_eq!(strand, Some("+".to_string()));

        let (start, end, strand) = GenbankParser::parse_location("complement(1000..2000)");
        assert_eq!(start, Some(1000));
        assert_eq!(end, Some(2000));
        assert_eq!(strand, Some("-".to_string()));
    }

    #[test]
    fn test_calculate_gc_content() {
        assert_eq!(GenbankParser::calculate_gc_content("ATGC"), 50.0);
        assert_eq!(GenbankParser::calculate_gc_content("GGCC"), 100.0);
        assert_eq!(GenbankParser::calculate_gc_content("AATT"), 0.0);
    }

    #[test]
    fn test_calculate_hash() {
        let hash1 = GenbankParser::calculate_hash("ATGC");
        let hash2 = GenbankParser::calculate_hash("ATGC");
        let hash3 = GenbankParser::calculate_hash("CGTA");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_infer_division() {
        assert_eq!(GenbankParser::infer_division("VRL"), Some(Division::Viral));
        assert_eq!(GenbankParser::infer_division("BCT"), Some(Division::Bacterial));
        assert_eq!(GenbankParser::infer_division("PHG"), Some(Division::Phage));
        assert_eq!(GenbankParser::infer_division("XXX"), None);
    }
}
