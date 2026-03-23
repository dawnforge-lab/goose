//! Telegram bot — teloxide-based message handler that routes messages into the priority queue.

use crate::commands;
use crate::queue::{PriorityQueue, QueueEvent, ReplyTarget};
use crate::telegram::access;
use anyhow::Result;
use spawnbot_common::config::TelegramConfig;
use std::sync::Arc;
use teloxide::prelude::*;

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

    /// Start the Telegram bot polling loop.
    ///
    /// This reads the bot token from the environment variable specified in config,
    /// sets up message handlers with access control, and polls for updates.
    /// This function runs until the bot is shut down (e.g., via Ctrl+C).
    pub async fn start(self) -> Result<()> {
        let token = std::env::var(&self.config.bot_token_env).map_err(|_| {
            anyhow::anyhow!(
                "Telegram bot token not found in env var: {}",
                self.config.bot_token_env
            )
        })?;

        let bot = Bot::new(token);
        let config = Arc::new(self.config);
        let queue = self.queue;

        tracing::info!(owner = config.owner_id, "Starting Telegram bot");

        let config_dep = config.clone();
        let queue_dep = queue.clone();

        let handler = Update::filter_message().endpoint(
            move |message: Message, bot: Bot| {
                let config = config_dep.clone();
                let queue = queue_dep.clone();
                async move {
                    handle_message(message, bot, config, queue).await
                }
            },
        );

        Dispatcher::builder(bot, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;

        Ok(())
    }
}

/// Handle a single incoming Telegram message.
///
/// Performs access control, then routes commands or regular messages
/// into the priority queue for LLM processing.
async fn handle_message(
    message: Message,
    bot: Bot,
    config: Arc<TelegramConfig>,
    queue: Arc<PriorityQueue>,
) -> ResponseResult<()> {
    let Some(user) = message.from.as_ref() else {
        return Ok(());
    };
    let user_id = user.id.0;
    let chat_id = message.chat.id.0;

    // Access control
    if !access::is_allowed(user_id, chat_id, &config) {
        tracing::warn!(user_id, chat_id, "Unauthorized Telegram message");
        return Ok(());
    }

    // Handle text messages
    if let Some(text) = message.text() {
        // Check for slash commands
        if text.starts_with('/') {
            if let Some(cmd) = commands::parse_command(text) {
                let response = format!("Command received: {:?}", cmd);
                bot.send_message(message.chat.id, response).await?;
                return Ok(());
            }
        }

        // Regular message — enqueue for LLM processing
        let event = QueueEvent::user(text.to_string(), ReplyTarget::Telegram(chat_id));
        queue.enqueue(event).await;

        // Response will come via the consumer when the LLM responds.
        // No acknowledgement sent here to avoid clutter.
    }

    // Handle voice messages
    if message.voice().is_some() {
        tracing::info!(chat_id, "Received voice message");
        // TODO: download file via bot.get_file(), transcribe with Whisper, enqueue
        bot.send_message(
            message.chat.id,
            "Voice message received (transcription not yet wired)",
        )
        .await?;
    }

    Ok(())
}
