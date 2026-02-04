-- Add parsed_at column to taxonomy_metadata for tracking when entries were parsed
ALTER TABLE taxonomy_metadata ADD COLUMN IF NOT EXISTS parsed_at TIMESTAMPTZ DEFAULT NOW();
