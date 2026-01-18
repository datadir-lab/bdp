//! UniProt DAT file parser
//!
//! Parses UniProt flat file format (DAT) with support for gzip compression.
//! See: https://web.expasy.org/docs/userman.html

use anyhow::{Context, Result};
use chrono::NaiveDate;
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use super::models::UniProtEntry;

/// Parser for UniProt DAT files
pub struct DatParser {
    /// Maximum number of entries to parse (None for unlimited)
    limit: Option<usize>,
}

impl DatParser {
    /// Create a new DAT parser with no limit
    pub fn new() -> Self {
        Self { limit: None }
    }

    /// Create a new DAT parser with a limit
    pub fn with_limit(limit: usize) -> Self {
        Self { limit: Some(limit) }
    }

    /// Parse a DAT file from a file path
    ///
    /// Automatically handles .gz compression based on file extension
    pub fn parse_file(&self, path: &Path) -> Result<Vec<UniProtEntry>> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            let decoder = GzDecoder::new(file);
            self.parse_reader(decoder)
        } else {
            self.parse_reader(file)
        }
    }

    /// Parse DAT data from bytes
    pub fn parse_bytes(&self, data: &[u8]) -> Result<Vec<UniProtEntry>> {
        // Try to decompress as gzip first
        if let Ok(decoder) = GzDecoder::new(data).read_to_end(&mut Vec::new()) {
            // Successfully decompressed as gzip
            let decompressed = {
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            };
            self.parse_reader(&decompressed[..])
        } else {
            // Not gzipped, parse as-is
            self.parse_reader(data)
        }
    }

    /// Parse DAT data from a reader
    fn parse_reader<R: Read>(&self, reader: R) -> Result<Vec<UniProtEntry>> {
        let buf_reader = BufReader::new(reader);
        let mut entries = Vec::new();
        let mut current_entry = EntryBuilder::new();
        let mut in_sequence = false;

        for line in buf_reader.lines() {
            let line = line.context("Failed to read line")?;

            // Check if we've reached the limit
            if let Some(limit) = self.limit {
                if entries.len() >= limit {
                    break;
                }
            }

            // End of entry
            if line.starts_with("//") {
                if let Some(entry) = current_entry.build()? {
                    entries.push(entry);
                }
                current_entry = EntryBuilder::new();
                in_sequence = false;
                continue;
            }

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse line based on type
            if line.starts_with("ID   ") {
                current_entry.parse_id_line(&line)?;
            } else if line.starts_with("AC   ") {
                current_entry.parse_ac_line(&line)?;
            } else if line.starts_with("DT   ") && line.contains("integrated into") {
                current_entry.parse_dt_line(&line)?;
            } else if line.starts_with("DE   ") && line.contains("RecName: Full=") {
                current_entry.parse_de_line(&line)?;
            } else if line.starts_with("GN   ") && line.contains("Name=") {
                current_entry.parse_gn_line(&line)?;
            } else if line.starts_with("OS   ") {
                current_entry.parse_os_line(&line)?;
            } else if line.starts_with("OX   ") && line.contains("NCBI_TaxID=") {
                current_entry.parse_ox_line(&line)?;
            } else if line.starts_with("SQ   ") {
                current_entry.parse_sq_line(&line)?;
                in_sequence = true;
            } else if in_sequence && line.starts_with("     ") {
                current_entry.parse_sequence_line(&line);
            }
        }

        Ok(entries)
    }
}

impl Default for DatParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing UniProtEntry from DAT lines
#[derive(Default)]
struct EntryBuilder {
    accession: Option<String>,
    entry_name: Option<String>,
    protein_name: Option<String>,
    gene_name: Option<String>,
    organism_name: Option<String>,
    taxonomy_id: Option<i32>,
    sequence_length: Option<i32>,
    mass_da: Option<i64>,
    release_date: Option<NaiveDate>,
    sequence: String,
}

impl EntryBuilder {
    fn new() -> Self {
        Self::default()
    }

