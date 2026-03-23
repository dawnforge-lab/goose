//! Heartbeat parser with task status tracking for HEARTBEAT.md.

/// Status of a task in the heartbeat.
#[derive(Debug, PartialEq, Clone)]
pub enum TaskStatus {
    /// `- [ ]` — not started
    Pending,
    /// `- [~]` — in progress
    Ongoing,
    /// `- [x]` — done
    Completed,
}

/// A single task parsed from HEARTBEAT.md.
#[derive(Debug, Clone)]
pub struct HeartbeatTask {
    pub text: String,
    pub status: TaskStatus,
}

/// Parse HEARTBEAT.md content into tasks.
///
/// Recognizes three checkbox formats:
/// - `- [ ] text` => Pending
/// - `- [~] text` => Ongoing
/// - `- [x] text` => Completed
pub fn parse_heartbeat(content: &str) -> Vec<HeartbeatTask> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(text) = trimmed.strip_prefix("- [ ] ") {
                Some(HeartbeatTask {
                    text: text.to_string(),
                    status: TaskStatus::Pending,
                })
            } else if let Some(text) = trimmed.strip_prefix("- [~] ") {
                Some(HeartbeatTask {
                    text: text.to_string(),
                    status: TaskStatus::Ongoing,
                })
            } else if let Some(text) = trimmed.strip_prefix("- [x] ") {
                Some(HeartbeatTask {
                    text: text.to_string(),
                    status: TaskStatus::Completed,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Update a task's status in the heartbeat content.
///
/// Finds the line containing `task_text` and replaces its checkbox marker
/// with the one corresponding to `new_status`. Returns the full updated content.
pub fn update_task_status(content: &str, task_text: &str, new_status: TaskStatus) -> String {
    let new_marker = match new_status {
        TaskStatus::Pending => "- [ ] ",
        TaskStatus::Ongoing => "- [~] ",
        TaskStatus::Completed => "- [x] ",
    };

    content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            let is_task = trimmed.starts_with("- [ ] ")
                || trimmed.starts_with("- [~] ")
                || trimmed.starts_with("- [x] ");

            if is_task {
                // Extract the task text from this line
                let line_text = if let Some(t) = trimmed.strip_prefix("- [ ] ") {
                    t
                } else if let Some(t) = trimmed.strip_prefix("- [~] ") {
                    t
                } else if let Some(t) = trimmed.strip_prefix("- [x] ") {
                    t
                } else {
                    return line.to_string();
                };

                if line_text == task_text {
                    // Preserve leading whitespace
                    let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                    format!("{}{}{}", leading_ws, new_marker, task_text)
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Check if there are any actionable tasks (Pending or Ongoing).
pub fn has_actionable_tasks(content: &str) -> bool {
    parse_heartbeat(content)
        .iter()
        .any(|t| t.status != TaskStatus::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_heartbeat() -> String {
        "# Heartbeat\n\n- [ ] Set up project\n- [~] Write tests\n- [x] Read spec\n".to_string()
    }

    #[test]
    fn test_parse_heartbeat_three_types() {
        let tasks = parse_heartbeat(&sample_heartbeat());
        assert_eq!(tasks.len(), 3);

        assert_eq!(tasks[0].text, "Set up project");
        assert_eq!(tasks[0].status, TaskStatus::Pending);

        assert_eq!(tasks[1].text, "Write tests");
        assert_eq!(tasks[1].status, TaskStatus::Ongoing);

        assert_eq!(tasks[2].text, "Read spec");
        assert_eq!(tasks[2].status, TaskStatus::Completed);
    }

    #[test]
    fn test_update_task_status_pending_to_completed() {
        let content = sample_heartbeat();
        let updated = update_task_status(&content, "Set up project", TaskStatus::Completed);
        assert!(updated.contains("- [x] Set up project"));
        // Others unchanged
        assert!(updated.contains("- [~] Write tests"));
        assert!(updated.contains("- [x] Read spec"));
    }

    #[test]
    fn test_update_task_status_ongoing_to_pending() {
        let content = sample_heartbeat();
        let updated = update_task_status(&content, "Write tests", TaskStatus::Pending);
        assert!(updated.contains("- [ ] Write tests"));
    }

    #[test]
    fn test_update_task_status_completed_to_ongoing() {
        let content = sample_heartbeat();
        let updated = update_task_status(&content, "Read spec", TaskStatus::Ongoing);
        assert!(updated.contains("- [~] Read spec"));
    }

    #[test]
    fn test_has_actionable_tasks_true() {
        assert!(has_actionable_tasks(&sample_heartbeat()));
    }

    #[test]
    fn test_has_actionable_tasks_false() {
        let content = "- [x] Done task 1\n- [x] Done task 2\n";
        assert!(!has_actionable_tasks(content));
    }

    #[test]
    fn test_has_actionable_tasks_empty() {
        assert!(!has_actionable_tasks("# No tasks here\n"));
    }

    #[test]
    fn test_parse_empty_content() {
        let tasks = parse_heartbeat("");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_update_nonexistent_task_leaves_unchanged() {
        let content = sample_heartbeat();
        let updated = update_task_status(&content, "Nonexistent task", TaskStatus::Completed);
        // Content should be unchanged (minus trailing newline handling)
        assert!(updated.contains("- [ ] Set up project"));
        assert!(updated.contains("- [~] Write tests"));
        assert!(updated.contains("- [x] Read spec"));
    }
}
