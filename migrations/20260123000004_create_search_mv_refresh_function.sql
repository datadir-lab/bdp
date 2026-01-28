-- Create functions for refreshing the search materialized view
-- This allows scheduled or on-demand refresh of the search index

-- Function to refresh the search materialized view concurrently
-- CONCURRENTLY allows the view to remain available during refresh
CREATE OR REPLACE FUNCTION refresh_search_mv_concurrent()
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    -- REFRESH MATERIALIZED VIEW CONCURRENTLY requires a unique index (which we have on id)
    -- This allows queries to continue using the old data while refresh is in progress
    REFRESH MATERIALIZED VIEW CONCURRENTLY search_registry_entries_mv;

    -- Log the refresh
    RAISE NOTICE 'Search materialized view refreshed at %', NOW();
END;
$$;

-- Function to refresh the search materialized view non-concurrently (faster but blocks reads)
-- Use this for initial population or when you can afford downtime
CREATE OR REPLACE FUNCTION refresh_search_mv()
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    REFRESH MATERIALIZED VIEW search_registry_entries_mv;
    RAISE NOTICE 'Search materialized view refreshed (non-concurrent) at %', NOW();
END;
$$;

-- Create a job to refresh the search MV periodically
-- This integrates with the existing pgmq job system
-- Refresh every 5 minutes (adjust as needed based on data freshness requirements)
CREATE OR REPLACE FUNCTION schedule_search_mv_refresh()
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    -- Check if the job is already scheduled
    IF NOT EXISTS (
        SELECT 1 FROM pgmq.queue
        WHERE message::jsonb->>'type' = 'refresh_search_mv'
        AND read_ct = 0
    ) THEN
        -- Add a job to refresh the search MV
        PERFORM pgmq.send(
            'default',
            jsonb_build_object(
                'type', 'refresh_search_mv',
                'scheduled_at', NOW()
            )::text
        );
    END IF;
END;
$$;

-- Comments
COMMENT ON FUNCTION refresh_search_mv_concurrent() IS 'Refreshes the search materialized view concurrently (non-blocking)';
COMMENT ON FUNCTION refresh_search_mv() IS 'Refreshes the search materialized view (blocking but faster)';
COMMENT ON FUNCTION schedule_search_mv_refresh() IS 'Schedules a job to refresh the search materialized view';

-- Perform initial population of the materialized view
-- This may take a while on large datasets
SELECT refresh_search_mv();
