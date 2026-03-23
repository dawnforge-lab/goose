//! ACP (Agent Communication Protocol) client for JSON-RPC communication with goosed.
//!
//! Goose uses ACP — JSON-RPC 2.0 over stdio. We spawn `goose acp` as a subprocess
//! and communicate via stdin/stdout with newline-delimited JSON messages.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// JSON-RPC 2.0 request envelope
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC 2.0 response/notification envelope
#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
    id: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    data: Option<Value>,
}

/// ACP client that manages JSON-RPC communication with the goosed child process.
pub struct AcpClient {
    child: Child,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    next_id: AtomicU64,
    initialized: bool,
    goose_cmd: String,
    goose_args: Vec<String>,
}

impl AcpClient {
    /// Spawn `goose acp` and set up stdio communication.
    pub async fn spawn(goose_cmd: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(goose_cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // let stderr pass through for debugging
            .spawn()
            .with_context(|| format!("Failed to spawn: {} {}", goose_cmd, args.join(" ")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdin of goose process"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout of goose process"))?;

        Ok(Self {
            child,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            next_id: AtomicU64::new(1),
            initialized: false,
            goose_cmd: goose_cmd.to_string(),
            goose_args: args.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Send a JSON-RPC request and wait for the matching response.
    ///
    /// Notifications (messages without an `id`) are skipped while waiting.
    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
            id,
        };

        let mut msg = serde_json::to_string(&req)?;
        msg.push('\n');

        // Send request
        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(msg.as_bytes())
                .await
                .with_context(|| format!("Failed to write to goose stdin for method '{}'", method))?;
            stdin.flush().await?;
        }

        // Read responses until we get the matching one
        loop {
            let line = {
                let mut stdout = self.stdout.lock().await;
                let mut line = String::new();
                let bytes_read = stdout
                    .read_line(&mut line)
                    .await
                    .with_context(|| "Failed to read from goose stdout")?;
                if bytes_read == 0 {
                    bail!("goose process closed stdout (EOF) while waiting for response to '{}'", method);
                }
                line
            };

            if line.trim().is_empty() {
                continue;
            }

            let resp: JsonRpcResponse = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse JSON-RPC response: {}", line.trim()))?;

            // Skip notifications (no id field)
            if resp.id.is_none() {
                tracing::trace!(method = ?resp.method, "Skipping notification while waiting for response");
                continue;
            }

            if resp.id == Some(id) {
                if let Some(error) = resp.error {
                    bail!(
                        "ACP error from '{}' (code {}): {}",
                        method,
                        error.code,
                        error.message
                    );
                }
                return Ok(resp.result.unwrap_or(Value::Null));
            }

            // Response for a different request id — log and skip
            tracing::warn!(
                expected_id = id,
                got_id = ?resp.id,
                "Received response for unexpected request id"
            );
        }
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let mut msg_str = serde_json::to_string(&msg)?;
        msg_str.push('\n');

        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(msg_str.as_bytes())
            .await
            .with_context(|| format!("Failed to send notification '{}'", method))?;
        stdin.flush().await?;
        Ok(())
    }

    /// Initialize the ACP connection. Must be called before any other method.
    pub async fn initialize(&mut self) -> Result<Value> {
        let result = self
            .request(
                "initialize",
                serde_json::json!({
                    "client_info": {
                        "name": "spawnbot",
                        "version": "0.1.0"
                    },
                    "client_capabilities": {
                        "fs": false,
                        "terminal": false
                    }
                }),
            )
            .await?;
        self.initialized = true;
        tracing::info!("ACP initialized");
        Ok(result)
    }

    /// Returns whether `initialize()` has been called successfully.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Create a new goose session.
    ///
    /// Returns the session ID from goose. If goose does not return one,
    /// a locally-generated ULID is used instead.
    pub async fn new_session(&self, cwd: &str) -> Result<String> {
        let result = self
            .request(
                "session/new",
                serde_json::json!({
                    "cwd": cwd
                }),
            )
            .await?;

        let session_id = result
            .get("session_id")
            .or_else(|| result.get("sessionId"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| ulid::Ulid::new().to_string());

        tracing::info!(session_id = %session_id, "New ACP session created");
        Ok(session_id)
    }

    /// Load an existing session by ID.
    pub async fn load_session(&self, session_id: &str) -> Result<Value> {
        self.request(
            "session/load",
            serde_json::json!({
                "session_id": session_id
            }),
        )
        .await
    }

    /// Close a session.
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let _ = self
            .request(
                "session/close",
                serde_json::json!({
                    "session_id": session_id
                }),
            )
            .await?;
        tracing::info!(session_id = %session_id, "ACP session closed");
        Ok(())
    }

    /// Send a prompt to the session and wait for the simple (non-streaming) response.
    ///
    /// This skips all streaming notifications and returns whatever text is in
    /// the final `prompt` response. For full streaming collection, use
    /// [`prompt_streaming`].
    pub async fn prompt(&self, session_id: &str, text: &str) -> Result<String> {
        let result = self
            .request(
                "prompt",
                serde_json::json!({
                    "session_id": session_id,
                    "prompt": [{
                        "type": "text",
                        "text": text
                    }]
                }),
            )
            .await?;

        // The prompt response itself may contain text directly
        if let Some(text) = result.get("text").and_then(|v| v.as_str()) {
            return Ok(text.to_string());
        }

        // Return the raw result for debugging
        Ok(serde_json::to_string_pretty(&result)?)
    }

    /// Send a prompt and collect ALL streaming notification content.
    ///
    /// This reads `session/notification` messages for streamed text blocks,
    /// concatenating them until the final `prompt` response arrives.
    pub async fn prompt_streaming(&self, session_id: &str, text: &str) -> Result<String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "prompt".to_string(),
            params: serde_json::json!({
                "session_id": session_id,
                "prompt": [{ "type": "text", "text": text }]
            }),
            id,
        };

