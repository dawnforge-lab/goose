use spawnbot_common::paths::WorkspacePaths;

/// Check whether the workspace's HEARTBEAT.md contains actionable tasks.
///
/// Returns `false` if the file doesn't exist or can't be read.
pub fn should_emit_heartbeat(workspace: &WorkspacePaths) -> bool {
    let path = workspace.heartbeat_md();
    if !path.exists() {
        return false;
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => spawnbot_identity::heartbeat::has_actionable_tasks(&content),
        Err(_) => false,
    }
}
