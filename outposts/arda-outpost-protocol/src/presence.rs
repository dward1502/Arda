//! Read-only, sanitized runtime-presence projection shared with display outposts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const RUNTIME_PRESENCE_SCHEMA_VERSION: &str = "arda.runtime-presence.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresenceProjection {
    pub projection_id: String,
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub nodes: Vec<PresenceNode>,
    pub edges: Vec<PresenceEdge>,
    pub source_receipt_refs: Vec<String>,
    pub redaction_class: RedactionClass,
}

impl RuntimePresenceProjection {
    /// Resolve the only state a renderer may display for this projection.
    ///
    /// Invalid, expired, or unreceipted data fails closed to an explicit idle,
    /// degraded scene rather than allowing decorative activity to imply work.
    pub fn scene_disposition_at(&self, now: DateTime<Utc>) -> SceneDisposition {
        if self.schema_version != RUNTIME_PRESENCE_SCHEMA_VERSION {
            return SceneDisposition::idle_degraded(DegradedReason::UnsupportedSchema);
        }
        if self.generated_at > self.valid_until {
            return SceneDisposition::idle_degraded(DegradedReason::InvalidValidityWindow);
        }
        if now > self.valid_until {
            return SceneDisposition::idle_degraded(DegradedReason::Expired);
        }
        if !self.has_verifiable_receipts() {
            return SceneDisposition::idle_degraded(DegradedReason::Unverifiable);
        }
        if !self.has_normalized_signals() {
            return SceneDisposition::idle_degraded(DegradedReason::InvalidSignal);
        }

        SceneDisposition {
            state: SceneState::Active,
            degraded_reason: None,
        }
    }

    fn has_verifiable_receipts(&self) -> bool {
        !self.projection_id.trim().is_empty()
            && !self.nodes.is_empty()
            && has_receipts(&self.source_receipt_refs)
            && self
                .nodes
                .iter()
                .all(|node| has_receipts(&node.source_receipt_refs))
            && self
                .edges
                .iter()
                .all(|edge| has_receipts(&edge.source_receipt_refs))
    }

    fn has_normalized_signals(&self) -> bool {
        self.nodes.iter().all(|node| {
            normalized(node.confidence)
                && node.resource_pressure.as_ref().is_none_or(|pressure| {
                    normalized(pressure.cpu)
                        && normalized(pressure.memory)
                        && normalized(pressure.provider)
                })
        }) && self.edges.iter().all(|edge| normalized(edge.confidence))
    }
}

fn has_receipts(receipts: &[String]) -> bool {
    !receipts.is_empty() && receipts.iter().all(|receipt| !receipt.trim().is_empty())
}

fn normalized(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PresenceNode {
    pub id: String,
    pub kind: PresenceNodeKind,
    pub label: String,
    pub lifecycle: LifecycleState,
    pub health: HealthState,
    pub confidence: f32,
    pub freshness_seconds: u64,
    pub resource_pressure: Option<ResourcePressure>,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub source_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PresenceEdge {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_type: PresenceEdgeType,
    pub confidence: f32,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub source_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourcePressure {
    pub cpu: f32,
    pub memory: f32,
    pub provider: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresenceNodeKind {
    Realm,
    Agent,
    Service,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresenceEdgeType {
    Collaboration,
    Handoff,
    Wait,
    Dependency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Idle,
    Starting,
    Active,
    WaitingApproval,
    Failed,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Degraded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RedactionClass {
    PublicOperational,
    PrivateMetadataRemoved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneDisposition {
    pub state: SceneState,
    pub degraded_reason: Option<DegradedReason>,
}

impl SceneDisposition {
    fn idle_degraded(reason: DegradedReason) -> Self {
        Self {
            state: SceneState::IdleDegraded,
            degraded_reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneState {
    Active,
    IdleDegraded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DegradedReason {
    Expired,
    Unverifiable,
    UnsupportedSchema,
    InvalidValidityWindow,
    InvalidSignal,
}
