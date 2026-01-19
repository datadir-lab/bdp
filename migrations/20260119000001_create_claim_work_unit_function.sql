-- Drop existing function if it exists
DROP FUNCTION IF EXISTS claim_work_unit(UUID, UUID, TEXT);

-- Create function to atomically claim a work unit using SKIP LOCKED
CREATE FUNCTION claim_work_unit(
    p_job_id UUID,
    p_worker_id UUID,
    p_hostname TEXT
)
RETURNS TABLE (
    id UUID,
    batch_number INT,
    start_offset BIGINT,
    end_offset BIGINT,
    record_count INT
) AS $$
BEGIN
    RETURN QUERY
    UPDATE ingestion_work_units
    SET
        status = 'processing',
        worker_id = p_worker_id,
        worker_hostname = p_hostname,
        started_at = NOW(),
        heartbeat_at = NOW()
    WHERE ingestion_work_units.id = (
        SELECT ingestion_work_units.id
        FROM ingestion_work_units
        WHERE job_id = p_job_id
          AND status = 'pending'
        ORDER BY batch_number
        LIMIT 1
        FOR UPDATE SKIP LOCKED
    )
    RETURNING
        ingestion_work_units.id,
        ingestion_work_units.batch_number,
        ingestion_work_units.start_offset,
        ingestion_work_units.end_offset,
        ingestion_work_units.record_count;
END;
$$ LANGUAGE plpgsql;
