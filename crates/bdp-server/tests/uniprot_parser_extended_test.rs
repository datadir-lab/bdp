//! Comprehensive tests for UniProt DAT parser - Extended metadata fields
//!
//! Tests all DAT format fields including:
//! - DE: Alternative names, EC numbers
//! - FT: Feature table (domains, sites, modifications, variants)
//! - DR: Database cross-references
//! - CC: Comments (function, location, disease)
//! - PE: Protein existence level
//! - KW: Keywords
//! - OG: Organelle
//! - OH: Organism hosts

use bdp_server::ingest::uniprot::parser::DatParser;

/// Sample DAT entry with all extended metadata fields
const COMPREHENSIVE_DAT_ENTRY: &str = r#"ID   TEST_HUMAN              Reviewed;         461 AA.
AC   P12345;
DT   01-JAN-1990, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Epidermal growth factor receptor;
DE            Short=EGFR;
DE            EC=2.7.10.1;
DE   AltName: Full=Proto-oncogene c-ErbB-1;
DE   AltName: Full=Receptor tyrosine-protein kinase erbB-1;
GN   Name=EGFR; Synonyms=ERBB, ERBB1;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
OC   Eukaryota; Metazoa; Chordata; Craniata; Vertebrata; Euteleostomi;
OC   Mammalia; Eutheria; Euarchontoglires; Primates; Haplorrhini;
OC   Catarrhini; Hominidae; Homo.
OG   Plasmid R6-5.
OH   NCBI_TaxID=9913; Bos taurus (Bovine).
OH   NCBI_TaxID=9615; Canis lupus familiaris (Dog).
FT   DOMAIN          57..167; Receptor L domain 1.
FT   DOMAIN          168..310; Receptor L domain 2.
FT   BINDING         745; ATP binding site.
FT   MOD_RES         1068; Phosphothreonine; by autocatalysis.
FT   VARIANT         521; R -> K; in dbSNP:rs1050171.
DR   EMBL; M28668; AAA35808.1; -; mRNA.
DR   PIR; A00001; TVHUA1.
DR   PDB; 1IVO; X-ray; 2.40 A; A/B=25-642.
DR   PDB; 1M14; X-ray; 2.60 A; A=672-998.
DR   GO; GO:0005886; C:plasma membrane; IEA:UniProtKB-SubCell.
DR   GO; GO:0004714; F:transmembrane receptor protein tyrosine kinase activity; IDA:UniProtKB.
DR   InterPro; IPR000494; EGF receptor, L domain.
DR   InterPro; IPR009030; Growth factor receptor cys-rich, N-terminal.
DR   Pfam; PF00757; Furin-like; 2.
DR   Pfam; PF01030; Receptor_L_domain; 2.
DR   SMART; SM00181; FU; 7.
DR   PROSITE; PS00022; EGF_1; 9.
CC   -!- FUNCTION: Receptor for epidermal growth factor (EGF) and related
CC       growth factors including TGF-alpha, amphiregulin, betacellulin,
CC       heparin-binding EGF-like growth factor, GP30 and vaccinia virus
CC       growth factor.
CC   -!- CATALYTIC ACTIVITY: ATP + a [protein]-L-tyrosine = ADP + a
CC       [protein]-L-tyrosine phosphate.
CC   -!- SUBUNIT: Interacts with ERRFI1 and CNKSR1. Interacts with
CC       phosphorylated CLPTM1L.
CC   -!- SUBCELLULAR LOCATION: Cell membrane; Single-pass type I membrane
CC       protein. Note=Internalized upon ligand binding.
CC   -!- DISEASE: Defects in EGFR are a cause of lung cancer (LNCR)
CC       [MIM:211980]. Defects in EGFR are associated with inflammatory
CC       skin and bowel disease, neonatal, 2 (NISBD2) [MIM:616069].
CC   -!- SIMILARITY: Belongs to the protein kinase superfamily. Tyr
CC       protein kinase family. EGF receptor subfamily.
PE   1: Evidence at protein level;
KW   3D-structure; ATP-binding; Cell membrane; Disease mutation;
KW   Disulfide bond; EGF-like domain; Glycoprotein; Kinase; Membrane;
KW   Nucleotide-binding; Phosphoprotein; Polymorphism; Receptor;
KW   Serine/threonine-protein kinase; Signal; Transferase;
KW   Transmembrane; Transmembrane helix; Tyrosine-protein kinase.
SQ   SEQUENCE   461 AA;  52964 MW;  DD04711F9B8643F5 CRC64;
     MRPSGTAGAA LLALLAALCP ASRALEEKKV CQGTSNKLTQ LGTFEDHFLS LQRMFNNCEV
     VLGNLEITYV QRNYDLSFLK TIQEVAGYVL IALNTVERIP LENLQIIRGN MYYENSYALA
     VLSNYDANKT GLKELPMRNL QEILHGAVRF SNNPALCNVE SIQWRDIVSS DFLSNMSMDF
     QNHLGSCQKC DPSCPNGSSD CAGGCSASYM NWTCADAAFC PRLKRNEMGD MKLKCVQLFN
     LEQEIRTLAT RTIIEDMMQR QPRPNNGLDK DPKQRLGGRT DFSPDLRSFC MKETMAQPIV
     MRAAVRNALL WRRGEKGYVK IPSHVVTNEV DVPHNDSEDP LNVDPVEHDR LVHFCSRRTV
     FSQEDLIFLP FEGEDTKENC FRNEGLKFSA HGTRGSYLRL KKSLSRLPIG SRAQGSQAQT
     RVRAMAIYKQ SQHMTEVVRR CPHHERCSDG DGLAPPQHLI DRVNGSQAYI C
