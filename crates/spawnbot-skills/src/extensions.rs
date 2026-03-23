//! Extension install/remove with config directory storage.
//!
//! Extensions are managed as config directories under `~/.spawnbot/extensions/`.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    pub description: String,
}

pub fn install_extension(extensions_dir: &Path, config: &ExtensionConfig) -> Result<()> {
    let ext_dir = extensions_dir.join(&config.name);
    if ext_dir.exists() {
        bail!("Extension '{}' already installed", config.name);
    }
    std::fs::create_dir_all(&ext_dir)?;
    let config_path = ext_dir.join("config.json");
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&config_path, content)?;
    Ok(())
}

pub fn remove_extension(extensions_dir: &Path, name: &str) -> Result<()> {
    let ext_dir = extensions_dir.join(name);
    if !ext_dir.exists() {
        bail!("Extension '{}' not found", name);
    }
    std::fs::remove_dir_all(&ext_dir)?;
    Ok(())
}

pub fn list_extensions(extensions_dir: &Path) -> Result<Vec<ExtensionConfig>> {
    if !extensions_dir.exists() {
        return Ok(vec![]);
    }
    let mut extensions = Vec::new();
    for entry in std::fs::read_dir(extensions_dir)? {
        let entry = entry?;
        let config_path = entry.path().join("config.json");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: ExtensionConfig = serde_json::from_str(&content)?;
            extensions.push(config);
        }
    }
    extensions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(extensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_extension() -> ExtensionConfig {
        ExtensionConfig {
            name: "github-mcp".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()],
            env: [("GITHUB_TOKEN".to_string(), "ghp_xxx".to_string())]
                .into_iter()
                .collect(),
            description: "GitHub integration via MCP".to_string(),
        }
    }

    #[test]
    fn install_and_list_extension() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");

        let config = test_extension();
        install_extension(&ext_dir, &config).unwrap();

        // Verify directory and config file were created
        let config_path = ext_dir.join("github-mcp").join("config.json");
        assert!(config_path.exists());

        // List should return it
        let extensions = list_extensions(&ext_dir).unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0].name, "github-mcp");
        assert_eq!(extensions[0].command, "npx");
        assert_eq!(extensions[0].env.get("GITHUB_TOKEN").unwrap(), "ghp_xxx");
    }

    #[test]
    fn install_duplicate_extension_fails() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");

        let config = test_extension();
        install_extension(&ext_dir, &config).unwrap();

        let err = install_extension(&ext_dir, &config).unwrap_err();
        assert!(err.to_string().contains("already installed"));
    }

    #[test]
    fn remove_extension_deletes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");

        let config = test_extension();
        install_extension(&ext_dir, &config).unwrap();

        let ext_path = ext_dir.join("github-mcp");
        assert!(ext_path.exists());

        remove_extension(&ext_dir, "github-mcp").unwrap();
        assert!(!ext_path.exists());
    }

    #[test]
    fn remove_nonexistent_extension_fails() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");
        std::fs::create_dir_all(&ext_dir).unwrap();

        let err = remove_extension(&ext_dir, "nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn list_extensions_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");

        let extensions = list_extensions(&ext_dir).unwrap();
        assert!(extensions.is_empty());
    }

    #[test]
    fn list_extensions_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("extensions");

        let mut config_a = test_extension();
        config_a.name = "alpha-ext".to_string();
        install_extension(&ext_dir, &config_a).unwrap();

        let mut config_b = test_extension();
        config_b.name = "beta-ext".to_string();
        install_extension(&ext_dir, &config_b).unwrap();

        let extensions = list_extensions(&ext_dir).unwrap();
        assert_eq!(extensions.len(), 2);
        assert_eq!(extensions[0].name, "alpha-ext");
        assert_eq!(extensions[1].name, "beta-ext");
    }
}
