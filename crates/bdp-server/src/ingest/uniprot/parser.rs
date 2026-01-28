//! UniProt DAT file parser
//!
//! Parses UniProt flat file format (DAT) with support for gzip and tar.gz compression.
//! See: https://web.expasy.org/docs/userman.html

// TODO: Parse additional UniProt DAT fields for comprehensive protein metadata
//
// CRITICAL ARCHITECTURE DECISION:
// - Organism data should be stored under 'ncbi' organization (not 'uniprot')
//   because NCBI Taxonomy is the canonical source
// - UniProt proteins should REFERENCE organisms from ncbi:org/{taxonomy_id}
// - This enables proper versioning: NCBI taxonomy updates independently from UniProt releases
//
// HIGH PRIORITY (Phase 2):
// - FT (Feature Table) - Protein domains, active sites, binding sites, PTMs, variants
//   Storage: New table protein_features (feature_type, start_pos, end_pos, description)
// - DR (Database Cross-References) - Links to PDB, GO, InterPro, KEGG, Pfam, RefSeq
//   Storage: New table protein_cross_references (database_name, database_id, metadata JSONB)
//   IMPORTANT: RefSeq and Gene IDs should link to future NCBI data sources
// - CC (Comments) - Structured annotations (FUNCTION, SUBCELLULAR LOCATION, DISEASE, etc.)
//   Storage: Add comments JSONB field to protein_metadata OR new protein_annotations table
// - PE (Protein Existence) - Evidence level 1-5
//   Storage: Add protein_existence INT field to protein_metadata
// - KW (Keywords) - Controlled vocabulary terms for functional classification
//   Storage: Many-to-many relationship with keywords table
// - DE Alternative Names - AltName, SubName, Short names, EC numbers
//   Storage: Add alternative_names TEXT[] and ec_numbers TEXT[] to protein_metadata
// - ALL REMAINING FIELDS - Capture everything in additional_metadata JSONB for now
//   This preserves all data even if we haven't structured it yet
//
// MEDIUM PRIORITY (Phase 3):
// - OG (Organelle) - Mitochondrion, Plastid, Plasmid origin
// - OH (Organism Host) - Viral host organisms
// - DT (Date) - Entry history (created, last sequence update, last annotation update)
//
// LOWER PRIORITY (Phase 4):
// - References (RN, RP, RC, RX, RG, RA, RT, RL) - Publication metadata
//   Consider linking to PubMed IDs from future NCBI PubMed data source
//
// See docs/uniprot-ingestion-optimization-plan.md for detailed specifications
// and SQL schema suggestions for each field type.

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use tar::Archive;

