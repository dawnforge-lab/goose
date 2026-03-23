use anyhow::Result;
use spawnbot_common::config::SpawnbotConfig;
use spawnbot_common::paths::{self, WorkspacePaths};
use std::path::Path;

/// Scaffold the full Spawnbot workspace from a config.
///
/// Creates:
/// - `~/.spawnbot/` with config.yaml, skills/, extensions/
/// - workspace/ with identity docs, CRONS.yaml, POLLERS.yaml
/// - workspace/memory/ with daily/, entities/, knowledge/ subdirs
/// - workspace/poller-state/, inbox/, sessions/
/// - Initializes memory.db
pub fn scaffold_workspace(config: &SpawnbotConfig) -> Result<()> {
    let home = paths::spawnbot_home();
    let ws = WorkspacePaths::new(config.workspace.clone());

    // Create ~/.spawnbot/ structure
    create_dirs(&[
        &home,
        &paths::skills_dir(),
        &paths::extensions_dir(),
    ])?;

    // Save config
    config.save(&paths::config_path())?;

    // Create workspace directories
    create_dirs(&[
        ws.root(),
        &ws.memory_dir(),
        &ws.memory_daily(),
        &ws.memory_entities(),
        &ws.memory_knowledge(),
        &ws.poller_state_dir(),
        &ws.inbox_dir(),
        &ws.sessions_dir(),
    ])?;

    // Write identity documents (only if they don't exist — don't overwrite)
    write_if_absent(
        &ws.soul_md(),
        &crate::templates::soul_md("Spawnbot", "User"),
    )?;
    write_if_absent(
        &ws.user_md(),
        &crate::templates::user_md("User", ""),
    )?;
    write_if_absent(&ws.goals_md(), &crate::templates::goals_md())?;
    write_if_absent(
        &ws.playbook_md(),
        &crate::templates::playbook_md(&format!("{:?}", config.autonomy.mode).to_lowercase()),
    )?;
    write_if_absent(&ws.heartbeat_md(), &crate::templates::heartbeat_md())?;
    write_if_absent(&ws.crons_yaml(), &crate::templates::crons_yaml())?;
    write_if_absent(&ws.pollers_yaml(), &crate::templates::pollers_yaml())?;

    // Initialize memory database
    spawnbot_memory::db::init_db(&ws.memory_db())?;

    Ok(())
}

/// Scaffold with custom bot and user names (used by onboarding wizard).
pub fn scaffold_workspace_with_names(
    config: &SpawnbotConfig,
    bot_name: &str,
    user_name: &str,
    user_role: &str,
) -> Result<()> {
    let home = paths::spawnbot_home();
    let ws = WorkspacePaths::new(config.workspace.clone());

    create_dirs(&[
        &home,
        &paths::skills_dir(),
        &paths::extensions_dir(),
    ])?;

    config.save(&paths::config_path())?;

    create_dirs(&[
        ws.root(),
        &ws.memory_dir(),
        &ws.memory_daily(),
        &ws.memory_entities(),
        &ws.memory_knowledge(),
        &ws.poller_state_dir(),
        &ws.inbox_dir(),
        &ws.sessions_dir(),
    ])?;

    write_if_absent(&ws.soul_md(), &crate::templates::soul_md(bot_name, user_name))?;
    write_if_absent(&ws.user_md(), &crate::templates::user_md(user_name, user_role))?;
    write_if_absent(&ws.goals_md(), &crate::templates::goals_md())?;
    write_if_absent(
        &ws.playbook_md(),
        &crate::templates::playbook_md(&format!("{:?}", config.autonomy.mode).to_lowercase()),
    )?;
    write_if_absent(&ws.heartbeat_md(), &crate::templates::heartbeat_md())?;
    write_if_absent(&ws.crons_yaml(), &crate::templates::crons_yaml())?;
    write_if_absent(&ws.pollers_yaml(), &crate::templates::pollers_yaml())?;

    spawnbot_memory::db::init_db(&ws.memory_db())?;

    Ok(())
}

fn create_dirs(dirs: &[&Path]) -> Result<()> {
    for dir in dirs {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn write_if_absent(path: &Path, content: &str) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spawnbot_common::config::*;
    use spawnbot_common::types::*;
    use std::path::PathBuf;

    fn test_config(workspace: PathBuf) -> SpawnbotConfig {
        SpawnbotConfig {
            version: 1,
            workspace,
            llm: LlmConfig {
                provider: LlmProvider::Anthropic,
                model: "claude-sonnet-4".into(),
                api_key_env: "ANTHROPIC_API_KEY".into(),
            },
            embeddings: EmbeddingsConfig::default(),
            whisper: WhisperConfig::default(),
            telegram: TelegramConfig::default(),
            autonomy: AutonomyConfig::default(),
            skills: vec![],
        }
    }

    #[test]
    fn test_scaffold_creates_directories() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let _config = test_config(workspace.clone());

        // We can't easily test scaffold_workspace because it writes to ~/.spawnbot/
        // Instead, test the internal helpers
        let dirs = [
            workspace.join("memory"),
            workspace.join("memory").join("daily"),
            workspace.join("sessions"),
        ];
        let dir_refs: Vec<&Path> = dirs.iter().map(|p| p.as_path()).collect();
        create_dirs(&dir_refs).unwrap();

        for d in &dirs {
            assert!(d.exists(), "{} should exist", d.display());
        }
    }

    #[test]
    fn test_write_if_absent_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");

        write_if_absent(&path, "Hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Hello");
    }

    #[test]
    fn test_write_if_absent_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");

        std::fs::write(&path, "Original").unwrap();
        write_if_absent(&path, "New content").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Original");
    }

    #[test]
    fn test_scaffold_workspace_with_names() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let _config = test_config(workspace.clone());

        // Temporarily override home dir paths — we test the workspace part only
        // by directly calling the components
        let ws = WorkspacePaths::new(workspace.clone());
        create_dirs(&[
            ws.root(),
            &ws.memory_dir(),
            &ws.memory_daily(),
            &ws.memory_entities(),
            &ws.memory_knowledge(),
            &ws.poller_state_dir(),
            &ws.inbox_dir(),
            &ws.sessions_dir(),
        ])
        .unwrap();

        write_if_absent(&ws.soul_md(), &crate::templates::soul_md("TestBot", "Alice")).unwrap();
        write_if_absent(&ws.user_md(), &crate::templates::user_md("Alice", "Dev")).unwrap();
        write_if_absent(&ws.goals_md(), &crate::templates::goals_md()).unwrap();
        write_if_absent(&ws.heartbeat_md(), &crate::templates::heartbeat_md()).unwrap();

        // Initialize DB
        spawnbot_memory::db::init_db(&ws.memory_db()).unwrap();

        // Verify files exist
        assert!(ws.soul_md().exists());
        assert!(ws.user_md().exists());
        assert!(ws.goals_md().exists());
        assert!(ws.heartbeat_md().exists());
        assert!(ws.memory_db().exists());
        assert!(ws.memory_daily().exists());
        assert!(ws.sessions_dir().exists());

        // Verify content
        let soul = std::fs::read_to_string(ws.soul_md()).unwrap();
        assert!(soul.contains("TestBot"));
        assert!(soul.contains("Alice"));
    }
}
