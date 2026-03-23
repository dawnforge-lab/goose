//! Spawnbot daemon CLI — process management and startup.

use anyhow::{Context, Result};
use clap::Parser;
use spawnbot_common::config::SpawnbotConfig;
use spawnbot_common::paths::{self, WorkspacePaths};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "spawnbot", about = "Spawnbot — your personal AI agent")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Parser, Debug)]
enum CliCommand {
    /// Start the daemon
    Start {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Run the setup wizard
    Setup,
    /// Stop the running daemon
    Stop,
    /// Run diagnostics
    Doctor,
    /// Configure LLM provider and model
    Configure,
    /// Show configuration
    Config,
    /// Factory reset — deletes everything in ~/.spawnbot
    Nuke,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // No subcommand: start if configured, otherwise guide through setup
        None => {
            // Check if Goose is configured (provider/model)
            let goose_config = dirs::config_dir()
                .expect("Could not determine config directory")
                .join("goose/config.yaml");
            if !goose_config.exists() {
                println!("\n  First time? Let's set up your LLM provider.\n");
                run_configure()?;
            }

            // Check if Spawnbot is configured (autonomy, telegram, etc.)
            let config_path = paths::config_path();
            if !config_path.exists() {
                run_setup()?;
            }

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(start_daemon(None))?;
        }
        Some(CliCommand::Start { config }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(start_daemon(config))?;
        }
        Some(CliCommand::Setup) => {
            run_setup()?;
        }
        Some(CliCommand::Configure) => {
            run_configure()?;
        }
        Some(CliCommand::Stop) => {
            stop_daemon()?;
        }
        Some(CliCommand::Doctor) => {
            run_doctor()?;
        }
        Some(CliCommand::Config) => {
            let config_path = paths::config_path();
            if config_path.exists() {
                println!("{}", std::fs::read_to_string(&config_path)?);
            } else {
                println!("Not configured yet. Run: spawnbot setup");
            }
        }
        Some(CliCommand::Nuke) => {
            run_nuke()?;
        }
    }

    Ok(())
}

fn run_setup() -> Result<()> {
    let result = spawnbot_onboarding::wizard::run_onboarding()?;
    spawnbot_onboarding::scaffold::scaffold_workspace(
        &result.config,
    )?;

    if result.auto_start {
        let _ = spawnbot_onboarding::autostart::install_autostart();
    }

    println!("\n  Workspace created at ~/.spawnbot/");
    println!("  Run 'spawnbot' to start.\n");
    Ok(())
}

fn run_configure() -> Result<()> {
    // Resolve goose binary from our install dir to avoid picking up upstream goose
    let goose_bin = paths::spawnbot_home().join("bin").join("goose");
    let goose_cmd = if goose_bin.exists() {
        goose_bin.to_string_lossy().to_string()
    } else {
        "goose".to_string()
    };

    let status = std::process::Command::new(&goose_cmd)
        .arg("configure")
        .status()
        .with_context(|| format!("Failed to run: {} configure", goose_cmd))?;

    if !status.success() {
        anyhow::bail!("Provider configuration failed");
    }
    Ok(())
}

fn run_doctor() -> Result<()> {
    let config_path = paths::config_path();
    let home = paths::spawnbot_home();

    println!("Spawnbot Diagnostics\n");
    println!("  Config:       {}", if config_path.exists() { "OK" } else { "MISSING — run 'spawnbot setup'" });
    println!("  Home:         {}", home.display());
    println!("  Workspace:    {}", if home.join("workspace").exists() { "OK" } else { "MISSING" });

    if config_path.exists() {
        let config = SpawnbotConfig::load(&config_path)?;
        let ws = WorkspacePaths::new(config.workspace);
        println!("  SOUL.md:      {}", if ws.soul_md().exists() { "OK" } else { "MISSING" });
        println!("  Memory DB:    {}", if ws.memory_db().exists() { "OK" } else { "MISSING" });
        println!("  HEARTBEAT.md: {}", if ws.heartbeat_md().exists() { "OK" } else { "MISSING" });
    }

    let pid_path = home.join("daemon.pid");
    if pid_path.exists() {
        let pid = std::fs::read_to_string(&pid_path).unwrap_or_default();
        println!("  Daemon PID:   {}", pid.trim());
    } else {
        println!("  Daemon:       not running");
    }

    // Check Goose
    let goose_available = std::process::Command::new("goose")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!("  Goose:        {}", if goose_available { "OK" } else { "NOT FOUND" });

    Ok(())
}

