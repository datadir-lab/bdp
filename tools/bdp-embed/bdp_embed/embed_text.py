def build_embed_text(entry: dict, source_type: str) -> str:
    """Build the text to embed for a registry entry.

    Uses source-type-specific templates to produce the most semantically
    meaningful text. Unknown types fall through to the generic fallback.
    """
    def _join(*parts) -> str:
        return " ".join(p.strip() for p in parts if p and str(p).strip())

    match source_type:
        case "protein":
            return _join(
                entry.get("name", ""),
                entry.get("gene_name", ""),
                entry.get("organism", ""),
                entry.get("function", ""),
                entry.get("go_terms", ""),
            )
        case "genome":
            return _join(
                entry.get("name", ""),
                entry.get("organism", ""),
                entry.get("assembly_level", ""),
                entry.get("annotation_source", ""),
            )
        case "taxonomy":
            return _join(
                entry.get("name", ""),
                entry.get("common_name", ""),
                entry.get("lineage", ""),
                entry.get("rank", ""),
            )
        case "transcript":
            return _join(
                entry.get("name", ""),
                entry.get("gene_name", ""),
                entry.get("biotype", ""),
                entry.get("organism", ""),
            )
        case "annotation":
            return _join(
                entry.get("name", ""),
                entry.get("description", ""),
                entry.get("assay_type", ""),
                entry.get("organism", ""),
                entry.get("tissue", ""),
            )
        case "structure":
            return _join(
                entry.get("name", ""),
                entry.get("organism", ""),
                entry.get("method", ""),
                entry.get("molecule_names", ""),
            )
        case "domain":
            return _join(
                entry.get("name", ""),
                entry.get("description", ""),
                entry.get("domain_type", ""),
                entry.get("member_dbs", ""),
            )
        case "pathway":
            genes = " ".join(entry.get("gene_list", [])[:20])
            return _join(
                entry.get("name", ""),
                entry.get("organism", ""),
                entry.get("description", ""),
                f"genes: {genes}" if genes else "",
            )
        case "ontology_term":
            return _join(
                entry.get("name", ""),
                entry.get("definition", ""),
                f"synonyms: {entry.get('synonyms', '')}",
                f"namespace: {entry.get('namespace', '')}",
            )
        case "compound":
            return _join(
                entry.get("name", ""),
                entry.get("synonyms", ""),
                entry.get("bioactivity", ""),
                f"targets: {entry.get('targets', '')}",
            )
        case "variant":
            return _join(
                entry.get("gene", ""),
                entry.get("consequence", ""),
                entry.get("clinical_significance", ""),
                entry.get("trait", ""),
            )
        case "literature":
            # Raw text, no template prefix
            return _join(entry.get("title", ""), entry.get("abstract", ""))
        case _:
            # Generic fallback for types not yet explicitly handled
            return _join(
                entry.get("name", ""),
                entry.get("description", ""),
                source_type,
                entry.get("organism", ""),
            )
