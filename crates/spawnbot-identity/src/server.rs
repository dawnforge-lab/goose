//! MCP server for managing Spawnbot identity documents.

use rmcp::{
    ErrorData as McpError,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use spawnbot_common::changes_log::ChangesLog;
use spawnbot_common::paths::WorkspacePaths;
use std::path::PathBuf;

#[derive(Deserialize, JsonSchema)]
pub struct IdentityReadParams {
    /// Document name: SOUL, USER, GOALS, PLAYBOOK, or HEARTBEAT
    document: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct IdentityUpdateParams {
    /// Document name: SOUL, USER, GOALS, PLAYBOOK, or HEARTBEAT
    document: String,
    /// The full new content for the document
    content: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct IdentitySectionUpdateParams {
    /// Document name: SOUL, USER, GOALS, PLAYBOOK, or HEARTBEAT
    document: String,
    /// The section heading to update (text after `## `)
    section: String,
    /// The new content for the section body
    content: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct HeartbeatCheckParams {}

#[derive(Deserialize, JsonSchema)]
pub struct HeartbeatUpdateParams {
    /// The task text to find (exact match of the text after the checkbox)
    task_text: String,
    /// New status: pending, ongoing, or completed
    status: String,
}

/// MCP server that manages Spawnbot identity documents
/// (SOUL.md, USER.md, GOALS.md, PLAYBOOK.md, HEARTBEAT.md).
#[derive(Clone)]
pub struct IdentityServer {
    workspace: WorkspacePaths,
    changes_log_path: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl IdentityServer {
    pub fn new(workspace_root: PathBuf) -> Self {
        let workspace = WorkspacePaths::new(workspace_root);
        let changes_log_path = spawnbot_common::paths::changes_log_path();
        Self {
            workspace,
            changes_log_path,
            tool_router: Self::tool_router(),
        }
    }

    fn doc_path(&self, name: &str) -> Result<PathBuf, McpError> {
        match name.to_uppercase().as_str() {
            "SOUL" => Ok(self.workspace.soul_md()),
            "USER" => Ok(self.workspace.user_md()),
            "GOALS" => Ok(self.workspace.goals_md()),
            "PLAYBOOK" => Ok(self.workspace.playbook_md()),
            "HEARTBEAT" => Ok(self.workspace.heartbeat_md()),
            _ => Err(McpError::invalid_params(
                format!("Unknown document: {}", name),
                None,
            )),
        }
    }

    #[tool(description = "Read an identity document (SOUL, USER, GOALS, PLAYBOOK, or HEARTBEAT)")]
    async fn identity_read(
        &self,
        Parameters(params): Parameters<IdentityReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.doc_path(&params.document)?;
        let content = crate::documents::read_document(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Replace the full content of an identity document")]
    async fn identity_update(
        &self,
        Parameters(params): Parameters<IdentityUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.doc_path(&params.document)?;
        crate::documents::write_document(&path, &params.content)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(
            &self.changes_log_path,
            &format!("{}.md", params.document),
            "full",
            "Full document update",
        );
        Ok(CallToolResult::success(vec![Content::text(
            "Document updated",
        )]))
    }

    #[tool(description = "Update a specific section of an identity document by heading")]
    async fn identity_section_update(
        &self,
        Parameters(params): Parameters<IdentitySectionUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.doc_path(&params.document)?;
        crate::documents::update_section(&path, &params.section, &params.content)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(
            &self.changes_log_path,
            &format!("{}.md", params.document),
            &format!("section:{}", params.section),
            "Section update",
        );
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Section '{}' updated",
            params.section
        ))]))
    }

    #[tool(description = "Check HEARTBEAT.md for actionable tasks (pending or ongoing)")]
    async fn heartbeat_check(
        &self,
        Parameters(_params): Parameters<HeartbeatCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.workspace.heartbeat_md();
        let content = crate::documents::read_document(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let tasks = crate::heartbeat::parse_heartbeat(&content);
        let has_actionable = crate::heartbeat::has_actionable_tasks(&content);
        let summary = serde_json::json!({
            "has_actionable": has_actionable,
            "tasks": tasks.iter().map(|t| serde_json::json!({
                "text": t.text,
                "status": format!("{:?}", t.status),
            })).collect::<Vec<_>>(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            summary.to_string(),
        )]))
    }

    #[tool(description = "Update a task's status in HEARTBEAT.md")]
    async fn heartbeat_update(
        &self,
        Parameters(params): Parameters<HeartbeatUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.workspace.heartbeat_md();
        let content = crate::documents::read_document(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let new_status = match params.status.as_str() {
            "pending" => crate::heartbeat::TaskStatus::Pending,
            "ongoing" => crate::heartbeat::TaskStatus::Ongoing,
            "completed" => crate::heartbeat::TaskStatus::Completed,
            _ => {
                return Err(McpError::invalid_params(
                    "Status must be: pending, ongoing, or completed",
                    None,
                ))
            }
        };
        let updated =
            crate::heartbeat::update_task_status(&content, &params.task_text, new_status);
        crate::documents::write_document(&path, &updated)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(
            &self.changes_log_path,
            "HEARTBEAT.md",
            &format!("task:{}", params.task_text),
            &format!("Status -> {}", params.status),
        );
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Task '{}' -> {}",
            params.task_text, params.status
        ))]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for IdentityServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Spawnbot Identity Server - manages identity documents")
    }
}
