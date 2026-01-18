-- BDP Development Seed Data
-- Run with: just db-seed

-- System Organizations
INSERT INTO organizations (slug, name, website, is_system, description) VALUES
    ('uniprot', 'Universal Protein Resource', 'https://www.uniprot.org', true, 'Comprehensive protein sequence and annotation database'),
    ('ncbi', 'National Center for Biotechnology Information', 'https://www.ncbi.nlm.nih.gov', true, 'U.S. national resource for molecular biology information'),
    ('ensembl', 'Ensembl Genome Browser', 'https://www.ensembl.org', true, 'Genome browser for vertebrate genomes'),
    ('pdb', 'Protein Data Bank', 'https://www.rcsb.org', true, 'Repository for 3D structural data of biological macromolecules'),
    ('ebi', 'European Bioinformatics Institute', 'https://www.ebi.ac.uk', true, 'European bioinformatics research and services'),
    ('dev-lab', 'Development Lab', 'https://example.com', false, 'Test organization for development')
ON CONFLICT (slug) DO NOTHING;

-- Organisms (common model organisms)
INSERT INTO organisms (ncbi_taxonomy_id, scientific_name, common_name, rank, lineage) VALUES
    (9606, 'Homo sapiens', 'Human', 'species', 'Eukaryota; Metazoa; Chordata; Mammalia; Primates; Hominidae; Homo'),
    (10090, 'Mus musculus', 'Mouse', 'species', 'Eukaryota; Metazoa; Chordata; Mammalia; Rodentia; Muridae; Mus'),
    (7227, 'Drosophila melanogaster', 'Fruit fly', 'species', 'Eukaryota; Metazoa; Arthropoda; Insecta; Diptera; Drosophilidae; Drosophila'),
    (6239, 'Caenorhabditis elegans', 'Roundworm', 'species', 'Eukaryota; Metazoa; Nematoda; Chromadorea; Rhabditida; Rhabditidae; Caenorhabditis'),
    (559292, 'Saccharomyces cerevisiae', 'Baker\'s yeast', 'species', 'Eukaryota; Fungi; Ascomycota; Saccharomycetes; Saccharomycetales; Saccharomycetaceae; Saccharomyces'),
    (83333, 'Escherichia coli K-12', 'E. coli', 'strain', 'Bacteria; Proteobacteria; Gammaproteobacteria; Enterobacterales; Enterobacteriaceae; Escherichia')
ON CONFLICT (ncbi_taxonomy_id) DO NOTHING;

-- Tags
INSERT INTO tags (name, category, description) VALUES
    ('protein', 'type', 'Protein sequence data'),
    ('genome', 'type', 'Genome sequence data'),
    ('annotation', 'type', 'Genome annotation data'),
    ('structure', 'type', '3D protein structure data'),
    ('reference', 'quality', 'Reference quality dataset'),
    ('curated', 'quality', 'Manually curated data'),
    ('model-organism', 'organism', 'Model organism data'),
    ('human', 'organism', 'Human data'),
    ('mouse', 'organism', 'Mouse data'),
    ('benchmark', 'usage', 'Benchmark dataset')
ON CONFLICT (name) DO NOTHING;

-- Example registry entries (for testing)
-- Note: In production, these would be created by ingestion pipeline

-- Example: Human insulin protein
DO $$
DECLARE
    org_id uuid;
    entry_id uuid;
    human_id uuid;
BEGIN
    SELECT id INTO org_id FROM organizations WHERE slug = 'uniprot';
    SELECT id INTO human_id FROM organisms WHERE ncbi_taxonomy_id = 9606;

    IF org_id IS NOT NULL AND human_id IS NOT NULL THEN
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES (org_id, 'P01308', 'Insulin [Homo sapiens]', 'Insulin decreases blood glucose concentration', 'data_source')
        ON CONFLICT (slug) DO NOTHING
        RETURNING id INTO entry_id;

        IF entry_id IS NOT NULL THEN
            INSERT INTO data_sources (id, source_type, external_id, organism_id)
            VALUES (entry_id, 'protein', 'P01308', human_id)
            ON CONFLICT (id) DO NOTHING;

            INSERT INTO protein_metadata (data_source_id, accession, entry_name, protein_name, gene_name, sequence_length, mass_da)
            VALUES (entry_id, 'P01308', 'INS_HUMAN', 'Insulin', 'INS', 110, 11937)
            ON CONFLICT (data_source_id) DO NOTHING;
        END IF;
    END IF;
END $$;

-- Add tags to insulin entry
DO $$
DECLARE
    entry_id uuid;
    tag_id uuid;
BEGIN
    SELECT re.id INTO entry_id FROM registry_entries re WHERE re.slug = 'P01308';

    IF entry_id IS NOT NULL THEN
        FOR tag_id IN SELECT id FROM tags WHERE name IN ('protein', 'reference', 'curated', 'human')
        LOOP
            INSERT INTO entry_tags (entry_id, tag_id) VALUES (entry_id, tag_id) ON CONFLICT DO NOTHING;
        END LOOP;
    END IF;
END $$;

SELECT
    (SELECT COUNT(*) FROM organizations) as organizations,
    (SELECT COUNT(*) FROM organisms) as organisms,
    (SELECT COUNT(*) FROM tags) as tags,
    (SELECT COUNT(*) FROM registry_entries) as registry_entries,
    (SELECT COUNT(*) FROM data_sources) as data_sources;

\echo '✓ Seed data loaded successfully'
