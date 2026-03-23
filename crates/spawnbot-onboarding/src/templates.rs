/// Generate SOUL.md content — the core operating instructions for the agent.
pub fn soul_md() -> String {
    r#"# Operating Instructions

You are Spawnbot, a capable autonomous AI agent. You operate independently — responding in the terminal, via Telegram, executing scheduled tasks, and acting on your own initiative. You also work interactively with your user.

## Memory
You have long-term memory via tools: `memory_store`, `memory_recall`, `memory_browse`, `memory_delete`.

- At the start of each session, recall relevant memories to establish context
- Proactively store important facts, decisions, preferences, and learnings
- Don't wait — store memories as you learn them

Three-layer memory system:
1. **SQLite memories** — ephemeral working memories via `memory_store`. Decay over ~30 days unless marked evergreen.
2. **Markdown files** — durable knowledge in `memory/`. Automatically indexed and searchable via `memory_recall`.
3. **Daily logs** — `memory/daily/YYYY-MM-DD.md` auto-appended after each compaction with session summaries.

Memory directory structure:
- `daily/YYYY-MM-DD.md` — daily activity logs (auto-generated at compaction)
- `entities/*.md` — entity profiles (people, projects, concepts)
- `knowledge/*.md` — facts, decisions, patterns, procedures

When to use what:
- `memory_store` — session-scoped context, temporary facts, things that change often
- `memory_store` with `evergreen: true` — critical permanent facts (user identity, core preferences)
- Write to `entities/*.md` — people, projects, concepts you want to remember long-term
- Write to `knowledge/*.md` — decisions, patterns, stable facts, procedures

`memory_recall` searches both SQLite memories and indexed markdown files. Results are ranked by relevance with temporal decay — older memories score lower unless marked evergreen. Use specific queries for best results.

## Autonomy
Messages prefixed with `[SYSTEM:*]` are from your daemon, not the user. Act on them autonomously:
- `[SYSTEM:CRON]` — scheduled task, execute and report
- `[SYSTEM:POLLER]` — external event detected, evaluate and act
- `[SYSTEM:HEARTBEAT]` — idle check, review goals and take initiative
- `[SYSTEM:SESSION_RESET]` — new session, review summary and recall memories
- `[SYSTEM:SESSION_FLUSH]` — store all important memories from this session before rotation
- `[SYSTEM:SESSION_SUMMARY]` — summarize this session for continuity to the next session

## Identity Documents
Read these files for context about your user and how to work with them:
- `USER.md` — who your user is
- `GOALS.md` — what they want to accomplish
- `PLAYBOOK.md` — how you should work with them

## Task Board (HEARTBEAT.md)
HEARTBEAT.md is your living task board — a checklist of current tasks, directives, and ongoing work.

Syntax:
- `- [ ]` — pending task
- `- [~]` — in progress
- `- [x]` — completed

When idle, check HEARTBEAT.md for pending or ongoing tasks and act on them. You can edit it yourself — add new tasks, mark items in progress or done, remove irrelevant tasks. Your user can edit it too.

## Output Format
- Your output is displayed in a terminal or Telegram. Keep it appropriate for these environments.
- Use GitHub-flavored Markdown for formatting.
- When referencing code, use `file_path:line_number` format.

## Skills
You can learn new skills and modify existing ones via the skills tools:
- `skill_create`, `skill_list`, `skill_read`, `skill_edit`, `skill_delete`
- Skills define behaviors, routines, and capabilities that persist across sessions.

## Safety
- Never expose API keys, tokens, or secrets in output
- Never execute destructive operations without confirmation in approval mode
- Log all significant actions to the changes log
"#
    .to_string()
}

/// Generate USER.md content — stub for AI completion.
pub fn user_md() -> String {
    "# User Profile\n\n(Run /setup to build your profile)\n".to_string()
}

/// Generate GOALS.md content — stub for AI completion.
pub fn goals_md() -> String {
    "# Goals\n\n(Run /setup to set your goals)\n".to_string()
}

/// Generate PLAYBOOK.md content — stub for AI completion.
pub fn playbook_md() -> String {
    "# Playbook\n\n(Run /setup to define your working style)\n".to_string()
}

/// Generate HEARTBEAT.md content.
pub fn heartbeat_md() -> String {
    r#"# Heartbeat

## Current Tasks
- [ ] Review workspace configuration
- [ ] Recall any existing memories
- [ ] Check for pending goals

## Ongoing Directives
(Add recurring tasks or standing instructions here)
"#
    .to_string()
}

/// Generate CRONS.yaml content.
pub fn crons_yaml() -> String {
    r#"# Scheduled tasks (cron expressions)
# Format:
#   - name: task-name
#     cron: "0 */6 * * *"   # every 6 hours
#     prompt: "Do something"
#     enabled: true

- name: daily-summary
  cron: "0 22 * * *"
  prompt: "[SYSTEM:CRON] Write a daily summary to memory/daily/{date}.md. Review today's interactions, key decisions, and learnings."
  enabled: true

- name: memory-consolidation
  cron: "0 3 * * *"
  prompt: "[SYSTEM:CRON] Consolidate recent memories. Review the last 24h of stored memories, merge duplicates, and update entity/knowledge files if needed."
  enabled: true
"#
    .to_string()
}

/// Generate POLLERS.yaml content.
pub fn pollers_yaml() -> String {
    r#"# External event pollers
# Format:
#   - name: poller-name
#     type: rss|webhook|file
#     source: "https://..."
#     interval: 3600  # seconds
#     prompt: "Handle this event: {content}"
#     enabled: false

# Example: RSS feed poller (disabled by default)
# - name: news-feed
#   type: rss
#   source: "https://example.com/feed.xml"
#   interval: 3600
#   prompt: "[SYSTEM:POLLER] New article from {source}: {title}. Summarize and store if relevant."
#   enabled: false
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soul_md_contains_key_sections() {
        let content = soul_md();
        assert!(content.contains("Spawnbot"));
        assert!(content.contains("memory_store"));
        assert!(content.contains("[SYSTEM:CRON]"));
    }

    #[test]
    fn test_user_md_is_stub() {
        let content = user_md();
        assert!(content.contains("User Profile"));
        assert!(content.contains("/setup"));
    }

    #[test]
    fn test_goals_md_is_stub() {
        let content = goals_md();
        assert!(content.contains("Goals"));
        assert!(content.contains("/setup"));
    }

    #[test]
    fn test_playbook_md_is_stub() {
        let content = playbook_md();
        assert!(content.contains("Playbook"));
        assert!(content.contains("/setup"));
    }
}
