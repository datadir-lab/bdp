//! SQLite schema for audit trail

use crate::error::Result;
use rusqlite::Connection;

/// Initialize audit database schema
pub fn init_schema(conn: &Connection) -> Result<()> {
    // Create audit_events table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS audit_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            event_type TEXT NOT NULL,

            -- What happened
            source_spec TEXT,
            details TEXT NOT NULL,  -- JSON

            -- Machine context
            machine_id TEXT NOT NULL,

            -- Tamper detection
            event_hash TEXT,
            previous_hash TEXT,

            -- User annotations
            notes TEXT,
            archived BOOLEAN DEFAULT 0
        )
        "#,
        [],
    )?;

    // Create files table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_spec TEXT NOT NULL UNIQUE,
            file_path TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,

            downloaded_at DATETIME,
            download_event_id INTEGER,

            last_verified_at DATETIME,
            verification_status TEXT,

            FOREIGN KEY(download_event_id) REFERENCES audit_events(id) ON DELETE SET NULL
        )
        "#,
        [],
    )?;

    // Create generated_files table (for post-pull outputs)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS generated_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_file_id INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            tool TEXT NOT NULL,
            sha256 TEXT,
            size_bytes INTEGER,

            generated_at DATETIME,
            generation_event_id INTEGER,

            FOREIGN KEY(source_file_id) REFERENCES files(id) ON DELETE CASCADE,
            FOREIGN KEY(generation_event_id) REFERENCES audit_events(id) ON DELETE SET NULL
        )
        "#,
        [],
    )?;

    // Create audit_snapshots table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS audit_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id TEXT NOT NULL UNIQUE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            export_format TEXT NOT NULL,

            event_id_start INTEGER,
            event_id_end INTEGER,
            event_count INTEGER NOT NULL,

            chain_verified BOOLEAN,
            output_path TEXT,

            FOREIGN KEY(event_id_start) REFERENCES audit_events(id),
            FOREIGN KEY(event_id_end) REFERENCES audit_events(id)
        )
        "#,
        [],
    )?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON audit_events(timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_source ON audit_events(source_spec)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_type ON audit_events(event_type)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_files_source ON files(source_spec)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_generated_source ON generated_files(source_file_id)",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_init_schema() {
        let conn = Connection::open_in_memory().unwrap();
        let result = init_schema(&conn);
        assert!(result.is_ok());

        // Verify tables exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();

        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"audit_events".to_string()));
        assert!(tables.contains(&"files".to_string()));
        assert!(tables.contains(&"generated_files".to_string()));
        assert!(tables.contains(&"audit_snapshots".to_string()));
    }

    #[test]
    fn test_schema_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize twice
        init_schema(&conn).unwrap();
        let result = init_schema(&conn);

        // Should not error
        assert!(result.is_ok());
    }
}
