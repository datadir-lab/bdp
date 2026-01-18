-- ============================================================================
-- Ingestion Jobs Infrastructure
-- ============================================================================
--
-- This migration creates the necessary tables for tracking data ingestion jobs
-- and organization sync status. apalis will auto-create its own job tables
-- (apalis_jobs) when first initialized, but we pre-define the organization
-- sync status table for tracking ingestion state per data source organization.
--
-- Created: 2026-01-16
-- Phase: Phase 2 - Data Ingestion Infrastructure
-- ============================================================================

-- Organization sync status tracking
-- Tracks the last successful sync for each organization (e.g., UniProt)
CREATE TABLE organization_sync_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    last_sync_at TIMESTAMPTZ,
    last_version VARCHAR(64),
    last_external_version VARCHAR(64),
    status VARCHAR(50) NOT NULL DEFAULT 'idle',
    total_entries BIGINT DEFAULT 0,
    last_job_id UUID,
    last_error TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(organization_id)
);

-- Indexes for efficient querying
CREATE INDEX org_sync_status_org_id_idx ON organization_sync_status(organization_id);
CREATE INDEX org_sync_status_status_idx ON organization_sync_status(status);

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_organization_sync_status_updated_at
    BEFORE UPDATE ON organization_sync_status
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments for documentation
COMMENT ON TABLE organization_sync_status IS 'Tracks data ingestion sync status for each organization (e.g., UniProt, ChEMBL)';
COMMENT ON COLUMN organization_sync_status.organization_id IS 'Reference to the organization being synced';
COMMENT ON COLUMN organization_sync_status.last_sync_at IS 'Timestamp of the last successful sync';
COMMENT ON COLUMN organization_sync_status.last_version IS 'Internal version identifier of the last sync';
COMMENT ON COLUMN organization_sync_status.last_external_version IS 'External version identifier from the data source (e.g., UniProt release version)';
COMMENT ON COLUMN organization_sync_status.status IS 'Current sync status: idle, running, completed, failed';
COMMENT ON COLUMN organization_sync_status.total_entries IS 'Total number of entries processed in the last sync';
COMMENT ON COLUMN organization_sync_status.last_job_id IS 'Reference to the last apalis job ID';
COMMENT ON COLUMN organization_sync_status.last_error IS 'Error message if last sync failed';

-- Note: apalis will automatically create its job tables on first run:
-- - apalis_jobs: Main job queue table
-- The apalis library manages its own schema and migrations.
