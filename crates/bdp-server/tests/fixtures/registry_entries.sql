-- Test fixture: Sample registry entries
-- This file assumes the organizations fixture has been loaded

INSERT INTO registry_entries (organization_id, slug, name, entry_type, description)
SELECT
    o.id,
    'swissprot-human',
    'Swiss-Prot Human Proteins',
    'data_source',
    'Manually annotated and reviewed human proteins from UniProt/Swiss-Prot'
FROM organizations o
WHERE o.slug = 'uniprot';

INSERT INTO registry_entries (organization_id, slug, name, entry_type, description)
SELECT
    o.id,
    'refseq-human',
    'RefSeq Human Sequences',
    'data_source',
    'NCBI Reference Sequence Database - Human sequences'
FROM organizations o
WHERE o.slug = 'ncbi';

INSERT INTO registry_entries (organization_id, slug, name, entry_type, description)
SELECT
    o.id,
    'blast',
    'BLAST',
    'tool',
    'Basic Local Alignment Search Tool'
FROM organizations o
WHERE o.slug = 'ncbi';
