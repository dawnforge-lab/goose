//! Slash commands — parsing and dispatch for daemon commands.

use spawnbot_common::config::SpawnbotConfig;
use spawnbot_common::paths::WorkspacePaths;

/// Actions that commands can request from the daemon loop.
/// The command handler returns text + optionally an action for the caller to execute.
#[derive(Debug, PartialEq)]
pub enum CommandAction {
    None,
    ResetSession,
    NewSession,
    RotateSession,
    RestartGoose,
    StopDaemon,
    ClearMemory,
    NukeConfirmation,
    NukeExecute,
    SwitchMode(String),
    RunSetup,
}

#[derive(Debug, PartialEq)]
pub enum MemorySubcommand {
    Stats,
    Search(String),
    Reindex,
    Recent,
    Clear,
}

#[derive(Debug, PartialEq)]
pub enum SkillSubcommand {
    List,
    Show(String),
    Enable(String),
    Disable(String),
}

#[derive(Debug, PartialEq)]
pub enum GoalSubcommand {
    List,
    Add(String),
    Done(String),
}

#[derive(Debug, PartialEq)]
pub enum TaskSubcommand {
    List,
    Add(String),
    Start(String),
    Done(String),
}

#[derive(Debug, PartialEq)]
pub enum CronSubcommand {
    List,
    Enable(String),
    Disable(String),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    // Info
    Help,
    Status,
    Doctor,
    Config,

    // Identity
    Setup,
    Identity,
    Soul,
    WhoAmI,

    // Tasks & Goals
    Heartbeat,
    Tasks(TaskSubcommand),
    Goals(GoalSubcommand),

    // Memory
    Memory(MemorySubcommand),
    Remember(String),
    Forget(String),
    Recall(String),

    // Skills & Extensions
    Skills(SkillSubcommand),

    // Autonomy
    Crons(CronSubcommand),
    Pollers,
    Mode(Option<String>),

    // Logs
    Changelog,
    Errors,

    // Session management
    New,
    Reset,
    Rotate,
    Restart,
    Stop,

    // Approval
    Approve(String),
    Reject(String),
    Pending,

    // Destructive
    Nuke,
    NukeConfirm(String),

    // Misc
    Whisper,
    Ping,
    Version,
    Unknown(String),
}

