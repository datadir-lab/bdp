//! Generic data source parser trait
//!
//! Implement this trait for any data source type (proteins, genomes, compounds, etc.)

use anyhow::Result;
use async_trait::async_trait;

use super::types::GenericRecord;

/// Generic parser for any data source
#[async_trait]
pub trait DataSourceParser: Send + Sync {
    /// Parse a specific range of records from raw data
    ///
    /// # Arguments
    /// * `data` - Raw file data (may be decompressed)
    /// * `start_offset` - Start parsing from this record index
    /// * `end_offset` - Stop parsing at this record index (inclusive)
    ///
    /// # Returns
    /// Vector of parsed records in the specified range
    async fn parse_range(
        &self,
        data: &[u8],
        start_offset: usize,
        end_offset: usize,
    ) -> Result<Vec<GenericRecord>>;

    /// Get total record count without full parsing (optional optimization)
    ///
    /// Default implementation returns None, forcing full parse to count.
    /// Implementations can override if they can count records efficiently.
    async fn count_records(&self, data: &[u8]) -> Result<Option<usize>> {
        Ok(None)
    }

    /// Get the record type identifier for this parser
    ///
    /// E.g., "protein", "genome", "compound", etc.
    fn record_type(&self) -> &str;

    /// Get file formats this parser can generate for each record
    ///
    /// E.g., ["dat", "fasta", "json"] for proteins
    /// E.g., ["fasta", "gff", "json"] for genomes
    fn output_formats(&self) -> Vec<String>;
}

/// Helper trait for converting parsed records into specific file formats
#[async_trait]
pub trait RecordFormatter: Send + Sync {
    /// Generate file content for a specific format
    ///
    /// # Arguments
    /// * `record` - The generic record to format
    /// * `format` - The desired output format ("fasta", "json", etc.)
    ///
    /// # Returns
    /// Tuple of (content bytes, content type)
    async fn format_record(
        &self,
        record: &GenericRecord,
        format: &str,
    ) -> Result<(Vec<u8>, String)>;
}
