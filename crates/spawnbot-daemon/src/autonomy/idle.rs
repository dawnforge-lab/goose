use crate::autonomy::prompts;
use crate::queue::{EventSource, PriorityQueue, QueueEvent};
use spawnbot_common::config::AutonomyConfig;
use spawnbot_common::types::Priority;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// The idle loop monitors inactivity and enqueues escalating heartbeat prompts
/// at three tiers: base, escalation, and warning.
pub struct IdleLoop {
    last_activity: Arc<Mutex<Instant>>,
    queue: Arc<PriorityQueue>,
    config: AutonomyConfig,
}

impl IdleLoop {
    pub fn new(queue: Arc<PriorityQueue>, config: AutonomyConfig) -> Self {
        Self {
            last_activity: Arc::new(Mutex::new(Instant::now())),
            queue,
            config,
        }
    }

    /// Record that activity occurred, resetting idle timers.
    pub async fn touch(&self) {
        *self.last_activity.lock().await = Instant::now();
    }

    /// Get a handle to the last activity timestamp for external updates.
    pub fn activity_handle(&self) -> Arc<Mutex<Instant>> {
        self.last_activity.clone()
    }

    /// Run the idle monitoring loop. This runs forever and should be spawned as a task.
    pub async fn run(&self) {
        let base = Duration::from_millis(self.config.idle_base_interval);
        let escalation = Duration::from_millis(self.config.idle_escalation);
        let warning = Duration::from_millis(self.config.idle_warning);
        let check_interval = Duration::from_secs(60);

        let mut base_fired = false;
        let mut escalation_fired = false;
        let mut warning_fired = false;

        loop {
            tokio::time::sleep(check_interval).await;

            let elapsed = self.last_activity.lock().await.elapsed();

            if elapsed >= warning && !warning_fired {
                warning_fired = true;
                self.queue
                    .enqueue(QueueEvent::system(
                        Priority::Low,
                        prompts::idle_warning_prompt(),
                        EventSource::Idle,
                    ))
                    .await;
            } else if elapsed >= escalation && !escalation_fired {
                escalation_fired = true;
                self.queue
                    .enqueue(QueueEvent::system(
                        Priority::Normal,
                        prompts::idle_escalation_prompt(),
                        EventSource::Idle,
                    ))
                    .await;
            } else if elapsed >= base && !base_fired {
                base_fired = true;
                self.queue
                    .enqueue(QueueEvent::system(
                        Priority::Normal,
                        prompts::idle_base_prompt(),
                        EventSource::Idle,
                    ))
                    .await;
            }

            // Reset flags if activity occurred
            if elapsed < base {
                base_fired = false;
                escalation_fired = false;
                warning_fired = false;
            }
        }
    }
}
