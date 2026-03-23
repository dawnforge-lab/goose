//! MCP server exposing skill and extension management tools.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use spawnbot_common::changes_log::ChangesLog;
use std::path::PathBuf;

// ── Parameter structs ────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
struct SkillCreateParams {
    name: String,
    #[serde(rename = "type")]
    skill_type: String,
    description: String,
    definition: String,
}

#[derive(Deserialize, JsonSchema)]
struct SkillListParams {}

#[derive(Deserialize, JsonSchema)]
struct SkillReadParams {
    name: String,
}

#[derive(Deserialize, JsonSchema)]
struct SkillEditParams {
    name: String,
    definition: String,
}

#[derive(Deserialize, JsonSchema)]
struct SkillDeleteParams {
    name: String,
}

#[derive(Deserialize, JsonSchema)]
struct ExtensionInstallParams {
    name: String,
    command: String,
    args: Vec<String>,
    description: String,
}

#[derive(Deserialize, JsonSchema)]
struct ExtensionRemoveParams {
    name: String,
}

// ── Server ───────────────────────────────────────────────────────────

#[derive(Clone)]
#[allow(dead_code)]
pub struct SkillsServer {
    skills_dir: PathBuf,
    extensions_dir: PathBuf,
    changes_log_path: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SkillsServer {
    pub fn new(skills_dir: PathBuf, extensions_dir: PathBuf) -> Self {
        let changes_log_path = spawnbot_common::paths::changes_log_path();
        Self {
            skills_dir,
            extensions_dir,
            changes_log_path,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Create a new skill with name, type, description, and definition")]
    async fn skill_create(
        &self,
        params: Parameters<SkillCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let skill = crate::skills::SkillInfo {
            name: p.name.clone(),
            skill_type: p.skill_type,
            description: p.description,
            definition: p.definition,
            enabled: true,
        };
        crate::skills::create_skill(&self.skills_dir, &skill)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(&self.changes_log_path, "skills", &p.name, "Skill created");
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Skill '{}' created",
            p.name
        ))]))
    }

    #[tool(description = "List all available skills")]
    async fn skill_list(
        &self,
        _params: Parameters<SkillListParams>,
    ) -> Result<CallToolResult, McpError> {
        let skills = crate::skills::list_skills(&self.skills_dir)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&skills)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Read a skill's full definition")]
    async fn skill_read(
        &self,
        params: Parameters<SkillReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let skill = crate::skills::read_skill(&self.skills_dir, &params.0.name)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&skill)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Edit a skill's definition")]
    async fn skill_edit(
        &self,
        params: Parameters<SkillEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        crate::skills::edit_skill(&self.skills_dir, &p.name, &p.definition)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(
            &self.changes_log_path,
            "skills",
            &p.name,
            "Skill definition updated",
        );
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Skill '{}' updated",
            p.name
        ))]))
    }

    #[tool(description = "Delete a skill")]
    async fn skill_delete(
        &self,
        params: Parameters<SkillDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = &params.0.name;
        crate::skills::delete_skill(&self.skills_dir, name)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(&self.changes_log_path, "skills", name, "Skill deleted");
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Skill '{}' deleted",
            name
        ))]))
    }

    #[tool(description = "Install an MCP extension")]
    async fn extension_install(
        &self,
        params: Parameters<ExtensionInstallParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let config = crate::extensions::ExtensionConfig {
            name: p.name.clone(),
            command: p.command,
            args: p.args,
            env: Default::default(),
            description: p.description,
        };
        crate::extensions::install_extension(&self.extensions_dir, &config)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(
            &self.changes_log_path,
            "extensions",
            &p.name,
            "Extension installed",
        );
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Extension '{}' installed",
            p.name
        ))]))
    }

    #[tool(description = "Remove an MCP extension")]
    async fn extension_remove(
        &self,
        params: Parameters<ExtensionRemoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = &params.0.name;
        crate::extensions::remove_extension(&self.extensions_dir, name)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = ChangesLog::append(
            &self.changes_log_path,
            "extensions",
            name,
            "Extension removed",
        );
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Extension '{}' removed",
            name
        ))]))
    }
}

impl ServerHandler for SkillsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Spawnbot Skills Server - skill management and self-evolution")
    }
}
