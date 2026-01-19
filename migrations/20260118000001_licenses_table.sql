-- Licenses Table
-- Tracks software and data licenses for registry entries

CREATE TABLE licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,           -- "CC-BY-4.0"
    full_name VARCHAR(500) NOT NULL,             -- "Creative Commons Attribution 4.0"
    url TEXT,                                    -- https://creativecommons.org/licenses/by/4.0/
    spdx_identifier VARCHAR(100),                -- "CC-BY-4.0" (standardized)
    requires_attribution BOOLEAN DEFAULT FALSE,
    allows_commercial BOOLEAN DEFAULT TRUE,
    allows_derivatives BOOLEAN DEFAULT TRUE,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for lookups
CREATE INDEX licenses_name_idx ON licenses(name);
CREATE INDEX licenses_spdx_idx ON licenses(spdx_identifier);

-- Seed common bioinformatics licenses
INSERT INTO licenses (name, full_name, url, spdx_identifier, requires_attribution, allows_commercial, allows_derivatives, description) VALUES
('CC-BY-4.0', 'Creative Commons Attribution 4.0 International', 'https://creativecommons.org/licenses/by/4.0/', 'CC-BY-4.0', TRUE, TRUE, TRUE, 'Permits sharing and adaptation with attribution'),
('CC0-1.0', 'Creative Commons Zero v1.0 Universal', 'https://creativecommons.org/publicdomain/zero/1.0/', 'CC0-1.0', FALSE, TRUE, TRUE, 'Public domain dedication'),
('ODC-By-1.0', 'Open Data Commons Attribution License v1.0', 'https://opendatacommons.org/licenses/by/1-0/', 'ODC-By-1.0', TRUE, TRUE, TRUE, 'Database-specific attribution license'),
('Apache-2.0', 'Apache License 2.0', 'https://www.apache.org/licenses/LICENSE-2.0', 'Apache-2.0', TRUE, TRUE, TRUE, 'Permissive software license'),
('Proprietary', 'Proprietary License', NULL, NULL, NULL, FALSE, FALSE, 'Custom or restricted license terms');

-- Add license_id to registry_entries
ALTER TABLE registry_entries
ADD COLUMN license_id UUID REFERENCES licenses(id);

-- Index for license lookups
CREATE INDEX registry_entries_license_idx ON registry_entries(license_id);

-- Comments
COMMENT ON TABLE licenses IS 'Software and data licenses for registry entries';
COMMENT ON COLUMN licenses.spdx_identifier IS 'Standardized SPDX license identifier';
COMMENT ON COLUMN registry_entries.license_id IS 'License for this registry entry';
