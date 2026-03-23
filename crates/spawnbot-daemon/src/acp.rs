use anyhow::Result;
use std::process::Stdio;
use tokio::process::{Child, Command};

/// ACP (Agent Communication Protocol) client that manages a goosed subprocess
/// and communicates with it via JSON-RPC over stdin/stdout.
pub struct AcpClient {
    child: Child,
}

impl AcpClient {
    /// Spawn a new goosed process with the given command and arguments.
    pub async fn spawn(goose_cmd: &str, args: &[&str]) -> Result<Self> {
        let child = Command::new(goose_cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self { child })
    }

    /// Create a new session in the goosed process.
    ///
    /// Returns the session ID.
    pub async fn new_session(&mut self, _cwd: &str) -> Result<String> {
        // TODO: JSON-RPC call to create new session
        // Protocol: send {"jsonrpc":"2.0","method":"session.new","params":{"cwd":"..."},"id":1}
        // Response: {"jsonrpc":"2.0","result":{"session_id":"..."},"id":1}
        Ok(ulid::Ulid::new().to_string())
    }

    /// Send a prompt to the goosed process and collect the streaming response.
    ///
    /// Returns the full response text once streaming is complete.
    pub async fn prompt(&mut self, _session_id: &str, _text: &str) -> Result<String> {
        // TODO: JSON-RPC call to send prompt and collect streaming response
        // Protocol: send {"jsonrpc":"2.0","method":"session.prompt","params":{"session_id":"...","text":"..."},"id":2}
        // Response: streaming chunks, then final {"jsonrpc":"2.0","result":{"text":"..."},"id":2}
        Ok(String::new())
    }

    /// Check if the goosed process is still alive.
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }
}
