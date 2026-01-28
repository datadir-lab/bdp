//! UniProt parser adapter for generic ETL framework
//!
//! Implements DataSourceParser and RecordFormatter traits for UniProt proteins.

use anyhow::{Context, Result};

use super::models::UniProtEntry;
use super::parser::DatParser;
use crate::ingest::framework::{DataSourceParser, GenericRecord, RecordFormatter};

/// UniProt data source parser
pub struct UniProtParser {
    parser: DatParser,
}

impl UniProtParser {
    pub fn new() -> Self {
        Self {
            parser: DatParser::new(),
        }
    }
}

#[async_trait::async_trait]
impl DataSourceParser for UniProtParser {
    async fn parse_range(
        &self,
        data: &[u8],
        start_offset: usize,
        end_offset: usize,
    ) -> Result<Vec<GenericRecord>> {
        // Parse all entries from the data
        let all_entries = self.parser.parse_bytes(data)
            .context("Failed to parse UniProt DAT data")?;

        // Get the requested range
        let start = start_offset.min(all_entries.len());
        let end = (end_offset + 1).min(all_entries.len());

        if start >= end {
            return Ok(Vec::new());
        }

        // Convert the range to generic records
        let results: Vec<GenericRecord> = all_entries[start..end]
            .iter()
            .enumerate()
            .map(|(idx, entry)| entry_to_generic_record(entry, start + idx))
            .collect();

        Ok(results)
    }

    async fn count_records(&self, data: &[u8]) -> Result<Option<usize>> {
        let content = String::from_utf8_lossy(data);

        // Count occurrences of "//" which marks end of each record
        let count = content.matches("\n//").count();

        Ok(Some(count))
    }

    fn record_type(&self) -> &str {
        "protein"
    }

    fn output_formats(&self) -> Vec<String> {
        vec!["dat".to_string(), "fasta".to_string(), "json".to_string()]
    }
}

/// Convert UniProtEntry to GenericRecord
fn entry_to_generic_record(entry: &UniProtEntry, offset: usize) -> GenericRecord {
    // Compute sequence MD5
    let sequence_md5 = {
        use crate::ingest::framework::compute_md5;
        compute_md5(entry.sequence.as_bytes())
    };

    // Convert to JSONB-compatible structure
    let record_data = serde_json::json!({
        "accession": entry.accession,
        "entry_name": entry.entry_name,
        "protein_name": entry.protein_name,
        "gene_name": entry.gene_name,
        "organism_name": entry.organism_name,
        "taxonomy_id": entry.taxonomy_id,
        "sequence": entry.sequence,
        "sequence_length": entry.sequence_length,
        "mass_da": entry.mass_da,
        "release_date": entry.release_date.to_string(),
    });

    // Compute content MD5
    let content_md5 = {
        use crate::ingest::framework::compute_md5;
        // record_data is a serde_json::Value that we just created, serialization should not fail
        let json_str = serde_json::to_string(&record_data)
            .unwrap_or_else(|e| {
                tracing::error!("Failed to serialize record_data for MD5: {}", e);
                String::new()
            });
        compute_md5(json_str.as_bytes())
    };

    GenericRecord {
        record_type: "protein".to_string(),
        record_identifier: entry.accession.to_lowercase(),
        record_name: Some(entry.entry_name.to_lowercase()),
        record_data,
        content_md5: Some(content_md5),
        sequence_md5: Some(sequence_md5),
        source_file: None,
        source_offset: Some(offset as i64),
    }
}

/// UniProt record formatter for generating files
pub struct UniProtFormatter;

#[async_trait::async_trait]
impl RecordFormatter for UniProtFormatter {
    async fn format_record(
        &self,
        record: &GenericRecord,
        format: &str,
    ) -> Result<(Vec<u8>, String)> {
        match format {
            "fasta" => {
                let accession = record
                    .record_data
                    .get("accession")
                    .and_then(|v| v.as_str())
                    .context("Missing accession")?;

                let entry_name = record
                    .record_data
                    .get("entry_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");

                let protein_name = record
                    .record_data
                    .get("protein_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let organism = record
                    .record_data
                    .get("organism_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let taxonomy_id = record
                    .record_data
                    .get("taxonomy_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                let sequence = record
                    .record_data
                    .get("sequence")
                    .and_then(|v| v.as_str())
                    .context("Missing sequence")?;

                // Format: >sp|accession|entry_name protein_name OS=organism OX=taxonomy_id
                let fasta = format!(
                    ">sp|{}|{} {} OS={} OX={}\n{}\n",
                    accession, entry_name, protein_name, organism, taxonomy_id, sequence
                );

                Ok((fasta.into_bytes(), "text/plain".to_string()))
            }

            "json" => {
                let json = serde_json::to_string_pretty(&record.record_data)
                    .context("Failed to serialize to JSON")?;

                Ok((json.into_bytes(), "application/json".to_string()))
            }

            "dat" => {
                // For DAT format, we would need to reconstruct the original format
                // For now, just return JSON
                let json = serde_json::to_string_pretty(&record.record_data)
                    .context("Failed to serialize to JSON")?;

                Ok((json.into_bytes(), "text/plain".to_string()))
            }

            _ => anyhow::bail!("Unsupported format: {}", format),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_count_records() {
        let data = b"ID   P01234\nAC   P01234\n//\nID   P56789\nAC   P56789\n//\n";

        let parser = UniProtParser::new();
        let count = parser.count_records(data).await.unwrap();

        assert_eq!(count, Some(2));
    }
}
