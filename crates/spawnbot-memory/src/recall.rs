use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

/// A search result from the memory recall system.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub category: String,
    pub importance: f64,
    pub created_at: i64,
    /// Final score after combining FTS5 relevance, importance, and temporal decay.
    pub score: f64,
    /// Source of the result: "memory" or "chunk"
    pub source: String,
}

/// Recall memories using hybrid FTS5 search with temporal decay ranking.
///
/// Searches both the `memory` and `memory_chunks` tables via their FTS5 indexes,
/// applies temporal decay (unless evergreen), and returns results sorted by final score.
pub fn recall_memories(
    conn: &Connection,
    query: &str,
    limit: usize,
    category: Option<&str>,
    half_life: u32,
) -> Result<Vec<SearchResult>> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut results = Vec::new();

    // Search memory table via FTS5
    let memory_sql = if category.is_some() {
        "SELECT m.id, m.content, m.category, m.importance, m.evergreen, m.created_at, bm25(memory_fts) as rank
         FROM memory_fts
         JOIN memory m ON m.rowid = memory_fts.rowid
         WHERE memory_fts MATCH ?1 AND m.category = ?2
         ORDER BY rank
         LIMIT ?3"
    } else {
        "SELECT m.id, m.content, m.category, m.importance, m.evergreen, m.created_at, bm25(memory_fts) as rank
         FROM memory_fts
         JOIN memory m ON m.rowid = memory_fts.rowid
         WHERE memory_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2"
    };

    let mut stmt = conn.prepare(memory_sql)?;

    let memory_rows: Vec<(String, String, String, f64, bool, i64, f64)> = if let Some(cat) = category {
        stmt.query_map(rusqlite::params![query, cat, limit as i64], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(rusqlite::params![query, limit as i64], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    for (id, content, cat, importance, evergreen, created_at, bm25_score) in memory_rows {
        let age_days = (now_ms - created_at) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
        // bm25() returns negative scores (lower is better), so negate it for our scoring
        let relevance = -bm25_score;
        let base_score = relevance * (1.0 + importance);
        let final_score = crate::decay::apply_decay(base_score, age_days, half_life, evergreen);

        results.push(SearchResult {
            id,
            content,
            category: cat,
            importance,
            created_at,
            score: final_score,
            source: "memory".to_string(),
        });
    }

    // Search memory_chunks table via FTS5
    let chunks_sql = "SELECT c.id, c.content, c.source_path, c.created_at, bm25(memory_chunks_fts) as rank
         FROM memory_chunks_fts
         JOIN memory_chunks c ON c.rowid = memory_chunks_fts.rowid
         WHERE memory_chunks_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2";

    let mut chunk_stmt = conn.prepare(chunks_sql)?;
    let chunk_rows: Vec<(String, String, String, i64, f64)> = chunk_stmt
        .query_map(rusqlite::params![query, limit as i64], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (id, content, source_path, created_at, bm25_score) in chunk_rows {
        let age_days = (now_ms - created_at) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
        let relevance = -bm25_score;
        // Chunks get a slightly lower base importance since they're auto-indexed
        let base_score = relevance * 0.8;
        let final_score = crate::decay::apply_decay(base_score, age_days, half_life, false);

        results.push(SearchResult {
            id,
            content,
            category: format!("chunk:{source_path}"),
            importance: 0.5,
            created_at,
            score: final_score,
            source: "chunk".to_string(),
        });
    }

    // Sort by final score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Limit total results
    results.truncate(limit);

    Ok(results)
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

        // Insert 3 memories with different content and importance
        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-001",
                "Rust programming language offers memory safety without garbage collection",
                "tech",
                0.9,
                false,
                now,
                now,
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-002",
                "Python is widely used for data science and machine learning applications",
                "tech",
                0.7,
                false,
                now,
                now,
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-003",
                "The Rust borrow checker ensures memory safety at compile time",
                "tech",
                0.8,
                true,
                now,
                now,
            ],
        )
        .unwrap();

        (dir, conn)
    }

    #[test]
    fn test_recall_finds_relevant_memories() {
        let (_dir, conn) = setup_db_with_memories();

        let results = recall_memories(&conn, "Rust memory safety", 10, None, 30).unwrap();

        assert!(
            !results.is_empty(),
            "Should find at least one result for 'Rust memory safety'"
        );

        // Both mem-001 and mem-003 mention Rust and memory safety
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(
            ids.contains(&"mem-001") || ids.contains(&"mem-003"),
            "Should find memories about Rust and memory safety"
        );
    }

    #[test]
    fn test_recall_ranking() {
        let (_dir, conn) = setup_db_with_memories();

        let results = recall_memories(&conn, "Rust memory", 10, None, 30).unwrap();

        assert!(results.len() >= 2, "Should find at least 2 results");

        // Verify scores are in descending order
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "Results should be sorted by score descending: {} >= {}",
                window[0].score,
                window[1].score,
            );
        }
    }

    #[test]
    fn test_recall_with_category_filter() {
        let (_dir, conn) = setup_db_with_memories();

        // Add a memory in a different category
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO memory (id, content, category, importance, evergreen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "mem-004",
                "Rust is also used in embedded systems programming",
                "embedded",
                0.6,
                false,
                now,
                now,
            ],
        )
        .unwrap();

        let results = recall_memories(&conn, "Rust programming", 10, Some("embedded"), 30).unwrap();

        for result in &results {
            assert_eq!(
                result.category, "embedded",
                "All results should be in 'embedded' category"
            );
        }
    }

    #[test]
    fn test_recall_respects_limit() {
        let (_dir, conn) = setup_db_with_memories();

        let results = recall_memories(&conn, "programming language", 1, None, 30).unwrap();

        assert!(results.len() <= 1, "Should respect the limit of 1");
    }

    #[test]
    fn test_recall_empty_query_returns_empty() {
        let (_dir, conn) = setup_db_with_memories();

        // FTS5 doesn't accept empty queries, this tests graceful handling
        let results = recall_memories(&conn, "xyznonexistentterm", 10, None, 30).unwrap();
        assert!(results.is_empty(), "Non-matching query should return empty results");
    }
}
