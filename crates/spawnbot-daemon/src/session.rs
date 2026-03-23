//! Session manager — session lifecycle, SOUL.md system prompt, context re-injection.

use anyhow::{Context, Result};
use spawnbot_common::paths::WorkspacePaths;

use crate::acp::AcpClient;

pub struct SessionManager {
    acp: AcpClient,
    session_id: Option<String>,
    workspace: WorkspacePaths,
}

impl SessionManager {
    pub fn new(acp: AcpClient, workspace: WorkspacePaths) -> Self {
        Self {
            acp,
            session_id: None,
            workspace,
        }
    }

    /// Start the session manager: initialize ACP if needed, then resume or create session.
    pub async fn start(&mut self) -> Result<()> {
        // Ensure ACP is initialized before doing anything
        if !self.acp.is_initialized() {
            self.acp
                .initialize()
                .await
                .with_context(|| "Failed to initialize ACP connection during session start")?;
        }

        // Try to resume existing session
        let session_file = self.workspace.daemon_session_id();
        if session_file.exists() {
            let id = std::fs::read_to_string(&session_file)
                .with_context(|| "Failed to read session ID file")?;
            let id = id.trim().to_string();
            if !id.is_empty() {
                tracing::info!(session_id = %id, "Attempting to resume existing session");
                // Try to load the session — if it fails, create a new one
                match self.acp.load_session(&id).await {
                    Ok(_) => {
                        self.session_id = Some(id.clone());
                        tracing::info!(session_id = %id, "Resumed existing session");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!(
                            session_id = %id,
                            error = %e,
                            "Failed to resume session, creating new one"
                        );
                    }
                }
            }
        }

        // No existing session or resume failed — create one with identity context
        self.create_session_with_soul().await
    }

    /// Create a session with SOUL.md as system prompt and inject identity context.
    pub async fn create_session_with_soul(&mut self) -> Result<()> {
        let soul = std::fs::read_to_string(self.workspace.soul_md()).unwrap_or_default();
        let cwd = self.workspace.root().to_string_lossy().to_string();

        // Create a new ACP session
        let id = self
            .acp
            .new_session(&cwd)
            .await
            .with_context(|| "Failed to create new ACP session")?;
        self.save_session_id(&id)?;
        self.session_id = Some(id.clone());

        // Inject identity context as the first prompt
        if !soul.is_empty() {
            let user_md = std::fs::read_to_string(self.workspace.user_md()).unwrap_or_default();
            let goals_md = std::fs::read_to_string(self.workspace.goals_md()).unwrap_or_default();
            let init_context = format!(
                "[SYSTEM:SESSION_RESET] New session initialized.\n\n\
                 ## SOUL\n{}\n\n\
                 ## User\n{}\n\n\
                 ## Goals\n{}",
                soul.trim(),
                user_md.trim(),
                goals_md.trim()
            );
            let _ = self.prompt(&init_context).await;
        }

        tracing::info!(session_id = %id, "Created session with identity context");
        Ok(())
    }

    /// Send a prompt to the current session using streaming to collect the full response.
    pub async fn prompt(&mut self, text: &str) -> Result<String> {
        let session_id = self
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No active session — call start() first"))?;

        self.acp
            .prompt_streaming(session_id, text)
            .await
            .with_context(|| "Failed to send prompt to ACP")
    }

    /// Rotate the session: close the old one and create a new one with identity context.
    pub async fn rotate(&mut self) -> Result<()> {
        tracing::info!("Rotating session");

        // Ask for a summary of the current session state before rotating
        if let Some(session_id) = self.session_id.clone() {
            let _ = self
                .prompt("[SYSTEM] Session rotating. Summarize current state for continuity.")
                .await;

            // Close the old session
            if let Err(e) = self.acp.close_session(&session_id).await {
                tracing::warn!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to close old session during rotation"
                );
            }
        }

        // Create a new session with identity context
        self.create_session_with_soul().await?;

        tracing::info!(session = ?self.session_id, "Session rotated");
        Ok(())
    }

    /// Restart the goosed process (crash recovery).
    pub async fn restart_goose(&mut self) -> Result<()> {
        tracing::warn!("Restarting goosed process");
        self.acp
            .restart()
            .await
            .with_context(|| "Failed to restart goose process")?;
        self.create_session_with_soul().await?;
        tracing::info!("Goosed restarted and session recreated");
        Ok(())
    }

    /// Re-inject critical context after compaction.
    pub async fn reinject_context(&mut self) -> Result<()> {
        let heartbeat = std::fs::read_to_string(self.workspace.heartbeat_md()).unwrap_or_default();
        let goals = std::fs::read_to_string(self.workspace.goals_md()).unwrap_or_default();
        let context = format!(
            "[CONTEXT REFRESH]\n\n## Current Tasks\n{}\n\n## Active Goals\n{}",
            heartbeat.trim(),
            goals.trim()
        );
        self.prompt(&context).await?;
        Ok(())
    }

    /// Get the current session ID.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Check if ACP client is alive.
    pub fn is_alive(&mut self) -> bool {
        self.acp.is_alive()
    }

    /// Persist session ID to disk for resume across restarts.
    fn save_session_id(&self, id: &str) -> Result<()> {
        let session_file = self.workspace.daemon_session_id();
        if let Some(parent) = session_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&session_file, id)
            .with_context(|| "Failed to write session ID file")?;
        Ok(())
    }
}
