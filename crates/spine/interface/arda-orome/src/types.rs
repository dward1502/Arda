// sigil: REPAIR
use crate::Priority;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub source: String,
    pub sender: String,
    pub content: String,
    pub received_at_utc: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub is_illuvatar: bool,
}

impl InboundMessage {
    pub fn new(
        source: impl Into<String>,
        sender: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            sender: sender.into(),
            content: content.into(),
            received_at_utc: Utc::now().to_rfc3339(),
            channel: None,
            is_illuvatar: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub provider: String,
    pub channel: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub stream: bool,
    pub priority: String,
    pub created_at_utc: String,
}

impl OutboundMessage {
    pub fn new(
        provider: impl Into<String>,
        channel: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            channel: channel.into(),
            subject: subject.into(),
            body: body.into(),
            stream: false,
            priority: "normal".to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardroomPost {
    pub from_agent: String,
    pub message_type: String,
    pub priority: Priority,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub mentions: Vec<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
    pub posted_at_utc: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BoardroomTriadScores {
    pub aurelius: Option<f64>,
    pub bacon: Option<f64>,
    pub sun_tzu: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardroomOracleLink {
    pub query_id: Option<String>,
    pub verdict_locator: Option<String>,
    pub verdict_found: bool,
    pub outcome: Option<String>,
    pub triad_scores: BoardroomTriadScores,
    pub resonance_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardroomQuorumDecision {
    pub threshold: usize,
    pub approvals: Vec<String>,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardroomManweRouteEvidence {
    pub route_evidence: Option<String>,
    pub selected_provider: Option<String>,
    pub selected_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardroomQuorumPacket {
    pub schema_version: String,
    pub packet_id: String,
    pub session_id: String,
    pub topic: String,
    pub created_at_utc: String,
    pub evidence_paths: Vec<String>,
    pub oracle: BoardroomOracleLink,
    pub quorum: BoardroomQuorumDecision,
    pub manwe_route: BoardroomManweRouteEvidence,
    pub discord_projection_permitted: bool,
    pub operator_approval_required: bool,
    pub operator_approved: bool,
    pub status: String,
    pub status_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatingRoomEventKind {
    Status,
    Alert,
    Decision,
    Command,
}

impl std::fmt::Display for OperatingRoomEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            OperatingRoomEventKind::Status => "status",
            OperatingRoomEventKind::Alert => "alert",
            OperatingRoomEventKind::Decision => "decision",
            OperatingRoomEventKind::Command => "command",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatingRoomEvent {
    pub schema_version: String,
    pub event_id: String,
    pub kind: OperatingRoomEventKind,
    pub topic: String,
    pub subject: String,
    pub body: String,
    pub evidence_paths: Vec<String>,
    pub safety_state: String,
    pub discord_projection_permitted: bool,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommsEventType {
    Inbound,
    Status,
    Alert,
    Decision,
    OutboundProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommsEventVisibility {
    Internal,
    OperatorVisible,
    OperatorOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommsEventRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromotionState {
    Unpromoted,
    Projected,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommsEvent {
    pub schema_version: String,
    pub event_id: String,
    pub event_type: CommsEventType,
    pub semantic_channel: String,
    pub visibility: CommsEventVisibility,
    pub risk: CommsEventRisk,
    pub summary: String,
    pub canonical_refs: Vec<String>,
    pub promotion_state: PromotionState,
    pub raw_content_redacted: bool,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CouncilDiscussionNote {
    pub schema_version: String,
    pub note_id: String,
    pub session_id: String,
    pub agent: String,
    pub summary: String,
    pub risk: CommsEventRisk,
    pub source_class: String,
    pub semantic_channel: String,
    pub discussion_only: bool,
    pub promotion_state: PromotionState,
    pub canonical_refs: Vec<String>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CouncilDiscussionProjection {
    pub surface: String,
    pub semantic_channel: String,
    pub dispatch_channel: String,
    pub note_id: String,
    pub session_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CouncilDiscussionPromotion {
    pub schema_version: String,
    pub promotion_id: String,
    pub note_id: String,
    pub session_id: String,
    pub task_ref: String,
    pub promotion_state: PromotionState,
    #[serde(default)]
    pub is_authoritative: bool,
    #[serde(default)]
    pub canonical_write_authorized: bool,
    #[serde(default)]
    pub queue_mutated: bool,
    #[serde(default)]
    pub requires_human_approval: bool,
    #[serde(default)]
    pub authority_boundary: String,
    pub canonical_refs: Vec<String>,
    pub promoted_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CouncilApprovalDecision {
    pub schema_version: String,
    pub decision_id: String,
    pub session_id: String,
    pub promotion_id: Option<String>,
    pub note_id: Option<String>,
    pub approver: String,
    pub approved: bool,
    pub status: String,
    pub reason: String,
    pub canonical_refs: Vec<String>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CouncilCommandSeat {
    pub seat: String,
    pub agent_id: String,
    pub role: String,
    pub authority: String,
    pub use_when: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManweRouteHint {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub route_evidence: Option<String>,
    pub latency_ms: Option<u64>,
    pub estimated_input_tokens: Option<u64>,
    pub estimated_output_tokens: Option<u64>,
    #[serde(default)]
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCouncilSummaryFallbackMetadata {
    pub provider: String,
    pub reason: Option<String>,
    pub fallback_used: bool,
    pub semantic_channel_fallback_used: bool,
    pub semantic_channel_env_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCouncilSummaryRoute {
    pub schema_version: String,
    pub route_id: String,
    pub session_id: String,
    pub summary: String,
    pub semantic_channel: String,
    pub dispatch_channel: String,
    pub output_classification: String,
    pub source_task: Option<String>,
    pub canonical_refs: Vec<String>,
    pub promotable: bool,
    pub is_authoritative: bool,
    pub provider_used: Option<String>,
    pub model_used: Option<String>,
    pub route_evidence: Option<String>,
    pub latency_ms: Option<u64>,
    pub estimated_tokens: Option<u64>,
    pub fallback_metadata: LocalCouncilSummaryFallbackMetadata,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskApprovalProposal {
    pub schema_version: String,
    pub proposal_id: String,
    pub task_ref: String,
    pub scope: String,
    pub risk: CommsEventRisk,
    pub action_summary: String,
    pub requested_by: String,
    pub canonical_refs: Vec<String>,
    #[serde(default)]
    pub delivery_metadata: std::collections::BTreeMap<String, serde_json::Value>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskApprovalProjection {
    pub surface: String,
    pub proposal_id: String,
    pub task_ref: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub delivery_metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskApprovalPacket {
    pub schema_version: String,
    pub approval_id: String,
    pub proposal_id: String,
    pub task_ref: String,
    pub scope: String,
    pub risk: CommsEventRisk,
    pub action_summary: String,
    pub receipt_id: String,
    pub approved_by: String,
    #[serde(default)]
    pub delivery_metadata: std::collections::BTreeMap<String, serde_json::Value>,
    pub approved_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubagentCompletionPacket {
    pub schema_version: String,
    pub completion_id: String,
    pub task_ref: String,
    pub agent: String,
    pub summary: String,
    #[serde(default)]
    pub verification: Vec<String>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    pub risk: CommsEventRisk,
    pub next_action: String,
    pub status: String,
    pub review_required: bool,
    pub canonical_refs: Vec<String>,
    pub completed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubagentCompletionProjection {
    pub surface: String,
    pub semantic_channel: String,
    pub dispatch_channel: String,
    pub completion_id: String,
    pub task_ref: String,
    pub title: String,
    pub body: String,
}

impl BoardroomPost {
    pub fn new(
        from_agent: impl Into<String>,
        message_type: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            from_agent: from_agent.into(),
            message_type: message_type.into(),
            priority: Priority::Normal,
            subject: subject.into(),
            body: body.into(),
            mentions: Vec::new(),
            thread_id: None,
            posted_at_utc: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentClass {
    TaskRequest,
    Question,
    StatusCheck,
    Redirect,
    Social,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentRoute {
    Prometheus,
    Athena,
    Hades,
    Calendar,
    Boardroom,
    Hermes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub intent: IntentClass,
    pub priority: String,
    pub route_to: IntentRoute,
    pub joulework: f64,
    pub love_eq: f64,
    #[serde(default)]
    pub triad_passed: Option<bool>,
    #[serde(default)]
    pub triad_score: Option<f64>,
    pub confidence: f64,
    pub tier: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionDisposition {
    Note,
    Reroute,
    Override,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionMessage {
    pub source: String,
    pub sender: String,
    pub content: String,
    pub received_at_utc: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
}

impl InterruptionMessage {
    pub fn new(
        source: impl Into<String>,
        sender: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            sender: sender.into(),
            content: content.into(),
            received_at_utc: Utc::now().to_rfc3339(),
            channel: None,
            run_id: None,
            session_id: None,
            task_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionEnvelope {
    pub schema_version: String,
    pub event_id: String,
    pub message: InterruptionMessage,
    pub ledger_writes: Vec<String>,
    pub decision: InterruptionLedgerDecision,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionLedgerDecision {
    PolicySafe,
    RequiresOperatorReview,
    PolicyBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskApprovalEnvelope {
    pub schema_version: String,
    pub proposal_id: String,
    pub approval_id: String,
    pub ledger_writes: Vec<String>,
    pub decision: InterruptionLedgerDecision,
    pub created_at_utc: String,
}

/// Request to deliver a personal reminder through Oromë.
/// Fatigue caps, quiet windows, and snooze/dismiss state are
/// checked by the adapter before every attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalReminderRequest {
    pub schema_version: String,
    pub reminder_id: String,
    pub item_id: String,
    pub operator_id: String,
    pub subject: String,
    pub body: String,
    pub provider: String,
    pub channel: String,
    pub attempt_number: u32,
    pub max_attempts: u32,
    pub quiet_mode: bool,
    pub snoozed_until_utc: Option<String>,
    pub created_at_utc: String,
}

impl Default for PersonalReminderRequest {
    fn default() -> Self {
        Self {
            schema_version: "arda.orome.personal-reminder.v1".to_string(),
            reminder_id: String::new(),
            item_id: String::new(),
            operator_id: String::new(),
            subject: String::new(),
            body: String::new(),
            provider: String::new(),
            channel: String::new(),
            attempt_number: 0,
            max_attempts: 3,
            quiet_mode: false,
            snoozed_until_utc: None,
            created_at_utc: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Delivery receipt for a personal reminder attempt.
/// "Attempted" and "Delivered" are never conflated — the caller
/// must check `state` to distinguish transport-level attempts from
/// confirmed delivery/acknowledgement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalReminderReceipt {
    pub schema_version: String,
    pub reminder_id: String,
    pub item_id: String,
    pub attempted_at_utc: String,
    pub state: PersonalReminderDeliveryState,
    pub attempt_number: u32,
    pub max_attempts: u32,
    pub provider: String,
    pub channel: String,
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub quiet_mode_active: bool,
    /// True if the reminder was suppressed this cycle due to quiet
    /// windows, fatigue caps, or explicit snooze/dismiss.
    pub suppressed: bool,
    #[serde(default)]
    pub non_clinical_disclosure: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalReminderDeliveryState {
    /// Transport attempted but not yet confirmed delivered.
    Attempted,
    /// Provider confirmed receipt of the message.
    Delivered,
    /// Operator acknowledged the reminder.
    Acknowledged,
    /// Operator deferred the reminder to a later time.
    Deferred,
    /// Operator dismissed the reminder.
    Dismissed,
    /// Reminder exhausted retry budget without delivery.
    Failed,
}

impl PersonalReminderReceipt {
    pub fn was_delivered(&self) -> bool {
        matches!(
            self.state,
            PersonalReminderDeliveryState::Delivered
                | PersonalReminderDeliveryState::Acknowledged
                | PersonalReminderDeliveryState::Deferred
                | PersonalReminderDeliveryState::Dismissed
        )
    }
}
