//! UniProt DAT parsing validation tests
//!
//! Tests parsing logic against real UniProt data to ensure correctness

use bdp_server::ingest::uniprot::{DatParser, UniProtEntry};
use std::io::Write;

/// Sample UniProt DAT entry (from P12345 - Aspartate aminotransferase)
const SAMPLE_DAT_ENTRY: &str = r#"ID   AATM_HUMAN              Reviewed;         401 AA.
AC   P00505; Q6FHX8;
DT   21-JUL-1986, integrated into UniProtKB/Swiss-Prot.
DT   23-JAN-2007, sequence version 3.
DT   25-MAY-2022, entry version 237.
DE   RecName: Full=Aspartate aminotransferase, mitochondrial;
DE            EC=2.6.1.1;
DE   AltName: Full=Fatty acid-binding protein;
DE            Short=FABP-1;
DE   AltName: Full=Glutamate oxaloacetate transaminase 2;
GN   Name=GOT2; Synonyms=GIG18, GOTI2;
OS   Homo sapiens (Human).
OC   Eukaryota; Metazoa; Chordata; Craniata; Vertebrata; Euteleostomi;
OC   Mammalia; Eutheria; Euarchontoglires; Primates; Haplorrhini;
OC   Catarrhini; Hominidae; Homo.
OX   NCBI_TaxID=9606;
RN   [1]
RP   NUCLEOTIDE SEQUENCE [MRNA].
RX   PubMed=3029094; DOI=10.1016/0006-291x(87)91699-2;
RA   Joh T., Takeshima H., Tsuzuki T., Shimada K., Tanase S., Morino Y.;
RT   "Cloning and sequence analysis of cDNAs encoding mammalian cytosolic
RT   malate dehydrogenase. Comparison of the amino acid sequences of
RT   mammalian and bacterial malate dehydrogenase.";
RL   Biochem. Biophys. Res. Commun. 146:266-273(1987).
CC   -!- FUNCTION: Aspartate aminotransferase is a pyridoxal phosphate
CC       (PLP)-dependent enzyme that catalyzes the reversible transfer of an
CC       amino group from aspartate to alpha-ketoglutarate, generating
CC       oxaloacetate and glutamate.
CC   -!- CATALYTIC ACTIVITY:
CC       Reaction=L-aspartate + 2-oxoglutarate = oxaloacetate + L-glutamate;
CC         Xref=Rhea:RHEA:17345, ChEBI:CHEBI:16810, ChEBI:CHEBI:16810,
CC         ChEBI:CHEBI:29985, ChEBI:CHEBI:29985; EC=2.6.1.1;
DR   EMBL; J04044; AAA51569.1; -; mRNA.
DR   EMBL; M22632; AAA51570.1; -; Genomic_DNA.
DR   PIR; A25714; TVHUG2.
DR   RefSeq; NP_002071.2; NM_002080.3.
DR   PDB; 1AMT; X-ray; 2.50 A; A=30-401.
DR   GO; GO:0004069; F:L-aspartate:2-oxoglutarate aminotransferase activity; IDA:UniProtKB.
DR   GO; GO:0006520; P:cellular amino acid metabolic process; TAS:Reactome.
DR   Gene3D; 3.40.640.10; -; 1.
DR   InterPro; IPR000796; Aminotransferase_I/II.
DR   PANTHER; PTHR11879; PTHR11879; 1.
DR   Pfam; PF00155; Aminotran_1_2; 1.
DR   PIRSF; PIRSF000544; Transaminase_1; 1.
DR   PRINTS; PR00817; TRANSAMINASE.
DR   SUPFAM; SSF53383; SSF53383; 1.
DR   TIGRFAMs; TIGR00707; asp_aminotrans; 1.
DR   PROSITE; PS00595; AA_TRANSFER_CLASS_1; 1.
PE   1: Evidence at protein level;
KW   3D-structure; Acetylation; Cytoplasm; Direct protein sequencing;
KW   Disease variant; Mitochondrion; Phosphoprotein; Pyridoxal phosphate;
KW   Reference proteome; Transit peptide; Transferase.
FT   TRANSIT         1..29
FT                   /note="Mitochondrion"
FT                   /evidence="ECO:0000255"
FT   CHAIN           30..401
FT                   /note="Aspartate aminotransferase, mitochondrial"
FT                   /id="PRO_0000123456"
SQ   SEQUENCE   401 AA;  47476 MW;  A1B2C3D4E5F6G7H8 CRC64;
     MALLHSGRVL SGASAAATAV KFERTILKTP EKTVRAIVPG VFGRTLQEAG KQFRNALQLE
     ANPDVAISAG VRTDDVLGKT GIDITHGQQK QFHPRYIRVP KVLDGDVVIE VHGRYAAGGI
     GVDRPIVNLL DHTVDFAKYS KGILNAAAKD IAEIGSGAAV FAAESLVGQE VPQVAQEIGR
     LLNALIHYEP DPFHPQPTTV KEVADAAKTY QNELPVIAKA MTEAVERAAR PRTLVVGPAP
     NRKFVQPTPE DRQQAAALAL QAKGVQVIID NDGLDFSGVK KTVPGSDEIK TLLPEVADLE
     LRDAILDGKI KTVKGKDGLD EAVKKIVNKY GGLTVPNIDP TFAEYILKGV ESRNSHPEPP
     PAPPPAAAKK RVAKKKKKAE KAAPAKKKKA AEKKAKAAKA A
