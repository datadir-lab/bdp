-- Tracks each bdp-embed pipeline run (embed → project → tiles).
-- status values: 'pending' | 'embedding' | 'projecting' | 'tiling' | 'complete' | 'failed'
-- Frontend reads current_run_id from /api/v1/vectors/stats to build tile URLs.
CREATE TABLE vector_projection_runs (
    run_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    stage_completed VARCHAR(20),
    entry_count     BIGINT,
    embedded_count  BIGINT,
    projected_count BIGINT,
    tile_prefix     TEXT,
    error_message   TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    projected_at    TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ
);
