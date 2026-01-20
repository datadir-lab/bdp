# Third-Party Data Attributions

This document lists third-party data sources used in the Biological Data Platform (BDP) and their required attributions.

---

## Gene Ontology (GO)

**License**: Creative Commons Attribution 4.0 International (CC BY 4.0)
**Website**: https://geneontology.org/
**Data Source**: Zenodo DOI Archives

### Attribution

Gene Ontology data is made available under the terms of the CC BY 4.0 license (https://creativecommons.org/licenses/by/4.0/).

When using BDP's Gene Ontology data, please cite:

1. **The Gene Ontology Consortium (2025)**
   "The Gene Ontology knowledgebase in 2026"
   *Nucleic Acids Research*, 54(D1):D1779-D1792
   doi: 10.1093/nar/gkaf1292

2. **The original GO paper:**
   Ashburner M, Ball CA, Blake JA, et al. (2000)
   "Gene ontology: tool for the unification of biology"
   *Nat Genet.* 25(1):25-9.
   doi: 10.1038/75556

3. **Data Release Information:**
   Include the specific release date and Zenodo DOI when available
   Example: "GO Release 2025-09-08 (DOI: 10.5281/zenodo.17382285)"

### Required Attribution Notice

Applications, downloads, or documentation containing GO data must include:

> "Gene Ontology data from the [release date] ([DOI]) is made available under the terms of the Creative Commons Attribution 4.0 International license (CC BY 4.0).
>
> Gene Ontology Consortium. The Gene Ontology knowledgebase in 2026. Nucleic Acids Research, 2025. https://geneontology.org/"

### CC BY 4.0 License Requirements

Under CC BY 4.0, we must provide:
- Identification of the creator(s) of the Licensed Material
- Copyright notice
- Reference to the CC BY 4.0 license
- Disclaimer of warranties
- URI/hyperlink to the source material

### Data Sources

- **GO Ontology**: Downloaded from Zenodo archives (https://doi.org/10.5281/zenodo.1205166)
- **GO Annotations (GOA)**: Downloaded from EBI FTP server (ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/)

---

## UniProt

**License**: Creative Commons Attribution 4.0 International (CC BY 4.0)
**Website**: https://www.uniprot.org/
**Citation**: [To be added when UniProt integration is finalized]

---

## NCBI Taxonomy

**License**: Public Domain (US Government Work)
**Website**: https://www.ncbi.nlm.nih.gov/taxonomy/
**Citation**: [To be added]

---

## GenBank

**License**: Public Domain (US Government Work)
**Website**: https://www.ncbi.nlm.nih.gov/genbank/
**Citation**: [To be added]

---

## How BDP Provides Attribution

### In Database

Each data source version is tracked in the `versions` table with:
- `external_version`: The release date (e.g., "2025-09-08")
- `metadata`: JSON field containing Zenodo DOI, citation information, and license details

### In API Responses

API endpoints returning GO data include attribution metadata:
```json
{
  "data": [...],
  "attribution": {
    "source": "Gene Ontology Consortium",
    "release": "2025-09-08",
    "doi": "10.5281/zenodo.17382285",
    "license": "CC BY 4.0",
    "citation": "The Gene Ontology Consortium (2025)...",
    "url": "https://geneontology.org/"
  }
}
```

### In User Interface

The web interface displays attribution notices when showing GO data.

### In Documentation

README and API documentation reference this file for proper attribution.

---

## License Compatibility

BDP is licensed under the GNU Affero General Public License v3.0 (AGPL v3).
All third-party data used is compatible with AGPL v3:
- CC BY 4.0 data (GO, UniProt) can be used in AGPL projects
- Public domain data (NCBI resources) can be used freely

---

*Last Updated: 2026-01-20*
