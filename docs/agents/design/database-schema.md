# Database Schema Design

Complete PostgreSQL database schema for BDP registry.

## Core Design Principles

1. **Unified Abstraction**: `registry_entries` is the base for all entities
2. **Table Inheritance**: Data sources and tools branch from registry_entries
3. **Flexible Metadata**: JSONB columns for extensibility
4. **Strong Relationships**: Foreign keys with cascade rules
5. **Search Optimization**: Full-text search indexes on key fields

## Schema Overview

```
organizations
    ↓
registry_entries (abstract base)
    ├─→ data_sources (proteins, genomes, etc.)
    │   └─→ protein_metadata (protein-specific fields)
    └─→ tools (bioinformatics software)

versions (versioning for any entry)
    ├─→ version_files (formats: FASTA, XML, etc.)
    ├─→ dependencies (links between versions)
    └─→ citations (academic references)

organisms (taxonomy)
tags (categorization)
```

## Tables

### Organizations

Data providers like UniProt, NCBI, Ensembl, or user-created organizations.

```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(100) UNIQUE NOT NULL,  -- 'uniprot', 'ncbi', 'ensembl'
    name VARCHAR(256) NOT NULL,
    website TEXT,
    description TEXT,
    logo_url TEXT,
    is_system BOOLEAN DEFAULT FALSE,  -- true for hardcoded orgs we scrape
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX organizations_slug_idx ON organizations(slug);
CREATE INDEX organizations_system_idx ON organizations(is_system);
```

**Key Fields**:
- `slug`: URL-friendly identifier (e.g., "uniprot", "ncbi")
- `is_system`: Distinguishes system organizations (UniProt, NCBI) from user-created ones
- System organizations have automated scrapers/cron jobs

**Example Data**:
```sql
INSERT INTO organizations (slug, name, website, is_system) VALUES
    ('uniprot', 'Universal Protein Resource', 'https://www.uniprot.org', true),
    ('ncbi', 'National Center for Biotechnology Information', 'https://www.ncbi.nlm.nih.gov', true),
    ('user-lab', 'Smith Lab Curated Proteins', 'https://smithlab.edu', false);
```

### Registry Entries (Base Table)

Abstract base for all registry items. Every data source and tool is a registry entry.

```sql
CREATE TABLE registry_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    slug VARCHAR(255) UNIQUE NOT NULL,  -- 'P01308', 'blast', 'swissprot-all'
    name VARCHAR(255) NOT NULL,
    description TEXT,
    entry_type VARCHAR(50) NOT NULL,  -- 'data_source' or 'tool'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT entry_type_check CHECK (entry_type IN ('data_source', 'tool'))
);

-- Indexes
CREATE INDEX registry_entries_org_idx ON registry_entries(organization_id);
CREATE INDEX registry_entries_type_idx ON registry_entries(entry_type);
CREATE INDEX registry_entries_slug_idx ON registry_entries(slug);

-- Full-text search
CREATE INDEX registry_entries_search_idx ON registry_entries
    USING GIN (to_tsvector('english', name || ' ' || COALESCE(description, '')));
```

**Key Fields**:
- `slug`: Unique identifier (e.g., "P01308", "blast")
- `entry_type`: Discriminator for inheritance ('data_source' or 'tool')
- Organization link ensures every entry belongs to a provider

### Data Sources

Inherits from registry_entries. Represents biological data (proteins, genomes, annotations).

```sql
CREATE TABLE data_sources (
    id UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    source_type VARCHAR(50) NOT NULL,  -- 'protein', 'genome', 'annotation', 'structure'
    external_id VARCHAR(100),  -- UniProt accession: P01308, NCBI ID, etc.
    organism_id UUID REFERENCES organisms(id) ON DELETE SET NULL,
    additional_metadata JSONB,  -- Flexible metadata storage

    CONSTRAINT source_type_check CHECK (source_type IN ('protein', 'genome', 'annotation', 'structure', 'other'))
);

-- Indexes
CREATE INDEX data_sources_type_idx ON data_sources(source_type);
CREATE INDEX data_sources_organism_idx ON data_sources(organism_id);
CREATE INDEX data_sources_external_id_idx ON data_sources(external_id);
CREATE INDEX data_sources_metadata_idx ON data_sources USING GIN (additional_metadata);
```

