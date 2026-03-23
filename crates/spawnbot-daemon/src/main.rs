use anyhow::Result;
use clap::{Parser, Subcommand};
use spawnbot_common::{config::SpawnbotConfig, paths};
use std::sync::Arc;

mod acp;
mod autonomy;
mod commands;
mod consumer;
mod queue;
mod session;
mod telegram;

#[derive(Parser)]
#[command(name = "spawnbot", about = "Spawnbot autonomous agent daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Start the daemon
    Start {
        #[arg(long)]
        daemon: bool,
    },
    /// Run setup wizard
    Setup,
    /// Run diagnostics
    Doctor,
    /// Show configuration
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(CliCommand::Setup) | None => {
            // Check if already set up
            let config_path = paths::config_path();
            if config_path.exists() {
                let config = SpawnbotConfig::load(&config_path)?;
                start_daemon(config).await?;
            } else {
                // Run onboarding
                let result = spawnbot_onboarding::wizard::run_onboarding()?;
                spawnbot_onboarding::scaffold::scaffold_workspace_with_names(
                    &result.config,
                    &result.bot_name,
                    &result.user_name,
                    &result.user_role,
                )?;
                println!("\n  Workspace created! Starting daemon...\n");
                start_daemon(result.config).await?;
            }
        }
        Some(CliCommand::Start { .. }) => {
            let config = SpawnbotConfig::load(&paths::config_path())?;
            start_daemon(config).await?;
        }
        Some(CliCommand::Doctor) => {
            println!("Running diagnostics...");
            let config_path = paths::config_path();
            println!(
                "  Config:    {}",
                if config_path.exists() { "OK" } else { "MISSING" }
            );
            let ws = paths::spawnbot_home().join("workspace");
            println!(
                "  Workspace: {}",
                if ws.exists() { "OK" } else { "MISSING" }
            );
        }
        Some(CliCommand::Config) => {
            let config_path = paths::config_path();
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                println!("{}", content);
            } else {
                println!("No config found. Run `spawnbot setup` first.");
            }
        }
    }

    Ok(())
}

async fn start_daemon(config: SpawnbotConfig) -> Result<()> {
    tracing::info!("Starting Spawnbot daemon");

    let workspace = spawnbot_common::paths::WorkspacePaths::new(config.workspace.clone());
    let priority_queue = Arc::new(queue::PriorityQueue::new());

    // Start autonomy engine
    let crons = autonomy::cron::load_crons(&workspace.crons_yaml()).unwrap_or_default();
    let cron_scheduler = autonomy::cron::CronScheduler::new(crons, priority_queue.clone());
    cron_scheduler.start().await?;

    let idle_loop = autonomy::idle::IdleLoop::new(priority_queue.clone(), config.autonomy.clone());
    let _idle_handle = tokio::spawn(async move { idle_loop.run().await });

    let pollers = autonomy::poller::load_pollers(&workspace.pollers_yaml()).unwrap_or_default();
    let poller_manager = autonomy::poller::PollerManager::new(
        pollers,
        workspace.poller_state_dir(),
        priority_queue.clone(),
    );
    poller_manager.start().await?;

    // Start Telegram if enabled
    if config.telegram.enabled {
        let tg = telegram::bot::TelegramBot::new(config.telegram.clone(), priority_queue.clone());
        tokio::spawn(async move {
            let _ = tg.start().await;
        });
    }

    tracing::info!("Daemon running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}
