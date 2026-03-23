use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, Select};
use spawnbot_common::config::*;
use spawnbot_common::types::*;

/// Run the 3-step TUI onboarding wizard.
/// Returns a populated SpawnbotConfig and auto-start preference.
pub fn run_onboarding() -> Result<OnboardingResult> {
    println!("\n  Welcome to Spawnbot Setup\n");

    // Step 1: Autonomy mode
    let modes = ["Act freely (yolo)", "Ask first (approval)"];
    let mode_idx = Select::new()
        .with_prompt("1. How should your bot operate?")
        .items(&modes)
        .default(0)
        .interact()?;
    let autonomy_mode = if mode_idx == 0 {
        AutonomyMode::Yolo
    } else {
        AutonomyMode::Approval
    };

    // Step 2: Telegram (optional)
    let enable_telegram = Confirm::new()
        .with_prompt("2. Enable Telegram integration?")
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
        telegram.owner_id = owner_id_str
            .parse()
            .with_context(|| format!("Invalid Telegram user ID: '{}'", owner_id_str))?;
    }

    // Step 3: Auto-start on login
    let auto_start = Confirm::new()
        .with_prompt("3. Start automatically on login?")
        .default(true)
        .interact()?;

    let workspace = spawnbot_common::paths::spawnbot_home().join("workspace");

    println!("\n  Configuration Summary:");
    println!("  Autonomy:     {:?}", autonomy_mode);
    println!(
        "  Telegram:     {}",
        if enable_telegram { "yes" } else { "no" }
    );
    println!(
        "  Auto-start:   {}",
        if auto_start { "yes" } else { "no" }
    );
    println!();

    let confirmed = Confirm::new()
        .with_prompt("Create workspace with these settings?")
        .default(true)
        .interact()?;

    if !confirmed {
        anyhow::bail!("Setup cancelled by user");
    }

    let config = SpawnbotConfig {
        version: 1,
        workspace,
        embeddings: EmbeddingsConfig::default(),
        whisper: WhisperConfig::default(),
        telegram,
        autonomy: AutonomyConfig {
            mode: autonomy_mode,
            ..AutonomyConfig::default()
        },
        skills: vec![],
    };

    Ok(OnboardingResult { config, auto_start })
}

/// Result from the onboarding wizard.
pub struct OnboardingResult {
    pub config: SpawnbotConfig,
    pub auto_start: bool,
}
