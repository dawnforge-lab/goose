use anyhow::Result;
use dialoguer::{Confirm, Input, Password, Select};
use spawnbot_common::config::*;
use spawnbot_common::types::*;
use std::path::PathBuf;

/// Run the 10-step TUI onboarding wizard.
/// Returns a populated SpawnbotConfig and metadata for template generation.
pub fn run_onboarding() -> Result<OnboardingResult> {
    println!("\n  Welcome to Spawnbot Setup\n");

    // Step 1: Bot name
    let bot_name: String = Input::new()
        .with_prompt("1. What should your bot be called?")
        .default("Spawnbot".into())
        .interact_text()?;

    // Step 2: User name
    let user_name: String = Input::new()
        .with_prompt("2. What's your name?")
        .interact_text()?;

    // Step 3: User role
    let user_role: String = Input::new()
        .with_prompt("3. What do you do? (e.g. 'Software engineer', 'Researcher')")
        .default("".into())
        .interact_text()?;

    // Step 4: LLM provider
    let providers = ["Anthropic", "OpenAI", "Google", "Ollama", "LiteLLM", "Custom"];
    let provider_idx = Select::new()
        .with_prompt("4. LLM provider")
        .items(&providers)
        .default(0)
        .interact()?;
    let provider = match provider_idx {
        0 => LlmProvider::Anthropic,
        1 => LlmProvider::Openai,
        2 => LlmProvider::Google,
        3 => LlmProvider::Ollama,
        4 => LlmProvider::Litellm,
        _ => LlmProvider::Custom,
    };

    // Step 5: Model name
    let default_model = match provider {
        LlmProvider::Anthropic => "claude-sonnet-4",
        LlmProvider::Openai => "gpt-4o",
        LlmProvider::Google => "gemini-2.5-pro",
        LlmProvider::Ollama => "llama3",
        _ => "",
    };
    let model: String = Input::new()
        .with_prompt("5. Model name")
        .default(default_model.into())
        .interact_text()?;

    // Step 6: API key
    let default_env = match provider {
        LlmProvider::Anthropic => "ANTHROPIC_API_KEY",
        LlmProvider::Openai => "OPENAI_API_KEY",
        LlmProvider::Google => "GOOGLE_API_KEY",
        _ => "LLM_API_KEY",
    };
    let api_key_env: String = Input::new()
        .with_prompt("6. API key environment variable name")
        .default(default_env.into())
        .interact_text()?;

    // Check if the env var is set; if not, prompt for the value
    if std::env::var(&api_key_env).is_err() {
        let set_now = Confirm::new()
            .with_prompt(format!("   {} is not set. Enter it now?", api_key_env))
            .default(true)
            .interact()?;
        if set_now {
            let _key: String = Password::new()
                .with_prompt(format!("   {}", api_key_env))
                .interact()?;
            println!("   Note: Set this in your shell profile to persist across sessions.");
        }
    }

    // Step 7: Autonomy mode
    let modes = ["Yolo (act autonomously)", "Approval (ask before acting)"];
    let mode_idx = Select::new()
        .with_prompt("7. Autonomy mode")
        .items(&modes)
        .default(0)
        .interact()?;
    let autonomy_mode = if mode_idx == 0 {
        AutonomyMode::Yolo
    } else {
        AutonomyMode::Approval
    };

    // Step 8: Telegram (optional)
    let enable_telegram = Confirm::new()
        .with_prompt("8. Enable Telegram integration?")
        .default(false)
        .interact()?;
    let mut telegram = TelegramConfig::default();
    if enable_telegram {
        telegram.enabled = true;
        telegram.bot_token_env = Input::new()
            .with_prompt("   Bot token env var")
            .default("TELEGRAM_BOT_TOKEN".into())
            .interact_text()?;
        let owner_id_str: String = Input::new()
            .with_prompt("   Your Telegram user ID")
            .interact_text()?;
        telegram.owner_id = owner_id_str.parse().unwrap_or(0);
    }

    // Step 9: Workspace path
    let default_ws = spawnbot_common::paths::spawnbot_home()
        .join("workspace")
        .to_string_lossy()
        .to_string();
    let workspace_str: String = Input::new()
        .with_prompt("9. Workspace path")
        .default(default_ws)
        .interact_text()?;
    let workspace = PathBuf::from(workspace_str);

    // Step 10: Confirm
    println!("\n  Configuration Summary:");
    println!("  Bot name:     {}", bot_name);
    println!("  User:         {} ({})", user_name, user_role);
    println!("  Provider:     {:?}", provider);
    println!("  Model:        {}", model);
    println!("  Autonomy:     {:?}", autonomy_mode);
    println!("  Telegram:     {}", if enable_telegram { "yes" } else { "no" });
    println!("  Workspace:    {}", workspace.display());
    println!();

    let confirmed = Confirm::new()
        .with_prompt("10. Create workspace with these settings?")
        .default(true)
        .interact()?;

    if !confirmed {
        anyhow::bail!("Setup cancelled by user");
    }

    let config = SpawnbotConfig {
        version: 1,
        workspace,
        llm: LlmConfig {
            provider,
            model,
            api_key_env,
        },
        embeddings: EmbeddingsConfig::default(),
        whisper: WhisperConfig::default(),
        telegram,
        autonomy: AutonomyConfig {
            mode: autonomy_mode,
            ..AutonomyConfig::default()
        },
        skills: vec![],
    };

    Ok(OnboardingResult {
        config,
        bot_name,
        user_name,
        user_role,
    })
}

/// Result from the onboarding wizard.
pub struct OnboardingResult {
    pub config: SpawnbotConfig,
    pub bot_name: String,
    pub user_name: String,
    pub user_role: String,
}
