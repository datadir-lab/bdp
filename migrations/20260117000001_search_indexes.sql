-- Search performance indexes
-- This migration adds indexes for autocomplete (trigram) and optimized version lookups

-- Enable pg_trgm extension for trigram similarity search (autocomplete)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram index for autocomplete on registry_entries.name
-- This enables fast prefix/fuzzy matching for the suggestions endpoint
CREATE INDEX IF NOT EXISTS idx_registry_entries_name_trgm
    ON registry_entries USING GIN (name gin_trgm_ops);

-- Trigram index for autocomplete on organizations.name
CREATE INDEX IF NOT EXISTS idx_organizations_name_trgm
    ON organizations USING GIN (name gin_trgm_ops);

-- Full-text search index for organizations (if not already exists)
CREATE INDEX IF NOT EXISTS idx_organizations_fts
    ON organizations USING GIN (
        to_tsvector('english', name || ' ' || COALESCE(description, ''))
    );

-- Version lookup optimization index
-- This speeds up queries that need to find the latest version for an entry
CREATE INDEX IF NOT EXISTS idx_versions_entry_id_published
    ON versions (entry_id, published_at DESC);

-- Comment on indexes for documentation
COMMENT ON INDEX idx_registry_entries_name_trgm IS 'Trigram index for fast autocomplete on registry entry names';
COMMENT ON INDEX idx_organizations_name_trgm IS 'Trigram index for fast autocomplete on organization names';
COMMENT ON INDEX idx_organizations_fts IS 'Full-text search index for organizations';
COMMENT ON INDEX idx_versions_entry_id_published IS 'Optimizes queries for latest version lookup';