pub fn parse_command(input: &str) -> Option<Command> {
    let input = input.trim();
    if !input.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = input[1..].splitn(3, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let arg1 = parts.get(1).map(|s| s.trim().to_string());
    let arg2 = parts.get(2).map(|s| s.trim().to_string());
    // For commands that take freeform text, join everything after the command
    let rest = if parts.len() > 1 {
        Some(parts[1..].join(" ").trim().to_string())
    } else {
        None
    };

    Some(match cmd.as_str() {
        // Info
        "help" | "h" | "?" => Command::Help,
        "status" | "s" => Command::Status,
        "doctor" | "doc" => Command::Doctor,
        "config" | "cfg" => Command::Config,
        "ping" => Command::Ping,
        "version" | "v" => Command::Version,

        // Identity
        "setup" => Command::Setup,
        "identity" | "id" => Command::Identity,
        "soul" => Command::Soul,
        "whoami" | "who" => Command::WhoAmI,

        // Tasks & Goals
        "heartbeat" | "hb" => Command::Heartbeat,
        "task" | "tasks" | "t" => match arg1.as_deref() {
            Some("add") => Command::Tasks(TaskSubcommand::Add(arg2.unwrap_or_default())),
            Some("start") => Command::Tasks(TaskSubcommand::Start(arg2.unwrap_or_default())),
            Some("done") => Command::Tasks(TaskSubcommand::Done(arg2.unwrap_or_default())),
            _ => Command::Tasks(TaskSubcommand::List),
        },
        "goal" | "goals" | "g" => match arg1.as_deref() {
            Some("add") => Command::Goals(GoalSubcommand::Add(arg2.unwrap_or_default())),
            Some("done") => Command::Goals(GoalSubcommand::Done(arg2.unwrap_or_default())),
            _ => Command::Goals(GoalSubcommand::List),
        },

        // Memory
        "memory" | "mem" | "m" => match arg1.as_deref() {
            Some("search") | Some("find") => {
                Command::Memory(MemorySubcommand::Search(arg2.unwrap_or_default()))
            }
            Some("reindex") => Command::Memory(MemorySubcommand::Reindex),
            Some("recent") => Command::Memory(MemorySubcommand::Recent),
            Some("clear") | Some("wipe") => Command::Memory(MemorySubcommand::Clear),
            _ => Command::Memory(MemorySubcommand::Stats),
        },
        "remember" | "rem" => Command::Remember(rest.unwrap_or_default()),
        "forget" => Command::Forget(arg1.unwrap_or_default()),
        "recall" | "r" => Command::Recall(rest.unwrap_or_default()),

        // Skills
        "skill" | "skills" => match arg1.as_deref() {
            Some("show") => Command::Skills(SkillSubcommand::Show(arg2.unwrap_or_default())),
            Some("enable") => Command::Skills(SkillSubcommand::Enable(arg2.unwrap_or_default())),
            Some("disable") => {
                Command::Skills(SkillSubcommand::Disable(arg2.unwrap_or_default()))
            }
            _ => Command::Skills(SkillSubcommand::List),
        },

        // Autonomy
        "cron" | "crons" => match arg1.as_deref() {
            Some("enable") => Command::Crons(CronSubcommand::Enable(arg2.unwrap_or_default())),
            Some("disable") => Command::Crons(CronSubcommand::Disable(arg2.unwrap_or_default())),
            _ => Command::Crons(CronSubcommand::List),
        },
        "pollers" | "poller" => Command::Pollers,
        "mode" => Command::Mode(arg1),

        // Logs
        "changelog" | "changes" | "log" => Command::Changelog,
        "errors" | "err" => Command::Errors,

        // Session management
        "new" => Command::New,
        "reset" => Command::Reset,
        "rotate" => Command::Rotate,
        "restart" => Command::Restart,
        "stop" | "quit" | "exit" => Command::Stop,

        // Approval
        "approve" | "yes" | "y" => Command::Approve(arg1.unwrap_or_default()),
        "reject" | "no" | "n" => Command::Reject(arg1.unwrap_or_default()),
        "pending" | "proposals" => Command::Pending,

        // Destructive
        "nuke" => match arg1.as_deref() {
            Some(confirm) if confirm.to_uppercase() == "NUKE" => {
                Command::NukeConfirm("NUKE".to_string())
            }
            _ => Command::Nuke,
        },

        // Misc
        "whisper" => Command::Whisper,

        other => Command::Unknown(other.to_string()),
    })
}

pub fn help_text() -> String {
    [
        "Spawnbot Commands",
        "",
        "Basics",
        "  /status           Current status",
        "  /ping             Check if alive",
        "  /help             This help",
        "",
        "Tasks & Goals",
        "  /tasks            Show task board",
        "  /task add <text>  Add a task",
        "  /task done <text> Mark task complete",
        "  /goals            Show goals",
        "  /goal add <text>  Add a goal",
        "",
        "Memory",
        "  /recall <query>   Search memories",
        "  /remember <text>  Store a memory",
        "  /forget <id>      Delete a memory",
        "  /memory           Memory stats",
        "  /memory clear     Wipe all memories (confirmation)",
        "  /memory reindex   Re-index markdown files",
        "",
        "Identity",
        "  /setup            Guided identity build",
        "  /soul             Show personality",
        "  /whoami           Show user profile",
        "  /identity         All identity docs",
        "",
        "Skills & Autonomy",
        "  /skills           List skills",
        "  /crons            Scheduled tasks",
        "  /pollers          RSS/webhook watchers",
        "  /mode             Current mode (yolo/approval)",
        "  /mode yolo        Switch to autonomous",
        "  /mode approval    Switch to ask-first",
        "",
        "Approval mode",
        "  /pending          Pending proposals",
        "  /y <id>           Approve",
        "  /n <id>           Reject",
        "",
        "Session",
        "  /new              Start a fresh session",
        "  /reset            Flush session, keep memory",
        "  /rotate           Summarize + new session",
        "  /restart          Restart Goose process",
        "  /stop             Stop the daemon",
        "",
        "System",
        "  /doctor           Diagnostics",
        "  /errors           Recent errors",
        "  /changelog        Recent changes",
        "  /config           Show config",
        "  /nuke             Full wipe (/nuke NUKE to confirm)",
    ]
    .join("\n")
}

/// Context for command handlers — provides access to config and workspace paths.
pub struct DaemonContext {
    pub config: SpawnbotConfig,
    pub workspace: WorkspacePaths,
}

/// Dispatch a parsed command. Returns response text + an optional action for the daemon to execute.
pub async fn handle_command(cmd: &Command, ctx: &DaemonContext) -> (String, CommandAction) {
    match cmd {
        // Info
        Command::Help => (help_text(), CommandAction::None),
        Command::Ping => ("pong".to_string(), CommandAction::None),
        Command::Version => (
            format!("Spawnbot v{}", env!("CARGO_PKG_VERSION")),
            CommandAction::None,
        ),
        Command::Status => (handle_status(ctx), CommandAction::None),
        Command::Doctor => (handle_doctor(ctx), CommandAction::None),
        Command::Config => (handle_config(ctx), CommandAction::None),

        // Identity
        Command::Setup => (
            "Starting identity build...".to_string(),
            CommandAction::RunSetup,
        ),
        Command::Identity => (handle_identity(ctx), CommandAction::None),
        Command::Soul => (handle_soul(ctx), CommandAction::None),
        Command::WhoAmI => (handle_whoami(ctx), CommandAction::None),

        // Tasks & Goals
        Command::Heartbeat | Command::Tasks(TaskSubcommand::List) => {
            (handle_heartbeat(ctx), CommandAction::None)
        }
        Command::Goals(GoalSubcommand::List) => (handle_goals(ctx), CommandAction::None),

        // Memory
        Command::Memory(MemorySubcommand::Stats) => (handle_memory_stats(ctx), CommandAction::None),
        Command::Memory(MemorySubcommand::Clear) => (
            "Are you sure? This deletes ALL memories from the database.\nSend /memory clear again to confirm."
                .to_string(),
            CommandAction::ClearMemory,
        ),

        // Skills & Autonomy
        Command::Skills(SkillSubcommand::List) => (handle_skills(ctx), CommandAction::None),
        Command::Crons(CronSubcommand::List) => (handle_crons(ctx), CommandAction::None),
        Command::Pollers => (handle_pollers(ctx), CommandAction::None),

        // Logs
        Command::Changelog => (handle_changelog(ctx), CommandAction::None),
        Command::Errors => (handle_errors(ctx), CommandAction::None),

        // Mode
        Command::Mode(None) => (
            format!("Current mode: {:?}", ctx.config.autonomy.mode),
            CommandAction::None,
        ),
        Command::Mode(Some(mode)) => (
            format!("Switching to {} mode", mode),
            CommandAction::SwitchMode(mode.clone()),
        ),

        // Approval
        Command::Pending => ("No pending proposals".to_string(), CommandAction::None),

        // Session
        Command::New => (
            "Starting fresh session...".to_string(),
            CommandAction::NewSession,
        ),
        Command::Reset => (
            "Resetting session (memory and identity preserved)...".to_string(),
            CommandAction::ResetSession,
        ),
        Command::Rotate => (
            "Rotating session (summarizing first)...".to_string(),
            CommandAction::RotateSession,
        ),
        Command::Restart => (
            "Restarting Goose...".to_string(),
            CommandAction::RestartGoose,
        ),
        Command::Stop => (
            "Stopping Spawnbot daemon...".to_string(),
            CommandAction::StopDaemon,
        ),

        // Destructive
        Command::Nuke => (
            "This will DELETE everything — config, memory, skills, identity.\n\
             Type /nuke NUKE to confirm."
                .to_string(),
            CommandAction::NukeConfirmation,
        ),
        Command::NukeConfirm(_) => (
            "Nuking everything... Goodbye.".to_string(),
            CommandAction::NukeExecute,
        ),

        _ => (format!("{:?} — handler not wired yet", cmd), CommandAction::None),
    }
}

fn handle_status(ctx: &DaemonContext) -> String {
    let ws = &ctx.workspace;
    let mut lines = vec!["Spawnbot Status".to_string(), String::new()];
    lines.push(format!("  Workspace:  {}", ws.root().display()));

    // Read provider/model from Goose's own config
    let goose_config_path = dirs::config_dir()
        .expect("Failed to determine config directory")
        .join("goose/config.yaml");
    if goose_config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&goose_config_path) {
            if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(provider) = val.get("GOOSE_PROVIDER").and_then(|v| v.as_str()) {
                    lines.push(format!("  Provider:   {}", provider));
                }
                if let Some(model) = val.get("GOOSE_MODEL").and_then(|v| v.as_str()) {
                    lines.push(format!("  Model:      {}", model));
                }
            }
        }
    }

    lines.push(format!("  Autonomy:   {:?}", ctx.config.autonomy.mode));
    lines.push(format!(
        "  Telegram:   {}",
        if ctx.config.telegram.enabled {
            "enabled"
        } else {
            "disabled"
        }
    ));

    // Uptime from daemon heartbeat
    let hb_path = spawnbot_common::paths::spawnbot_home().join("daemon.heartbeat");
    if hb_path.exists() {
        lines.push(format!(
            "  Heartbeat:  {}",
            std::fs::read_to_string(&hb_path)
                .unwrap_or_default()
                .trim()
                .to_string()
        ));
    }

    // Pending tasks
    let heartbeat = std::fs::read_to_string(ws.heartbeat_md()).unwrap_or_default();
    let pending = heartbeat.lines().filter(|l| l.trim().starts_with("- [ ]")).count();
    let ongoing = heartbeat.lines().filter(|l| l.trim().starts_with("- [~]")).count();
    if pending > 0 || ongoing > 0 {
        lines.push(format!("  Tasks:      {} pending, {} ongoing", pending, ongoing));
    }

    lines.join("\n")
}

