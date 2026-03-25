#[derive(Debug, Clone)]
pub struct Pathway {
    pub reactome_id: String, // 'R-HSA-9612973'
    pub name: String,
    pub species_name: String, // 'Homo sapiens'
    pub reactome_release: String,
}

#[derive(Debug, Clone)]
pub struct ProteinPathwayLink {
    pub uniprot_acc: String, // 'P04637'
    pub reactome_id: String, // 'R-HSA-9612973'
    pub pathway_name: String,
    pub evidence_type: Option<String>,
    pub species_name: String,
    pub reactome_release: String,
}

#[derive(Debug, Default)]
pub struct ParsedReactome {
    pub pathways: Vec<Pathway>,
    pub protein_pathway_links: Vec<ProteinPathwayLink>,
}
