use crate::queue::PriorityQueue;
use anyhow::Result;
use spawnbot_common::config::TelegramConfig;
use std::sync::Arc;

/// Telegram bot coordinator — handles incoming messages via teloxide
/// and routes them into the priority queue.
pub struct TelegramBot {
    config: TelegramConfig,
    queue: Arc<PriorityQueue>,
}

impl TelegramBot {
    pub fn new(config: TelegramConfig, queue: Arc<PriorityQueue>) -> Self {
        Self { config, queue }
    }

    /// Start the Telegram bot.
    ///
    /// In production, this sets up teloxide handlers for text, voice, and command messages.
    /// Currently a skeleton that logs configuration.
    pub async fn start(&self) -> Result<()> {
        // TODO: Set up teloxide bot with handlers
        // - Text messages: enqueue as user events
        // - Voice messages: transcribe via Whisper, then enqueue
        // - Commands: parse and handle (/status, /memory, etc.)
        // - Access control: check is_allowed before processing
        tracing::info!(owner = self.config.owner_id, "Telegram bot configured");
        let _ = &self.queue; // suppress unused warning in skeleton
        Ok(())
    }
}
