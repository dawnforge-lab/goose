use crate::types::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnbotConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_workspace")]
    pub workspace: PathBuf,
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
    #[serde(default)]
    pub whisper: WhisperConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub autonomy: AutonomyConfig,
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_embeddings_provider")]
    pub provider: String,
    #[serde(default = "default_embeddings_model")]
    pub model: String,
    #[serde(default)]
    pub api_key_env: String,
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_embeddings_provider(),
            model: default_embeddings_model(),
            api_key_env: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WhisperConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default = "default_whisper_language")]
    pub language: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token_env: String,
    #[serde(default)]
    pub owner_id: u64,
    #[serde(default)]
    pub allowed_users: Vec<u64>,
    #[serde(default)]
    pub allowed_chats: Vec<i64>,
    #[serde(default = "default_telegram_mode")]
    pub mode: TelegramMode,
    #[serde(default)]
    pub ngrok_token_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyConfig {
    #[serde(default = "default_autonomy_mode")]
    pub mode: AutonomyMode,
    #[serde(default = "default_idle_base")]
    pub idle_base_interval: u64,
    #[serde(default = "default_idle_escalation")]
    pub idle_escalation: u64,
    #[serde(default = "default_idle_warning")]
    pub idle_warning: u64,
    #[serde(default = "default_decay_half_life")]
    pub decay_half_life: u32,
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            mode: default_autonomy_mode(),
            idle_base_interval: default_idle_base(),
            idle_escalation: default_idle_escalation(),
            idle_warning: default_idle_warning(),
            decay_half_life: default_decay_half_life(),
        }
    }
}

// Defaults
fn default_version() -> u32 {
    1
}
fn default_workspace() -> PathBuf {
    crate::paths::spawnbot_home().join("workspace")
}
fn default_embeddings_provider() -> String {
    "gemini".into()
}
fn default_embeddings_model() -> String {
    "text-embedding-004".into()
}
fn default_whisper_language() -> String {
    "en".into()
}
fn default_telegram_mode() -> TelegramMode {
    TelegramMode::Polling
}
fn default_autonomy_mode() -> AutonomyMode {
    AutonomyMode::Yolo
}
fn default_idle_base() -> u64 {
    1_800_000
}
fn default_idle_escalation() -> u64 {
    7_200_000
}
fn default_idle_warning() -> u64 {
    21_600_000
}
fn default_decay_half_life() -> u32 {
    30
}

impl SpawnbotConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Self =
            serde_yaml::from_str(&content).with_context(|| "Failed to parse config.yaml")?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content =
            serde_yaml::to_string(self).with_context(|| "Failed to serialize config")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");

        let config = SpawnbotConfig {
            version: 1,
            workspace: PathBuf::from("/tmp/test-workspace"),
            embeddings: EmbeddingsConfig::default(),
            whisper: WhisperConfig::default(),
            telegram: TelegramConfig::default(),
            autonomy: AutonomyConfig::default(),
            skills: vec!["memory-management".into()],
        };

        config.save(&path).unwrap();
        let loaded = SpawnbotConfig::load(&path).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.workspace, PathBuf::from("/tmp/test-workspace"));
        assert_eq!(loaded.autonomy.mode, AutonomyMode::Yolo);
        assert_eq!(loaded.autonomy.decay_half_life, 30);
    }

    #[test]
    fn test_config_defaults() {
        let yaml = r#"
version: 1
"#;
        let config: SpawnbotConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.version, 1);
        assert!(!config.embeddings.enabled);
        assert_eq!(config.autonomy.mode, AutonomyMode::Yolo);
        assert_eq!(config.autonomy.idle_base_interval, 1_800_000);
    }
}
