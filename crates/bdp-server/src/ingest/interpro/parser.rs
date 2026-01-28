// InterPro Data Parser
//
// Parses protein2ipr.dat.gz and entry.list files from InterPro FTP.
//
// File Formats:
// 1. protein2ipr.dat.gz - TSV file with protein matches
//    Format: UniProtAccession MD5 Length InterProID InterProName SignatureDB SignatureAcc SignatureName Start Stop Score Status Date
//    Example: P12345 abc123def456 154 IPR000001 Kringle Pfam PF00051 Kringle domain 10 100 1.2E-10 T 01-JAN-2024
//
// 2. entry.list - TSV file with InterPro entry metadata
//    Format: ENTRY_AC ENTRY_TYPE ENTRY_NAME
//    Example: IPR000001 Domain Kringle

use crate::ingest::interpro::models::{EntryType, InterProEntry, ProteinMatch, SignatureDatabase};
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use tracing::{debug, warn};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid TSV format at line {line}: {message}")]
    InvalidFormat { line: usize, message: String },

    #[error("Invalid entry type: {0}")]
    InvalidEntryType(String),

    #[error("Invalid signature database: {0}")]
    InvalidSignatureDatabase(String),

    #[error("Invalid integer value at line {line}: {message}")]
    InvalidInteger { line: usize, message: String },

    #[error("Invalid float value at line {line}: {message}")]
    InvalidFloat { line: usize, message: String },
}

pub type Result<T> = std::result::Result<T, ParserError>;

// ============================================================================
// Protein2Ipr Parser
// ============================================================================

/// Parser for protein2ipr.dat.gz file
pub struct Protein2IprParser {
    /// Current line number (for error reporting)
    line_number: usize,
}

impl Protein2IprParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self { line_number: 0 }
    }

    /// Parse a gzipped protein2ipr.dat file
    pub fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<ProteinMatch>> {
        let file = std::fs::File::open(path)?;
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);

        self.parse_reader(reader)
    }

    /// Parse protein2ipr data from a reader
    pub fn parse_reader<R: Read>(&mut self, reader: BufReader<R>) -> Result<Vec<ProteinMatch>> {
        let mut matches = Vec::new();
        self.line_number = 0;

        for line_result in reader.lines() {
            self.line_number += 1;
            let line = line_result?;

            // Skip empty lines and comments
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            match self.parse_line(&line) {
                Ok(protein_match) => matches.push(protein_match),
                Err(e) => {
                    // Log error but continue parsing
                    warn!("Skipping line {} due to parse error: {}", self.line_number, e);
                },
            }
        }

        debug!("Parsed {} protein matches from {} lines", matches.len(), self.line_number);

        Ok(matches)
    }

    /// Parse a single TSV line into a ProteinMatch
    ///
    /// Format: UniProtAccession MD5 Length InterProID InterProName SignatureDB SignatureAcc SignatureName Start Stop Score Status Date
    /// Example: P12345 abc123def456 154 IPR000001 Kringle Pfam PF00051 Kringle domain 10 100 1.2E-10 T 01-JAN-2024
    pub fn parse_line(&self, line: &str) -> Result<ProteinMatch> {
        let fields: Vec<&str> = line.split('\t').collect();

        // Minimum required fields: 13 (some fields may be optional)
        if fields.len() < 11 {
            return Err(ParserError::InvalidFormat {
                line: self.line_number,
                message: format!("Expected at least 11 fields, got {}", fields.len()),
            });
        }

        // Parse fields
        let uniprot_accession = fields[0].to_string();
        // fields[1] = MD5 (skip)
        // fields[2] = Length (skip)

        let interpro_id = fields[3].to_string();
        let interpro_name = fields[4].to_string();

        let signature_database = fields[5]
            .parse::<SignatureDatabase>()
            .map_err(|e| ParserError::InvalidSignatureDatabase(e))?;

        let signature_accession = fields[6].to_string();

        let signature_name = if !fields[7].is_empty() {
            Some(fields[7].to_string())
        } else {
            None
        };

        let start_position = fields[8]
            .parse::<i32>()
            .map_err(|e| ParserError::InvalidInteger {
                line: self.line_number,
                message: format!("Failed to parse start position: {}", e),
            })?;

        let end_position = fields[9]
            .parse::<i32>()
            .map_err(|e| ParserError::InvalidInteger {
                line: self.line_number,
                message: format!("Failed to parse end position: {}", e),
            })?;

        // E-value and score are optional
        let e_value = if fields.len() > 10 && !fields[10].is_empty() {
            Some(
                fields[10]
                    .parse::<f64>()
                    .map_err(|e| ParserError::InvalidFloat {
                        line: self.line_number,
                        message: format!("Failed to parse e-value: {}", e),
                    })?,
            )
        } else {
            None
        };

        let score = if fields.len() > 11 && !fields[11].is_empty() {
            Some(
                fields[11]
                    .parse::<f64>()
                    .map_err(|e| ParserError::InvalidFloat {
                        line: self.line_number,
                        message: format!("Failed to parse score: {}", e),
                    })?,
            )
        } else {
            None
        };

        Ok(ProteinMatch {
            uniprot_accession,
            interpro_id,
            interpro_name,
            signature_database,
            signature_accession,
            signature_name,
            start_position,
            end_position,
            e_value,
            score,
        })
    }
}

impl Default for Protein2IprParser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Entry List Parser
// ============================================================================

/// Parser for entry.list file
pub struct EntryListParser {
    line_number: usize,
}

