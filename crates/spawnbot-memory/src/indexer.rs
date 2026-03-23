use anyhow::{Context, Result};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::Path;
use walkdir::WalkDir;

/// Maximum chunk size in characters. Chunks exceeding this will be split.
const MAX_CHUNK_SIZE: usize = 2000;

/// Target chunk size. Chunks larger than this (but under MAX_CHUNK_SIZE) will be
/// split at paragraph boundaries.
const TARGET_CHUNK_SIZE: usize = 1600;

/// Re-index markdown files from the given directory into the `memory_chunks` table.
///
/// - Walks the directory recursively for `.md` files
/// - Computes SHA-256 hash of each file to skip unchanged files
/// - Splits files by `## ` headings, then by `\n\n` if chunks exceed TARGET_CHUNK_SIZE
/// - Hard-limits chunks to MAX_CHUNK_SIZE characters
/// - Returns the number of chunks processed (inserted or updated)
pub fn reindex(conn: &Connection, memory_dir: &Path) -> Result<usize> {
    let mut total_chunks = 0;

    for entry in WalkDir::new(memory_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map_or(false, |ext| ext == "md")
        })
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let file_hash = hex_sha256(&content);
        let source_path = path
            .strip_prefix(memory_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Check if any chunks from this file have the same hash (file unchanged)
        let existing_hash: Option<String> = conn
            .query_row(
                "SELECT content_hash FROM memory_chunks WHERE source_path = ?1 LIMIT 1",
                [&source_path],
                |row| row.get(0),
            )
            .ok();

        if existing_hash.as_deref() == Some(&file_hash) {
            // File unchanged, count existing chunks
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM memory_chunks WHERE source_path = ?1",
                [&source_path],
                |row| row.get(0),
            )?;
            total_chunks += count as usize;
            continue;
        }

        // Remove old chunks for this file
        conn.execute(
            "DELETE FROM memory_chunks WHERE source_path = ?1",
            [&source_path],
        )?;

        // Split into chunks
        let chunks = split_into_chunks(&content);

        let now = chrono::Utc::now().timestamp_millis();

        for chunk in &chunks {
            let chunk_id = ulid::Ulid::new().to_string();
            conn.execute(
                "INSERT INTO memory_chunks (id, source_path, content, content_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![chunk_id, source_path, chunk, file_hash, now],
            )?;
        }

        total_chunks += chunks.len();
    }

    Ok(total_chunks)
}

/// Split markdown content into chunks:
/// 1. First split by `## ` headings
/// 2. If a section exceeds TARGET_CHUNK_SIZE, split by `\n\n` (paragraphs)
/// 3. Hard limit each chunk to MAX_CHUNK_SIZE characters
fn split_into_chunks(content: &str) -> Vec<String> {
    let mut chunks = Vec::new();

    // Split by ## headings
    let sections = split_by_headings(content);

    for section in sections {
        if section.trim().is_empty() {
            continue;
        }

        if section.len() <= TARGET_CHUNK_SIZE {
            chunks.push(section);
        } else {
            // Split by paragraphs
            let paragraphs = section.split("\n\n");
            let mut current_chunk = String::new();

            for paragraph in paragraphs {
                if current_chunk.len() + paragraph.len() + 2 > TARGET_CHUNK_SIZE
                    && !current_chunk.is_empty()
                {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                }

                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(paragraph);
            }

            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
            }
        }
    }

    // Hard limit: split any chunk that exceeds MAX_CHUNK_SIZE
    let mut final_chunks = Vec::new();
    for chunk in chunks {
        if chunk.len() <= MAX_CHUNK_SIZE {
            final_chunks.push(chunk);
        } else {
            // Split at character boundary, trying to find a newline near the limit
            let mut remaining = chunk.as_str();
            while !remaining.is_empty() {
                if remaining.len() <= MAX_CHUNK_SIZE {
                    final_chunks.push(remaining.to_string());
                    break;
                }
                // Find a good split point near MAX_CHUNK_SIZE
                let split_at = find_split_point(remaining, MAX_CHUNK_SIZE);
                final_chunks.push(remaining[..split_at].to_string());
                remaining = &remaining[split_at..];
            }
        }
    }

    final_chunks
}

