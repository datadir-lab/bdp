-- Tags
-- Categorization and filtering.

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
