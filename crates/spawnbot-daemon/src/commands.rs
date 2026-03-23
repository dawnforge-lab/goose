//! Slash commands — parsing and dispatch for daemon commands.

use spawnbot_common::config::SpawnbotConfig;
use spawnbot_common::paths::WorkspacePaths;

#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Status,
    Doctor,
    Config,
    Identity,
    Heartbeat,
    Changelog,
    Rotate,
    Restart,
    Wipe,
    Approve(String),
    Reject(String),
    Whisper,
    Unknown(String),
}

pub fn parse_command(input: &str) -> Option<Command> {
    let input = input.trim();
    if !input.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let arg = parts.get(1).map(|s| s.trim().to_string());

    Some(match cmd.as_str() {
        "help" => Command::Help,
        "status" => Command::Status,
        "doctor" => Command::Doctor,
        "config" => Command::Config,
        "identity" => Command::Identity,
        "heartbeat" => Command::Heartbeat,
        "changelog" => Command::Changelog,
        "rotate" => Command::Rotate,
        "restart" => Command::Restart,
        "wipe" => Command::Wipe,
        "approve" => Command::Approve(arg.unwrap_or_default()),
        "reject" => Command::Reject(arg.unwrap_or_default()),
        "whisper" => Command::Whisper,
        other => Command::Unknown(other.to_string()),
    })
}

pub fn help_text() -> String {
    [
        "Spawnbot Commands",
        "",
        "  /help       — Show this help",
        "  /status     — Current daemon status",
        "  /doctor     — Run diagnostics",
        "  /config     — Show current config",
        "  /identity   — Show identity documents",
        "  /heartbeat  — Show HEARTBEAT.md",
        "  /changelog  — Show recent changes",
        "  /rotate     — Force session rotation",
        "  /restart    — Restart goosed process",
        "  /wipe       — Factory reset (destructive!)",
        "  /approve ID — Approve a pending proposal",
        "  /reject ID  — Reject a pending proposal",
        "  /whisper    — Voice message (Whisper STT)",
    ]
    .join("\n")
}

/// Context for command handlers — provides access to config and workspace paths.
pub struct DaemonContext {
    pub config: SpawnbotConfig,
    pub workspace: WorkspacePaths,
}

/// Dispatch a parsed command to its handler and return the response string.
pub async fn handle_command(cmd: &Command, ctx: &DaemonContext) -> String {
    match cmd {
        Command::Help => help_text(),
        Command::Status => handle_status(ctx),
        Command::Doctor => handle_doctor(ctx),
        Command::Config => handle_config(ctx),
        Command::Identity => handle_identity(ctx),
        Command::Heartbeat => handle_heartbeat(ctx),
        Command::Changelog => handle_changelog(ctx),
        _ => format!("Command {:?} — not yet implemented", cmd),
    }
}

fn handle_status(ctx: &DaemonContext) -> String {
    let ws = &ctx.workspace;
    let mut lines = vec!["Spawnbot Status".to_string(), String::new()];
    lines.push(format!("  Workspace:  {}", ws.root().display()));
    lines.push(format!("  Provider:   {:?}", ctx.config.llm.provider));
    lines.push(format!("  Model:      {}", ctx.config.llm.model));
    lines.push(format!("  Autonomy:   {:?}", ctx.config.autonomy.mode));
    lines.push(format!(
        "  Telegram:   {}",
        if ctx.config.telegram.enabled {
            "enabled"
        } else {
            "disabled"
        }
    ));
    lines.join("\n")
}

fn handle_doctor(ctx: &DaemonContext) -> String {
    let ws = &ctx.workspace;
    let mut lines = vec!["Diagnostics".to_string(), String::new()];
    lines.push(format!(
        "  Config:       {}",
        if spawnbot_common::paths::config_path().exists() {
            "OK"
        } else {
            "MISSING"
        }
    ));
    lines.push(format!(
        "  Workspace:    {}",
        if ws.root().exists() { "OK" } else { "MISSING" }
    ));
    lines.push(format!(
        "  SOUL.md:      {}",
        if ws.soul_md().exists() {
            "OK"
        } else {
            "MISSING"
        }
    ));
    lines.push(format!(
        "  Memory DB:    {}",
        if ws.memory_db().exists() {
            "OK"
        } else {
            "MISSING"
        }
    ));
    lines.push(format!(
        "  HEARTBEAT.md: {}",
        if ws.heartbeat_md().exists() {
            "OK"
        } else {
            "MISSING"
        }
    ));

    // Check error log
    let error_log = ws.error_log();
    let recent_errors = crate::error_log::ErrorLog::recent(&error_log, 5).unwrap_or_default();
    if recent_errors.is_empty() {
        lines.push("  Errors:       none".to_string());
    } else {
        lines.push(format!("  Errors:       {} recent", recent_errors.len()));
        for err in &recent_errors {
            lines.push(format!("    {}", err));
        }
    }

    // Check daemon heartbeat
    let hb_path = spawnbot_common::paths::spawnbot_home().join("daemon.heartbeat");
    if hb_path.exists() {
        let content = std::fs::read_to_string(&hb_path).unwrap_or_default();
        lines.push(format!("  Heartbeat:    {}", content.trim()));
    } else {
        lines.push("  Heartbeat:    no data".to_string());
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
        "SOUL.md",
        "USER.md",
        "GOALS.md",
        "PLAYBOOK.md",
        "HEARTBEAT.md",
    ];
    let paths = [
        ws.soul_md(),
        ws.user_md(),
        ws.goals_md(),
        ws.playbook_md(),
        ws.heartbeat_md(),
    ];
    let mut lines = vec!["Identity Documents".to_string(), String::new()];
    for (name, path) in docs.iter().zip(paths.iter()) {
        let status = if path.exists() {
            let meta = std::fs::metadata(path).ok();
            let size = meta.map(|m| m.len()).unwrap_or(0);
            format!("OK ({} bytes)", size)
        } else {
            "MISSING".to_string()
        };
        lines.push(format!("  {}: {}", name, status));
    }
    lines.join("\n")
}

fn handle_heartbeat(ctx: &DaemonContext) -> String {
    match std::fs::read_to_string(ctx.workspace.heartbeat_md()) {
        Ok(content) => content,
        Err(_) => "HEARTBEAT.md not found".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_command("/help"), Some(Command::Help));
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(parse_command("/status"), Some(Command::Status));
    }

    #[test]
    fn test_parse_doctor() {
        assert_eq!(parse_command("/doctor"), Some(Command::Doctor));
    }

    #[test]
    fn test_parse_approve_with_id() {
        assert_eq!(
            parse_command("/approve abc123"),
            Some(Command::Approve("abc123".to_string()))
        );
    }

    #[test]
    fn test_parse_reject_with_id() {
        assert_eq!(
            parse_command("/reject xyz789"),
            Some(Command::Reject("xyz789".to_string()))
        );
    }

    #[test]
    fn test_parse_unknown_command() {
        assert_eq!(
            parse_command("/foobar"),
            Some(Command::Unknown("foobar".to_string()))
        );
    }

    #[test]
    fn test_not_a_command() {
        assert_eq!(parse_command("hello world"), None);
    }

    #[test]
    fn test_help_text_not_empty() {
        let text = help_text();
        assert!(text.contains("/help"));
        assert!(text.contains("/status"));
        assert!(text.contains("/doctor"));
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(parse_command("/HELP"), Some(Command::Help));
        assert_eq!(parse_command("/Status"), Some(Command::Status));
    }

    #[test]
    fn test_parse_with_leading_whitespace() {
        assert_eq!(parse_command("  /help"), Some(Command::Help));
    }
}
