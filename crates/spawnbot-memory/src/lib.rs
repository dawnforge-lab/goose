//! Spawnbot memory system — SQLite + FTS5 + semantic search MCP server

pub mod browse;
pub mod db;
pub mod decay;
pub mod dedup;
pub mod delete;
pub mod embeddings;
pub mod indexer;
pub mod recall;
pub mod server;
pub mod store;