fn run_nuke() -> Result<()> {
    let home = paths::spawnbot_home();
    if !home.exists() {
        println!("Nothing to nuke — ~/.spawnbot doesn't exist.");
        return Ok(());
    }

    println!("This will permanently delete:");
    println!("  {}", home.display());
    println!("  Including: config, memory, skills, identity, everything.");
    println!();

    let confirm: String = dialoguer::Input::new()
        .with_prompt("Type NUKE to confirm")
        .interact_text()?;

    if confirm.trim() != "NUKE" {
        println!("Cancelled.");
        return Ok(());
    }

    // Remove autostart first
    let _ = spawnbot_onboarding::autostart::remove_autostart();

    // Stop daemon if running
    let _ = stop_daemon();

    // Delete everything
    std::fs::remove_dir_all(&home)?;
    println!("\n  Nuked. Everything deleted.");
    println!("  Run 'spawnbot' to start fresh.\n");
    Ok(())
}

async fn start_daemon(config_path: Option<String>) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load config
    let config_file = config_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(paths::config_path);
    let config = SpawnbotConfig::load(&config_file)
        .with_context(|| format!("Failed to load config from {}", config_file.display()))?;

    let workspace = WorkspacePaths::new(config.workspace.clone());

    // Write PID file
    let pid_path = paths::spawnbot_home().join("daemon.pid");
    std::fs::create_dir_all(paths::spawnbot_home())?;
    std::fs::write(&pid_path, std::process::id().to_string())?;

    tracing::info!(
        pid = std::process::id(),
        workspace = %workspace.root().display(),
        "Spawnbot daemon starting"
    );

    // --- Priority queue (shared across all producers and consumer) ---
    let queue = Arc::new(spawnbot_daemon::queue::PriorityQueue::new());
    let user_waiting = Arc::new(AtomicBool::new(false));

    // --- Spawn ACP client and initialize ---
    tracing::info!("Spawning goose ACP process");
    let mut acp = spawnbot_daemon::acp::AcpClient::spawn("goose", &["acp"])
        .await
        .with_context(|| "Failed to spawn goose ACP process")?;
    acp.initialize()
        .await
        .with_context(|| "Failed to initialize ACP connection")?;

    // --- Session manager ---
    let mut session_manager =
        spawnbot_daemon::session::SessionManager::new(acp, workspace.clone());
    session_manager
        .start()
        .await
        .with_context(|| "Failed to start session manager")?;

    let session = Arc::new(tokio::sync::Mutex::new(session_manager));

    // --- Queue consumer (processes events by sending prompts to the LLM) ---
    let consumer = spawnbot_daemon::consumer::QueueConsumer::new(
        queue.clone(),
        session.clone(),
        user_waiting.clone(),
    );
    tokio::spawn(async move {
        consumer.run().await;
    });

    // --- Cron scheduler ---
    let cron_jobs = spawnbot_daemon::autonomy::cron::load_crons(&workspace.crons_yaml())
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load cron jobs, continuing without them");
            vec![]
        });
    if !cron_jobs.is_empty() {
        let cron_scheduler =
            spawnbot_daemon::autonomy::cron::CronScheduler::new(cron_jobs, queue.clone())
                .await
                .with_context(|| "Failed to create cron scheduler")?;
        cron_scheduler
            .start()
            .await
            .with_context(|| "Failed to start cron scheduler")?;
        // Leak the scheduler handle so it lives for the daemon lifetime.
        // It will be cleaned up when the process exits.
        std::mem::forget(cron_scheduler);
    }

    // --- Idle loop ---
    let idle_loop = spawnbot_daemon::autonomy::idle::IdleLoop::new(
        queue.clone(),
        config.autonomy.clone(),
    );
    tokio::spawn(async move {
        idle_loop.run().await;
    });

    // --- Telegram bot (if enabled) ---
    if config.telegram.enabled {
        let telegram_bot = spawnbot_daemon::telegram::bot::TelegramBot::new(
            config.telegram.clone(),
            queue.clone(),
        );
        tokio::spawn(async move {
            if let Err(e) = telegram_bot.start().await {
                tracing::error!(error = %e, "Telegram bot failed");
            }
        });
    }

    // --- Daemon heartbeat writer ---
    let heartbeat_path = paths::spawnbot_home().join("daemon.heartbeat");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let _ = std::fs::write(&heartbeat_path, chrono::Utc::now().to_rfc3339());
        }
    });

    // --- Inbox cleanup ---
    let inbox_dir = workspace.inbox_dir();
    tokio::spawn(async move {
        spawnbot_daemon::inbox::start_inbox_cleanup(inbox_dir).await;
    });

    // --- First-run identity setup ---
    if needs_identity_setup(&workspace) {
        tracing::info!("First run detected — triggering identity build");
        let setup_prompt = spawnbot_daemon::autonomy::prompts::setup_prompt();
        let mut session_guard = session.lock().await;
        let _ = session_guard.prompt(&setup_prompt).await;
        drop(session_guard);
    }

    tracing::info!("Daemon started — press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::signal::ctrl_c()
        .await
        .with_context(|| "Failed to listen for Ctrl+C")?;

    tracing::info!("Shutdown signal received");

    // Cleanup PID file on shutdown
    let _ = std::fs::remove_file(&pid_path);

    tracing::info!("Spawnbot daemon stopped");
    Ok(())
}

