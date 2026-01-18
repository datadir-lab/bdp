-- Triggers and Functions
-- Maintains denormalized caches and auto-updates timestamps

-- Update Dependency Cache
-- Maintains denormalized dependency cache in versions table
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

-- Update Version Size
-- Maintains total size in versions table
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

-- Update Timestamps
-- Auto-update updated_at columns
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
