use crate::acp::AcpClient;
use anyhow::Result;
use spawnbot_common::paths::WorkspacePaths;

/// Manages the lifecycle of a goosed session — creation, resumption, rotation.
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

    /// Start the session manager. Tries to resume an existing session, otherwise creates new.
    pub async fn start(&mut self) -> Result<()> {
        let session_file = self.workspace.daemon_session_id();
        if session_file.exists() {
            let id = std::fs::read_to_string(&session_file)?;
            let id = id.trim().to_string();
            if !id.is_empty() {
                self.session_id = Some(id);
                tracing::info!(session = ?self.session_id, "Resumed existing session");
                return Ok(());
            }
        }
        self.create_session().await
    }

    async fn create_session(&mut self) -> Result<()> {
        let cwd = self.workspace.root().to_string_lossy().to_string();
        let id = self.acp.new_session(&cwd).await?;
        self.save_session_id(&id)?;
        self.session_id = Some(id);
        tracing::info!(session = ?self.session_id, "Created new session");
        Ok(())
    }

    fn save_session_id(&self, id: &str) -> Result<()> {
        let path = self.workspace.daemon_session_id();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, id)?;
        Ok(())
    }

    /// Send a prompt to the current session and return the response.
    pub async fn prompt(&mut self, text: &str) -> Result<String> {
        let sid = self.session_id.as_deref().unwrap_or("");
        self.acp.prompt(sid, text).await
    }

    /// Rotate the session: summarize the current session, then create a new one.
    pub async fn rotate(&mut self) -> Result<()> {
        if self.session_id.is_some() {
            let _ = self
                .prompt("[SYSTEM:SESSION_SUMMARY] Summarize this session for continuity.")
                .await;
        }
        self.create_session().await
    }

    /// Restart the goosed process and reconnect.
    pub async fn restart_goose(&mut self) -> Result<()> {
        // TODO: respawn goosed process and reconnect
        tracing::warn!("Goose restart requested — not yet implemented");
        Ok(())
    }
}