fn handle_doctor(ctx: &DaemonContext) -> String {
    let ws = &ctx.workspace;
    let mut lines = vec!["Diagnostics".to_string(), String::new()];

    let checks: Vec<(&str, bool)> = vec![
        ("Config", spawnbot_common::paths::config_path().exists()),
        ("Workspace", ws.root().exists()),
        ("SOUL.md", ws.soul_md().exists()),
        ("USER.md", ws.user_md().exists()),
        ("GOALS.md", ws.goals_md().exists()),
        ("PLAYBOOK.md", ws.playbook_md().exists()),
        ("HEARTBEAT.md", ws.heartbeat_md().exists()),
        ("Memory DB", ws.memory_db().exists()),
        ("CRONS.yaml", ws.crons_yaml().exists()),
    ];

    for (name, ok) in &checks {
        lines.push(format!(
            "  {:<14} {}",
            format!("{}:", name),
            if *ok { "OK" } else { "MISSING" }
        ));
    }

    // Error log
    let recent_errors = crate::error_log::ErrorLog::recent(&ws.error_log(), 5).unwrap_or_default();
    if recent_errors.is_empty() {
        lines.push("  Errors:        none".to_string());
    } else {
        lines.push(format!("  Errors:        {} recent", recent_errors.len()));
    }

    // Daemon heartbeat
    let hb_path = spawnbot_common::paths::spawnbot_home().join("daemon.heartbeat");
    if hb_path.exists() {
        let content = std::fs::read_to_string(&hb_path).unwrap_or_default();
        lines.push(format!("  Heartbeat:     {}", content.trim()));
    } else {
        lines.push("  Heartbeat:     not running".to_string());
    }

    lines.join("\n")
}