**Key Fields**:
- `source_type`: Category of biological data
- `external_id`: Original identifier from provider
- `organism_id`: Links to taxonomy (optional)
- `additional_metadata`: JSONB for extensibility (e.g., `{"isoforms": 3, "ptm_count": 12}`)

**Important**: No `is_aggregate` flag - any data source can have dependencies

### Tools

Inherits from registry_entries. Represents bioinformatics software/packages.

```sql
CREATE TABLE tools (
    id UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    tool_type VARCHAR(50),  -- 'alignment', 'assembly', 'variant_calling', 'visualization'
    repository_url TEXT,
    homepage_url TEXT,
    license VARCHAR(100),
    additional_metadata JSONB
);

-- Indexes
CREATE INDEX tools_type_idx ON tools(tool_type);
CREATE INDEX tools_metadata_idx ON tools USING GIN (additional_metadata);
```

### Versions

Version management for any registry entry (data sources or tools).

```sql
CREATE TABLE versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES registry_entries(id) ON DELETE CASCADE,
    version VARCHAR(64) NOT NULL,  -- Our opinionated: '1.0', '1.1', '2.0'
    external_version VARCHAR(64),  -- Original: '2025_01' (UniProt), '2.14.0' (BLAST)
    release_date DATE,
    size_bytes BIGINT,  -- Total size of all files
    download_count BIGINT DEFAULT 0,
    additional_metadata JSONB,
    dependency_cache JSONB,  -- Cached dependency list for performance
    dependency_count INT DEFAULT 0,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(entry_id, version)
);

-- Indexes
CREATE INDEX versions_entry_id_idx ON versions(entry_id);
CREATE INDEX versions_version_idx ON versions(version);
CREATE INDEX versions_release_date_idx ON versions(release_date);
CREATE INDEX versions_dependency_cache_idx ON versions USING GIN (dependency_cache);
```

**Dual Versioning**:
- `version`: Our semantic versioning (1.0, 1.1, 2.0) - user-friendly, predictable
- `external_version`: Original provider version (2025_01, v2.14.0) - preserved for reference

**Dependency Caching**:
- `dependency_cache`: JSONB array of dependency IDs for fast queries
- `dependency_count`: Denormalized count for quick lookups
- Updated via trigger when dependencies change

### Version Files

Multiple file formats per version (e.g., FASTA, XML, JSON for same protein).

```sql
CREATE TABLE version_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    format VARCHAR(50) NOT NULL,  -- 'fasta', 'xml', 'dat', 'json', 'tar.gz'
    s3_key TEXT NOT NULL,  -- S3 path: proteins/uniprot/P01308/1.0/P01308.fasta
    checksum VARCHAR(64) NOT NULL,  -- SHA-256
    size_bytes BIGINT NOT NULL,
    compression VARCHAR(20),  -- 'gzip', 'bzip2', 'none'
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(version_id, format)
);

-- Indexes
CREATE INDEX version_files_version_id_idx ON version_files(version_id);
CREATE INDEX version_files_format_idx ON version_files(format);
CREATE INDEX version_files_s3_key_idx ON version_files(s3_key);
```

**Key Design**:
- Each format is a separate row
- Independent checksums per format
- S3 key structure: `{category}/{org}/{entry}/{version}/{filename}`

### Dependencies

Links between versions - any version can depend on others.

```sql
CREATE TABLE dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    depends_on_entry_id UUID NOT NULL REFERENCES registry_entries(id) ON DELETE CASCADE,
    depends_on_version VARCHAR(64) NOT NULL,
    dependency_type VARCHAR(50) DEFAULT 'required',  -- 'required', 'optional'
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(version_id, depends_on_entry_id)
);

-- Indexes
CREATE INDEX dependencies_version_id_idx ON dependencies(version_id);
CREATE INDEX dependencies_depends_on_entry_idx ON dependencies(depends_on_entry_id);
CREATE INDEX dependencies_depends_on_version_idx ON dependencies(depends_on_version);
```