//
"#;

#[test]
fn test_parse_all_de_fields() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Primary name
    assert_eq!(entry.protein_name, "Epidermal growth factor receptor");

    // Alternative names
    assert_eq!(entry.alternative_names.len(), 2);
    assert!(entry
        .alternative_names
        .contains(&"Proto-oncogene c-ErbB-1".to_string()));
    assert!(entry
        .alternative_names
        .contains(&"Receptor tyrosine-protein kinase erbB-1".to_string()));

    // EC numbers
    assert_eq!(entry.ec_numbers.len(), 1);
    assert_eq!(entry.ec_numbers[0], "2.7.10.1");
}

#[test]
fn test_parse_ft_features() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have 5 features
    assert_eq!(entry.features.len(), 5);

    // Check DOMAIN feature
    let domain1 = entry
        .features
        .iter()
        .find(|f| f.description.contains("Receptor L domain 1"))
        .unwrap();
    assert_eq!(domain1.feature_type, "DOMAIN");
    assert_eq!(domain1.start_pos, Some(57));
    assert_eq!(domain1.end_pos, Some(167));

    // Check BINDING feature
    let binding = entry
        .features
        .iter()
        .find(|f| f.feature_type == "BINDING")
        .unwrap();
    assert_eq!(binding.start_pos, Some(745));
    assert_eq!(binding.end_pos, Some(745));
    assert!(binding.description.contains("ATP binding site"));

    // Check MOD_RES feature
    let mod_res = entry
        .features
        .iter()
        .find(|f| f.feature_type == "MOD_RES")
        .unwrap();
    assert_eq!(mod_res.start_pos, Some(1068));
    assert!(mod_res.description.contains("Phosphothreonine"));

    // Check VARIANT feature
    let variant = entry
        .features
        .iter()
        .find(|f| f.feature_type == "VARIANT")
        .unwrap();
    assert_eq!(variant.start_pos, Some(521));
    assert!(variant.description.contains("R -> K"));
}

#[test]
fn test_parse_dr_cross_references() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have multiple cross-references
    assert!(entry.cross_references.len() >= 10);

    // Check EMBL reference
    let embl = entry
        .cross_references
        .iter()
        .find(|r| r.database == "EMBL")
        .unwrap();
    assert_eq!(embl.database_id, "M28668");
    assert_eq!(embl.metadata[0], "AAA35808.1");

    // Check PDB references
    let pdb_refs: Vec<_> = entry
        .cross_references
        .iter()
        .filter(|r| r.database == "PDB")
        .collect();
    assert_eq!(pdb_refs.len(), 2);
    assert_eq!(pdb_refs[0].database_id, "1IVO");
    assert_eq!(pdb_refs[0].metadata[0], "X-ray");

    // Check GO references
    let go_refs: Vec<_> = entry
        .cross_references
        .iter()
        .filter(|r| r.database == "GO")
        .collect();
    assert_eq!(go_refs.len(), 2);

    // Check InterPro references
    let interpro_refs: Vec<_> = entry
        .cross_references
        .iter()
        .filter(|r| r.database == "InterPro")
        .collect();
    assert_eq!(interpro_refs.len(), 2);

    // Check Pfam references
    let pfam_refs: Vec<_> = entry
        .cross_references
        .iter()
        .filter(|r| r.database == "Pfam")
        .collect();
    assert_eq!(pfam_refs.len(), 2);

    // Check PROSITE reference
    let prosite = entry
        .cross_references
        .iter()
        .find(|r| r.database == "PROSITE")
        .unwrap();
    assert_eq!(prosite.database_id, "PS00022");
}

