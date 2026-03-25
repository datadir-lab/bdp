// crates/bdp-ingest/src/pipelines/reactome/parser.rs
//
// Reactome uses TSV files, not OBO — no shared parser.

use crate::pipelines::reactome::models::*;
use anyhow::Result;
use tracing::warn;

/// Parse ReactomePathways.txt
///
/// Format (tab-separated, no header):
///   reactome_id \t name \t species
pub fn parse_pathways(content: &str, release: &str) -> Result<Vec<Pathway>> {
    let mut pathways = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 3 {
            warn!(line = line_num, "ReactomePathways: skipping malformed line");
            continue;
        }

        pathways.push(Pathway {
            reactome_id: cols[0].trim().to_string(),
            name: cols[1].trim().to_string(),
            species_name: cols[2].trim().to_string(),
            reactome_release: release.to_string(),
        });
    }

    Ok(pathways)
}

/// Parse UniProt2Reactome.txt (or UniProt2Reactome_All_Levels.txt)
///
/// Format (tab-separated, no header):
///   uniprot_acc \t reactome_id \t url \t pathway_name \t evidence_code \t species
///
/// Optionally filter to a specific species (e.g., "Homo sapiens").
pub fn parse_uniprot_reactome(
    content: &str,
    release: &str,
    species_filter: Option<&str>,
) -> Result<Vec<ProteinPathwayLink>> {
    let mut links = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 6 {
            warn!(line = line_num, "UniProt2Reactome: skipping malformed line");
            continue;
        }

        let species = cols[5].trim();
        if let Some(filter) = species_filter {
            if species != filter {
                continue;
            }
        }

        let uniprot_acc = cols[0].trim().to_string();
        // Skip isoform-specific entries (e.g., P04637-1)
        if uniprot_acc.contains('-') {
            continue;
        }

        links.push(ProteinPathwayLink {
            uniprot_acc,
            reactome_id: cols[1].trim().to_string(),
            pathway_name: cols[3].trim().to_string(),
            evidence_type: {
                let ev = cols[4].trim();
                if ev.is_empty() { None } else { Some(ev.to_string()) }
            },
            species_name: species.to_string(),
            reactome_release: release.to_string(),
        });
    }

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pathways() {
        let content = "R-HSA-9612973\tActivation of AMPK downstream\tHomo sapiens\nR-MMU-9612973\tActivation of AMPK downstream\tMus musculus\n";
        let pathways = parse_pathways(content, "114").unwrap();
        assert_eq!(pathways.len(), 2);
        assert_eq!(pathways[0].reactome_id, "R-HSA-9612973");
        assert_eq!(pathways[0].species_name, "Homo sapiens");
    }

    #[test]
    fn test_parse_uniprot_reactome() {
        let content = "P04637\tR-HSA-9612973\thttps://reactome.org/...\tActivation of AMPK\tTAS\tHomo sapiens\nP12345\tR-MMU-9612973\thttps://reactome.org/...\tSome pathway\tTAS\tMus musculus\n";
        let links = parse_uniprot_reactome(content, "114", Some("Homo sapiens")).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uniprot_acc, "P04637");
        assert_eq!(links[0].reactome_id, "R-HSA-9612973");
        assert_eq!(links[0].evidence_type.as_deref(), Some("TAS"));
    }

    #[test]
    fn test_skip_isoforms() {
        let content = "P04637-1\tR-HSA-9612973\thttp://\tpathway\tTAS\tHomo sapiens\n";
        let links = parse_uniprot_reactome(content, "114", None).unwrap();
        assert_eq!(links.len(), 0, "isoform entries should be skipped");
    }
}
