# Organization Metadata Guide

This guide provides templates and examples for adding metadata to organizations during ingestion pipeline creation.

## Overview

When creating a new ingestion pipeline for a data source, you should populate organization metadata including licensing, citation, version strategy, and contact information. This metadata is displayed on the organization page in the web interface.

## Required Fields

The following fields should be populated when creating organizations:

```rust
sqlx::query!(
    r#"
    INSERT INTO organizations (
        id, name, slug, description, website, is_system,
        license, license_url, citation, citation_url,
        version_strategy, version_description,
        data_source_url, documentation_url, contact_email
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
    ON CONFLICT (slug) DO NOTHING
    "#,
    id,
    name,
    slug,
    description,
    website,
    is_system,
    license,
    license_url,
    citation,
    citation_url,
    version_strategy,
    version_description,
    data_source_url,
    documentation_url,
    contact_email
)
```

## Field Descriptions

- **license**: License type (e.g., "CC-BY-4.0", "MIT", "Public Domain", "Custom")
- **license_url**: Link to full license text
- **citation**: Recommended citation text for papers using this data
- **citation_url**: Link to citation guidelines page
- **version_strategy**: How versions are managed by the organization (e.g., "date-based", "semantic", "release-based")
- **version_description**: Detailed description of the organization's versioning approach. Note: The web interface also displays BDP's internal versioning strategy alongside this, explaining how BDP tracks ingestion jobs with internal semantic versions (e.g., 1.0, 1.1) mapped to external versions
- **data_source_url**: Link to the original data source (e.g., FTP site)
- **documentation_url**: Link to official documentation
- **contact_email**: Contact email for questions

## Examples

### UniProt (Already Implemented)

```rust
id: Uuid::new_v4(),
name: "Universal Protein Resource",
slug: "uniprot",
description: "UniProt Knowledgebase - Protein sequences and functional information",
website: Some("https://www.uniprot.org"),
is_system: true,
license: Some("CC-BY-4.0"),
license_url: Some("https://creativecommons.org/licenses/by/4.0/"),
citation: Some("UniProt Consortium (2023). UniProt: the Universal Protein Knowledgebase in 2023. Nucleic Acids Research."),
citation_url: Some("https://www.uniprot.org/help/publications"),
version_strategy: Some("date-based"),
version_description: Some("UniProt releases follow YYYY_MM format (e.g., 2025_01). Each release is a complete snapshot of the database."),
data_source_url: Some("https://ftp.uniprot.org/pub/databases/uniprot/"),
documentation_url: Some("https://www.uniprot.org/help"),
contact_email: Some("help@uniprot.org")
```

### NCBI Taxonomy

```rust
id: Uuid::new_v4(),
name: "NCBI Taxonomy",
slug: "ncbi-taxonomy",
description: "NCBI Taxonomy - Curated classification and nomenclature for all organisms in public sequence databases",
website: Some("https://www.ncbi.nlm.nih.gov/taxonomy"),
is_system: true,
license: Some("Public Domain"),
license_url: Some("https://www.ncbi.nlm.nih.gov/home/about/policies/"),
citation: Some("Schoch CL, et al. (2020). NCBI Taxonomy: a comprehensive update on curation, resources and tools. Database."),
citation_url: Some("https://www.ncbi.nlm.nih.gov/books/NBK25497/#chapter2.Citing_NCBI_Data_and_Servic"),
version_strategy: Some("date-based"),
version_description: Some("NCBI Taxonomy is updated continuously. Snapshots are taken periodically for stable releases."),
data_source_url: Some("https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/"),
documentation_url: Some("https://www.ncbi.nlm.nih.gov/books/NBK53758/"),
contact_email: Some("info@ncbi.nlm.nih.gov")
```

### GenBank/RefSeq

```rust
id: Uuid::new_v4(),
name: "NCBI GenBank",
slug: "ncbi-genbank",
description: "GenBank - NIH genetic sequence database, containing all publicly available nucleotide sequences",
website: Some("https://www.ncbi.nlm.nih.gov/genbank/"),
is_system: true,
license: Some("Public Domain"),
license_url: Some("https://www.ncbi.nlm.nih.gov/home/about/policies/"),
citation: Some("Sayers EW, et al. (2024). GenBank. Nucleic Acids Research. 52(D1): D126-D131."),
citation_url: Some("https://www.ncbi.nlm.nih.gov/books/NBK25497/#chapter2.Citing_NCBI_Data_and_Servic"),
version_strategy: Some("release-based"),
version_description: Some("GenBank releases are numbered sequentially (e.g., 250.0, 251.0). New releases are published bimonthly."),
data_source_url: Some("https://ftp.ncbi.nlm.nih.gov/genbank/"),
documentation_url: Some("https://www.ncbi.nlm.nih.gov/genbank/release/"),
contact_email: Some("info@ncbi.nlm.nih.gov")
```

### Gene Ontology (GO)

