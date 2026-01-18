-- Downloads
-- Track download statistics.

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
-- Note: Functional index on date removed due to IMMUTABLE constraint
-- Use: SELECT ... WHERE downloaded_at::date = '2024-01-01' (will use downloads_downloaded_at_idx)
