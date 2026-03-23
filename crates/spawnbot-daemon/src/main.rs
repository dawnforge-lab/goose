//! Spawnbot daemon CLI — process management and startup.

use anyhow::{Context, Result};
use clap::Parser;
use spawnbot_common::config::SpawnbotConfig;
use spawnbot_common::paths::{self, WorkspacePaths};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "spawnbot", about = "Spawnbot daemon")]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Parser, Debug)]
enum CliCommand {
    /// Start the daemon
    Start {
        /// Path to config file (default: ~/.spawnbot/config.yaml)
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Start { config } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(start_daemon(config))?;
        }
        CliCommand::Stop => {
            stop_daemon()?;
        }
        CliCommand::Status => {
            show_status()?;
        }
    }

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