    /// Parse ID line: ID   ENTRY_NAME   ...
    fn parse_id_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            self.entry_name = Some(parts[1].to_string());
        }
        Ok(())
    }

    /// Parse AC line: AC   P12345; P67890;
    fn parse_ac_line(&mut self, line: &str) -> Result<()> {
        if self.accession.is_none() {
            let ac_part = line.trim_start_matches("AC   ");
            if let Some(first_ac) = ac_part.split(';').next() {
                self.accession = Some(first_ac.trim().to_string());
            }
        }
        Ok(())
    }

    /// Parse DT line: DT   01-JAN-1990, integrated into UniProtKB/Swiss-Prot.
    fn parse_dt_line(&mut self, line: &str) -> Result<()> {
        if self.release_date.is_none() {
            let dt_part = line.trim_start_matches("DT   ");
            if let Some(date_str) = dt_part.split(',').next() {
                self.release_date = Some(parse_date(date_str.trim())?);
            }
        }
        Ok(())
    }

    /// Parse DE line: DE   RecName: Full=Protein name;
    fn parse_de_line(&mut self, line: &str) -> Result<()> {
        if self.protein_name.is_none() {
            if let Some(start) = line.find("RecName: Full=") {
                let name_part = &line[start + 14..];
                if let Some(end) = name_part.find([';', '{']) {
                    self.protein_name = Some(name_part[..end].trim().to_string());
                } else {
                    self.protein_name = Some(name_part.trim().to_string());
                }
            }
        }
        Ok(())
    }

    /// Parse GN line: GN   Name=GENE; ...
    fn parse_gn_line(&mut self, line: &str) -> Result<()> {
        if self.gene_name.is_none() {
            if let Some(start) = line.find("Name=") {
                let name_part = &line[start + 5..];
                if let Some(end) = name_part.find([';', ' ', '{']) {
                    self.gene_name = Some(name_part[..end].trim().to_string());
                }
            }
        }
        Ok(())
    }

    /// Parse OS line: OS   Homo sapiens (Human).
    fn parse_os_line(&mut self, line: &str) -> Result<()> {
        let os_part = line.trim_start_matches("OS   ").trim();
        if let Some(existing) = &mut self.organism_name {
            existing.push(' ');
            existing.push_str(os_part);
        } else {
            self.organism_name = Some(os_part.to_string());
        }
        // Remove trailing period
        if let Some(name) = &mut self.organism_name {
            *name = name.trim_end_matches('.').to_string();
        }
        Ok(())
    }

    /// Parse OX line: OX   NCBI_TaxID=9606;
    fn parse_ox_line(&mut self, line: &str) -> Result<()> {
        if let Some(start) = line.find("NCBI_TaxID=") {
            let tax_part = &line[start + 11..];
            if let Some(end) = tax_part.find([';', ' ']) {
                let tax_str = &tax_part[..end];
                self.taxonomy_id = Some(tax_str.parse().context("Failed to parse taxonomy ID")?);
            }
        }
        Ok(())
    }

    /// Parse SQ line: SQ   SEQUENCE   123 AA;  14078 MW;  ...
    fn parse_sq_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Find sequence length (e.g., "123" before "AA;")
        for i in 0..parts.len() {
            if parts[i] == "AA;" && i > 0 {
                if let Ok(length) = parts[i - 1].parse::<i32>() {
                    self.sequence_length = Some(length);
                }
            }
            if parts[i] == "MW;" && i > 0 {
                if let Ok(mass) = parts[i - 1].parse::<i64>() {
                    self.mass_da = Some(mass);
                }
            }
        }
        Ok(())
    }

    /// Parse sequence line (spaces and sequence data)
    fn parse_sequence_line(&mut self, line: &str) {
        // Sequence lines contain amino acids separated by spaces
        let seq_part = line.trim();
        for chunk in seq_part.split_whitespace() {
            self.sequence.push_str(chunk);
        }
    }

    /// Build the final UniProtEntry
    fn build(self) -> Result<Option<UniProtEntry>> {
        // Skip entries with missing required fields
        let accession = match self.accession {
            Some(a) => a,
            None => return Ok(None),
        };
        let entry_name = match self.entry_name {
            Some(e) => e,
            None => return Ok(None),
        };
        let protein_name = match self.protein_name {
            Some(p) => p,
            None => return Ok(None),
        };
        let organism_name = match self.organism_name {
            Some(o) => o,
            None => return Ok(None),
        };
        let taxonomy_id = match self.taxonomy_id {
            Some(t) => t,
            None => return Ok(None),
        };
        let sequence_length = match self.sequence_length {
            Some(l) => l,
            None => return Ok(None),
        };
        let mass_da = match self.mass_da {
            Some(m) => m,
            None => return Ok(None),
        };
        let release_date = match self.release_date {
            Some(d) => d,
            None => return Ok(None),
        };

        if self.sequence.is_empty() {
            return Ok(None);
        }

        Ok(Some(UniProtEntry {
            accession,
            entry_name,
            protein_name,
            gene_name: self.gene_name,
            organism_name,
            taxonomy_id,
            sequence: self.sequence,
            sequence_length,
            mass_da,
            release_date,
        }))
    }
}

