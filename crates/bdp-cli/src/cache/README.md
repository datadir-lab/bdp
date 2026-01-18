# BDP CLI Cache

This directory contains caching mechanisms for the CLI to improve performance and reduce server load.

## Purpose

The CLI cache provides:

- Local caching of frequently accessed data
- Offline query capabilities
- Reduced latency for repeated queries
- Bandwidth optimization

## Cache Strategies

### Local File Cache
- Stores query results in local files
- Uses JSON or MessagePack for serialization
- Implements TTL (Time To Live) for cache invalidation

### Memory Cache
- In-memory caching for the current session
- LRU (Least Recently Used) eviction policy
- Configurable size limits

## Cache Location

By default, cache files are stored in:
- Linux/macOS: `~/.cache/bdp/`
- Windows: `%LOCALAPPDATA%\bdp\cache\`

Override with `BDP_CACHE_DIR` environment variable.

## Cache Management

Users can manage the cache with CLI commands:

```bash
# Clear all cache
bdp cache clear

# Show cache statistics
bdp cache stats

# Set cache expiration
bdp cache set-ttl --days 7
```

## Implementation Guidelines

- Use `serde` for serialization
- Implement atomic file operations to prevent corruption
- Add cache versioning for schema changes
- Respect user privacy and don't cache sensitive data
- Provide clear error messages if cache is corrupted
