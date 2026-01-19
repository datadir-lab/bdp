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
    UPDATE ingestion_work_units wu
    SET
        status = 'processing',
        worker_id = p_worker_id::TEXT,
        claimed_at = NOW(),
        heartbeat_at = NOW()
    WHERE wu.id = (
        SELECT wu2.id
        FROM ingestion_work_units wu2
        WHERE wu2.job_id = p_job_id
          AND wu2.status = 'pending'
        ORDER BY wu2.batch_number
        LIMIT 1
        FOR UPDATE SKIP LOCKED
    )
    RETURNING
        wu.id,
        wu.batch_number::INT,
        wu.start_offset::BIGINT,
        wu.end_offset::BIGINT,
        wu.record_count::INT;
END;
$$ LANGUAGE plpgsql;