/// Parse date in format "01-JAN-1990"
fn parse_date(date_str: &str) -> Result<NaiveDate> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid date format: {}", date_str);
    }

    let day: u32 = parts[0].parse().context("Failed to parse day")?;
    let month = match parts[1] {
        "JAN" => 1,
        "FEB" => 2,
        "MAR" => 3,
        "APR" => 4,
        "MAY" => 5,
        "JUN" => 6,
        "JUL" => 7,
        "AUG" => 8,
        "SEP" => 9,
        "OCT" => 10,
        "NOV" => 11,
        "DEC" => 12,
        _ => anyhow::bail!("Invalid month: {}", parts[1]),
    };
    let year: i32 = parts[2].parse().context("Failed to parse year")?;

    NaiveDate::from_ymd_opt(year, month, day)
        .with_context(|| format!("Invalid date: {}", date_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let date = parse_date("01-JAN-1990").unwrap();
        assert_eq!(date.year(), 1990);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);

        let date = parse_date("31-DEC-2024").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 31);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("01-XXX-1990").is_err());
    }

    #[test]
    fn test_parser_with_limit() {
        let parser = DatParser::with_limit(5);
        assert_eq!(parser.limit, Some(5));
    }

    #[test]
    fn test_entry_builder_accession() {
        let mut builder = EntryBuilder::new();
        builder.parse_ac_line("AC   P12345; P67890;").unwrap();
        assert_eq!(builder.accession, Some("P12345".to_string()));
    }

    #[test]
    fn test_entry_builder_id() {
        let mut builder = EntryBuilder::new();
        builder.parse_id_line("ID   TEST_HUMAN     Reviewed;         100 AA.").unwrap();
        assert_eq!(builder.entry_name, Some("TEST_HUMAN".to_string()));
    }

    #[test]
    fn test_entry_builder_protein_name() {
        let mut builder = EntryBuilder::new();
        builder
            .parse_de_line("DE   RecName: Full=Test protein;")
            .unwrap();
        assert_eq!(builder.protein_name, Some("Test protein".to_string()));
    }

    #[test]
    fn test_entry_builder_protein_name_with_flags() {
        let mut builder = EntryBuilder::new();
        builder
            .parse_de_line("DE   RecName: Full=Test protein {ECO:0000255};")
            .unwrap();
        assert_eq!(builder.protein_name, Some("Test protein".to_string()));
    }

    #[test]
    fn test_entry_builder_gene_name() {
        let mut builder = EntryBuilder::new();
        builder.parse_gn_line("GN   Name=TEST; Synonyms=TST;").unwrap();
        assert_eq!(builder.gene_name, Some("TEST".to_string()));
    }

    #[test]
    fn test_entry_builder_organism() {
        let mut builder = EntryBuilder::new();
        builder.parse_os_line("OS   Homo sapiens (Human).").unwrap();
        assert_eq!(builder.organism_name, Some("Homo sapiens (Human)".to_string()));
    }

    #[test]
    fn test_entry_builder_taxonomy() {
        let mut builder = EntryBuilder::new();
        builder.parse_ox_line("OX   NCBI_TaxID=9606;").unwrap();
        assert_eq!(builder.taxonomy_id, Some(9606));
    }

    #[test]
    fn test_entry_builder_sequence_info() {
        let mut builder = EntryBuilder::new();
        builder
            .parse_sq_line("SQ   SEQUENCE   123 AA;  14078 MW;  B4840739BF7D4121 CRC64;")
            .unwrap();
        assert_eq!(builder.sequence_length, Some(123));
        assert_eq!(builder.mass_da, Some(14078));
    }

    #[test]
    fn test_entry_builder_sequence_line() {
        let mut builder = EntryBuilder::new();
        builder.parse_sequence_line("     MKTAYIAKQR QISFVKSHFS RQLEERLGLI");
        assert_eq!(builder.sequence, "MKTAYIAKQRQISFVKSHFSRQLEERLGLI");
    }
}