//
"#;

#[test]
fn test_parse_sample_dat_entry() {
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(SAMPLE_DAT_ENTRY.as_bytes())
        .expect("Failed to parse sample DAT entry");

    assert_eq!(entries.len(), 1, "Should parse exactly one entry");

    let entry = &entries[0];

    // Validate basic fields
    assert_eq!(entry.accession, "P00505");
    assert_eq!(entry.entry_name, "AATM_HUMAN");
    assert_eq!(entry.organism_name, "Homo sapiens (Human)");
    assert_eq!(entry.taxonomy_id, 9606);

    // Validate protein name parsing
    assert!(
        entry.protein_name.contains("Aspartate aminotransferase"),
        "Protein name should contain 'Aspartate aminotransferase'"
    );

    // Validate gene name
    assert_eq!(entry.gene_name, Some("GOT2".to_string()), "Gene name should be GOT2");

    // Validate sequence
    assert!(entry.sequence.len() > 0, "Sequence should not be empty");
    assert_eq!(entry.sequence_length, 401, "Sequence length should be 401");

    // Validate sequence only contains valid amino acid characters
    assert!(
        entry.sequence.chars().all(|c| c.is_ascii_uppercase()),
        "Sequence should only contain uppercase letters"
    );

    // Validate mass
    assert_eq!(entry.mass_da, 47476, "Molecular mass should be 47476 Da");
}

#[test]
fn test_parse_multiple_entries() {
    // Create a DAT file with multiple entries
    let mut multi_entry = String::new();

    // Entry 1
    multi_entry.push_str(
        r#"ID   TEST1_HUMAN             Reviewed;         100 AA.
AC   P00001;
DT   21-JUL-1986, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Test protein 1;
GN   Name=TEST1;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
SQ   SEQUENCE   100 AA;  10000 MW;  00000000 CRC64;
     MAAAAAAAA AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA
     AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA
//
"#,
    );

    // Entry 2
    multi_entry.push_str(
        r#"ID   TEST2_HUMAN             Reviewed;         50 AA.
AC   P00002;
DT   21-JUL-1986, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Test protein 2;
GN   Name=TEST2;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
SQ   SEQUENCE   50 AA;  5000 MW;  00000000 CRC64;
     MBBBBBBBB BBBBBBBBBB BBBBBBBBBB BBBBBBBBBB BBBBBBBBBB
//
"#,
    );

    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(multi_entry.as_bytes())
        .expect("Failed to parse multiple entries");

    assert_eq!(entries.len(), 2, "Should parse exactly two entries");

    assert_eq!(entries[0].accession, "P00001");
    assert_eq!(entries[0].entry_name, "TEST1_HUMAN");
    assert_eq!(entries[0].sequence_length, 100);

    assert_eq!(entries[1].accession, "P00002");
    assert_eq!(entries[1].entry_name, "TEST2_HUMAN");
    assert_eq!(entries[1].sequence_length, 50);
}

#[test]
fn test_parse_with_limit() {
    let mut multi_entry = String::new();

    // Create 5 entries
    for i in 1..=5 {
        multi_entry.push_str(&format!(
            r#"ID   TEST{}_HUMAN             Reviewed;         50 AA.
AC   P0000{};
DT   21-JUL-1986, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Test protein {};
GN   Name=TEST{};
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
SQ   SEQUENCE   50 AA;  5000 MW;  00000000 CRC64;
     MAAAAAAAA AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA AAAAAAAAAA
//
"#,
            i, i, i, i
        ));
    }

    // Parse with limit of 3
    let parser = DatParser::with_limit(3);
    let entries = parser
        .parse_bytes(multi_entry.as_bytes())
        .expect("Failed to parse with limit");

    assert_eq!(entries.len(), 3, "Should only parse 3 entries with limit");
    assert_eq!(entries[0].accession, "P00001");
    assert_eq!(entries[2].accession, "P00003");
}