/// Split content by `## ` headings, keeping the heading with its section.
fn split_by_headings(content: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        if line.starts_with("## ") && !current.is_empty() {
            sections.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }

    if !current.is_empty() {
        sections.push(current);
    }

    sections
}

/// Find a good split point near the target position, preferring newlines.
fn find_split_point(text: &str, target: usize) -> usize {
    // Look for a newline within the last 200 chars before target
    let search_start = target.saturating_sub(200);
    if let Some(pos) = text[search_start..target].rfind('\n') {
        return search_start + pos + 1;
    }
    // Look for a space
    if let Some(pos) = text[search_start..target].rfind(' ') {
        return search_start + pos + 1;
    }
    // Hard split at target
    target
}

/// Compute hex-encoded SHA-256 hash of content.
fn hex_sha256(content: &str) -> String {
    let hash = Sha256::digest(content.as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_reindex_markdown_files() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let memory_dir = dir.path().join("memories");
        fs::create_dir_all(&memory_dir).unwrap();

        // Create a markdown file
        fs::write(
            memory_dir.join("test.md"),
            "# Title\n\nSome intro content.\n\n## Section One\n\nContent of section one.\n\n## Section Two\n\nContent of section two.\n",
        )
        .unwrap();

        let count = reindex(&conn, &memory_dir).unwrap();
        assert!(count > 0, "Should index at least one chunk");

        // Verify chunks are in the database
        let db_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_chunks", [], |row| row.get(0))
            .unwrap();
        assert!(db_count > 0, "Chunks should be in database");
    }

    #[test]
    fn test_reindex_skips_unchanged() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let memory_dir = dir.path().join("memories");
        fs::create_dir_all(&memory_dir).unwrap();

        fs::write(memory_dir.join("test.md"), "## Section\n\nContent here.\n").unwrap();

        let count1 = reindex(&conn, &memory_dir).unwrap();
        let count2 = reindex(&conn, &memory_dir).unwrap();

        assert_eq!(count1, count2, "Second reindex should find same chunk count");
    }

    #[test]
    fn test_reindex_updates_changed_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let memory_dir = dir.path().join("memories");
        fs::create_dir_all(&memory_dir).unwrap();

        fs::write(memory_dir.join("test.md"), "## Section\n\nOriginal content.\n").unwrap();
        reindex(&conn, &memory_dir).unwrap();

        // Get original content
        let original: String = conn
            .query_row(
                "SELECT content FROM memory_chunks WHERE source_path = 'test.md' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(original.contains("Original"));

        // Update the file
        fs::write(
            memory_dir.join("test.md"),
            "## Section\n\nUpdated content with new information.\n",
        )
        .unwrap();
        reindex(&conn, &memory_dir).unwrap();

        // Verify content was updated
        let updated: String = conn
            .query_row(
                "SELECT content FROM memory_chunks WHERE source_path = 'test.md' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(updated.contains("Updated"), "Content should be updated");
    }

    #[test]
    fn test_split_into_chunks_basic() {
        let content = "## Section One\n\nParagraph one.\n\n## Section Two\n\nParagraph two.\n";
        let chunks = split_into_chunks(content);
        assert!(chunks.len() >= 2, "Should split into at least 2 chunks");
    }

    #[test]
    fn test_split_long_content() {
        // Create content that exceeds TARGET_CHUNK_SIZE
        let paragraph = "This is a long paragraph that contains many words. ".repeat(50);
        let content = format!("## Section\n\n{paragraph}\n\n{paragraph}");
        let chunks = split_into_chunks(&content);

        for chunk in &chunks {
            assert!(
                chunk.len() <= MAX_CHUNK_SIZE,
                "No chunk should exceed MAX_CHUNK_SIZE ({MAX_CHUNK_SIZE}), got {}",
                chunk.len()
            );
        }
    }

    #[test]
    fn test_reindex_fts_searchable() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = init_db(&db_path).unwrap();

        let memory_dir = dir.path().join("memories");
        fs::create_dir_all(&memory_dir).unwrap();

        fs::write(
            memory_dir.join("test.md"),
            "## Quantum Computing\n\nQuantum computing uses qubits for computation.\n",
        )
        .unwrap();

        reindex(&conn, &memory_dir).unwrap();

        // Search via FTS
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_chunks_fts WHERE memory_chunks_fts MATCH 'quantum'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count > 0, "Indexed chunks should be searchable via FTS5");
    }
}