```rust
id: Uuid::new_v4(),
name: "Gene Ontology",
slug: "gene-ontology",
description: "Gene Ontology - Structured, controlled vocabulary for gene and protein functions",
website: Some("http://geneontology.org"),
is_system: true,
license: Some("CC-BY-4.0"),
license_url: Some("https://creativecommons.org/licenses/by/4.0/"),
citation: Some("Gene Ontology Consortium (2023). The Gene Ontology knowledgebase in 2023. Genetics. 224(1)."),
citation_url: Some("http://geneontology.org/docs/go-citation-policy/"),
version_strategy: Some("date-based"),
version_description: Some("GO releases are dated snapshots (e.g., 2025-01-15). The ontology is continuously updated, with official releases on a regular schedule."),
data_source_url: Some("http://current.geneontology.org/products/pages/downloads.html"),
documentation_url: Some("http://geneontology.org/docs/"),
contact_email: Some("help@geneontology.org")
```

### Ensembl

```rust
id: Uuid::new_v4(),
name: "Ensembl",
slug: "ensembl",
description: "Ensembl - Genome browser and annotation database for vertebrate genomes",
website: Some("https://www.ensembl.org"),
is_system: true,
license: Some("Apache-2.0"),
license_url: Some("https://www.apache.org/licenses/LICENSE-2.0"),
citation: Some("Cunningham F, et al. (2022). Ensembl 2022. Nucleic Acids Research. 50(D1): D988-D995."),
citation_url: Some("https://www.ensembl.org/info/about/publications.html"),
version_strategy: Some("release-based"),
version_description: Some("Ensembl uses sequential release numbers (e.g., 108, 109). New releases occur every 3-4 months."),
data_source_url: Some("https://ftp.ensembl.org/pub/"),
documentation_url: Some("https://www.ensembl.org/info/index.html"),
contact_email: Some("helpdesk@ensembl.org")
```

### Protein Data Bank (PDB)

```rust
id: Uuid::new_v4(),
name: "Protein Data Bank",
slug: "pdb",
description: "PDB - Repository for 3D structural data of large biological molecules",
website: Some("https://www.rcsb.org"),
is_system: true,
license: Some("CC0-1.0"),
license_url: Some("https://creativecommons.org/publicdomain/zero/1.0/"),
citation: Some("Burley SK, et al. (2023). RCSB Protein Data Bank. Nucleic Acids Research. 51(D1): D488-D508."),
citation_url: Some("https://www.rcsb.org/pages/policies#References"),
version_strategy: Some("continuous"),
version_description: Some("PDB is continuously updated with new structures. Weekly snapshots are available for stable references."),
data_source_url: Some("https://ftp.wwpdb.org/pub/pdb/data/"),
documentation_url: Some("https://www.rcsb.org/docs/"),
contact_email: Some("info@rcsb.org")
```

## Implementation Steps

When adding a new data source ingest pipeline:

1. **Create the ingestion script** (e.g., `examples/run_SOURCENAME_ingestion.rs`)

2. **Add `get_or_create_organization` function** with full metadata:

```rust
async fn get_or_create_organization(pool: &sqlx::PgPool) -> Result<Uuid> {
    const SLUG: &str = "your-source-slug";

    // Check if organization exists
    let result = sqlx::query!(
        r#"SELECT id FROM organizations WHERE slug = $1"#,
        SLUG
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        // Create organization with full metadata
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (
                id, name, slug, description, website, is_system,
                license, license_url, citation, citation_url,
                version_strategy, version_description,
                data_source_url, documentation_url, contact_email
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Your Organization Name",
            SLUG,
            "Description",
            Some("https://website"),
            true,
            Some("License"),
            Some("https://license-url"),
            Some("Citation text"),
            Some("https://citation-url"),
            Some("version-strategy"),
            Some("Version description"),
            Some("https://data-source"),
            Some("https://docs"),
            Some("contact@email.com")
        )
        .execute(pool)
        .await?;

        // Fetch the ID in case another process created it concurrently
        let record = sqlx::query!(
            r#"SELECT id FROM organizations WHERE slug = $1"#,
            SLUG
        )
        .fetch_one(pool)
        .await?;

        Ok(record.id)
    }
}
```

3. **Research the metadata**:
   - Visit the organization's website
   - Check their About/Legal/Citation pages
   - Look for licensing information
   - Find official citation guidelines
   - Identify version numbering scheme

4. **Run the migration** to add the new columns (if not done already):
   ```bash
   sqlx migrate run
   ```

5. **Test the ingestion** to ensure the organization is created with proper metadata

## Resources

- [Creative Commons Licenses](https://creativecommons.org/licenses/)
- [SPDX License List](https://spdx.org/licenses/)
- [Citing NCBI Services](https://www.ncbi.nlm.nih.gov/books/NBK25497/#chapter2.Citing_NCBI_Data_and_Servic)
- [EBI Citation](https://www.ebi.ac.uk/about/terms-of-use/)
