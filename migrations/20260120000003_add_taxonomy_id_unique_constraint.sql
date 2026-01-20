-- Add UNIQUE constraint on taxonomy_id to support foreign key references
-- This allows other tables to reference taxonomy_metadata by taxonomy_id

ALTER TABLE taxonomy_metadata
ADD CONSTRAINT taxonomy_metadata_taxonomy_id_unique UNIQUE (taxonomy_id);
