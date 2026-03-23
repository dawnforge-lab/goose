use anyhow::Result;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct ErrorLog;

impl ErrorLog {
    /// Log an error from an unattended operation
    pub fn log(log_path: &Path, source: &str, error: &str) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let entry = format!("{} | {} | {}\n", timestamp, source, error);

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

    /// Read recent error entries (last N lines)
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
    fn test_log_and_read_back() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("sessions").join("error.log");

        ErrorLog::log(&log_path, "cron::daily_reflection", "ACP timeout after 30s").unwrap();
        ErrorLog::log(
            &log_path,
            "poller::rss",
            "Feed fetch failed: connection refused",
        )
        .unwrap();

        let recent = ErrorLog::recent(&log_path, 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert!(recent[0].contains("poller::rss"));
        assert!(recent[1].contains("cron::daily_reflection"));
    }

    #[test]
    fn test_read_nonexistent_file() {
        let entries = ErrorLog::recent(Path::new("/nonexistent/error.log"), 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_log_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("deeply").join("nested").join("error.log");

        ErrorLog::log(&log_path, "test", "should create dirs").unwrap();

        let recent = ErrorLog::recent(&log_path, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert!(recent[0].contains("should create dirs"));
    }

    #[test]
    fn test_recent_limits_count() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("error.log");

        for i in 0..10 {
            ErrorLog::log(&log_path, "test", &format!("error {}", i)).unwrap();
        }

        let recent = ErrorLog::recent(&log_path, 3).unwrap();
        assert_eq!(recent.len(), 3);
        // Most recent first
        assert!(recent[0].contains("error 9"));
        assert!(recent[1].contains("error 8"));
        assert!(recent[2].contains("error 7"));
    }
}
