//! Cron scheduler — uses tokio-cron-scheduler to fire events on cron schedules.

use crate::autonomy::prompts;
use crate::queue::{EventSource, PriorityQueue, QueueEvent};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use spawnbot_common::types::Priority;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobConfig {
    pub name: String,
    pub cron: String,
    pub prompt: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Load cron job configurations from a YAML file.
///
/// Returns an empty vec if the file does not exist.
pub fn load_crons(path: &std::path::Path) -> Result<Vec<CronJobConfig>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path)?;
    let crons: Vec<CronJobConfig> = serde_yaml::from_str(&content)?;
    Ok(crons)
}

/// Manages cron-based scheduled jobs that enqueue events at specified intervals.
pub struct CronScheduler {
    scheduler: JobScheduler,
}

impl CronScheduler {
    /// Create a new cron scheduler and register all enabled jobs.
    ///
    /// Each job, when fired, enqueues a system event into the priority queue
    /// with the configured prompt text wrapped in a cron system prompt.
    pub async fn new(jobs: Vec<CronJobConfig>, queue: Arc<PriorityQueue>) -> Result<Self> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create job scheduler: {}", e))?;

        for job_config in &jobs {
            if !job_config.enabled {
                continue;
            }

            let queue = queue.clone();
            let name = job_config.name.clone();
            let prompt_text = job_config.prompt.clone();
            let cron_expr = job_config.cron.clone();

            match Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
                let queue = queue.clone();
                let name = name.clone();
                let prompt_text = prompt_text.clone();
                Box::pin(async move {
                    tracing::info!(name = %name, "Cron job fired");
                    queue
                        .enqueue(QueueEvent::system(
                            Priority::Normal,
                            prompts::cron_prompt(&name, &prompt_text),
                            EventSource::Cron(name),
                        ))
                        .await;
                }) as Pin<Box<dyn Future<Output = ()> + Send>>
            }) {
                Ok(job) => {
                    scheduler
                        .add(job)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to add cron job '{}': {}",
                                job_config.name,
                                e
                            )
                        })?;
                    tracing::info!(
                        name = %job_config.name,
                        cron = %job_config.cron,
                        "Cron job registered"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        name = %job_config.name,
                        error = %e,
                        "Failed to create cron job"
                    );
                }
            }
        }

        Ok(Self { scheduler })
    }

    /// Start the cron scheduler. Jobs will begin firing on their configured schedules.
    pub async fn start(&self) -> Result<()> {
        self.scheduler
            .start()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start cron scheduler: {}", e))?;
        tracing::info!("Cron scheduler started");
        Ok(())
    }

    /// Stop the cron scheduler and all registered jobs.
    pub async fn stop(mut self) -> Result<()> {
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to stop cron scheduler: {}", e))?;
        tracing::info!("Cron scheduler stopped");
        Ok(())
    }
}
