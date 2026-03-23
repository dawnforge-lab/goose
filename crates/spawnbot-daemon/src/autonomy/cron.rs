use crate::queue::PriorityQueue;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    queue: Arc<PriorityQueue>,
    jobs: Vec<CronJobConfig>,
}

impl CronScheduler {
    pub fn new(jobs: Vec<CronJobConfig>, queue: Arc<PriorityQueue>) -> Self {
        Self { queue, jobs }
    }

    /// Start all enabled cron jobs as background tasks.
    pub async fn start(&self) -> Result<()> {
        for job in &self.jobs {
            if !job.enabled {
                continue;
            }
            let _queue = self.queue.clone();
            let name = job.name.clone();
            let _prompt = job.prompt.clone();
            let cron_expr = job.cron.clone();

            tokio::spawn(async move {
                // Parse cron and schedule
                tracing::info!(name = %name, cron = %cron_expr, "Cron job registered");
                // In production, use tokio-cron-scheduler here
                // For now, this is a placeholder that logs registration
            });
        }
        Ok(())
    }
}
