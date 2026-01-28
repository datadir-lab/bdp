use anyhow::Result;
use tracing::{error, info};

fn main() -> Result<()> {
    // Enable logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Minimal valid UniProt DAT entry
    let sample_dat = r#"ID   001R_FRG3G              Reviewed;         256 AA.
AC   Q6GZX4;
DT   28-JUN-2011, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Putative transcription factor 001R;
GN   ORFNames=FV3-001R;
OS   Frog virus 3 (isolate Goorha) (FV-3).
OC   Viruses; Varidnaviria; Bamfordvirae; Nucleocytoviricota; Megaviricetes.
OX   NCBI_TaxID=654924;
RN   [1]
RP   NUCLEOTIDE SEQUENCE [LARGE SCALE GENOMIC DNA].
RX   PubMed=15165820; DOI=10.1016/j.virol.2004.02.019;
RA   Tan W.G., Barkman T.J., Gregory Chinchar V., Essani K.;
RT   "Comparative genomic analyses of frog virus 3, type species of the genus
RT   Ranavirus (family Iridoviridae).";
RL   Virology 323:70-84(2004).
CC   -!- FUNCTION: Putative transcription factor.
DR   EMBL; AY548484; AAT09660.1; -; Genomic_DNA.
DR   RefSeq; YP_031579.1; NC_005946.1.
DR   GeneID; 2947773; -.
SQ   SEQUENCE   256 AA;  29735 MW;  B4840739BF7D4121 CRC64;
     MAFSAEDVLK EYDRRRRMEA LLLSLYYPND RKLLDYKEWS PPRVQVECPK APVEWNNPPS
     EKGLIVGHFS GIKYKGEKAQ ASEVDVNKMC CWVSKFKDAM RRYQGIQTCK IPGKVLSDLD
     AKIKAYNLTV EGVEGFVRYS RVTKQHVAAF LKELRHSKQY ENVNLIHYIL TDKRVDIQHL
     EKDLVKDFKA LVESAHRMRQ GHMINVKYIL YQLLKKHGHG PDGPDILTVK TGSKGVLYDD
     SFRKIYTDLG WKFTPL
//
ID   002L_FRG3G              Reviewed;         123 AA.
AC   Q6GZX3;
DT   28-JUN-2011, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Uncharacterized protein 002L;
GN   ORFNames=FV3-002L;
OS   Frog virus 3 (isolate Goorha) (FV-3).
OC   Viruses; Varidnaviria; Bamfordvirae; Nucleocytoviricota; Megaviricetes.
OX   NCBI_TaxID=654924;
SQ   SEQUENCE   123 AA;  14447 MW;  4C8345D5C7B42E60 CRC64;
     MSIIGATRLQ NDKSDTYSAG PCYAGGCSAF TPRGTCGKDW DLGEPRVFGH KGEFGEKPYV
     GIIGDERKYK GCIECDLKKH HGGLDYRYSD TGVQGVKVSD YRCKPMNLPE GKHGSAGFEK
     PEP
//
"#;

    info!("Sample DAT data:");
    info!("{}", sample_dat);
    info!("=== Testing parse_bytes (full file) ===");

    // Test 1: Parse full file
    let parser = bdp_server::ingest::uniprot::parser::DatParser::new();
    match parser.parse_bytes(sample_dat.as_bytes()) {
        Ok(entries) => {
            info!(count = entries.len(), "Parsed entries from full file");
            for (i, entry) in entries.iter().enumerate() {
                info!(
                    index = i,
                    accession = %entry.accession,
                    name = %entry.protein_name,
                    "Entry"
                );
            }
        },
        Err(e) => {
            error!(error = %e, "parse_bytes failed");
        },
    }

    info!("=== Testing parse_range (first entry only) ===");

    // Test 2: Parse range (entry 0 to 0)
    match parser.parse_range(sample_dat.as_bytes(), 0, 0) {
        Ok(entries) => {
            info!(count = entries.len(), range = "0-0", "Parsed entries from range");
            for (i, entry) in entries.iter().enumerate() {
                info!(
                    index = i,
                    accession = %entry.accession,
                    name = %entry.protein_name,
                    "Entry"
                );
            }
        },
        Err(e) => {
            error!(error = %e, "parse_range failed");
        },
    }

    info!("=== Testing parse_range (second entry only) ===");

    // Test 3: Parse range (entry 1 to 1)
    match parser.parse_range(sample_dat.as_bytes(), 1, 1) {
        Ok(entries) => {
            info!(count = entries.len(), range = "1-1", "Parsed entries from range");
            for (i, entry) in entries.iter().enumerate() {
                info!(
                    index = i,
                    accession = %entry.accession,
                    name = %entry.protein_name,
                    "Entry"
                );
            }
        },
        Err(e) => {
            error!(error = %e, "parse_range failed");
        },
    }

    info!("=== Testing parse_range (both entries) ===");

    // Test 4: Parse range (entry 0 to 1)
    match parser.parse_range(sample_dat.as_bytes(), 0, 1) {
        Ok(entries) => {
            info!(count = entries.len(), range = "0-1", "Parsed entries from range");
            for (i, entry) in entries.iter().enumerate() {
                info!(
                    index = i,
                    accession = %entry.accession,
                    name = %entry.protein_name,
                    "Entry"
                );
            }
        },
        Err(e) => {
            error!(error = %e, "parse_range failed");
        },
    }

    Ok(())
}
