use std::path::PathBuf;

/// Returns ~/.spawnbot/
pub fn spawnbot_home() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".spawnbot")
}

/// Returns path to config.yaml
pub fn config_path() -> PathBuf {
    spawnbot_home().join("config.yaml")
}

/// Returns path to changes.log
pub fn changes_log_path() -> PathBuf {
    spawnbot_home().join("changes.log")
}

/// Returns path to skills directory
pub fn skills_dir() -> PathBuf {
    spawnbot_home().join("skills")
}

/// Returns path to extensions directory
pub fn extensions_dir() -> PathBuf {
    spawnbot_home().join("extensions")
}

/// Workspace-relative paths (require workspace root)
#[derive(Clone)]
pub struct WorkspacePaths {
    root: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }
    pub fn soul_md(&self) -> PathBuf {
        self.root.join("SOUL.md")
    }
    pub fn user_md(&self) -> PathBuf {
        self.root.join("USER.md")
    }
    pub fn goals_md(&self) -> PathBuf {
        self.root.join("GOALS.md")
    }
    pub fn playbook_md(&self) -> PathBuf {
        self.root.join("PLAYBOOK.md")
    }
    pub fn heartbeat_md(&self) -> PathBuf {
        self.root.join("HEARTBEAT.md")
    }
    pub fn crons_yaml(&self) -> PathBuf {
        self.root.join("CRONS.yaml")
    }
    pub fn pollers_yaml(&self) -> PathBuf {
        self.root.join("POLLERS.yaml")
    }
    pub fn memory_db(&self) -> PathBuf {
        self.root.join("memory.db")
    }
    pub fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }
    pub fn memory_daily(&self) -> PathBuf {
        self.root.join("memory").join("daily")
    }
    pub fn memory_entities(&self) -> PathBuf {
        self.root.join("memory").join("entities")
    }
    pub fn memory_knowledge(&self) -> PathBuf {
        self.root.join("memory").join("knowledge")
    }
    pub fn poller_state_dir(&self) -> PathBuf {
        self.root.join("poller-state")
    }
    pub fn inbox_dir(&self) -> PathBuf {
        self.root.join("inbox")
    }
    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }
    pub fn daemon_session_id(&self) -> PathBuf {
        self.root.join("sessions").join("daemon-session-id")
    }
    pub fn error_log(&self) -> PathBuf {
        self.root.join("sessions").join("error.log")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawnbot_home() {
        let home = spawnbot_home();
        assert!(home.to_str().unwrap().contains(".spawnbot"));
    }

    #[test]
    fn test_workspace_paths() {
        let ws = WorkspacePaths::new(PathBuf::from("/tmp/ws"));
        assert_eq!(ws.soul_md(), PathBuf::from("/tmp/ws/SOUL.md"));
        assert_eq!(ws.memory_db(), PathBuf::from("/tmp/ws/memory.db"));
        assert_eq!(ws.memory_daily(), PathBuf::from("/tmp/ws/memory/daily"));
    }
}
