//! Document reader/writer with section-level updates for identity markdown files.

use anyhow::{Context, Result, bail};
use std::path::Path;

/// Read an identity document from disk.
pub fn read_document(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read document: {}", path.display()))
}

/// Write an identity document (full replacement).
pub fn write_document(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write document: {}", path.display()))
}

/// Update a specific section in a markdown document by heading.
///
/// Parses the document by `## ` headings, replaces the matching section's body,
/// and preserves everything else.
pub fn update_section(path: &Path, section_heading: &str, new_content: &str) -> Result<()> {
    let content = read_document(path)?;
    let updated = replace_section(&content, section_heading, new_content)?;
    write_document(path, &updated)
}

/// Replace a section's body in a markdown document string.
///
/// Sections are delimited by `## ` headings. The section matching `section_heading`
/// has its body replaced with `new_content`. All other sections are preserved as-is.
fn replace_section(doc: &str, section_heading: &str, new_content: &str) -> Result<String> {
    let lines: Vec<&str> = doc.lines().collect();
    let mut result = String::new();
    let mut found = false;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("## ") {
            let heading = line.trim_start_matches("## ").trim();

            if heading == section_heading {
                found = true;
                // Write the heading line
                result.push_str(line);
                result.push('\n');
                // Write the new content
                result.push_str(new_content);
                // Ensure new content ends with newline
                if !new_content.ends_with('\n') {
                    result.push('\n');
                }
                // Skip the old section body until next heading or end
                i += 1;
                while i < lines.len() && !lines[i].starts_with("## ") {
                    i += 1;
                }
                continue;
            } else {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }

        i += 1;
    }

    if !found {
        bail!("Section '{}' not found in document", section_heading);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc() -> String {
        "# Title\n\n## Section A\nContent A line 1\nContent A line 2\n\n## Section B\nContent B line 1\n\n## Section C\nContent C line 1\nContent C line 2\n".to_string()
    }

    #[test]
    fn test_replace_section_b() {
        let doc = sample_doc();
        let updated = replace_section(&doc, "Section B", "New B content\n").unwrap();

        // Section A unchanged
        assert!(updated.contains("Content A line 1"));
        assert!(updated.contains("Content A line 2"));

        // Section B replaced
        assert!(updated.contains("## Section B\n"));
        assert!(updated.contains("New B content"));
        assert!(!updated.contains("Content B line 1"));

        // Section C unchanged
        assert!(updated.contains("Content C line 1"));
        assert!(updated.contains("Content C line 2"));
    }

    #[test]
    fn test_replace_section_a() {
        let doc = sample_doc();
        let updated = replace_section(&doc, "Section A", "Replaced A\n").unwrap();

        assert!(updated.contains("## Section A\n"));
        assert!(updated.contains("Replaced A"));
        assert!(!updated.contains("Content A line 1"));
        // Others preserved
        assert!(updated.contains("Content B line 1"));
        assert!(updated.contains("Content C line 1"));
    }

    #[test]
    fn test_replace_section_c() {
        let doc = sample_doc();
        let updated = replace_section(&doc, "Section C", "Replaced C\n").unwrap();

        assert!(updated.contains("Replaced C"));
        assert!(!updated.contains("Content C line 1"));
        // Others preserved
        assert!(updated.contains("Content A line 1"));
        assert!(updated.contains("Content B line 1"));
    }

    #[test]
    fn test_section_not_found() {
        let doc = sample_doc();
        let result = replace_section(&doc, "Nonexistent", "body");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Section 'Nonexistent' not found"));
    }

    #[test]
    fn test_write_and_read_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("TEST.md");
        let content = "# Test\n\nHello world\n";

        write_document(&path, content).unwrap();
        let read_back = read_document(&path).unwrap();
        assert_eq!(read_back, content);
    }

    #[test]
    fn test_update_section_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("DOC.md");
        let content = "## Section A\nOld A\n\n## Section B\nOld B\n";
        write_document(&path, content).unwrap();

        update_section(&path, "Section B", "New B\n").unwrap();

        let result = read_document(&path).unwrap();
        assert!(result.contains("Old A"));
        assert!(!result.contains("Old B"));
        assert!(result.contains("New B"));
    }

    #[test]
    fn test_read_nonexistent_document() {
        let result = read_document(Path::new("/nonexistent/path/DOC.md"));
        assert!(result.is_err());
    }
}
