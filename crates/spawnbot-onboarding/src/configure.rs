//! Provider configuration — writes to ~/.config/goose/config.yaml
//! so that `goose acp` picks up the right provider and model.

use anyhow::{Context, Result};
use dialoguer::{Input, Password, Select};
use std::collections::HashMap;
use std::path::PathBuf;

struct ProviderInfo {
    name: &'static str,
    display: &'static str,
    default_model: &'static str,
    api_key_env: &'static str,
    models: &'static [&'static str],
}

const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        name: "anthropic",
        display: "Anthropic (Claude)",
        default_model: "claude-sonnet-4-20250514",
        api_key_env: "ANTHROPIC_API_KEY",
        models: &[
            "claude-sonnet-4-20250514",
            "claude-opus-4-20250514",
            "claude-haiku-4-20250506",
        ],
    },
    ProviderInfo {
        name: "openai",
        display: "OpenAI (GPT)",
        default_model: "gpt-4o",
        api_key_env: "OPENAI_API_KEY",
        models: &["gpt-4o", "gpt-4o-mini", "gpt-4.1", "o3", "o4-mini"],
    },
    ProviderInfo {
        name: "google",
        display: "Google (Gemini)",
        default_model: "gemini-2.5-pro",
        api_key_env: "GOOGLE_API_KEY",
        models: &["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
    },
    ProviderInfo {
        name: "ollama",
        display: "Ollama (local)",
        default_model: "llama3",
        api_key_env: "",
        models: &["llama3", "llama3.1", "mistral", "codellama", "deepseek-coder"],
    },
    ProviderInfo {
        name: "openrouter",
        display: "OpenRouter (any model)",
        default_model: "anthropic/claude-sonnet-4",
        api_key_env: "OPENROUTER_API_KEY",
        models: &[
            "anthropic/claude-sonnet-4",
            "anthropic/claude-opus-4",
            "openai/gpt-4o",
            "google/gemini-2.5-pro",
            "meta-llama/llama-3.1-405b",
        ],
    },
];

/// Run the provider configuration dialog.
/// Writes to ~/.config/goose/config.yaml.
pub fn run_configure() -> Result<()> {
    println!("\n  LLM Provider Setup\n");

    // Check if any API keys are already in env
    let mut auto_detected = None;
    for provider in PROVIDERS {
        if !provider.api_key_env.is_empty() && std::env::var(provider.api_key_env).is_ok() {
            auto_detected = Some(provider);
            break;
        }
    }

    let provider = if let Some(detected) = auto_detected {
        println!(
            "  Found {} in your environment.\n",
            detected.api_key_env
        );
        let use_it = dialoguer::Confirm::new()
            .with_prompt(format!("  Use {}?", detected.display))
            .default(true)
            .interact()?;
        if use_it {
            detected
        } else {
            select_provider()?
        }
    } else {
        select_provider()?
    };

    // Select model
    let model = select_model(provider)?;

    // API key
    let api_key = if provider.api_key_env.is_empty() {
        // Ollama doesn't need a key
        None
    } else if std::env::var(provider.api_key_env).is_ok() {
        println!("  Using {} from environment.", provider.api_key_env);
        None // already set in env, no need to store
    } else {
        println!(
            "\n  {} needs an API key.",
            provider.display
        );
        let key: String = Password::new()
            .with_prompt(format!("  Paste your {} key", provider.api_key_env))
            .interact()?;
        if key.is_empty() {
            anyhow::bail!("API key is required for {}", provider.display);
        }
        Some((provider.api_key_env.to_string(), key))
    };

    // Write config
    write_goose_config(provider.name, &model, api_key.as_ref())?;

    println!("\n  Provider configured: {} ({})", provider.display, model);
    println!();

    Ok(())
}

fn select_provider() -> Result<&'static ProviderInfo> {
    let items: Vec<&str> = PROVIDERS.iter().map(|p| p.display).collect();
    let idx = Select::new()
        .with_prompt("  Choose your LLM provider")
        .items(&items)
        .default(0)
        .interact()?;
    Ok(&PROVIDERS[idx])
}

fn select_model(provider: &ProviderInfo) -> Result<String> {
    if provider.models.is_empty() {
        let model: String = Input::new()
            .with_prompt("  Model name")
            .default(provider.default_model.into())
            .interact_text()?;
        return Ok(model);
    }

    // Show model list with option to type custom
    let mut items: Vec<String> = provider.models.iter().map(|m| m.to_string()).collect();
    items.push("Other (type a model name)".to_string());

    let idx = Select::new()
        .with_prompt("  Choose a model")
        .items(&items)
        .default(0)
        .interact()?;

    if idx == items.len() - 1 {
        let model: String = Input::new()
            .with_prompt("  Model name")
            .default(provider.default_model.into())
            .interact_text()?;
        Ok(model)
    } else {
        Ok(provider.models[idx].to_string())
    }
}

fn goose_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not determine config directory")
        .join("goose")
}

fn write_goose_config(
    provider: &str,
    model: &str,
    api_key: Option<&(String, String)>,
) -> Result<()> {
    let config_dir = goose_config_path();
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");

    // Load existing config or start fresh
    let mut config: HashMap<String, serde_yaml::Value> = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_yaml::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    config.insert(
        "GOOSE_PROVIDER".to_string(),
        serde_yaml::Value::String(provider.to_string()),
    );
    config.insert(
        "GOOSE_MODEL".to_string(),
        serde_yaml::Value::String(model.to_string()),
    );

    // Store API key in config (Goose also supports keyring, but file is simpler)
    if let Some((env_name, key)) = api_key {
        config.insert(
            env_name.to_string(),
            serde_yaml::Value::String(key.to_string()),
        );
    }

    let yaml = serde_yaml::to_string(&config)
        .with_context(|| "Failed to serialize config")?;
    std::fs::write(&config_path, yaml)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(())
}

/// Check if a provider is already configured.
pub fn is_configured() -> bool {
    let config_path = goose_config_path().join("config.yaml");
    if !config_path.exists() {
        return false;
    }
    // Check if GOOSE_PROVIDER is set in env or config
    if std::env::var("GOOSE_PROVIDER").is_ok() {
        return true;
    }
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = serde_yaml::from_str::<HashMap<String, serde_yaml::Value>>(&content) {
            return config.contains_key("GOOSE_PROVIDER");
        }
    }
    false
}
