//! Spawnbot daemon CLI — process management and startup.

use anyhow::{Context, Result};
use clap::Parser;
use spawnbot_common::config::SpawnbotConfig;
use spawnbot_common::paths::{self, WorkspacePaths};

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

    // Spawn daemon heartbeat writer
    let heartbeat_path = paths::spawnbot_home().join("daemon.heartbeat");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let _ = std::fs::write(&heartbeat_path, chrono::Utc::now().to_rfc3339());
        }
    });

    // Spawn inbox cleanup
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