#[test]
fn test_parse_cc_comments() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have multiple comments
    assert_eq!(entry.comments.len(), 6);

    // Check FUNCTION comment
    let function = entry
        .comments
        .iter()
        .find(|c| c.topic == "FUNCTION")
        .unwrap();
    assert!(function
        .text
        .contains("Receptor for epidermal growth factor"));
    assert!(function.text.contains("TGF-alpha"));

    // Check CATALYTIC ACTIVITY comment
    let catalytic = entry
        .comments
        .iter()
        .find(|c| c.topic == "CATALYTIC ACTIVITY")
        .unwrap();
    assert!(catalytic.text.contains("ATP + a [protein]-L-tyrosine"));

    // Check SUBUNIT comment
    let subunit = entry
        .comments
        .iter()
        .find(|c| c.topic == "SUBUNIT")
        .unwrap();
    assert!(subunit.text.contains("ERRFI1"));

    // Check SUBCELLULAR LOCATION comment
    let location = entry
        .comments
        .iter()
        .find(|c| c.topic == "SUBCELLULAR LOCATION")
        .unwrap();
    assert!(location.text.contains("Cell membrane"));
    assert!(location
        .text
        .contains("Single-pass type I membrane protein"));

    // Check DISEASE comment
    let disease = entry
        .comments
        .iter()
        .find(|c| c.topic == "DISEASE")
        .unwrap();
    assert!(disease.text.contains("lung cancer"));
    assert!(disease.text.contains("LNCR"));

    // Check SIMILARITY comment
    let similarity = entry
        .comments
        .iter()
        .find(|c| c.topic == "SIMILARITY")
        .unwrap();
    assert!(similarity.text.contains("protein kinase superfamily"));
}

#[test]
fn test_parse_pe_protein_existence() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have protein existence level 1 (Evidence at protein level)
    assert_eq!(entry.protein_existence, Some(1));
}

#[test]
fn test_parse_kw_keywords() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have 19 keywords total
    assert_eq!(entry.keywords.len(), 19);

    // Check for specific keywords
    assert!(entry.keywords.contains(&"3D-structure".to_string()));
    assert!(entry.keywords.contains(&"ATP-binding".to_string()));
    assert!(entry.keywords.contains(&"Kinase".to_string()));
    assert!(entry.keywords.contains(&"Receptor".to_string()));
    assert!(entry
        .keywords
        .contains(&"Tyrosine-protein kinase".to_string()));
    assert!(entry.keywords.contains(&"Transmembrane".to_string()));
}

#[test]
fn test_parse_og_organelle() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have plasmid organelle
    assert_eq!(entry.organelle, Some("Plasmid R6-5".to_string()));
}

#[test]
fn test_parse_oh_organism_hosts() {
    let parser = DatParser::new();
    let entries = parser.parse_dat_string(COMPREHENSIVE_DAT_ENTRY).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have 2 organism hosts
    assert_eq!(entry.organism_hosts.len(), 2);
    assert!(entry
        .organism_hosts
        .contains(&"Bos taurus (Bovine)".to_string()));
    assert!(entry
        .organism_hosts
        .contains(&"Canis lupus familiaris (Dog)".to_string()));
}

#[test]
fn test_parse_minimal_entry_without_extended_fields() {
    let minimal_entry = r#"ID   MIN_TEST                Reviewed;         100 AA.
AC   P99999;
DT   01-JAN-2000, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Minimal test protein;
OS   Escherichia coli.
OX   NCBI_TaxID=562;
OC   Bacteria; Proteobacteria.
SQ   SEQUENCE   100 AA;  11000 MW;  1234567890ABCDEF CRC64;
     MKTIIALSYI FCLVFADYKD DDDKMKTIII ALSYIFCLVF ADYKDDDDKM KTIIALSYIF
     CLVFADYKDD DDKMKTIIAL SYIFCLVFAD YKDDDDKMKT
//
"#;

    let parser = DatParser::new();
    let entries = parser.parse_dat_string(minimal_entry).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Core fields should be present
    assert_eq!(entry.accession, "P99999");
    assert_eq!(entry.protein_name, "Minimal test protein");

    // Extended fields should be empty
    assert!(entry.alternative_names.is_empty());
    assert!(entry.ec_numbers.is_empty());
    assert!(entry.features.is_empty());
    assert!(entry.cross_references.is_empty());
    assert!(entry.comments.is_empty());
    assert_eq!(entry.protein_existence, None);
    assert!(entry.keywords.is_empty());
    assert_eq!(entry.organelle, None);
    assert!(entry.organism_hosts.is_empty());
}