impl EntryListParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self { line_number: 0 }
    }

    /// Parse entry.list file
    pub fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<InterProEntry>> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);

        self.parse_reader(reader)
    }

    /// Parse entry.list from a reader
    pub fn parse_reader<R: Read>(&mut self, reader: BufReader<R>) -> Result<Vec<InterProEntry>> {
        let mut entries = Vec::new();
        self.line_number = 0;

        for line_result in reader.lines() {
            self.line_number += 1;
            let line = line_result?;

            // Skip empty lines and comments
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            match self.parse_line(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Skipping line {} due to parse error: {}", self.line_number, e);
                },
            }
        }

        debug!("Parsed {} InterPro entries from {} lines", entries.len(), self.line_number);

        Ok(entries)
    }

    /// Parse a single TSV line into an InterProEntry
    ///
    /// Format: ENTRY_AC ENTRY_TYPE ENTRY_NAME
    /// Example: IPR000001 Domain Kringle
    pub fn parse_line(&self, line: &str) -> Result<InterProEntry> {
        let fields: Vec<&str> = line.split('\t').collect();

        if fields.len() < 3 {
            return Err(ParserError::InvalidFormat {
                line: self.line_number,
                message: format!("Expected at least 3 fields, got {}", fields.len()),
            });
        }

        let interpro_id = fields[0].to_string();

        let entry_type = EntryType::from_str(fields[1]).map_err(|e| {
            ParserError::InvalidEntryType(format!("Line {}: {}", self.line_number, e))
        })?;

        let name = fields[2].to_string();

        // Short name and description may be in additional fields
        let short_name = if fields.len() > 3 && !fields[3].is_empty() {
            Some(fields[3].to_string())
        } else {
            None
        };

        let description = if fields.len() > 4 && !fields[4].is_empty() {
            Some(fields[4].to_string())
        } else {
            None
        };

        Ok(InterProEntry {
            interpro_id,
            entry_type,
            name,
            short_name,
            description,
        })
    }
}

impl Default for EntryListParser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_protein_match_line() {
        let parser = Protein2IprParser::new();
        let line = "P12345\tabc123\t154\tIPR000001\tKringle\tPfam\tPF00051\tKringle domain\t10\t100\t1.2E-10\tT\t01-JAN-2024";

        let result = parser.parse_line(line);
        assert!(result.is_ok());

        let match_data = result.unwrap();
        assert_eq!(match_data.uniprot_accession, "P12345");
        assert_eq!(match_data.interpro_id, "IPR000001");
        assert_eq!(match_data.interpro_name, "Kringle");
        assert_eq!(match_data.signature_database, SignatureDatabase::Pfam);
        assert_eq!(match_data.signature_accession, "PF00051");
        assert_eq!(match_data.signature_name, Some("Kringle domain".to_string()));
        assert_eq!(match_data.start_position, 10);
        assert_eq!(match_data.end_position, 100);
        assert!(match_data.e_value.is_some());
        assert_eq!(match_data.e_value.unwrap(), 1.2E-10);
    }

    #[test]
    fn test_parse_protein_match_minimal() {
        let parser = Protein2IprParser::new();
        let line = "Q9Y6K9\txyz789\t200\tIPR029058\tAlpha/Beta hydrolase fold\tSMART\tSM00130\t\t50\t150\t\t";

        let result = parser.parse_line(line);
        assert!(result.is_ok());

        let match_data = result.unwrap();
        assert_eq!(match_data.uniprot_accession, "Q9Y6K9");
        assert_eq!(match_data.signature_database, SignatureDatabase::Smart);
        assert_eq!(match_data.signature_name, None);
        assert_eq!(match_data.e_value, None);
        assert_eq!(match_data.score, None);
    }

    #[test]
    fn test_parse_protein_match_invalid_fields() {
        let parser = Protein2IprParser::new();
        let line = "P12345\tabc123"; // Too few fields

        let result = parser.parse_line(line);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParserError::InvalidFormat { .. }));
    }

    #[test]
    fn test_parse_entry_list_line() {
        let parser = EntryListParser::new();
        let line = "IPR000001\tDomain\tKringle";

        let result = parser.parse_line(line);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.interpro_id, "IPR000001");
        assert_eq!(entry.entry_type, EntryType::Domain);
        assert_eq!(entry.name, "Kringle");
    }

    #[test]
    fn test_parse_entry_list_with_description() {
        let parser = EntryListParser::new();
        let line = "IPR000001\tDomain\tKringle\tKringle dom\tA protein domain found in blood clotting proteins";

        let result = parser.parse_line(line);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.interpro_id, "IPR000001");
        assert_eq!(entry.short_name, Some("Kringle dom".to_string()));
        assert_eq!(
            entry.description,
            Some("A protein domain found in blood clotting proteins".to_string())
        );
    }

    #[test]
    fn test_parse_entry_list_invalid_type() {
        let parser = EntryListParser::new();
        let line = "IPR000001\tInvalidType\tKringle";

        let result = parser.parse_line(line);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParserError::InvalidEntryType(_)));
    }

    #[test]
    fn test_parse_multiple_databases() {
        let parser = Protein2IprParser::new();

        let databases = vec![
            ("Pfam", SignatureDatabase::Pfam),
            ("SMART", SignatureDatabase::Smart),
            ("PROSITE", SignatureDatabase::Prosite),
            ("PANTHER", SignatureDatabase::Panther),
        ];

        for (db_str, expected) in databases {
            let line =
                format!("P12345\tabc\t100\tIPR000001\tTest\t{}\tSIG001\tTest sig\t10\t50", db_str);
            let result = parser.parse_line(&line);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().signature_database, expected);
        }
    }
}
