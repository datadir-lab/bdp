-- Create cache_entries table
CREATE TABLE IF NOT EXISTS cache_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec TEXT NOT NULL UNIQUE,
    resolved TEXT NOT NULL,
    format TEXT NOT NULL,
    checksum TEXT NOT NULL,
    size INTEGER NOT NULL,
    cached_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_accessed DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    path TEXT NOT NULL
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_cache_entries_spec ON cache_entries(spec);
CREATE INDEX IF NOT EXISTS idx_cache_entries_resolved ON cache_entries(resolved);
CREATE INDEX IF NOT EXISTS idx_cache_entries_cached_at ON cache_entries(cached_at);
CREATE INDEX IF NOT EXISTS idx_cache_entries_last_accessed ON cache_entries(last_accessed);
