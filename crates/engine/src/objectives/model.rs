use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAuthority {
    pub project_id: String,
    pub contract_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewLeaf {
    pub id: String,
    pub project_id: Option<String>,
    pub workspace_root: String,
    pub authority: String,
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<LeafExecutionSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafExecutionSpec {
    pub objective: String,
    pub execution_prompt: String,
    pub verification_prompt: String,
    pub review_prompt: String,
    pub approval_envelope: Value,
    pub objective_plan_receipt: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewObjective {
    pub id: String,
    pub source_id: String,
    pub idempotency_key: String,
    pub operator_id: String,
    pub text: String,
    pub priority: i64,
    pub projects: Vec<ProjectAuthority>,
    pub leaves: Vec<NewLeaf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveState {
    PendingApproval,
    Approved,
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl ObjectiveState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "pending_approval" => Self::PendingApproval,
            "approved" => Self::Approved,
            "running" => Self::Running,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveRecord {
    pub id: String,
    pub source_id: String,
    pub operator_id: String,
    pub text: String,
    pub priority: i64,
    pub revision: i64,
    pub state: ObjectiveState,
    pub project_ids: Vec<String>,
    pub terminal_receipt_digest: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeafStage {
    Execute,
    Verify,
    Review,
    Close,
    Complete,
    Cancelled,
    Failed,
}

impl LeafStage {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Execute => "execute",
            Self::Verify => "verify",
            Self::Review => "review",
            Self::Close => "close",
            Self::Complete => "complete",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "execute" => Self::Execute,
            "verify" => Self::Verify,
            "review" => Self::Review,
            "close" => Self::Close,
            "complete" => Self::Complete,
            "cancelled" => Self::Cancelled,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafRecord {
    pub id: String,
    pub objective_id: String,
    pub project_id: Option<String>,
    pub workspace_root: String,
    pub authority: String,
    pub stage: LeafStage,
    pub attempt: i64,
    pub lease_owner: Option<String>,
    pub lease_expires_ms: Option<i64>,
    pub current_receipt_digest: Option<String>,
    pub project_contract_digest: Option<String>,
    pub execution: Option<LeafExecutionSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedLeaf {
    pub objective_id: String,
    pub leaf_id: String,
    pub project_id: Option<String>,
    pub workspace_root: String,
    pub authority: String,
    pub stage: LeafStage,
    pub attempt: i64,
    pub lease_owner: String,
    pub lease_expires_ms: i64,
    pub current_receipt_digest: Option<String>,
    pub project_contract_digest: Option<String>,
    pub execution: Option<LeafExecutionSpec>,
    #[serde(default)]
    pub dependency_receipts: Vec<StageReceipt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStage {
    Execute,
    Verify,
    Review,
    Close,
}

impl ReceiptStage {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Execute => "execute",
            Self::Verify => "verify",
            Self::Review => "review",
            Self::Close => "close",
        }
    }

    pub(crate) const fn leaf_stage(self) -> LeafStage {
        match self {
            Self::Execute => LeafStage::Execute,
            Self::Verify => LeafStage::Verify,
            Self::Review => LeafStage::Review,
            Self::Close => LeafStage::Close,
        }
    }

    pub(crate) const fn next_leaf_stage(self) -> LeafStage {
        match self {
            Self::Execute => LeafStage::Verify,
            Self::Verify => LeafStage::Review,
            Self::Review => LeafStage::Close,
            Self::Close => LeafStage::Complete,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageReceipt {
    pub contract: String,
    pub stage: ReceiptStage,
    pub digest: String,
    pub predecessor_digest: Option<String>,
    pub run_path: String,
    pub provider: String,
    pub model: String,
    pub started_at_ms: i64,
    pub completed_at_ms: i64,
    pub verdict: String,
    pub context_outcome_receipt_id: Option<String>,
    pub context_outcome_receipt_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_digest: Option<String>,
}

impl StageReceipt {
    pub fn computed_binding_digest(&self) -> Result<String, serde_json::Error> {
        let mut canonical = self.clone();
        canonical.binding_digest = None;
        let encoded = serde_json::to_vec(&canonical)?;
        Ok(format!("sha256:{:x}", Sha256::digest(encoded)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlAction {
    Approve { revision: i64 },
    Reject,
    Pause,
    Resume,
    Cancel,
    Reprioritize { priority: i64 },
    Revise { text: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub id: String,
    pub objective_id: String,
    pub next_wake_ms: i64,
    pub recurrence: Option<String>,
    pub idempotency_key: String,
}
