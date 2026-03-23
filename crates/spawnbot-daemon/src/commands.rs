/// Subcommands for the /memory command.
#[derive(Debug, PartialEq)]
pub enum MemorySubcommand {
    Stats,
    Search(String),
    Reindex,
}

/// All supported slash commands for interactive control.
#[derive(Debug, PartialEq)]
pub enum Command {
    Reset,
    Clear,
    Nuke,
    Doctor,
    Memory(MemorySubcommand),
    Status,
    Skills,
    Identity,
    Crons,
    Pollers,
    Heartbeat,
    Changelog,
    Config,
    Help,
}

/// Parse a slash command from user input.
///
/// Returns `None` if the input is not a recognized command.
pub fn parse_command(input: &str) -> Option<Command> {
    let input = input.trim();
    if !input.starts_with('/') {
        return None;
    }
    let parts: Vec<&str> = input[1..].splitn(3, ' ').collect();
    match parts.first().map(|s| s.to_lowercase()).as_deref() {
        Some("reset") => Some(Command::Reset),
        Some("clear") => Some(Command::Clear),
        Some("nuke") => Some(Command::Nuke),
        Some("doctor") => Some(Command::Doctor),
        Some("status") => Some(Command::Status),
        Some("skills") => Some(Command::Skills),
        Some("identity") => Some(Command::Identity),
        Some("crons") => Some(Command::Crons),
        Some("pollers") => Some(Command::Pollers),
        Some("heartbeat") => Some(Command::Heartbeat),
        Some("changelog") => Some(Command::Changelog),
        Some("config") => Some(Command::Config),
        Some("help") => Some(Command::Help),
        Some("memory") => match parts.get(1).map(|s| s.to_lowercase()).as_deref() {
            Some("stats") | None => Some(Command::Memory(MemorySubcommand::Stats)),
            Some("reindex") => Some(Command::Memory(MemorySubcommand::Reindex)),
            Some("search") => {
                let query = parts.get(2).unwrap_or(&"").to_string();
                Some(Command::Memory(MemorySubcommand::Search(query)))
            }
            _ => Some(Command::Memory(MemorySubcommand::Stats)),
        },
        _ => None,
    }
}

/// Generate the help text listing all available commands.
pub fn help_text() -> String {
    r#"Available commands:
  /status     — Show daemon status
  /memory     — Memory stats, search, reindex
  /skills     — List skills
  /identity   — Show identity documents
  /heartbeat  — Show current tasks
  /crons      — List scheduled jobs
  /pollers    — List pollers
  /changelog  — Recent changes
  /config     — Show configuration
  /doctor     — Run diagnostics
  /reset      — Reset current session
  /clear      — Clear session history
  /nuke       — Factory reset (requires confirmation)
  /help       — Show this help"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_commands() {
        assert_eq!(parse_command("/reset"), Some(Command::Reset));
        assert_eq!(parse_command("/clear"), Some(Command::Clear));
        assert_eq!(parse_command("/nuke"), Some(Command::Nuke));
        assert_eq!(parse_command("/doctor"), Some(Command::Doctor));
        assert_eq!(parse_command("/status"), Some(Command::Status));
        assert_eq!(parse_command("/skills"), Some(Command::Skills));
        assert_eq!(parse_command("/identity"), Some(Command::Identity));
        assert_eq!(parse_command("/crons"), Some(Command::Crons));
        assert_eq!(parse_command("/pollers"), Some(Command::Pollers));
        assert_eq!(parse_command("/heartbeat"), Some(Command::Heartbeat));
        assert_eq!(parse_command("/changelog"), Some(Command::Changelog));
        assert_eq!(parse_command("/config"), Some(Command::Config));
        assert_eq!(parse_command("/help"), Some(Command::Help));
    }

    #[test]
    fn test_parse_memory_subcommands() {
        assert_eq!(
            parse_command("/memory"),
            Some(Command::Memory(MemorySubcommand::Stats))
        );
        assert_eq!(
            parse_command("/memory stats"),
            Some(Command::Memory(MemorySubcommand::Stats))
        );
        assert_eq!(
            parse_command("/memory reindex"),
            Some(Command::Memory(MemorySubcommand::Reindex))
        );
        assert_eq!(
            parse_command("/memory search hello world"),
            Some(Command::Memory(MemorySubcommand::Search(
                "hello world".into()
            )))
        );
    }

    #[test]
    fn test_not_a_command() {
        assert_eq!(parse_command("hello"), None);
        assert_eq!(parse_command(""), None);
        assert_eq!(parse_command("not a /command"), None);
    }

    #[test]
    fn test_unknown_command() {
        assert_eq!(parse_command("/unknown"), None);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(parse_command("/RESET"), Some(Command::Reset));
        assert_eq!(parse_command("/Status"), Some(Command::Status));
        assert_eq!(parse_command("/MEMORY reindex"), Some(Command::Memory(MemorySubcommand::Reindex)));
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(parse_command("  /reset  "), Some(Command::Reset));
    }
}
