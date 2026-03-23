use crate::autonomy::prompts;
use crate::queue::{EventSource, PriorityQueue, QueueEvent};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use spawnbot_common::types::Priority;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollerConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub poller_type: String,
    pub source: String,
    #[serde(default = "default_interval")]
    pub interval: u64,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}

fn default_interval() -> u64 {
    3600
}

/// Load poller configurations from a YAML file.
///
/// Returns an empty vec if the file does not exist.
pub fn load_pollers(path: &Path) -> Result<Vec<PollerConfig>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path)?;
    let pollers: Vec<PollerConfig> = serde_yaml::from_str(&content)?;
    Ok(pollers)
}

/// Manages pollers that monitor external sources (RSS, etc.) and enqueue events.
pub struct PollerManager {
    pollers: Vec<PollerConfig>,
    state_dir: PathBuf,
    queue: Arc<PriorityQueue>,
}

impl PollerManager {
    pub fn new(pollers: Vec<PollerConfig>, state_dir: PathBuf, queue: Arc<PriorityQueue>) -> Self {
        Self {
            pollers,
            state_dir,
            queue,
        }
    }

    /// Start all enabled pollers as background tasks.
    pub async fn start(&self) -> Result<()> {
        for poller in &self.pollers {
            if !poller.enabled {
                continue;
            }
            let queue = self.queue.clone();
            let config = poller.clone();
            let state_dir = self.state_dir.clone();

            tokio::spawn(async move {
                tracing::info!(name = %config.name, "Poller registered");
                let interval = std::time::Duration::from_secs(config.interval);
                loop {
                    if config.poller_type == "rss" {
                        if let Err(e) = poll_rss(&config, &state_dir, &queue).await {
                            tracing::error!(name = %config.name, error = %e, "RSS poll failed");
                        }
                    }
                    tokio::time::sleep(interval).await;
                }
            });
        }
        Ok(())
    }
}

async fn poll_rss(
    config: &PollerConfig,
    state_dir: &Path,
    queue: &PriorityQueue,
) -> Result<()> {
    let response = reqwest::get(&config.source).await?.bytes().await?;
    let feed = feed_rs::parser::parse(&response[..])?;

    // Load seen IDs
    let state_file = state_dir.join(format!("{}.json", config.name));
    let mut seen: HashSet<String> = if state_file.exists() {
        let content = std::fs::read_to_string(&state_file)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashSet::new()
    };

    let mut new_items = Vec::new();
    for entry in &feed.entries {
        let id = entry.id.clone();
        if !seen.contains(&id) {
            seen.insert(id);
            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_default();
            new_items.push(title);
        }
    }

    // Save state
    std::fs::create_dir_all(state_dir)?;
    std::fs::write(&state_file, serde_json::to_string(&seen)?)?;

    // Enqueue new items
    for title in new_items {
        let prompt_text = prompts::poller_prompt(&config.name, &title);
        queue
            .enqueue(QueueEvent::system(
                Priority::Normal,
                prompt_text,
                EventSource::Poller(config.name.clone()),
            ))
            .await;
    }

    Ok(())
}
