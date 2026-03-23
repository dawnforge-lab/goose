use crate::queue::{EventSource, PriorityQueue, QueueEvent, ReplyTarget};
use crate::session::SessionManager;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Consumes events from the priority queue and routes them through the session manager.
///
/// User messages get priority — if a user is waiting, system events are deferred.
pub struct QueueConsumer {
    queue: Arc<PriorityQueue>,
    session: Arc<Mutex<SessionManager>>,
    user_waiting: Arc<AtomicBool>,
}

impl QueueConsumer {
    pub fn new(queue: Arc<PriorityQueue>, session: Arc<Mutex<SessionManager>>) -> Self {
        Self {
            queue,
            session,
            user_waiting: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle to the user_waiting flag for external signaling.
    pub fn user_waiting_handle(&self) -> Arc<AtomicBool> {
        self.user_waiting.clone()
    }

    /// Run the consumer loop. This runs forever and should be spawned as a task.
    pub async fn run(&self) {
        loop {
            let event = self.queue.dequeue().await;

            // Yield to user messages if a user is waiting
            if self.user_waiting.load(Ordering::Relaxed) {
                if !matches!(event.source, EventSource::User { .. }) {
                    // Re-enqueue system event and wait briefly
                    self.queue.enqueue(event).await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            }

            if let Err(e) = self.process_event(event).await {
                tracing::error!(error = %e, "Failed to process queue event");
            }
        }
    }

    async fn process_event(&self, event: QueueEvent) -> Result<()> {
        let response = {
            let mut session = self.session.lock().await;
            session.prompt(&event.content).await?
        };

        // Route response based on source
        match &event.source {
            EventSource::User { reply_to } => match reply_to {
                ReplyTarget::Telegram(chat_id) => {
                    tracing::info!(chat_id, "Would send Telegram reply");
                    // TODO: send via TelegramBot
                    let _ = &response;
                }
                ReplyTarget::Tui => {
                    println!("{}", response);
                }
                ReplyTarget::Desktop => {
                    tracing::info!("Desktop reply: {}", response);
                }
            },
            _ => {
                tracing::info!(source = ?event.source, "System event processed");
            }
        }

        Ok(())
    }
}
