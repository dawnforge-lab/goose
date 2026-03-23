use anyhow::Result;
use spawnbot_common::types::AutonomyMode;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: ProposalStatus,
}

pub struct ApprovalManager {
    mode: AutonomyMode,
    pending: Arc<Mutex<Vec<Proposal>>>,
}

impl ApprovalManager {
    pub fn new(mode: AutonomyMode) -> Self {
        Self {
            mode,
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if a tool call requires approval in the current mode
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        if self.mode == AutonomyMode::Yolo {
            return false;
        }
        matches!(
            tool_name,
            "identity_update"
                | "identity_section_update"
                | "skill_create"
                | "skill_edit"
                | "skill_delete"
                | "extension_install"
                | "extension_remove"
        )
    }

    /// Create a proposal for a tool call that requires approval
    pub async fn propose(&self, tool_name: &str, args: &serde_json::Value) -> Result<String> {
        let id = ulid::Ulid::new().to_string();
        let proposal = Proposal {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            args: args.clone(),
            description: format!("Tool call: {} with args: {}", tool_name, args),
            created_at: chrono::Utc::now(),
            status: ProposalStatus::Pending,
        };
        self.pending.lock().await.push(proposal);
        tracing::info!(proposal_id = %id, tool = %tool_name, "Proposal created — awaiting approval");
        Ok(id)
    }

    /// Resolve a proposal (approve or reject)
    pub async fn resolve(&self, proposal_id: &str, approved: bool) -> Result<Option<Proposal>> {
        let mut pending = self.pending.lock().await;
        if let Some(pos) = pending.iter().position(|p| p.id == proposal_id) {
            let mut proposal = pending.remove(pos);
            proposal.status = if approved {
                ProposalStatus::Approved
            } else {
                ProposalStatus::Rejected
            };
            Ok(Some(proposal))
        } else {
            Ok(None)
        }
    }

    /// List all pending proposals
    pub async fn pending_proposals(&self) -> Vec<Proposal> {
        self.pending.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_yolo_mode_never_requires_approval() {
        let mgr = ApprovalManager::new(AutonomyMode::Yolo);
        assert!(!mgr.requires_approval("identity_update"));
        assert!(!mgr.requires_approval("skill_create"));
        assert!(!mgr.requires_approval("skill_delete"));
        assert!(!mgr.requires_approval("extension_install"));
        assert!(!mgr.requires_approval("extension_remove"));
        assert!(!mgr.requires_approval("some_random_tool"));
    }

    #[tokio::test]
    async fn test_approval_mode_requires_approval_for_sensitive_tools() {
        let mgr = ApprovalManager::new(AutonomyMode::Approval);
        assert!(mgr.requires_approval("identity_update"));
        assert!(mgr.requires_approval("identity_section_update"));
        assert!(mgr.requires_approval("skill_create"));
        assert!(mgr.requires_approval("skill_edit"));
        assert!(mgr.requires_approval("skill_delete"));
        assert!(mgr.requires_approval("extension_install"));
        assert!(mgr.requires_approval("extension_remove"));
    }

    #[tokio::test]
    async fn test_approval_mode_allows_non_sensitive_tools() {
        let mgr = ApprovalManager::new(AutonomyMode::Approval);
        assert!(!mgr.requires_approval("memory_store"));
        assert!(!mgr.requires_approval("memory_search"));
        assert!(!mgr.requires_approval("some_random_tool"));
    }

    #[tokio::test]
    async fn test_propose_and_approve() {
        let mgr = ApprovalManager::new(AutonomyMode::Approval);
        let args = serde_json::json!({"section": "Communication", "content": "Be concise"});

        let id = mgr.propose("identity_update", &args).await.unwrap();
        assert!(!id.is_empty());

        // Should be in pending list
        let pending = mgr.pending_proposals().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tool_name, "identity_update");

        // Approve it
        let resolved = mgr.resolve(&id, true).await.unwrap();
        assert!(resolved.is_some());
        let proposal = resolved.unwrap();
        assert!(matches!(proposal.status, ProposalStatus::Approved));

        // Pending list should be empty now
        let pending = mgr.pending_proposals().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_propose_and_reject() {
        let mgr = ApprovalManager::new(AutonomyMode::Approval);
        let args = serde_json::json!({"name": "dangerous-skill"});

        let id = mgr.propose("skill_create", &args).await.unwrap();

        let resolved = mgr.resolve(&id, false).await.unwrap();
        assert!(resolved.is_some());
        let proposal = resolved.unwrap();
        assert!(matches!(proposal.status, ProposalStatus::Rejected));
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_proposal() {
        let mgr = ApprovalManager::new(AutonomyMode::Approval);
        let resolved = mgr.resolve("nonexistent-id", true).await.unwrap();
        assert!(resolved.is_none());
    }
}