fn stop_daemon() -> Result<()> {
    let pid_path = paths::spawnbot_home().join("daemon.pid");
    if !pid_path.exists() {
        println!("No running daemon found (no PID file)");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_path)
        .with_context(|| "Failed to read PID file")?;
    let pid: u32 = pid_str
        .trim()
        .parse()
        .with_context(|| "Invalid PID in daemon.pid")?;

    // Send SIGTERM on Unix
    #[cfg(unix)]
    {
        use std::process::Command;
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("Sent stop signal to daemon (PID {})", pid);
                let _ = std::fs::remove_file(&pid_path);
            }
            _ => {
                println!("Failed to stop daemon (PID {}) — process may not exist", pid);
                let _ = std::fs::remove_file(&pid_path);
            }
        }
    }

    #[cfg(not(unix))]
    {
        println!("Stop command is only supported on Unix systems (PID {})", pid);
    }

    Ok(())
}

fn needs_identity_setup(workspace: &WorkspacePaths) -> bool {
    match std::fs::read_to_string(workspace.user_md()) {
        Ok(content) => content.contains("(Run /setup to build your profile)"),
        Err(_) => true,
    }
}

fn show_status() -> Result<()> {
    let pid_path = paths::spawnbot_home().join("daemon.pid");
    if pid_path.exists() {
        let pid = std::fs::read_to_string(&pid_path)?;
        println!("Daemon PID: {}", pid.trim());

        let hb_path = paths::spawnbot_home().join("daemon.heartbeat");
        if hb_path.exists() {
            let hb = std::fs::read_to_string(&hb_path)?;
            println!("Last heartbeat: {}", hb.trim());
        }
    } else {
        println!("Daemon is not running");
    }
    Ok(())
}
