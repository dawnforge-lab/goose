use anyhow::Result;
use rusqlite::Connection;

/// Delete a memory by its ID.
///
/// Returns `true` if a memory with the given ID was found and deleted,
/// `false` if no memory with that ID exists.
///
/// The CASCADE foreign key on `memory_vec_map` handles cleanup of vector mappings.
/// FTS5 triggers automatically handle removal from the full-text search index.
pub fn delete_memory(conn: &Connection, id: &str) -> Result<bool> {
    let rows_affected = conn.execute("DELETE FROM memory WHERE id = ?1", [id])?;
    Ok(rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::tempdir;

    #[test]
    fn test_delete_existing_memory() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["mem-001", "Test memory", "test", 0.5, false, now, now],
        )
        .unwrap();

        let deleted = delete_memory(&conn, "mem-001").unwrap();
        assert!(deleted, "Should return true for existing memory");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory WHERE id = 'mem-001'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "Memory should be deleted from database");
    }

    #[test]
    fn test_delete_nonexistent_memory() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let deleted = delete_memory(&conn, "nonexistent-id").unwrap();
        assert!(!deleted, "Should return false for nonexistent memory");
    }

    #[test]
    fn test_delete_removes_from_fts() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-001",
                "Unique searchable content about quantum computing",
                "science",
                0.5,
                false,
                now,
                now,
            ],
        )
        .unwrap();

        // Verify FTS finds it
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH 'quantum'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "FTS should find the memory before deletion");

        delete_memory(&conn, "mem-001").unwrap();

        // Verify FTS no longer finds it
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH 'quantum'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "FTS should not find the memory after deletion");
    }
}