**Important Notes**:
- This table can have millions of rows (e.g., `uniprot:all@1.0` → 567k proteins)
- Proper indexing is critical for performance
- `dependency_cache` in `versions` table provides fast access to full list
- Use pagination for API queries

**Example**:
```sql
-- uniprot:all@1.0 depends on all proteins
INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
SELECT
    (SELECT id FROM versions WHERE entry_id = (SELECT id FROM registry_entries WHERE slug = 'all') AND version = '1.0'),
    re.id,
    '1.0'
FROM registry_entries re
JOIN data_sources ds ON ds.id = re.id
WHERE ds.source_type = 'protein' AND re.organization_id = (SELECT id FROM organizations WHERE slug = 'uniprot');
```

### Organisms (Taxonomy)

Biological taxonomy information.

```sql
CREATE TABLE organisms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ncbi_taxonomy_id INT UNIQUE,
    scientific_name VARCHAR(255) NOT NULL,
    common_name VARCHAR(255),
    rank VARCHAR(50),  -- 'species', 'genus', 'family', 'order'
    lineage TEXT,  -- Full taxonomic lineage
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX organisms_taxonomy_id_idx ON organisms(ncbi_taxonomy_id);
CREATE INDEX organisms_scientific_name_idx ON organisms(scientific_name);
CREATE INDEX organisms_rank_idx ON organisms(rank);
```

**Example Data**:
```sql
INSERT INTO organisms (ncbi_taxonomy_id, scientific_name, common_name, rank, lineage) VALUES
    (9606, 'Homo sapiens', 'Human', 'species',
     'cellular organisms; Eukaryota; Opisthokonta; Metazoa; Eumetazoa; Bilateria; Deuterostomia; Chordata; Craniata; Vertebrata; Gnathostomata; Teleostomi; Euteleostomi; Sarcopterygii; Dipnotetrapodomorpha; Tetrapoda; Amniota; Mammalia; Theria; Eutheria; Boreoeutheria; Euarchontoglires; Primates; Haplorrhini; Simiiformes; Catarrhini; Hominoidea; Hominidae; Homininae; Homo; Homo sapiens');
```

### Protein Metadata

Protein-specific fields extending data_sources.

```sql
CREATE TABLE protein_metadata (
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    accession VARCHAR(50) NOT NULL UNIQUE,  -- P01308
    entry_name VARCHAR(255),  -- INS_HUMAN
    protein_name TEXT,
    gene_name VARCHAR(255),
    sequence_length INT,
    mass_da BIGINT,  -- Molecular mass in Daltons
    sequence_checksum VARCHAR(64),  -- MD5 of amino acid sequence

    CONSTRAINT accession_format_check CHECK (accession ~ '^[A-Z0-9]+$')
);

-- Indexes
CREATE INDEX protein_metadata_accession_idx ON protein_metadata(accession);
CREATE INDEX protein_metadata_gene_name_idx ON protein_metadata(gene_name);

-- Full-text search
CREATE INDEX protein_metadata_search_idx ON protein_metadata
    USING GIN (to_tsvector('english',
        accession || ' ' ||
        COALESCE(entry_name, '') || ' ' ||
        COALESCE(protein_name, '') || ' ' ||
        COALESCE(gene_name, '')
    ));
```

### Citations

Academic references for data sources and tools.

```sql
CREATE TABLE citations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    citation_type VARCHAR(50),  -- 'primary', 'method', 'review'
    doi VARCHAR(255),
    pubmed_id VARCHAR(50),
    title TEXT,
    journal VARCHAR(255),
    publication_date DATE,
    volume VARCHAR(50),
    pages VARCHAR(50),
    authors TEXT,  -- Comma-separated author names
    bibtex TEXT,  -- Pre-generated BibTeX entry
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX citations_version_id_idx ON citations(version_id);
CREATE INDEX citations_doi_idx ON citations(doi);
CREATE INDEX citations_pubmed_idx ON citations(pubmed_id);
```

