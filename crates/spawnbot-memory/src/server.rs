use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone)]
pub struct MemoryServer {
    db_path: std::path::PathBuf,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
pub struct MemoryStoreParams {
    /// The content of the memory to store
    pub content: String,
    /// Category for organizing the memory (e.g. "tech", "personal", "project")
    pub category: String,
    /// Importance score from 0.0 to 1.0 (default: 0.5)
    #[serde(default = "default_importance")]
    pub importance: f64,
    /// If true, this memory will not decay over time (default: false)
    #[serde(default)]
    pub evergreen: bool,
}

fn default_importance() -> f64 {
    0.5
}

#[derive(Deserialize, JsonSchema)]
pub struct MemoryRecallParams {
    /// Natural language search query
    pub query: String,
    /// Maximum number of results to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Optional category filter
    pub category: Option<String>,
    /// Half-life in days for temporal decay (default: 30)
    #[serde(default = "default_half_life")]
    pub half_life: u32,
}

fn default_limit() -> usize {
    10
}

fn default_half_life() -> u32 {
    30
}

#[derive(Deserialize, JsonSchema)]
pub struct MemoryBrowseParams {
    /// Optional category to filter by
    pub category: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default = "default_browse_limit")]
    pub limit: usize,
    /// Number of results to skip for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_browse_limit() -> usize {
    20
}

#[derive(Deserialize, JsonSchema)]
pub struct MemoryDeleteParams {
    /// The ID of the memory to delete
    pub id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct MemoryReindexParams {
    /// Path to the directory containing markdown files to index
    pub path: String,
}

#[tool_router]
impl MemoryServer {
    pub fn new(db_path: std::path::PathBuf) -> anyhow::Result<Self> {
        // Ensure the database schema exists
        crate::db::init_db(&db_path)?;
        Ok(Self {
            db_path,
            tool_router: Self::tool_router(),
        })
    }

    fn conn(&self) -> anyhow::Result<rusqlite::Connection> {
        Ok(rusqlite::Connection::open(&self.db_path)?)
    }

    #[tool(description = "Store a memory with content, category, importance, and evergreen flag")]
    fn memory_store(
        &self,
        Parameters(params): Parameters<MemoryStoreParams>,
    ) -> Result<String, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let result = crate::store::store_memory(
            &conn,
            &params.content,
            &params.category,
            params.importance,
            params.evergreen,
        )
        .map_err(|e| e.to_string())?;

        match &result {
            crate::store::StoreResult::Inserted { id } => {
                Ok(format!("Memory stored with ID: {id}"))
            }
            crate::store::StoreResult::Merged { id } => {
                Ok(format!("Memory merged with existing ID: {id}"))
            }
        }
    }

    #[tool(description = "Search memories using natural language query with hybrid FTS5 + temporal decay ranking")]
    fn memory_recall(
        &self,
        Parameters(params): Parameters<MemoryRecallParams>,
    ) -> Result<String, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let results = crate::recall::recall_memories(
            &conn,
            &params.query,
            params.limit,
            params.category.as_deref(),
            params.half_life,
        )
        .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&results).map_err(|e| e.to_string())
    }

    #[tool(description = "Browse memories by category, ordered by importance")]
    fn memory_browse(
        &self,
        Parameters(params): Parameters<MemoryBrowseParams>,
    ) -> Result<String, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let entries = crate::browse::browse_memories(
            &conn,
            params.category.as_deref(),
            params.limit,
            params.offset,
        )
        .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())
    }

    #[tool(description = "Delete a memory by its ID")]
    fn memory_delete(
        &self,
        Parameters(params): Parameters<MemoryDeleteParams>,
    ) -> Result<String, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let deleted =
            crate::delete::delete_memory(&conn, &params.id).map_err(|e| e.to_string())?;

        if deleted {
            Ok(format!("Memory {} deleted", params.id))
        } else {
            Ok(format!("Memory {} not found", params.id))
        }
    }

    #[tool(description = "Re-index markdown files from a directory into searchable chunks")]
    fn memory_reindex(
        &self,
        Parameters(params): Parameters<MemoryReindexParams>,
    ) -> Result<String, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let path = std::path::Path::new(&params.path);
        let count =
            crate::indexer::reindex(&conn, path).map_err(|e| e.to_string())?;

        Ok(format!("Indexed {count} chunks from {}", params.path))
    }
}

#[rmcp::tool_handler]
impl rmcp::handler::server::ServerHandler for MemoryServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Spawnbot Memory Server - semantic memory with FTS5 search")
    }
}
