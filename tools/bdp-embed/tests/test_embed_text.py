from bdp_embed.embed_text import build_embed_text


def test_protein_includes_gene_and_organism():
    entry = {"name": "Insulin", "gene_name": "INS", "organism": "Homo sapiens",
             "function": "glucose metabolism", "go_terms": "GO:0005179"}
    result = build_embed_text(entry, "protein")
    assert "Insulin" in result
    assert "INS" in result
    assert "Homo sapiens" in result
    assert "glucose metabolism" in result


def test_protein_skips_empty_fields():
    entry = {"name": "Insulin"}
    result = build_embed_text(entry, "protein")
    assert result.strip() == "Insulin"
    assert "  " not in result  # no double spaces from empty joins


def test_genome_includes_assembly_level():
    entry = {"name": "GRCh38", "organism": "Homo sapiens", "assembly_level": "Chromosome"}
    result = build_embed_text(entry, "genome")
    assert "GRCh38" in result
    assert "Chromosome" in result


def test_pathway_limits_gene_list():
    entry = {"name": "Glycolysis", "gene_list": [f"gene{i}" for i in range(50)]}
    result = build_embed_text(entry, "pathway")
    # Only first 20 genes included
    assert "gene19" in result
    assert "gene20" not in result


def test_literature_uses_raw_text():
    entry = {"title": "BRCA1 and DNA repair", "abstract": "We studied..."}
    result = build_embed_text(entry, "literature")
    assert result == "BRCA1 and DNA repair We studied..."


def test_unknown_type_uses_generic_fallback():
    entry = {"name": "Foo", "description": "Bar", "organism": "E. coli"}
    result = build_embed_text(entry, "novel_future_type")
    assert "Foo" in result
    assert "Bar" in result
    assert "E. coli" in result


def test_empty_entry_does_not_crash():
    result = build_embed_text({}, "protein")
    assert isinstance(result, str)
