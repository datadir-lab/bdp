# Search Performance Optimization Guide

This document describes the search performance optimizations implemented in BDP and how to configure PostgreSQL for optimal search performance.

## Overview

BDP's search functionality is optimized to handle millions of data sources with sub-second response times. The key optimization is a materialized view that pre-computes expensive aggregations and joins.

## Architecture

### Materialized View

The `search_registry_entries_mv` materialized view pre-computes:

- Latest version information for each data source
- External version metadata
- Available file formats (aggregated from all versions)
- Total download counts across all versions
- Organism information (from taxonomy metadata)
- Full-text search vectors

This eliminates N+1 query problems and reduces complex joins from O(n×m) to O(1).

### Indexes

Multiple indexes ensure fast filtering and ranking:

1. **Full-text search index** (GIN): Primary search vector index
2. **Filter indexes** (B-tree): Entry type, source type, organization
3. **Pattern indexes** (text_pattern_ops): ILIKE queries on organism names
4. **Array index** (GIN): Format array containment queries
5. **Unique index**: Required for concurrent materialized view refresh

## PostgreSQL Configuration

For optimal search performance with millions of data sources, tune these PostgreSQL settings:

### Memory Settings

```sql
-- Increase work_mem for sorting and ranking operations
-- Each search query may use this much memory per sort/hash operation
ALTER SYSTEM SET work_mem = '256MB';

-- Increase shared_buffers to cache more frequently accessed data
-- Rule of thumb: 25% of system RAM (adjust based on dedicated vs shared server)
ALTER SYSTEM SET shared_buffers = '4GB';

-- Set effective_cache_size to inform the query planner
-- Should be ~50-75% of total system RAM
ALTER SYSTEM SET effective_cache_size = '12GB';

-- Increase maintenance_work_mem for faster index creation and VACUUM
ALTER SYSTEM SET maintenance_work_mem = '1GB';
```

### Query Planner Settings

```sql
-- Enable parallel query execution for large searches
ALTER SYSTEM SET max_parallel_workers_per_gather = 4;
ALTER SYSTEM SET max_parallel_workers = 8;
ALTER SYSTEM SET parallel_tuple_cost = 0.01;
ALTER SYSTEM SET parallel_setup_cost = 100;

-- Lower random_page_cost for SSD storage
ALTER SYSTEM SET random_page_cost = 1.1;

-- Adjust cost parameters for full-text search
ALTER SYSTEM SET cpu_tuple_cost = 0.01;
ALTER SYSTEM SET cpu_operator_cost = 0.0025;
```

### Connection Settings

```sql
-- Increase max connections for high-concurrency search workload
ALTER SYSTEM SET max_connections = 200;

-- Connection pooling settings (adjust based on workload)
ALTER SYSTEM SET idle_in_transaction_session_timeout = '5min';
ALTER SYSTEM SET statement_timeout = '30s';
```

### Apply Configuration Changes

After modifying settings, reload PostgreSQL:

```bash
# Reload configuration (for most settings)
SELECT pg_reload_conf();

# Or restart PostgreSQL (required for shared_buffers changes)
sudo systemctl restart postgresql
```

### Verify Settings

```sql
-- Check current settings
SELECT name, setting, unit, context
FROM pg_settings
WHERE name IN (
    'work_mem',
    'shared_buffers',
    'effective_cache_size',
    'maintenance_work_mem',
    'max_parallel_workers_per_gather'
);
```

## Materialized View Refresh Strategy

The materialized view needs to be refreshed periodically to reflect new data.

### Refresh Modes

**Concurrent Refresh** (Recommended for Production)
```sql
SELECT refresh_search_mv_concurrent();
```
- Non-blocking: Searches continue during refresh
- Slower: ~2-3x longer than non-concurrent
- Safe for production use during business hours

**Non-Concurrent Refresh** (Faster but Blocking)
```sql
SELECT refresh_search_mv();
```
- Blocking: Searches are unavailable during refresh
- Faster: Completes in 30-50% less time
- Best for maintenance windows or initial population

### Scheduling Refresh

#### Using Cron

Refresh every 5 minutes (concurrent mode):

```bash
# Add to crontab
*/5 * * * * cd /path/to/bdp && cargo run --release --example refresh_search_index
```

Refresh nightly at 2 AM (non-concurrent mode for speed):

