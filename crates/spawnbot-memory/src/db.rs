use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS memory (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            category TEXT NOT NULL,
            importance REAL DEFAULT 0.5,
            evergreen BOOLEAN DEFAULT FALSE,
            embedding BLOB,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
            content, content=memory, content_rowid=rowid
        );

        CREATE TRIGGER IF NOT EXISTS memory_ai AFTER INSERT ON memory BEGIN
            INSERT INTO memory_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_ad AFTER DELETE ON memory BEGIN
            INSERT INTO memory_fts(memory_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_au AFTER UPDATE ON memory BEGIN
            INSERT INTO memory_fts(memory_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO memory_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TABLE IF NOT EXISTS memory_vec_map (
            vec_rowid INTEGER PRIMARY KEY AUTOINCREMENT,
            memory_id TEXT NOT NULL UNIQUE REFERENCES memory(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS memory_chunks (
            id TEXT PRIMARY KEY,
            source_path TEXT NOT NULL,
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            embedding BLOB,
            created_at INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_chunks_fts USING fts5(
            content, content=memory_chunks, content_rowid=rowid
        );

        CREATE TRIGGER IF NOT EXISTS memory_chunks_ai AFTER INSERT ON memory_chunks BEGIN
            INSERT INTO memory_chunks_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_chunks_ad AFTER DELETE ON memory_chunks BEGIN
            INSERT INTO memory_chunks_fts(memory_chunks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_chunks_au AFTER UPDATE ON memory_chunks BEGIN
            INSERT INTO memory_chunks_fts(memory_chunks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO memory_chunks_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TABLE IF NOT EXISTS memory_chunks_vec_map (
            vec_rowid INTEGER PRIMARY KEY AUTOINCREMENT,
            chunk_id TEXT NOT NULL UNIQUE REFERENCES memory_chunks(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL,
            applied_at INTEGER NOT NULL
        );

        INSERT OR IGNORE INTO schema_version VALUES (1, strftime('%s', 'now') * 1000);
    ",
    )?;

    // Note: sqlite-vec virtual tables (memory_vec, memory_chunks_vec) are skipped for now.
    // They require the sqlite-vec extension which may not be available in all environments.
    // Vector search will gracefully degrade to FTS5-only when not available.

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_creates_tables() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"memory".to_string()));
        assert!(tables.contains(&"memory_chunks".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
        assert!(tables.contains(&"memory_vec_map".to_string()));
        assert!(tables.contains(&"memory_chunks_vec_map".to_string()));
    }

    #[test]
    fn test_schema_version() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();
        let version: i64 = conn
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_idempotent_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let _conn1 = init_db(&db_path).unwrap();
        // Second init should succeed without error
        let conn2 = init_db(&db_path).unwrap();
        let version: i64 = conn2
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }
}
