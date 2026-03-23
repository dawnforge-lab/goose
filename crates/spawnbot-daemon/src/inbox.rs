use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Delete files in inbox/ older than 1 hour. Runs every 10 minutes.
pub async fn start_inbox_cleanup(inbox_dir: PathBuf) {
    let mut interval = tokio::time::interval(Duration::from_secs(600));
    loop {
        interval.tick().await;
        cleanup_old_files(&inbox_dir, Duration::from_secs(3600));
    }
}

fn cleanup_old_files(dir: &std::path::Path, max_age: Duration) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let cutoff = SystemTime::now() - max_age;
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified < cutoff {
                    let _ = std::fs::remove_file(entry.path());
                    tracing::debug!(path = ?entry.path(), "Cleaned up old inbox file");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_cleanup_with_zero_max_age_deletes_all_files() {
        let dir = tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        fs::create_dir_all(&inbox).unwrap();

        // Create files
        let file_a = inbox.join("message_a.txt");
        let file_b = inbox.join("message_b.txt");
        {
            let mut f = fs::File::create(&file_a).unwrap();
            f.write_all(b"data a").unwrap();
        }
        {
            let mut f = fs::File::create(&file_b).unwrap();
            f.write_all(b"data b").unwrap();
        }

        // With max_age=0, every file is "old"
        cleanup_old_files(&inbox, Duration::from_secs(0));

        assert!(!file_a.exists(), "File A should be deleted with zero max_age");
        assert!(!file_b.exists(), "File B should be deleted with zero max_age");
    }

    #[test]
    fn test_cleanup_preserves_recent_files() {
        let dir = tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        fs::create_dir_all(&inbox).unwrap();

        // Create a file just now
        let new_file = inbox.join("new_message.txt");
        {
            let mut f = fs::File::create(&new_file).unwrap();
            f.write_all(b"new data").unwrap();
        }

        // Run cleanup with 1 hour max age — file was just created, should survive
        cleanup_old_files(&inbox, Duration::from_secs(3600));

        assert!(new_file.exists(), "Recent file should not be deleted");
    }

    #[test]
    fn test_cleanup_nonexistent_dir() {
        // Should not panic on nonexistent directory
        cleanup_old_files(
            &std::path::PathBuf::from("/nonexistent/inbox"),
            Duration::from_secs(3600),
        );
    }

    #[test]
    fn test_cleanup_empty_dir() {
        let dir = tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        fs::create_dir_all(&inbox).unwrap();

        // Should not panic on empty directory
        cleanup_old_files(&inbox, Duration::from_secs(3600));
    }
}
