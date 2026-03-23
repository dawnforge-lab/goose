use anyhow::Result;
use rusqlite::Connection;

/// Result of a store operation, indicating whether a new memory was inserted
/// or an existing one was updated/merged.
#[derive(Debug, PartialEq)]
pub enum StoreResult {
    /// A new memory was inserted with the given ID.
    Inserted { id: String },
    /// An existing memory was updated (merged). Contains the existing memory's ID.
    Merged { id: String },
}

impl StoreResult {
    pub fn id(&self) -> &str {
        match self {
            StoreResult::Inserted { id } | StoreResult::Merged { id } => id,
        }
    }
}

/// Store a memory, checking for duplicates first.
///
/// If a near-duplicate is found via FTS5 search, the existing memory is updated
/// with the new content (merged). Otherwise, a new memory is inserted.
///
/// The FTS5 triggers automatically handle indexing on insert/update/delete.
pub fn store_memory(
    conn: &Connection,
    content: &str,
    category: &str,
    importance: f64,
    evergreen: bool,
) -> Result<StoreResult> {
    // Check for duplicates first
    if let Some(existing_id) = crate::dedup::check_duplicate(conn, content)? {
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE memory SET content = ?1, category = ?2, importance = ?3, evergreen = ?4, updated_at = ?5
             WHERE id = ?6",
            rusqlite::params![content, category, importance, evergreen, now, existing_id],
        )?;
        return Ok(StoreResult::Merged { id: existing_id });
    }

    // Generate ULID for new memory
    let id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, content, category, importance, evergreen, now, now],
    )?;

    Ok(StoreResult::Inserted { id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::tempdir;

    fn setup_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();
        (dir, conn)
    }

    #[test]
    fn test_insert_new_memory() {
        let (_dir, conn) = setup_db();
        let result = store_memory(
            &conn,
            "Rust is a systems programming language",
            "tech",
            0.8,
            false,
        )
        .unwrap();

        match &result {
            StoreResult::Inserted { id } => {
                assert!(!id.is_empty(), "ID should not be empty");
                // Verify it's in the database
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM memory WHERE id = ?1",
                        [id.as_str()],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(count, 1);
            }
            StoreResult::Merged { .. } => panic!("Expected Inserted, got Merged"),
        }
    }

    #[test]
    fn test_insert_duplicate_gets_merged() {
        let (_dir, conn) = setup_db();

        // Insert initial memory
        let result1 = store_memory(
            &conn,
            "Rust programming language is great for building reliable software",
            "tech",
            0.7,
            false,
        )
        .unwrap();
        let first_id = result1.id().to_string();

        // Insert near-duplicate
        let result2 = store_memory(
            &conn,
            "Rust programming language is great for building reliable software systems",
            "tech",
            0.9,
            false,
        )
        .unwrap();

        match &result2 {
            StoreResult::Merged { id } => {
                assert_eq!(id, &first_id, "Should merge with existing memory");
                // Verify the content was updated
                let content: String = conn
                    .query_row(
                        "SELECT content FROM memory WHERE id = ?1",
                        [id.as_str()],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert!(content.contains("systems"), "Content should be updated");
            }
            StoreResult::Inserted { .. } => panic!("Expected Merged, got Inserted"),
        }

        // Verify only one memory exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_insert_different_memories() {
        let (_dir, conn) = setup_db();

        store_memory(&conn, "Rust is a systems language", "tech", 0.7, false).unwrap();

        store_memory(
            &conn,
            "Python is great for data science",
            "tech",
            0.6,
            false,
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Two distinct memories should exist");
    }
}