        let mut msg = serde_json::to_string(&req)?;
        msg.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(msg.as_bytes())
                .await
                .with_context(|| "Failed to write streaming prompt to goose stdin")?;
            stdin.flush().await?;
        }

        // Collect streamed text from notifications until we get the final response
        let mut collected_text = String::new();
        loop {
            let line = {
                let mut stdout = self.stdout.lock().await;
                let mut line = String::new();
                let bytes_read = stdout
                    .read_line(&mut line)
                    .await
                    .with_context(|| "Failed to read streaming response from goose stdout")?;
                if bytes_read == 0 {
                    bail!("goose process closed stdout (EOF) during streaming prompt");
                }
                line
            };

            if line.trim().is_empty() {
                continue;
            }

            let resp: JsonRpcResponse = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse streaming response: {}", line.trim()))?;

            if resp.id.is_none() {
                // Notification — extract text content blocks
                if let Some(params) = &resp.params {
                    if let Some(content) = params.get("content") {
                        if let Some(arr) = content.as_array() {
                            for block in arr {
                                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                    collected_text.push_str(text);
                                }
                            }
                        }
                    }
                }
                continue;
            }

            if resp.id == Some(id) {
                if let Some(error) = resp.error {
                    bail!(
                        "ACP streaming prompt error (code {}): {}",
                        error.code,
                        error.message
                    );
                }
                break;
            }
        }

        Ok(collected_text)
    }

    /// Cancel a running prompt.
    pub async fn cancel(&self, session_id: &str) -> Result<()> {
        self.notify(
            "cancel",
            serde_json::json!({
                "session_id": session_id
            }),
        )
        .await
    }

    /// Check if the goosed child process is still alive.
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Kill the current goosed process and spawn a fresh one.
    ///
    /// The new process is initialized automatically.
    pub async fn restart(&mut self) -> Result<()> {
        tracing::info!("Restarting goose process");
        let _ = self.child.kill().await;

        let args_refs: Vec<&str> = self.goose_args.iter().map(|s| s.as_str()).collect();
        let mut new_client = Self::spawn(&self.goose_cmd, &args_refs).await?;
        new_client.initialize().await?;

        // Swap the entire struct so the old one (with the killed child) is dropped cleanly
        // and this self now holds the new client's internals.
        std::mem::swap(self, &mut new_client);
        // new_client now holds the old (killed) child — its Drop impl will try to kill it
        // again, which is harmless.

        tracing::info!("Goose process restarted and initialized");
        Ok(())
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        // Best-effort kill of the child process on drop
        let _ = self.child.start_kill();
    }
}
