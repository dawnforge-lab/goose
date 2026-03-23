//! Queue consumer — dequeues events and sends prompts to the session manager.

use crate::queue::PriorityQueue;
use crate::session::SessionManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct QueueConsumer {
    queue: Arc<PriorityQueue>,
    session: Arc<Mutex<SessionManager>>,
    user_waiting: Arc<AtomicBool>,
}

impl QueueConsumer {
    pub fn new(
        queue: Arc<PriorityQueue>,
        session: Arc<Mutex<SessionManager>>,
        user_waiting: Arc<AtomicBool>,
    ) -> Self {
        Self {
            queue,
            session,
            user_waiting,
        }
    }

    /// Main consumer loop: dequeue events, yield to user messages, prompt session
    pub async fn run(&self) {
        loop {
            let event = self.queue.dequeue().await;

            // If a user is waiting for a response, yield briefly to let
            // their higher-priority message be enqueued and processed first
            if self.user_waiting.load(Ordering::Relaxed) {
                // Re-enqueue the current event and loop to pick up user message
                self.queue.enqueue(event).await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }

            tracing::info!(
                source = %event.source,
                priority = ?event.priority,
                "Processing queue event"
            );

            let mut session = self.session.lock().await;
            match session.prompt(&event.content).await {
                Ok(response) => {
                    tracing::debug!(
                        source = %event.source,
                        response_len = response.len(),
                        "Event processed"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        source = %event.source,
                        error = %e,
                        "Failed to process event"
                    );
                }
            }
        }
    }
}
