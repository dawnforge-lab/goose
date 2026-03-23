/// System prompt for heartbeat checks — inspect HEARTBEAT.md for pending tasks.
pub fn heartbeat_prompt() -> String {
    "[SYSTEM:HEARTBEAT] Check HEARTBEAT.md for pending or ongoing tasks. \
     If there are actionable items, work on the highest priority one. \
     If all tasks are complete, review GOALS.md for next steps. \
     Store any learnings from this check."
        .to_string()
}

/// System prompt for cron-triggered events.
pub fn cron_prompt(cron_name: &str, user_prompt: &str) -> String {
    format!("[SYSTEM:CRON:{}] {}", cron_name, user_prompt)
}

/// System prompt for poller-detected events (RSS, webhooks, etc.).
pub fn poller_prompt(poller_name: &str, content: &str) -> String {
    format!(
        "[SYSTEM:POLLER:{}] New event detected:\n{}",
        poller_name, content
    )
}

/// System prompt sent before a session is rotated.
pub fn session_rotation_prompt() -> String {
    "[SYSTEM:SESSION_FLUSH] This session is being rotated. \
     Store all important memories from this session before it ends. \
     Include key decisions, learnings, and action items."
        .to_string()
}

/// System prompt sent at the start of a new session after rotation.
pub fn session_reset_prompt(summary: &str) -> String {
    format!(
        "[SYSTEM:SESSION_RESET] Previous session summary:\n{}\n\n\
         Recall relevant memories and continue where you left off.",
        summary
    )
}

/// Tier 1 idle prompt — basic heartbeat check after initial idle period.
pub fn idle_base_prompt() -> String {
    "[SYSTEM:HEARTBEAT] You've been idle for a while. \
     Check HEARTBEAT.md for pending tasks. \
     Review recent memories for anything that needs follow-up."
        .to_string()
}

/// Tier 2 idle prompt — escalated check after extended idle period.
pub fn idle_escalation_prompt() -> String {
    "[SYSTEM:HEARTBEAT] Extended idle period. \
     Review GOALS.md for strategic objectives. \
     Consider proactive actions that would help your user."
        .to_string()
}

/// Tier 3 idle prompt — warning level, triggers memory consolidation.
pub fn idle_warning_prompt() -> String {
    "[SYSTEM:HEARTBEAT] Long idle period detected. \
     Perform memory consolidation — review and organize recent memories. \
     Write a daily summary if one hasn't been created today."
        .to_string()
}