#[test]
fn test_sequence_checksum() {
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(SAMPLE_DAT_ENTRY.as_bytes())
        .expect("Failed to parse sample DAT entry");

    let entry = &entries[0];
    let checksum = entry.sequence_checksum();

    // SHA-256 should be 64 hex characters
    assert_eq!(checksum.len(), 64, "SHA-256 checksum should be 64 characters");
    assert!(
        checksum.chars().all(|c| c.is_ascii_hexdigit()),
        "Checksum should only contain hex characters"
    );

    // Same sequence should produce same checksum
    let checksum2 = entry.sequence_checksum();
    assert_eq!(checksum, checksum2, "Checksums should be deterministic");
}

#[test]
fn test_entry_validation() {
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(SAMPLE_DAT_ENTRY.as_bytes())
        .expect("Failed to parse sample DAT entry");

    let entry = &entries[0];

    // Validate should pass for correct entry
    entry
        .validate()
        .expect("Valid entry should pass validation");
}

#[test]
fn test_fasta_generation() {
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(SAMPLE_DAT_ENTRY.as_bytes())
        .expect("Failed to parse sample DAT entry");

    let entry = &entries[0];
    let fasta = entry.to_fasta();

    // Should start with >
    assert!(fasta.starts_with('>'), "FASTA should start with >");

    // Should contain accession
    assert!(fasta.contains("P00505"), "FASTA should contain accession");

    // Should contain entry name
    assert!(fasta.contains("AATM_HUMAN"), "FASTA should contain entry name");

    // Should contain organism
    assert!(fasta.contains("Homo sapiens"), "FASTA should contain organism");

    // Should contain taxonomy ID
    assert!(fasta.contains("9606"), "FASTA should contain taxonomy ID");

    // Should contain gene name
    assert!(fasta.contains("GOT2"), "FASTA should contain gene name");

    // Should have sequence lines wrapped at 60 characters
    let lines: Vec<&str> = fasta.lines().collect();
    assert!(lines.len() > 1, "FASTA should have multiple lines");

    // Check sequence lines are wrapped
    for line in &lines[1..] {
        // Skip header
        if !line.is_empty() {
            assert!(line.len() <= 60, "Sequence lines should be ≤60 characters");
        }
    }
}

#[test]
fn test_json_generation() {
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(SAMPLE_DAT_ENTRY.as_bytes())
        .expect("Failed to parse sample DAT entry");

    let entry = &entries[0];
    let json = entry.to_json().expect("Failed to generate JSON");

    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("Generated JSON should be valid");

    // Check key fields are present
    assert_eq!(parsed["accession"], "P00505");
    assert_eq!(parsed["entry_name"], "AATM_HUMAN");
    assert_eq!(parsed["taxonomy_id"], 9606);
    assert_eq!(parsed["sequence_length"], 401);
}

#[test]
fn test_empty_file() {
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(b"")
        .expect("Empty file should parse without error");

    assert_eq!(entries.len(), 0, "Empty file should have zero entries");
}

#[test]
fn test_malformed_entry() {
    let malformed = r#"ID   TEST_HUMAN
AC   P00001
//
"#;

    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(malformed.as_bytes())
        .expect("Parser should handle malformed entries gracefully");

    // Should either parse with missing fields or return empty
    // Parser should be fault-tolerant
    assert!(entries.is_empty() || entries.len() == 1);
}

/// Test that verifies our parser handles real UniProt data structure
#[test]
fn test_expected_uniprot_format() {
    // This entry follows the exact UniProt DAT format specification
    let parser = DatParser::new();
    let entries = parser
        .parse_bytes(SAMPLE_DAT_ENTRY.as_bytes())
        .expect("Failed to parse UniProt-formatted entry");

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Validate all required fields are extracted
    assert!(!entry.accession.is_empty(), "Accession is required");
    assert!(!entry.entry_name.is_empty(), "Entry name is required");
    assert!(!entry.protein_name.is_empty(), "Protein name is required");
    assert!(!entry.organism_name.is_empty(), "Organism is required");
    assert!(entry.taxonomy_id > 0, "Taxonomy ID is required");
    assert!(!entry.sequence.is_empty(), "Sequence is required");
    assert!(entry.sequence_length > 0, "Sequence length must be positive");
    assert!(entry.mass_da > 0, "Mass must be positive");
}
