use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

/// A memory entry returned by browsing.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: String,
    pub importance: f64,
    pub evergreen: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Browse memories, optionally filtering by category, ordered by importance descending.
pub fn browse_memories(
    conn: &Connection,
    category: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<MemoryEntry>> {
    let sql = if category.is_some() {
        "SELECT id, content, category, importance, evergreen, created_at, updated_at
         FROM memory
         WHERE category = ?1
         ORDER BY importance DESC, updated_at DESC
         LIMIT ?2 OFFSET ?3"
    } else {
        "SELECT id, content, category, importance, evergreen, created_at, updated_at
         FROM memory
         ORDER BY importance DESC, updated_at DESC
         LIMIT ?1 OFFSET ?2"
    };

    let mut stmt = conn.prepare(sql)?;

    let entries: Vec<MemoryEntry> = if let Some(cat) = category {
        stmt.query_map(
            rusqlite::params![cat, limit as i64, offset as i64],
            |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    importance: row.get(3)?,
                    evergreen: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(rusqlite::params![limit as i64, offset as i64], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                category: row.get(2)?,
                importance: row.get(3)?,
                evergreen: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::tempdir;

    fn setup_db_with_memories() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["mem-001", "High importance tech", "tech", 0.9, false, now, now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["mem-002", "Low importance tech", "tech", 0.3, false, now, now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["mem-003", "Personal note", "personal", 0.5, true, now, now],
        )
        .unwrap();

        (dir, conn)
    }

    #[test]
    fn test_browse_all() {
        let (_dir, conn) = setup_db_with_memories();
        let entries = browse_memories(&conn, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 3);
        // Should be ordered by importance descending
        assert!(entries[0].importance >= entries[1].importance);
    }

    #[test]
    fn test_browse_by_category() {
        let (_dir, conn) = setup_db_with_memories();
        let entries = browse_memories(&conn, Some("tech"), 100, 0).unwrap();
        assert_eq!(entries.len(), 2);
        for entry in &entries {
            assert_eq!(entry.category, "tech");
        }
    }

    #[test]
    fn test_browse_with_limit() {
        let (_dir, conn) = setup_db_with_memories();
        let entries = browse_memories(&conn, None, 1, 0).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_browse_with_offset() {
        let (_dir, conn) = setup_db_with_memories();
        let all = browse_memories(&conn, None, 100, 0).unwrap();
        let offset = browse_memories(&conn, None, 100, 1).unwrap();
        assert_eq!(offset.len(), all.len() - 1);
    }
}