fn handle_config(ctx: &DaemonContext) -> String {
    serde_yaml::to_string(&ctx.config)
        .unwrap_or_else(|_| "Failed to serialize config".to_string())
}

fn handle_identity(ctx: &DaemonContext) -> String {
    let ws = &ctx.workspace;
    let docs = [
        ("SOUL.md", ws.soul_md()),
        ("USER.md", ws.user_md()),
        ("GOALS.md", ws.goals_md()),
        ("PLAYBOOK.md", ws.playbook_md()),
        ("HEARTBEAT.md", ws.heartbeat_md()),
    ];
    let mut lines = vec!["Identity Documents".to_string(), String::new()];
    for (name, path) in &docs {
        let status = if path.exists() {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            format!("OK ({} bytes)", size)
        } else {
            "MISSING".to_string()
        };
        lines.push(format!("  {:<14} {}", name, status));
    }
    lines.join("\n")
}

fn handle_soul(ctx: &DaemonContext) -> String {
    std::fs::read_to_string(ctx.workspace.soul_md()).unwrap_or_else(|_| "SOUL.md not found".into())
}

fn handle_whoami(ctx: &DaemonContext) -> String {
    std::fs::read_to_string(ctx.workspace.user_md()).unwrap_or_else(|_| "USER.md not found".into())
}

