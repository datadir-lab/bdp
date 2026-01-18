-- Test fixture: System organizations
-- This file is used by SQLx tests to populate test data

INSERT INTO organizations (slug, name, is_system, website, description) VALUES
    ('uniprot', 'UniProt', true, 'https://www.uniprot.org', 'Universal Protein Resource'),
    ('ncbi', 'NCBI', true, 'https://www.ncbi.nlm.nih.gov', 'National Center for Biotechnology Information'),
    ('ensembl', 'Ensembl', true, 'https://www.ensembl.org', 'Ensembl Genome Browser');