use super::models::{Comment, CrossReference, ProteinFeature, Publication, UniProtEntry};

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
    ///
    /// Handles:
    /// - Plain DAT files
    /// - Gzipped DAT files (.dat.gz)
    /// - Tar-gzipped archives (.tar.gz) containing DAT files
    pub fn parse_bytes(&self, data: &[u8]) -> Result<Vec<UniProtEntry>> {
        // Try to decompress as gzip
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();

        match decoder.read_to_end(&mut decompressed) {
            Ok(size) => {
                tracing::info!(
                    compressed_size = data.len(),
                    decompressed_size = size,
                    "Decompressed gzip data"
                );

                // Try to parse as tar archive first
                match self.parse_tar_archive(&decompressed) {
                    Ok(entries) => {
                        tracing::info!("Successfully extracted and parsed DAT from tar archive");
                        Ok(entries)
                    }
                    Err(tar_err) => {
                        // Not a valid tar archive, try parsing as plain DAT file
                        tracing::info!(
                            error = %tar_err,
                            "Failed to parse as tar, trying as plain DAT file"
                        );
                        self.parse_reader(&decompressed[..])
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to decompress as gzip, trying plain DAT");
                // Not gzipped, try parsing as plain DAT
                self.parse_reader(data)
            }
        }
    }

    /// Parse UniProt DAT format from a string
    ///
    /// Convenience method for testing and small datasets.
    /// Handles uncompressed DAT format text.
    pub fn parse_dat_string(&self, data: &str) -> Result<Vec<UniProtEntry>> {
        // For string input, parse directly as DAT (no decompression)
        self.parse_reader(data.as_bytes())
    }

    /// Check if data is a tar archive by looking for tar magic number
    #[allow(dead_code)]
    fn is_tar_archive(&self, data: &[u8]) -> bool {
        if data.len() < 512 {
            tracing::debug!(size = data.len(), "Data too small to be tar archive");
            return false;
        }

        // Tar files have "ustar" magic at offset 257 (with null terminator at 262, version at 263-264)
        // Check both old tar (ustar\0) and new tar (ustar  00)
        let has_ustar = &data[257..262] == b"ustar";

        if has_ustar {
            tracing::debug!("Found ustar magic number at offset 257");
        } else {
            // Show what we actually found for debugging
            if data.len() >= 262 {
                let magic = String::from_utf8_lossy(&data[257..262]);
                tracing::debug!(
                    found_magic = %magic,
                    "No ustar magic found, showing first 20 bytes as hex: {:02x?}",
                    &data[0..20.min(data.len())]
                );
            }
        }

        has_ustar
    }

    /// Extract and parse the first .dat file from a tar archive
    fn parse_tar_archive(&self, tar_data: &[u8]) -> Result<Vec<UniProtEntry>> {
        let mut archive = Archive::new(tar_data);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            // Look for .dat files (not .dat.gz, we already decompressed)
            if path.extension().and_then(|s| s.to_str()) == Some("dat") {
                tracing::info!("Extracting DAT file from tar: {}", path.display());
                let mut dat_content = Vec::new();
                entry.read_to_end(&mut dat_content)?;
                return self.parse_reader(&dat_content[..]);
            }
        }

        Err(anyhow!("No .dat file found in tar archive"))
    }

    /// Parse a specific range of entries (for streaming/parallel processing)
    ///
    /// # Arguments
    /// * `data` - Raw DAT file data (will be decompressed if needed)
    /// * `start_offset` - Start parsing from this entry index (0-based)
    /// * `end_offset` - Stop parsing at this entry index (inclusive)
    pub fn parse_range(&self, data: &[u8], start_offset: usize, end_offset: usize) -> Result<Vec<UniProtEntry>> {
        tracing::info!(
            start_offset,
            end_offset,
            input_size = data.len(),
            "parse_range called"
        );

        // First decompress/extract if needed (same as parse_bytes)
        let dat_data = self.extract_dat_data(data)?;

        tracing::info!(
            extracted_size = dat_data.len(),
            starts_with_id = dat_data.starts_with(b"ID   "),
            "Extracted DAT data"
        );

        // Now parse only the requested range
        let buf_reader = BufReader::new(&dat_data[..]);
        let mut entries = Vec::new();
        let mut current_entry = EntryBuilder::new();
        let mut in_sequence = false;
        let mut entry_index = 0;
        let mut lines_processed = 0;
        let mut entries_skipped_no_build = 0;

        for line in buf_reader.lines() {
            let line = line.context("Failed to read line")?;

            // End of entry
            if line.starts_with("//") {
                // Check if the entry we just finished is in our range
                if entry_index >= start_offset && entry_index <= end_offset {
                    if entry_index == start_offset {
                        tracing::info!(
                            entry_index,
                            lines_processed,
                            "Processing first entry in range"
                        );
                    }

                    match current_entry.build()? {
                        Some(entry) => {
                            entries.push(entry);
                            if entries.len() == 1 {
                                tracing::info!(entry_index, "Successfully built first entry!");
                            }
                        }
                        None => {
                            entries_skipped_no_build += 1;
                            if entries_skipped_no_build <= 3 || entry_index == start_offset {
                                tracing::warn!(
                                    entry_index,
                                    lines_processed_for_entry = lines_processed,
                                    "Entry skipped - build() returned None (missing required fields)"
                                );
                            }
                        }
                    }
                }

                entry_index += 1;

                // Stop if we've passed the end offset
                if entry_index > end_offset {
                    break;
                }

                current_entry = EntryBuilder::new();
                in_sequence = false;
                lines_processed = 0;
                continue;
            }

            // Only process lines for entries we care about
            if entry_index >= start_offset && entry_index <= end_offset {
                self.process_line(&line, &mut current_entry, &mut in_sequence)?;
                lines_processed += 1;
            }
        }

        if entries.is_empty() && entries_skipped_no_build > 0 {
            tracing::warn!(
                start_offset,
                end_offset,
                entries_skipped_no_build,
                "No entries parsed - all entries skipped due to missing required fields"
            );
        }

        Ok(entries)
    }

    /// Parse a specific range of entries from pre-decompressed DAT data
    ///
    /// This method skips the decompression step and assumes data is already in plain DAT format.
    /// Use this when reading from the cache to avoid redundant decompression.
    ///
    /// # Arguments
    /// * `dat_data` - Already decompressed DAT file data (plain text format)
    /// * `start_offset` - Start parsing from this entry index (0-based)
    /// * `end_offset` - Stop parsing at this entry index (inclusive)
    pub fn parse_range_predecompressed(&self, dat_data: &[u8], start_offset: usize, end_offset: usize) -> Result<Vec<UniProtEntry>> {
        tracing::info!(
            start_offset,
            end_offset,
            input_size = dat_data.len(),
            "parse_range_predecompressed called (skipping extraction)"
        );

        // Validate that this is actually DAT format
        if !dat_data.starts_with(b"ID   ") {
            anyhow::bail!("Data does not appear to be decompressed DAT format (should start with 'ID   ')");
        }

        // Parse the requested range directly (no extraction needed)
        let buf_reader = BufReader::new(&dat_data[..]);
        let mut entries = Vec::new();
        let mut current_entry = EntryBuilder::new();
        let mut in_sequence = false;
        let mut entry_index = 0;
        let mut lines_processed = 0;
        let mut entries_skipped_no_build = 0;

        for line in buf_reader.lines() {
            let line = line.context("Failed to read line")?;

            // End of entry
            if line.starts_with("//") {
                // Check if the entry we just finished is in our range
                if entry_index >= start_offset && entry_index <= end_offset {
                    if entry_index == start_offset {
                        tracing::info!(
                            entry_index,
                            lines_processed,
                            "Processing first entry in range"
                        );
                    }

                    match current_entry.build()? {
                        Some(entry) => {
                            entries.push(entry);
                            if entries.len() == 1 {
                                tracing::info!(entry_index, "Successfully built first entry!");
                            }
                        }
                        None => {
                            entries_skipped_no_build += 1;
                            if entries_skipped_no_build <= 3 || entry_index == start_offset {
                                tracing::warn!(
                                    entry_index,
                                    lines_processed_for_entry = lines_processed,
                                    "Entry skipped - build() returned None (missing required fields)"
                                );
                            }
                        }
                    }
                }

                entry_index += 1;

                // Stop if we've passed the end offset
                if entry_index > end_offset {
                    break;
                }

                current_entry = EntryBuilder::new();
                in_sequence = false;
                lines_processed = 0;
                continue;
            }

            // Only process lines for entries we care about
            if entry_index >= start_offset && entry_index <= end_offset {
                self.process_line(&line, &mut current_entry, &mut in_sequence)?;
                lines_processed += 1;
            }
        }

        if entries.is_empty() && entries_skipped_no_build > 0 {
            tracing::warn!(
                start_offset,
                end_offset,
                entries_skipped_no_build,
                "No entries parsed - all entries skipped due to missing required fields"
            );
        }

        Ok(entries)
    }

    /// Count total entries from pre-decompressed DAT data (for efficiency)
    ///
    /// This method assumes the data is already decompressed DAT format.
    /// Use this when reading from cache to avoid redundant decompression.
    pub fn count_entries_predecompressed(&self, dat_data: &[u8]) -> Result<usize> {
        let buf_reader = BufReader::new(&dat_data[..]);
        let mut count = 0;

        for line in buf_reader.lines() {
            let line = line.context("Failed to read line")?;
            if line.starts_with("//") {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Count total entries without full parsing (for efficiency)
    pub fn count_entries(&self, data: &[u8]) -> Result<usize> {
        let dat_data = self.extract_dat_data(data)?;
        let buf_reader = BufReader::new(&dat_data[..]);
        let mut count = 0;

        for line in buf_reader.lines() {
            let line = line.context("Failed to read line")?;
            if line.starts_with("//") {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Extract DAT data from compressed/archived formats
    pub fn extract_dat_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Try to decompress as gzip first
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();

        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => {
                tracing::debug!(size = decompressed.len(), "Successfully decompressed gzip");
                // Try to extract from tar if it's an archive
                match self.extract_from_tar(&decompressed) {
                    Ok(dat_content) => {
                        tracing::debug!(size = dat_content.len(), "Successfully extracted DAT from tar");
                        Ok(dat_content)
                    },
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to extract from tar, trying as plain DAT");
                        // Check if decompressed data looks like DAT (starts with "ID   ")
                        if decompressed.starts_with(b"ID   ") {
                            Ok(decompressed)
                        } else {
                            Err(anyhow!("Decompressed data is not DAT format and tar extraction failed: {}", e))
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Not gzipped, trying tar extraction on raw data");
                // Data is not gzipped - might be already decompressed tar archive
                match self.extract_from_tar(data) {
                    Ok(dat_content) => {
                        tracing::debug!(size = dat_content.len(), "Successfully extracted DAT from tar (raw)");
                        Ok(dat_content)
                    }
                    Err(tar_err) => {
                        tracing::warn!(error = %tar_err, "Failed to extract from tar, trying as plain DAT");
                        // Check if raw data looks like DAT
                        if data.starts_with(b"ID   ") {
                            Ok(data.to_vec())
                        } else {
                            Err(anyhow!("Data is not gzipped, not a tar archive, and does not look like DAT format. Gzip error: {}, Tar error: {}", e, tar_err))
                        }
                    }
                }
            }
        }
    }

    /// Extract first .dat or .dat.gz file from tar archive
    fn extract_from_tar(&self, tar_data: &[u8]) -> Result<Vec<u8>> {
        let mut archive = Archive::new(tar_data);
        let mut found_files = Vec::new();

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            let path_str = path.to_string_lossy().to_string();
            found_files.push(path_str.clone());

            tracing::debug!(file = %path_str, "Found file in tar archive");

            // Check for .dat.gz files (need to decompress after extraction)
            if path_str.ends_with(".dat.gz") {
                tracing::info!(file = %path_str, "Extracting and decompressing .dat.gz file from tar");
                let mut compressed_content = Vec::new();
                entry.read_to_end(&mut compressed_content)?;

                // Decompress the .dat.gz content
                let mut decoder = GzDecoder::new(&compressed_content[..]);
                let mut dat_content = Vec::new();
                decoder.read_to_end(&mut dat_content)
                    .context(format!("Failed to decompress {} from tar", path_str))?;

                tracing::debug!(size = dat_content.len(), "Decompressed DAT content");
                return Ok(dat_content);
            }

            // Also check for plain .dat files
            if path.extension().and_then(|s| s.to_str()) == Some("dat") {
                tracing::info!(file = %path_str, "Extracting plain DAT file from tar");
                let mut dat_content = Vec::new();
                entry.read_to_end(&mut dat_content)?;
                return Ok(dat_content);
            }
        }

        tracing::warn!(files = ?found_files, "Files found in tar archive (none with .dat or .dat.gz extension)");
        Err(anyhow!("No .dat or .dat.gz file found in tar archive. Found {} files: {:?}", found_files.len(), found_files))
    }

    /// Process a single line during parsing
    fn process_line(&self, line: &str, current_entry: &mut EntryBuilder, in_sequence: &mut bool) -> Result<()> {
        // Skip empty lines
        if line.trim().is_empty() {
            return Ok(());
        }

        // Parse line based on type (matching parse_reader logic)
        if line.starts_with("ID   ") {
            current_entry.parse_id_line(line)?;
        } else if line.starts_with("AC   ") {
            current_entry.parse_ac_line(line)?;
        } else if line.starts_with("DT   ") {
            current_entry.parse_dt_line(line)?;
        } else if line.starts_with("DE   ") {
            current_entry.parse_de_line(line)?;
        } else if line.starts_with("GN   ") && line.contains("Name=") {
            current_entry.parse_gn_line(line)?;
        } else if line.starts_with("OS   ") {
            current_entry.parse_os_line(line)?;
        } else if line.starts_with("OX   ") && line.contains("NCBI_TaxID=") {
            current_entry.parse_ox_line(line)?;
        } else if line.starts_with("OC   ") {
            current_entry.parse_oc_line(line)?;
        } else if line.starts_with("FT   ") {
            current_entry.parse_ft_line(line)?;
        } else if line.starts_with("DR   ") {
            current_entry.parse_dr_line(line)?;
        } else if line.starts_with("CC   ") {
            current_entry.parse_cc_line(line)?;
        } else if line.starts_with("PE   ") {
            current_entry.parse_pe_line(line)?;
        } else if line.starts_with("KW   ") {
            current_entry.parse_kw_line(line)?;
        } else if line.starts_with("OG   ") {
            current_entry.parse_og_line(line)?;
        } else if line.starts_with("OH   ") {
            current_entry.parse_oh_line(line)?;
        } else if line.starts_with("RN   ") {
            current_entry.parse_rn_line(line)?;
        } else if line.starts_with("RP   ") {
            current_entry.parse_rp_line(line)?;
        } else if line.starts_with("RC   ") {
            current_entry.parse_rc_line(line)?;
        } else if line.starts_with("RX   ") {
            current_entry.parse_rx_line(line)?;
        } else if line.starts_with("RG   ") {
            current_entry.parse_rg_line(line)?;
        } else if line.starts_with("RA   ") {
            current_entry.parse_ra_line(line)?;
        } else if line.starts_with("RT   ") {
            current_entry.parse_rt_line(line)?;
        } else if line.starts_with("RL   ") {
            current_entry.parse_rl_line(line)?;
        } else if line.starts_with("SQ   ") {
            current_entry.parse_sq_line(line)?;
            *in_sequence = true;
        } else if *in_sequence && line.starts_with("     ") {
            current_entry.parse_sequence_line(line);
        }
        Ok(())
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
            } else if line.starts_with("DT   ") {
                current_entry.parse_dt_line(&line)?;
            } else if line.starts_with("DE   ") {
                current_entry.parse_de_line(&line)?;
            } else if line.starts_with("GN   ") && line.contains("Name=") {
                current_entry.parse_gn_line(&line)?;
            } else if line.starts_with("OS   ") {
                current_entry.parse_os_line(&line)?;
            } else if line.starts_with("OX   ") && line.contains("NCBI_TaxID=") {
                current_entry.parse_ox_line(&line)?;
            } else if line.starts_with("OC   ") {
                current_entry.parse_oc_line(&line)?;
            } else if line.starts_with("FT   ") {
                current_entry.parse_ft_line(&line)?;
            } else if line.starts_with("DR   ") {
                current_entry.parse_dr_line(&line)?;
            } else if line.starts_with("CC   ") {
                current_entry.parse_cc_line(&line)?;
            } else if line.starts_with("PE   ") {
                current_entry.parse_pe_line(&line)?;
            } else if line.starts_with("KW   ") {
                current_entry.parse_kw_line(&line)?;
            } else if line.starts_with("OG   ") {
                current_entry.parse_og_line(&line)?;
            } else if line.starts_with("OH   ") {
                current_entry.parse_oh_line(&line)?;
            } else if line.starts_with("RN   ") {
                current_entry.parse_rn_line(&line)?;
            } else if line.starts_with("RP   ") {
                current_entry.parse_rp_line(&line)?;
            } else if line.starts_with("RC   ") {
                current_entry.parse_rc_line(&line)?;
            } else if line.starts_with("RX   ") {
                current_entry.parse_rx_line(&line)?;
            } else if line.starts_with("RG   ") {
                current_entry.parse_rg_line(&line)?;
            } else if line.starts_with("RA   ") {
                current_entry.parse_ra_line(&line)?;
            } else if line.starts_with("RT   ") {
                current_entry.parse_rt_line(&line)?;
            } else if line.starts_with("RL   ") {
                current_entry.parse_rl_line(&line)?;
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

/// Helper to build Publication from R* lines
#[derive(Default, Debug)]
struct PublicationBuilder {
    reference_number: Option<i32>,
    position: Option<String>,
    comments: Vec<String>,
    pubmed_id: Option<String>,
    doi: Option<String>,
    author_group: Option<String>,
    authors: Vec<String>,
    title: Option<String>,
    location: Option<String>,
}

impl PublicationBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> Option<Publication> {
        let reference_number = self.reference_number?;
        Some(Publication {
            reference_number,
            position: self.position,
            comments: self.comments,
            pubmed_id: self.pubmed_id,
            doi: self.doi,
            author_group: self.author_group,
            authors: self.authors,
            title: self.title,
            location: self.location,
        })
    }
}

/// Builder for constructing UniProtEntry from DAT lines
#[derive(Default)]
struct EntryBuilder {
    // Core fields
    accession: Option<String>,
    entry_name: Option<String>,
    protein_name: Option<String>,
    gene_name: Option<String>,
    organism_name: Option<String>,
    taxonomy_id: Option<i32>,
    taxonomy_lineage: Vec<String>,
    sequence_length: Option<i32>,
    mass_da: Option<i64>,
    release_date: Option<NaiveDate>,
    sequence: String,

    // Extended metadata
    alternative_names: Vec<String>,
    ec_numbers: Vec<String>,
    features: Vec<ProteinFeature>,
    cross_references: Vec<CrossReference>,
    comments: Vec<Comment>,
    protein_existence: Option<i32>,
    keywords: Vec<String>,
    organelle: Option<String>,
    organism_hosts: Vec<String>,

    // Publications
    publications: Vec<Publication>,

    // Entry history dates
    entry_created: Option<NaiveDate>,
    sequence_updated: Option<NaiveDate>,
    annotation_updated: Option<NaiveDate>,

    // Parser state
    current_comment_topic: Option<String>,
    current_comment_text: String,
    current_publication: Option<PublicationBuilder>,
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
    /// DT   01-JAN-1990, sequence version 1.
    /// DT   01-JAN-1990, entry version 1.
    fn parse_dt_line(&mut self, line: &str) -> Result<()> {
        let dt_part = line.trim_start_matches("DT   ");
        if let Some(date_str) = dt_part.split(',').next() {
            let date = parse_date(date_str.trim())?;

            if dt_part.contains("integrated into") {
                // Entry creation date
                if self.entry_created.is_none() {
                    self.entry_created = Some(date);
                }
                // Also use as release_date for backwards compatibility
                if self.release_date.is_none() {
                    self.release_date = Some(date);
                }
            } else if dt_part.contains("sequence version") {
                // Last sequence update
                self.sequence_updated = Some(date);
            } else if dt_part.contains("entry version") {
                // Last annotation update
                self.annotation_updated = Some(date);
            }
        }
        Ok(())
    }

    /// Parse DE line: DE   RecName: Full=Protein name;
    ///
    /// Parses:
    /// - RecName: Full (primary protein name)
    /// - AltName: Full (alternative names)
    /// - SubName: Full (submitted names for TrEMBL)
    /// - Short names
    /// - EC numbers
    fn parse_de_line(&mut self, line: &str) -> Result<()> {
        // Parse RecName: Full
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

        // Parse AltName: Full
        if let Some(start) = line.find("AltName: Full=") {
            let name_part = &line[start + 14..];
            if let Some(end) = name_part.find([';', '{']) {
                self.alternative_names.push(name_part[..end].trim().to_string());
            } else {
                self.alternative_names.push(name_part.trim().to_string());
            }
        }

        // Parse SubName: Full
        if let Some(start) = line.find("SubName: Full=") {
            let name_part = &line[start + 14..];
            if let Some(end) = name_part.find([';', '{']) {
                self.alternative_names.push(name_part[..end].trim().to_string());
            } else {
                self.alternative_names.push(name_part.trim().to_string());
            }
        }

        // Parse EC numbers
        if let Some(start) = line.find("EC=") {
            let ec_part = &line[start + 3..];
            if let Some(end) = ec_part.find([';', ' ', '{']) {
                self.ec_numbers.push(ec_part[..end].trim().to_string());
            } else {
                self.ec_numbers.push(ec_part.trim().to_string());
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

    /// Parse OC line: OC   Viruses; Riboviria; Orthornavirae; ...
    ///
    /// OC lines contain the taxonomic lineage, which can span multiple lines.
    /// Each taxon is separated by semicolons, and the last taxon ends with a period.
    ///
    /// Example:
    /// ```
    /// OC   Viruses; Riboviria; Orthornavirae; Kitrinoviricota;
    /// OC   Flasuviricetes; Amarillovirales; Flaviviridae; Flavivirus.
    /// ```
    ///
    /// This produces: ["Viruses", "Riboviria", "Orthornavirae", "Kitrinoviricota",
    ///                 "Flasuviricetes", "Amarillovirales", "Flaviviridae", "Flavivirus"]
    fn parse_oc_line(&mut self, line: &str) -> Result<()> {
        let oc_part = line.trim_start_matches("OC   ");

        // Split by semicolons and process each taxon
        for taxon in oc_part.split(';') {
            let trimmed = taxon.trim().trim_end_matches('.');
            if !trimmed.is_empty() {
                self.taxonomy_lineage.push(trimmed.to_string());
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

    /// Parse FT line: FT   DOMAIN          50..150; Kinase domain.
    fn parse_ft_line(&mut self, line: &str) -> Result<()> {
        // Format: "FT   FEATURE_TYPE    START..END; Description."
        let ft_part = line.trim_start_matches("FT   ");

        // Parse feature type (first word)
        let parts: Vec<&str> = ft_part.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let feature_type = parts[0].to_string();

        // Parse position and description
        let rest = ft_part.trim_start_matches(&feature_type).trim();

        // Look for position range (e.g., "50..150" or "50")
        let mut start_pos = None;
        let mut end_pos = None;
        let description;

        if let Some(semicolon_pos) = rest.find(';') {
            let pos_part = &rest[..semicolon_pos].trim();
            description = rest[semicolon_pos + 1..].trim().to_string();

            // Parse position
            if let Some(range_pos) = pos_part.find("..") {
                // Range like "50..150"
                if let Ok(start) = pos_part[..range_pos].trim().parse::<i32>() {
                    start_pos = Some(start);
                }
                if let Ok(end) = pos_part[range_pos + 2..].trim().parse::<i32>() {
                    end_pos = Some(end);
                }
            } else {
                // Single position like "50"
                if let Ok(pos) = pos_part.trim().parse::<i32>() {
                    start_pos = Some(pos);
                    end_pos = Some(pos);
                }
            }
        } else {
            // No semicolon, might just be a position or description
            description = rest.to_string();
        }

        self.features.push(ProteinFeature {
            feature_type,
            start_pos,
            end_pos,
            description,
        });

        Ok(())
    }

    /// Parse DR line: DR   PDB; 1A2B; X-ray; 2.50 A; A/B=1-120.
    fn parse_dr_line(&mut self, line: &str) -> Result<()> {
        let dr_part = line.trim_start_matches("DR   ");
        let parts: Vec<&str> = dr_part.split(';').map(|s| s.trim()).collect();

        if parts.len() < 2 {
            return Ok(());
        }

        let database = parts[0].to_string();
        let database_id = parts[1].to_string();
        let metadata = parts[2..].iter().map(|s| s.to_string()).collect();

        self.cross_references.push(CrossReference {
            database,
            database_id,
            metadata,
        });

        Ok(())
    }

    /// Parse CC line: CC   -!- FUNCTION: Catalyzes the phosphorylation of proteins.
    fn parse_cc_line(&mut self, line: &str) -> Result<()> {
        let cc_part = line.trim_start_matches("CC   ");

        // Check for topic marker "-!- TOPIC:"
        if let Some(topic_start) = cc_part.find("-!- ") {
            // Finish previous comment if any
            self.finish_comment();

            let after_marker = &cc_part[topic_start + 4..];
            if let Some(colon_pos) = after_marker.find(':') {
                self.current_comment_topic = Some(after_marker[..colon_pos].trim().to_string());
                self.current_comment_text = after_marker[colon_pos + 1..].trim().to_string();
            }
        } else {
            // Continuation of previous comment
            if !cc_part.is_empty() {
                if !self.current_comment_text.is_empty() {
                    self.current_comment_text.push(' ');
                }
                self.current_comment_text.push_str(cc_part.trim());
            }
        }

        Ok(())
    }

    /// Finish the current comment and add it to the list
    fn finish_comment(&mut self) {
        if let Some(topic) = self.current_comment_topic.take() {
            if !self.current_comment_text.is_empty() {
                self.comments.push(Comment {
                    topic,
                    text: std::mem::take(&mut self.current_comment_text),
                });
            }
        }
    }

    /// Parse PE line: PE   1: Evidence at protein level;
    fn parse_pe_line(&mut self, line: &str) -> Result<()> {
        let pe_part = line.trim_start_matches("PE   ");
        if let Some(colon_pos) = pe_part.find(':') {
            if let Ok(level) = pe_part[..colon_pos].trim().parse::<i32>() {
                self.protein_existence = Some(level);
            }
        }
        Ok(())
    }

    /// Parse KW line: KW   ATP-binding; Kinase; Transferase.
    fn parse_kw_line(&mut self, line: &str) -> Result<()> {
        let kw_part = line.trim_start_matches("KW   ");
        for keyword in kw_part.split(';') {
            let kw = keyword.trim().trim_end_matches('.');
            if !kw.is_empty() {
                self.keywords.push(kw.to_string());
            }
        }
        Ok(())
    }

    /// Parse OG line: OG   Mitochondrion.
    fn parse_og_line(&mut self, line: &str) -> Result<()> {
        let og_part = line.trim_start_matches("OG   ").trim().trim_end_matches('.');
        if !og_part.is_empty() && self.organelle.is_none() {
            self.organelle = Some(og_part.to_string());
        }
        Ok(())
    }

    /// Parse OH line: OH   NCBI_TaxID=9606; Homo sapiens (Human).
    fn parse_oh_line(&mut self, line: &str) -> Result<()> {
        let oh_part = line.trim_start_matches("OH   ");
        // Extract organism name after semicolon
        if let Some(semicolon_pos) = oh_part.find(';') {
            let host = oh_part[semicolon_pos + 1..].trim().trim_end_matches('.').to_string();
            if !host.is_empty() {
                self.organism_hosts.push(host);
            }
        }
        Ok(())
    }

    /// Parse RN line: RN   [1]
    fn parse_rn_line(&mut self, line: &str) -> Result<()> {
        // Finish previous publication if any
        self.finish_publication();

        // Start new publication
        let rn_part = line.trim_start_matches("RN   ");
        if let Some(start) = rn_part.find('[') {
            if let Some(end) = rn_part.find(']') {
                if let Ok(num) = rn_part[start + 1..end].trim().parse::<i32>() {
                    let mut pub_builder = PublicationBuilder::new();
                    pub_builder.reference_number = Some(num);
                    self.current_publication = Some(pub_builder);
                }
            }
        }
        Ok(())
    }

    /// Parse RP line: RP   NUCLEOTIDE SEQUENCE [MRNA].
    fn parse_rp_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let rp_part = line.trim_start_matches("RP   ").trim();
            if let Some(existing) = &mut pub_builder.position {
                existing.push(' ');
                existing.push_str(rp_part);
            } else {
                pub_builder.position = Some(rp_part.to_string());
            }
        }
        Ok(())
    }

    /// Parse RC line: RC   STRAIN=Bristol N2; TISSUE=Sperm;
    fn parse_rc_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let rc_part = line.trim_start_matches("RC   ").trim();
            // Split by semicolon for multiple comments
            for comment in rc_part.split(';') {
                let trimmed = comment.trim();
                if !trimmed.is_empty() {
                    pub_builder.comments.push(trimmed.to_string());
                }
            }
        }
        Ok(())
    }

    /// Parse RX line: RX   PubMed=12345678; DOI=10.1234/example;
    fn parse_rx_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let rx_part = line.trim_start_matches("RX   ");

            // Extract PubMed ID
            if let Some(pmid_start) = rx_part.find("PubMed=") {
                let after_pmid = &rx_part[pmid_start + 7..];
                if let Some(end) = after_pmid.find(';') {
                    pub_builder.pubmed_id = Some(after_pmid[..end].trim().to_string());
                } else {
                    pub_builder.pubmed_id = Some(after_pmid.trim().to_string());
                }
            }

            // Extract DOI
            if let Some(doi_start) = rx_part.find("DOI=") {
                let after_doi = &rx_part[doi_start + 4..];
                if let Some(end) = after_doi.find(';') {
                    pub_builder.doi = Some(after_doi[..end].trim().to_string());
                } else {
                    pub_builder.doi = Some(after_doi.trim().to_string());
                }
            }
        }
        Ok(())
    }

    /// Parse RG line: RG   The C. elegans sequencing consortium;
    fn parse_rg_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let rg_part = line.trim_start_matches("RG   ").trim().trim_end_matches(';');
            if let Some(existing) = &mut pub_builder.author_group {
                existing.push(' ');
                existing.push_str(rg_part);
            } else {
                pub_builder.author_group = Some(rg_part.to_string());
            }
        }
        Ok(())
    }

    /// Parse RA line: RA   Smith J.D., Doe J.;
    fn parse_ra_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let ra_part = line.trim_start_matches("RA   ");
            // Split by comma for multiple authors
            for author in ra_part.split(',') {
                let trimmed = author.trim().trim_end_matches(';');
                if !trimmed.is_empty() {
                    pub_builder.authors.push(trimmed.to_string());
                }
            }
        }
        Ok(())
    }

    /// Parse RT line: RT   "Title of the article.";
    fn parse_rt_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let rt_part = line.trim_start_matches("RT   ").trim();
            // Remove quotes and trailing semicolon
            let title = rt_part.trim_matches('"').trim_end_matches(';');
            if let Some(existing) = &mut pub_builder.title {
                existing.push(' ');
                existing.push_str(title);
            } else {
                pub_builder.title = Some(title.to_string());
            }
        }
        Ok(())
    }

    /// Parse RL line: RL   J. Biol. Chem. 270:1234-1245(1995).
    fn parse_rl_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut pub_builder) = self.current_publication {
            let rl_part = line.trim_start_matches("RL   ").trim();
            if let Some(existing) = &mut pub_builder.location {
                existing.push(' ');
                existing.push_str(rl_part);
            } else {
                pub_builder.location = Some(rl_part.to_string());
            }
        }
        Ok(())
    }

    /// Finish the current publication and add it to the list
    fn finish_publication(&mut self) {
        if let Some(pub_builder) = self.current_publication.take() {
            if let Some(publication) = pub_builder.build() {
                self.publications.push(publication);
            }
        }
    }

    /// Build the final UniProtEntry
    fn build(mut self) -> Result<Option<UniProtEntry>> {
        // Finish any pending comment and publication
        self.finish_comment();
        self.finish_publication();
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
            taxonomy_lineage: self.taxonomy_lineage,
            sequence: self.sequence,
            sequence_length,
            mass_da,
            release_date,
            alternative_names: self.alternative_names,
            ec_numbers: self.ec_numbers,
            features: self.features,
            cross_references: self.cross_references,
            comments: self.comments,
            protein_existence: self.protein_existence,
            keywords: self.keywords,
            organelle: self.organelle,
            organism_hosts: self.organism_hosts,
            publications: self.publications,
            entry_created: self.entry_created,
            sequence_updated: self.sequence_updated,
            annotation_updated: self.annotation_updated,
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
    use chrono::Datelike;

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

    #[test]
    fn test_entry_builder_oc_line_single() {
        let mut builder = EntryBuilder::new();
        builder.parse_oc_line("OC   Eukaryota; Metazoa; Chordata.").unwrap();
        assert_eq!(builder.taxonomy_lineage, vec!["Eukaryota", "Metazoa", "Chordata"]);
    }

    #[test]
    fn test_entry_builder_oc_line_multiline() {
        let mut builder = EntryBuilder::new();
        builder.parse_oc_line("OC   Viruses; Riboviria; Orthornavirae; Kitrinoviricota;").unwrap();
        builder.parse_oc_line("OC   Flasuviricetes; Amarillovirales; Flaviviridae; Flavivirus.").unwrap();
        assert_eq!(
            builder.taxonomy_lineage,
            vec![
                "Viruses",
                "Riboviria",
                "Orthornavirae",
                "Kitrinoviricota",
                "Flasuviricetes",
                "Amarillovirales",
                "Flaviviridae",
                "Flavivirus"
            ]
        );
    }

    #[test]
    fn test_entry_builder_oc_line_trailing_period() {
        let mut builder = EntryBuilder::new();
        builder.parse_oc_line("OC   Bacteria; Proteobacteria; Gammaproteobacteria.").unwrap();
        assert_eq!(
            builder.taxonomy_lineage,
            vec!["Bacteria", "Proteobacteria", "Gammaproteobacteria"]
        );
    }

    #[test]
    fn test_entry_builder_oc_line_archaea() {
        let mut builder = EntryBuilder::new();
        builder.parse_oc_line("OC   Archaea; Euryarchaeota; Methanomicrobia.").unwrap();
        assert_eq!(
            builder.taxonomy_lineage,
            vec!["Archaea", "Euryarchaeota", "Methanomicrobia"]
        );
    }
}
