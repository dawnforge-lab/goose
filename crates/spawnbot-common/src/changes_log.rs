use anyhow::Result;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct ChangesLog;

impl ChangesLog {
    /// Append an entry to changes.log
    /// Format: 2026-03-23T14:30:00Z | SOUL.md | section:Communication | "description"
    pub fn append(log_path: &Path, document: &str, scope: &str, description: &str) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let entry = format!("{} | {} | {} | \"{}\"\n", timestamp, document, scope, description);

        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        file.write_all(entry.as_bytes())?;
        Ok(())
    }

    /// Read recent entries (last N lines)
    pub fn recent(log_path: &Path, count: usize) -> Result<Vec<String>> {
        if !log_path.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(log_path)?;
        let lines: Vec<String> = content
            .lines()
            .rev()
            .take(count)
            .map(String::from)
            .collect();
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_append_and_read() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("changes.log");

        ChangesLog::append(&log, "SOUL.md", "section:Communication", "Added concise style")
            .unwrap();
        ChangesLog::append(&log, "GOALS.md", "full", "Updated Q2 objectives").unwrap();

        let recent = ChangesLog::recent(&log, 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert!(recent[0].contains("GOALS.md"));
        assert!(recent[1].contains("SOUL.md"));
    }

    #[test]
    fn test_read_nonexistent() {
        let entries = ChangesLog::recent(Path::new("/nonexistent/changes.log"), 10).unwrap();
        assert!(entries.is_empty());
    }
}