fn handle_heartbeat(ctx: &DaemonContext) -> String {
    std::fs::read_to_string(ctx.workspace.heartbeat_md())
        .unwrap_or_else(|_| "HEARTBEAT.md not found".into())
}

fn handle_goals(ctx: &DaemonContext) -> String {
    std::fs::read_to_string(ctx.workspace.goals_md())
        .unwrap_or_else(|_| "GOALS.md not found".into())
}

fn handle_memory_stats(ctx: &DaemonContext) -> String {
    let db_path = ctx.workspace.memory_db();
    if !db_path.exists() {
        return "Memory DB not found".to_string();
    }
    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let memory_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM memory", [], |row| row.get(0))
                .unwrap_or(0);
            let chunk_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM memory_chunks", [], |row| row.get(0))
                .unwrap_or(0);
            format!(
                "Memory Stats\n\n  Memories:  {}\n  Chunks:    {}\n  DB size:   {} KB",
                memory_count,
                chunk_count,
                std::fs::metadata(&db_path)
                    .map(|m| m.len() / 1024)
                    .unwrap_or(0)
            )
        }
        Err(e) => format!("Failed to open memory DB: {}", e),
    }
}

fn handle_skills(ctx: &DaemonContext) -> String {
    let skills_dir = spawnbot_common::paths::skills_dir();
    match spawnbot_skills::skills::list_skills(&skills_dir) {
        Ok(skills) if skills.is_empty() => "No skills defined yet".to_string(),
        Ok(skills) => {
            let mut lines = vec![format!("Skills ({})", skills.len()), String::new()];
            for skill in &skills {
                let status = if skill.enabled { "on" } else { "off" };
                lines.push(format!("  [{}] {} — {}", status, skill.name, skill.description));
            }
            lines.join("\n")
        }
        Err(e) => format!("Error listing skills: {}", e),
    }
}

fn handle_crons(ctx: &DaemonContext) -> String {
    let path = ctx.workspace.crons_yaml();
    match crate::autonomy::cron::load_crons(&path) {
        Ok(crons) if crons.is_empty() => "No cron jobs defined".to_string(),
        Ok(crons) => {
            let mut lines = vec![format!("Cron Jobs ({})", crons.len()), String::new()];
            for cron in &crons {
                let status = if cron.enabled { "on" } else { "off" };
                lines.push(format!("  [{}] {} — {}", status, cron.name, cron.cron));
            }
            lines.join("\n")
        }
        Err(e) => format!("Error loading crons: {}", e),
    }
}

fn handle_pollers(ctx: &DaemonContext) -> String {
    let path = ctx.workspace.pollers_yaml();
    match crate::autonomy::poller::load_pollers(&path) {
        Ok(pollers) if pollers.is_empty() => "No pollers defined".to_string(),
        Ok(pollers) => {
            let mut lines = vec![format!("Pollers ({})", pollers.len()), String::new()];
            for poller in &pollers {
                let status = if poller.enabled { "on" } else { "off" };
                lines.push(format!(
                    "  [{}] {} — {} ({}s)",
                    status, poller.name, poller.source, poller.interval
                ));
            }
            lines.join("\n")
        }
        Err(e) => format!("Error loading pollers: {}", e),
    }
}

fn handle_changelog(_ctx: &DaemonContext) -> String {
    let log_path = spawnbot_common::paths::changes_log_path();
    match spawnbot_common::changes_log::ChangesLog::recent(&log_path, 20) {
        Ok(entries) if entries.is_empty() => "No changes recorded".to_string(),
        Ok(entries) => entries.join("\n"),
        Err(e) => format!("Error reading changelog: {}", e),
    }
}