```bash
0 2 * * * cd /path/to/bdp && cargo run --release --example refresh_search_index -- --no-concurrent
```

#### Using systemd Timer

Create `/etc/systemd/system/bdp-search-refresh.service`:

```ini
[Unit]
Description=BDP Search Index Refresh
After=network.target postgresql.service

[Service]
Type=oneshot
User=bdp
WorkingDirectory=/opt/bdp
ExecStart=/opt/bdp/target/release/examples/refresh_search_index
Environment=DATABASE_URL=postgresql://user:pass@localhost/bdp
StandardOutput=journal
StandardError=journal
```

Create `/etc/systemd/system/bdp-search-refresh.timer`:

```ini
[Unit]
Description=BDP Search Index Refresh Timer
Requires=bdp-search-refresh.service

[Timer]
OnBootSec=5min
OnUnitActiveSec=5min
Unit=bdp-search-refresh.service

[Install]
WantedBy=timers.target
```

Enable and start the timer:

```bash
sudo systemctl daemon-reload
sudo systemctl enable bdp-search-refresh.timer
sudo systemctl start bdp-search-refresh.timer
```

### Manual Refresh

```bash
# Concurrent refresh (safe for production)
cargo run --release --example refresh_search_index

# Non-concurrent refresh (faster)
cargo run --release --example refresh_search_index -- --no-concurrent
```

### Monitoring Refresh Performance

```sql
-- Check materialized view size
SELECT
    schemaname,
    matviewname,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||matviewname)) as size,
    n_tup_ins as rows
FROM pg_stat_user_tables
WHERE relname = 'search_registry_entries_mv';

-- Check last refresh time (requires tracking table)
SELECT * FROM pg_stat_user_tables
WHERE relname = 'search_registry_entries_mv';
```

## Performance Benchmarks

Expected performance metrics (varies by hardware and data size):

| Data Sources | Concurrent Refresh | Non-Concurrent Refresh | Search Query (p95) |
|--------------|-------------------|------------------------|-------------------|
| 100K         | ~30s              | ~10s                   | <50ms             |
| 1M           | ~5min             | ~2min                  | <100ms            |
| 10M          | ~30min            | ~15min                 | <200ms            |

Search query performance is nearly constant due to indexes, regardless of total data size.

## Troubleshooting

### Slow Searches

1. **Check if MV is up-to-date**: Compare row counts between base tables and MV
2. **Verify indexes exist**: Run `\d+ search_registry_entries_mv` in psql
3. **Check query plans**: Use `EXPLAIN ANALYZE` on search queries
4. **Monitor cache hit rates**: Check `pg_stat_database` for cache statistics

### Slow MV Refresh

1. **Increase maintenance_work_mem**: Helps with index building during refresh
2. **Check for locks**: Look for blocking queries with `pg_locks`
3. **Vacuum regularly**: Run `VACUUM ANALYZE` on base tables
4. **Consider partitioning**: For >10M data sources, partition the MV

### Out of Memory During Refresh

1. **Increase work_mem and maintenance_work_mem**: See configuration section above
2. **Reduce max_parallel_workers**: Lower parallelism uses less memory
3. **Use non-concurrent refresh**: More memory-efficient than concurrent

## Best Practices

1. **Refresh frequently**: 5-15 minute intervals keep data fresh without overwhelming the database
2. **Monitor refresh duration**: Alert if refresh takes longer than expected
3. **Use concurrent refresh in production**: Non-blocking is worth the performance cost
4. **Schedule non-concurrent refresh during low-traffic periods**: For faster updates
5. **Vacuum and analyze regularly**: Keeps query plans optimal
6. **Monitor disk space**: MV can be 30-50% of base table size

## Advanced: Read Replicas

For very high search query volume, consider using PostgreSQL read replicas:

1. Set up streaming replication to one or more read replicas
2. Direct all search queries to read replicas
3. Keep writes (data ingestion) on the primary
4. Refresh the MV on the primary; it replicates to read replicas

This horizontally scales search capacity without impacting write performance.

## Related Documentation

- [PostgreSQL Full-Text Search](https://www.postgresql.org/docs/current/textsearch.html)
- [Materialized Views](https://www.postgresql.org/docs/current/rules-materializedviews.html)
- [GIN Indexes](https://www.postgresql.org/docs/current/gin.html)