#[test]
fn test_parse_multiple_ec_numbers() {
    let multi_ec_entry = r#"ID   MULTI_EC                Reviewed;         250 AA.
AC   P11111;
DT   01-JAN-2000, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Multi-function enzyme;
DE            EC=1.1.1.1;
DE            EC=2.2.2.2;
DE            EC=3.3.3.3;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
OC   Eukaryota; Metazoa.
SQ   SEQUENCE   250 AA;  28000 MW;  ABCDEF1234567890 CRC64;
     MKTIIALSYI FCLVFADYKD DDDKMKTIII ALSYIFCLVF ADYKDDDDKM KTIIALSYIF
     CLVFADYKDD DDKMKTIIAL SYIFCLVFAD YKDDDDKMKT MKTIIALSYI FCLVFADYKD
     DDDKMKTIII ALSYIFCLVF ADYKDDDDKM KTIIALSYIF CLVFADYKDD DDKMKTIIAL
     SYIFCLVFAD YKDDDDKMKT MKTIIALSYI FCLVFADYKD DDDKMKTIII ALSYIFCLVF
     ADYKDDDDKM
//
"#;

    let parser = DatParser::new();
    let entries = parser.parse_dat_string(multi_ec_entry).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have 3 EC numbers
    assert_eq!(entry.ec_numbers.len(), 3);
    assert_eq!(entry.ec_numbers[0], "1.1.1.1");
    assert_eq!(entry.ec_numbers[1], "2.2.2.2");
    assert_eq!(entry.ec_numbers[2], "3.3.3.3");
}

#[test]
fn test_parse_multiline_comment() {
    let multiline_comment_entry = r#"ID   LONG_CC                 Reviewed;         150 AA.
AC   P22222;
DT   01-JAN-2000, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Long comment protein;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
OC   Eukaryota; Metazoa.
CC   -!- FUNCTION: This is a very long function description that spans
CC       multiple lines and contains detailed information about the
CC       protein function and mechanism of action.
SQ   SEQUENCE   150 AA;  17000 MW;  1234ABCD5678EFGH CRC64;
     MKTIIALSYI FCLVFADYKD DDDKMKTIII ALSYIFCLVF ADYKDDDDKM KTIIALSYIF
     CLVFADYKDD DDKMKTIIAL SYIFCLVFAD YKDDDDKMKT MKTIIALSYI FCLVFADYKD
     DDDKMKTIII ALSYIFCLVF ADYKDDDDKM
//
"#;

    let parser = DatParser::new();
    let entries = parser.parse_dat_string(multiline_comment_entry).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have 1 comment
    assert_eq!(entry.comments.len(), 1);

    let function = &entry.comments[0];
    assert_eq!(function.topic, "FUNCTION");

    // Check that multiple lines are concatenated with spaces
    assert!(function.text.contains("very long function description"));
    assert!(function.text.contains("spans multiple lines"));
    assert!(function.text.contains("mechanism of action"));
}

#[test]
fn test_protein_existence_levels() {
    for level in 1..=5 {
        let entry = format!(
            r#"ID   PE{}_TEST               Reviewed;         100 AA.
AC   P3333{};
DT   01-JAN-2000, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=PE level {} test;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
OC   Eukaryota; Metazoa.
PE   {}: Test evidence level;
SQ   SEQUENCE   100 AA;  11000 MW;  1234567890ABCDEF CRC64;
     MKTIIALSYI FCLVFADYKD DDDKMKTIII ALSYIFCLVF ADYKDDDDKM KTIIALSYIF
     CLVFADYKDD DDKMKTIIAL SYIFCLVFAD YKDDDDKMKT
//
"#,
            level, level, level, level
        );

        let parser = DatParser::new();
        let entries = parser.parse_dat_string(&entry).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].protein_existence, Some(level));
    }
}

#[test]
fn test_feature_without_position() {
    let no_pos_entry = r#"ID   NOPOS_TEST              Reviewed;         100 AA.
AC   P44444;
DT   01-JAN-2000, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=No position feature test;
OS   Homo sapiens (Human).
OX   NCBI_TaxID=9606;
OC   Eukaryota; Metazoa.
FT   CHAIN           General description without position.
SQ   SEQUENCE   100 AA;  11000 MW;  1234567890ABCDEF CRC64;
     MKTIIALSYI FCLVFADYKD DDDKMKTIII ALSYIFCLVF ADYKDDDDKM KTIIALSYIF
     CLVFADYKDD DDKMKTIIAL SYIFCLVFAD YKDDDDKMKT
//
"#;

    let parser = DatParser::new();
    let entries = parser.parse_dat_string(no_pos_entry).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    // Should have the feature even without position
    assert_eq!(entry.features.len(), 1);
    assert_eq!(entry.features[0].feature_type, "CHAIN");
    assert!(entry.features[0]
        .description
        .contains("General description"));
}