**BibTeX Example**:
```bibtex
@article{UniProt2023,
  author = {The UniProt Consortium},
  title = {UniProt: the Universal Protein Knowledgebase in 2023},
  journal = {Nucleic Acids Research},
  year = {2023},
  volume = {51},
  pages = {D523-D531},
  doi = {10.1093/nar/gkac1052}
}
```

### Tags

Categorization and filtering.

```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) UNIQUE NOT NULL,
    category VARCHAR(50),  -- 'organism', 'topic', 'format', 'tool_type'
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE entry_tags (
    entry_id UUID REFERENCES registry_entries(id) ON DELETE CASCADE,
    tag_id UUID REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);

-- Indexes
CREATE INDEX tags_category_idx ON tags(category);
CREATE INDEX entry_tags_entry_idx ON entry_tags(entry_id);
CREATE INDEX entry_tags_tag_idx ON entry_tags(tag_id);
```

**Example Tags**:
```sql
INSERT INTO tags (name, category) VALUES
    ('human', 'organism'),
    ('mouse', 'organism'),
    ('membrane-protein', 'topic'),
    ('signaling', 'topic'),
    ('fasta-format', 'format');
```

### Downloads

Track download statistics.

```sql
CREATE TABLE downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    file_id UUID REFERENCES version_files(id) ON DELETE SET NULL,
    downloaded_at TIMESTAMPTZ DEFAULT NOW(),
    user_agent TEXT,
    ip_address INET
);

-- Indexes (partitioned by time for performance)
CREATE INDEX downloads_version_id_idx ON downloads(version_id);
CREATE INDEX downloads_downloaded_at_idx ON downloads(downloaded_at DESC);

-- For analytics
CREATE INDEX downloads_date_idx ON downloads(DATE(downloaded_at));
```

### Version Mappings

Maps external versions to our internal semantic versions.

```sql
CREATE TABLE version_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_slug VARCHAR(100) NOT NULL,
    external_version VARCHAR(64) NOT NULL,
    internal_version VARCHAR(64) NOT NULL,
    release_date DATE,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(organization_slug, external_version)
);

-- Indexes
CREATE INDEX version_mappings_org_idx ON version_mappings(organization_slug);
CREATE INDEX version_mappings_external_idx ON version_mappings(external_version);
CREATE INDEX version_mappings_internal_idx ON version_mappings(internal_version);
```

**Example Mappings**:
```sql
INSERT INTO version_mappings (organization_slug, external_version, internal_version, release_date) VALUES
    ('uniprot', '2020_01', '1.0', '2020-01-15'),
    ('uniprot', '2020_02', '1.1', '2020-02-12'),
    ('uniprot', '2021_01', '2.0', '2021-01-13'),
    ('ncbi', 'v2.13.0', '1.0', '2022-05-01'),
    ('ncbi', 'v2.14.0', '1.1', '2023-11-15');
```

## Triggers and Functions

### Update Dependency Cache

Maintains denormalized dependency cache in `versions` table.

```sql
CREATE OR REPLACE FUNCTION update_dependency_cache()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE versions
    SET
        dependency_cache = (
            SELECT jsonb_agg(
                jsonb_build_object(
                    'entry_id', d.depends_on_entry_id,
                    'version', d.depends_on_version,
                    'type', d.dependency_type
                )
            )
            FROM dependencies d
            WHERE d.version_id = NEW.version_id
        ),
        dependency_count = (
            SELECT COUNT(*) FROM dependencies WHERE version_id = NEW.version_id
        )
    WHERE id = NEW.version_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER dependency_cache_trigger
AFTER INSERT OR UPDATE OR DELETE ON dependencies
FOR EACH ROW EXECUTE FUNCTION update_dependency_cache();
```

### Update Version Size

Maintains total size in `versions` table.

```sql
CREATE OR REPLACE FUNCTION update_version_size()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE versions
    SET size_bytes = (
        SELECT COALESCE(SUM(size_bytes), 0)
        FROM version_files
        WHERE version_id = NEW.version_id
    )
    WHERE id = NEW.version_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER version_size_trigger
AFTER INSERT OR UPDATE OR DELETE ON version_files
FOR EACH ROW EXECUTE FUNCTION update_version_size();
```

### Update Timestamps

Auto-update `updated_at` columns.

