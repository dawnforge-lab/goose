//! ACP (Agent Communication Protocol) client for JSON-RPC communication with goosed.

use anyhow::{Context, Result};
use std::process::Child;

/// ACP client that manages communication with the goosed child process.
pub struct AcpClient {
    child: Option<Child>,
    goose_cmd: String,
    goose_args: Vec<String>,
}

impl AcpClient {
    /// Spawn goosed as a child process
    pub fn spawn(goose_cmd: &str, args: &[&str]) -> Result<Self> {
        let child = std::process::Command::new(goose_cmd)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn goosed: {}", goose_cmd))?;

        Ok(Self {
            child: Some(child),
            goose_cmd: goose_cmd.to_string(),
            goose_args: args.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Create a new session and return its ID
    pub async fn new_session(&mut self, _cwd: &str) -> Result<String> {
        // In production: send JSON-RPC "session/new" with cwd parameter
        // For now, generate a session ID
        let id = ulid::Ulid::new().to_string();
        tracing::info!(session_id = %id, "Created new ACP session");
        Ok(id)
    }

    /// Send a prompt to the current session
    pub async fn prompt(&mut self, text: &str) -> Result<String> {
        tracing::debug!(prompt_len = text.len(), "Sending prompt to ACP");
        // In production: send JSON-RPC "session/prompt" with text
        // Return the streamed response collected as a string
        Ok(format!("[ACP response to: {}]", &text[..text.len().min(80)]))
    }

    /// Check if the goosed child process is still alive
    pub fn is_alive(&mut self) -> bool {
        match &mut self.child {
            Some(child) => match child.try_wait() {
                Ok(None) => true,   // Still running
                Ok(Some(_)) => false, // Exited
                Err(_) => false,
            },
            None => false,
        }
    }

    /// Restart the goosed process
    pub fn restart(&mut self) -> Result<()> {
        // Kill existing child if alive
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }

        let child = std::process::Command::new(&self.goose_cmd)
            .args(&self.goose_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to restart goosed: {}", self.goose_cmd))?;

        self.child = Some(child);
        tracing::info!("Restarted goosed process");
        Ok(())
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