fn handle_errors(ctx: &DaemonContext) -> String {
    let recent = crate::error_log::ErrorLog::recent(&ctx.workspace.error_log(), 20)
        .unwrap_or_default();
    if recent.is_empty() {
        "No recent errors".to_string()
    } else {
        let mut lines = vec![format!("Recent Errors ({})", recent.len()), String::new()];
        lines.extend(recent);
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help_variants() {
        assert_eq!(parse_command("/help"), Some(Command::Help));
        assert_eq!(parse_command("/h"), Some(Command::Help));
        assert_eq!(parse_command("/?"), Some(Command::Help));
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(parse_command("/status"), Some(Command::Status));
        assert_eq!(parse_command("/s"), Some(Command::Status));
    }

    #[test]
    fn test_parse_task_subcommands() {
        assert_eq!(
            parse_command("/tasks"),
            Some(Command::Tasks(TaskSubcommand::List))
        );
        assert_eq!(
            parse_command("/task add Buy groceries"),
            Some(Command::Tasks(TaskSubcommand::Add(
                "Buy groceries".to_string()
            )))
        );
        assert_eq!(
            parse_command("/task done Buy groceries"),
            Some(Command::Tasks(TaskSubcommand::Done(
                "Buy groceries".to_string()
            )))
        );
    }

    #[test]
    fn test_parse_goal_subcommands() {
        assert_eq!(
            parse_command("/goals"),
            Some(Command::Goals(GoalSubcommand::List))
        );
        assert_eq!(
            parse_command("/goal add Learn Rust"),
            Some(Command::Goals(GoalSubcommand::Add("Learn Rust".to_string())))
        );
    }

    #[test]
    fn test_parse_memory_subcommands() {
        assert_eq!(
            parse_command("/memory"),
            Some(Command::Memory(MemorySubcommand::Stats))
        );
        assert_eq!(
            parse_command("/memory search cats"),
            Some(Command::Memory(MemorySubcommand::Search("cats".to_string())))
        );
        assert_eq!(
            parse_command("/memory reindex"),
            Some(Command::Memory(MemorySubcommand::Reindex))
        );
        assert_eq!(
            parse_command("/mem find dogs"),
            Some(Command::Memory(MemorySubcommand::Search("dogs".to_string())))
        );
    }

    #[test]
    fn test_parse_shorthand_memory() {
        assert_eq!(
            parse_command("/recall what did I say about cats"),
            Some(Command::Recall(
                "what did I say about cats".to_string()
            ))
        );
        assert_eq!(
            parse_command("/remember Alice prefers tea"),
            Some(Command::Remember("Alice prefers tea".to_string()))
        );
        assert_eq!(
            parse_command("/forget abc123"),
            Some(Command::Forget("abc123".to_string()))
        );
    }

    #[test]
    fn test_parse_approval_shortcuts() {
        assert_eq!(
            parse_command("/y abc"),
            Some(Command::Approve("abc".to_string()))
        );
        assert_eq!(
            parse_command("/n abc"),
            Some(Command::Reject("abc".to_string()))
        );
        assert_eq!(parse_command("/pending"), Some(Command::Pending));
    }

    #[test]
    fn test_parse_mode() {
        assert_eq!(parse_command("/mode"), Some(Command::Mode(None)));
        assert_eq!(
            parse_command("/mode yolo"),
            Some(Command::Mode(Some("yolo".to_string())))
        );
    }

    #[test]
    fn test_parse_setup() {
        assert_eq!(parse_command("/setup"), Some(Command::Setup));
    }

    #[test]
    fn test_parse_identity_shortcuts() {
        assert_eq!(parse_command("/soul"), Some(Command::Soul));
        assert_eq!(parse_command("/whoami"), Some(Command::WhoAmI));
        assert_eq!(parse_command("/who"), Some(Command::WhoAmI));
    }

    #[test]
    fn test_parse_misc() {
        assert_eq!(parse_command("/ping"), Some(Command::Ping));
        assert_eq!(parse_command("/version"), Some(Command::Version));
        assert_eq!(parse_command("/v"), Some(Command::Version));
    }

    #[test]
    fn test_not_a_command() {
        assert_eq!(parse_command("hello world"), None);
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(parse_command("/HELP"), Some(Command::Help));
        assert_eq!(parse_command("/Status"), Some(Command::Status));
    }

    #[test]
    fn test_help_text_sections() {
        let text = help_text();
        assert!(text.contains("Basics"));
        assert!(text.contains("Tasks & Goals"));
        assert!(text.contains("Memory"));
        assert!(text.contains("Identity"));
        assert!(text.contains("Skills"));
        assert!(text.contains("Autonomy"));
        assert!(text.contains("/recall"));
        assert!(text.contains("/remember"));
        assert!(text.contains("/task add"));
        assert!(text.contains("/goal add"));
    }
}
