use anyhow::Result;
use rusqlite::Connection;

/// Check if a near-duplicate memory already exists using FTS5 search.
///
/// Returns the ID of the existing memory if a close match is found.
/// A match is considered "close" if the FTS5 bm25 score indicates high relevance
/// and the content lengths are similar (within 50% of each other).
pub fn check_duplicate(conn: &Connection, content: &str) -> Result<Option<String>> {
    // Extract significant words for the FTS5 query.
    // We take up to 8 words, using OR matching so that partial overlap still finds candidates.
    let query_words: Vec<&str> = content
        .split_whitespace()
        .filter(|w| w.len() > 3) // skip very short words (articles, prepositions)
        .take(8)
        .collect();

    if query_words.is_empty() {
        return Ok(None);
    }

    // Use FTS5 OR matching to find candidates, then filter by content similarity
    let fts_query = query_words.join(" OR ");

    let mut stmt = conn.prepare(
        "SELECT m.id, m.content, bm25(memory_fts) as rank
         FROM memory_fts
         JOIN memory m ON m.rowid = memory_fts.rowid
         WHERE memory_fts MATCH ?1
         ORDER BY rank
         LIMIT 5",
    )?;

    let results: Vec<(String, String, f64)> = stmt
        .query_map([&fts_query], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let content_len = content.len() as f64;
    let content_words: std::collections::HashSet<&str> =
        content.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric())).collect();

    for (id, existing_content, _rank) in results {
        let existing_len = existing_content.len() as f64;

        // Check length similarity: within 50% of each other
        let len_ratio = if content_len > existing_len {
            existing_len / content_len
        } else {
            content_len / existing_len
        };

        if len_ratio <= 0.5 {
            continue;
        }

        // Check word overlap: at least 60% of words in common
        let existing_words: std::collections::HashSet<&str> = existing_content
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .collect();
        let intersection = content_words.intersection(&existing_words).count();
        let union = content_words.union(&existing_words).count();

        if union > 0 {
            let jaccard = intersection as f64 / union as f64;
            if jaccard > 0.5 {
                return Ok(Some(id));
            }
        }
    }

    Ok(None)
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
    fn test_no_duplicate_in_empty_db() {
        let (_dir, conn) = setup_db();
        let result = check_duplicate(&conn, "This is a brand new memory").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_finds_duplicate() {
        let (_dir, conn) = setup_db();

        // Insert a memory
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-001",
                "Rust programming language is great for systems programming",
                "tech",
                0.5,
                false,
                now,
                now,
            ],
        )
        .unwrap();

        // Search for near-duplicate
        let result = check_duplicate(
            &conn,
            "Rust programming language is great for systems programming and more",
        )
        .unwrap();
        assert_eq!(result, Some("mem-001".to_string()));
    }

    #[test]
    fn test_no_duplicate_for_different_content() {
        let (_dir, conn) = setup_db();

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-001",
                "Rust programming language is great for systems programming",
                "tech",
                0.5,
                false,
                now,
                now,
            ],
        )
        .unwrap();

        // Very different content should not match
        let result =
            check_duplicate(&conn, "The weather today is sunny and warm with clear skies").unwrap();
        assert!(result.is_none());
    }
}