```sql
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER registry_entries_updated_at
BEFORE UPDATE ON registry_entries
FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER organizations_updated_at
BEFORE UPDATE ON organizations
FOR EACH ROW EXECUTE FUNCTION update_updated_at();
```

## Query Examples

### Get Data Source with All Versions

```sql
SELECT
    re.slug,
    re.name,
    o.name as organization,
    ds.source_type,
    json_agg(
        json_build_object(
            'version', v.version,
            'external_version', v.external_version,
            'release_date', v.release_date,
            'size', v.size_bytes,
            'formats', (
                SELECT json_agg(vf.format)
                FROM version_files vf
                WHERE vf.version_id = v.id
            )
        ) ORDER BY v.release_date DESC
    ) as versions
FROM registry_entries re
JOIN organizations o ON o.id = re.organization_id
JOIN data_sources ds ON ds.id = re.id
JOIN versions v ON v.entry_id = re.id
WHERE re.slug = 'P01308'
GROUP BY re.slug, re.name, o.name, ds.source_type;
```

### Search Across Proteins and Tools

```sql
SELECT
    re.slug,
    re.name,
    re.entry_type,
    o.name as organization,
    CASE
        WHEN pm.accession IS NOT NULL THEN pm.accession
        ELSE NULL
    END as protein_accession,
    ts_rank(
        to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')),
        plainto_tsquery('english', 'insulin')
    ) as rank
FROM registry_entries re
JOIN organizations o ON o.id = re.organization_id
LEFT JOIN data_sources ds ON ds.id = re.id
LEFT JOIN protein_metadata pm ON pm.data_source_id = ds.id
WHERE
    to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')) @@ plainto_tsquery('english', 'insulin')
ORDER BY rank DESC
LIMIT 20;
```

### Get Dependencies with Pagination

```sql
SELECT
    re.slug,
    re.name,
    d.depends_on_version,
    ds.source_type
FROM dependencies d
JOIN registry_entries re ON re.id = d.depends_on_entry_id
JOIN data_sources ds ON ds.id = re.id
WHERE d.version_id = ?
ORDER BY re.slug
LIMIT 1000 OFFSET 0;
```

### Get Aggregate Statistics

```sql
SELECT
    o.name as organization,
    COUNT(DISTINCT re.id) as total_entries,
    COUNT(DISTINCT CASE WHEN re.entry_type = 'data_source' THEN re.id END) as data_sources,
    COUNT(DISTINCT CASE WHEN re.entry_type = 'tool' THEN re.id END) as tools,
    COUNT(DISTINCT v.id) as total_versions,
    SUM(v.size_bytes) as total_size_bytes
FROM organizations o
LEFT JOIN registry_entries re ON re.organization_id = o.id
LEFT JOIN versions v ON v.entry_id = re.id
GROUP BY o.id, o.name
ORDER BY total_entries DESC;
```

## Performance Considerations

1. **Indexing Strategy**:
   - All foreign keys have indexes
   - Full-text search uses GIN indexes
   - JSONB columns have GIN indexes for fast lookups
   - Composite indexes for common query patterns

2. **Partitioning** (future optimization):
   - Partition `downloads` table by date (monthly)
   - Partition `dependencies` table by version_id hash

3. **Materialized Views** (future):
   - Popular packages/tools
   - Recent downloads
   - Trending searches

4. **Connection Pooling**:
   - Use PgBouncer or SQLx connection pool
   - Max connections: 2-5x CPU cores

5. **Query Optimization**:
   - Use prepared statements
   - Avoid N+1 queries with joins
   - Paginate large result sets
   - Cache frequent queries in Redis (later)

## Migration Strategy

Migrations managed by SQLx CLI:

```bash
sqlx migrate add initial_schema
sqlx migrate add add_version_mappings
sqlx migrate add add_protein_metadata
sqlx migrate run
```

Keep migrations atomic and reversible where possible.

## Related Documents

- [API Design](./api-design.md) - REST endpoints using this schema
- [Version Mapping](./version-mapping.md) - Version translation logic
- [Dependency Resolution](./dependency-resolution.md) - How dependencies work
