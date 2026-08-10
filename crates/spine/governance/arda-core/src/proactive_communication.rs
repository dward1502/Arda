//! Canonical proactive-event policy for personal continuity.
//!
//! Evaluation is deterministic and side-effect free. The result may authorize a
//! message or a narrowly pre-authorized reversible action, but it never delivers
//! either and never creates approval truth.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::capability_composition::ProactiveCommunicationMode;
use crate::governance_gates::{
    ActionReversibility, CoercionRisk, ConsentAuthority, HumanFacingActionReview,
    JouleWorkBudgetClass,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveUrgency {
    Background,
    TimeSensitive,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveSourceQuality {
    OperatorAuthored,
    Verified,
    Corroborated,
    Unverified,
    SensitiveInference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveEventOrigin {
    ObservedEvent,
    OperatorRoutine,
    OperatorObjective,
    OperatorPreference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveActionClass {
    Information,
    Proposal,
    ReversiblePreAuthorized,
    ExternalSideEffect,
    SensitiveInference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorFatigueLevel {
    Low,
    Elevated,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorOperatorResponse {
    None,
    Acknowledged,
    Snoozed,
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveChannel {
    OperatorSession,
    NativeHud,
    Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveCommunicationPolicy {
    pub max_delivery_attempts: u8,
    pub max_interruptions_per_window: u8,
    pub defer_noncritical_during_quiet_window: bool,
    pub suppress_at_high_fatigue: bool,
    pub require_evidence: bool,
    pub allow_reversible_pre_authorized_actions: bool,
    pub preferred_channel: ProactiveChannel,
    pub digest_channel: ProactiveChannel,
    pub max_message_chars: u16,
}

impl Default for ProactiveCommunicationPolicy {
    fn default() -> Self {
        Self {
            max_delivery_attempts: 1,
            max_interruptions_per_window: 3,
            defer_noncritical_during_quiet_window: true,
            suppress_at_high_fatigue: true,
            require_evidence: true,
            allow_reversible_pre_authorized_actions: false,
            preferred_channel: ProactiveChannel::OperatorSession,
            digest_channel: ProactiveChannel::Digest,
            max_message_chars: 280,
        }
    }
}

#[derive(Debug, Deserialize)]
struct PolicyFile {
    proactive_communication: ProactiveCommunicationPolicy,
}

#[derive(Debug, thiserror::Error)]
pub enum ProactiveCommunicationPolicyError {
    #[error("proactive communication policy io: {0}")]
    Io(#[from] std::io::Error),
    #[error("proactive communication policy parse: {0}")]
    Parse(#[from] toml::de::Error),
}

impl ProactiveCommunicationPolicy {
    pub fn load(path: &Path) -> Result<Self, ProactiveCommunicationPolicyError> {
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str::<PolicyFile>(&raw)?.proactive_communication)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveCommunicationInput {
    pub event_id: String,
    pub mode: ProactiveCommunicationMode,
    pub observed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
    pub origin: ProactiveEventOrigin,
    pub source_quality: ProactiveSourceQuality,
    pub uncertainty: f64,
    pub urgency: ProactiveUrgency,
    pub action_class: ProactiveActionClass,
    pub budget_class: JouleWorkBudgetClass,
    pub resource_budget_available: bool,
    pub in_quiet_window: bool,
    pub fatigue_level: OperatorFatigueLevel,
    pub interruptions_in_window: u8,
    pub prior_attempts: u8,
    pub prior_response: PriorOperatorResponse,
    pub human_impact_review: HumanFacingActionReview,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveEventOutcome {
    Ignore,
    PersistSilently,
    IncludeInNextDigest,
    NotifyOnce,
    PrepareProposal,
    ExecuteReversiblePreAuthorizedAction,
    RequestApproval,
    FailClosed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveCommunicationDisposition {
    pub schema_version: String,
    pub event_id: String,
    pub outcome: ProactiveEventOutcome,
    pub reason_code: String,
    pub explanation: String,
    pub evidence_available: bool,
    pub channel: Option<ProactiveChannel>,
    pub delivery_authorized: bool,
    pub action_authorized: bool,
    pub approval_granted: bool,
}

impl ProactiveCommunicationDisposition {
    pub const SCHEMA_VERSION: &'static str = "arda.proactive-event-disposition.v1";
}

pub fn evaluate_proactive_communication(
    policy: &ProactiveCommunicationPolicy,
    input: &ProactiveCommunicationInput,
) -> ProactiveCommunicationDisposition {
    let evidence_available = !input.evidence_refs.is_empty();
    let expired = input.evaluated_at >= input.expires_at;
    let observed_in_future = input.observed_at > input.evaluated_at;
    let terminal_response = matches!(
        input.prior_response,
        PriorOperatorResponse::Acknowledged | PriorOperatorResponse::Dismissed
    );
    let human_review_valid = input.human_impact_review.schema_version
        == HumanFacingActionReview::SCHEMA_VERSION
        && input.human_impact_review.semantic == HumanFacingActionReview::SEMANTIC
        && !input.human_impact_review.affected_parties.is_empty()
        && (0.0..=1.0).contains(&input.human_impact_review.uncertainty);
    let effective_uncertainty = input.uncertainty.max(input.human_impact_review.uncertainty);
    let consent_missing = matches!(
        input.human_impact_review.consent_authority,
        ConsentAuthority::Inferred | ConsentAuthority::Missing
    );
    let sensitive_inference =
        matches!(
            input.source_quality,
            ProactiveSourceQuality::SensitiveInference
        ) || matches!(input.action_class, ProactiveActionClass::SensitiveInference);

    let (outcome, reason_code) = if input.event_id.trim().is_empty()
        || observed_in_future
        || !(0.0..=1.0).contains(&input.uncertainty)
        || !human_review_valid
    {
        (ProactiveEventOutcome::FailClosed, "invalid_event_contract")
    } else if sensitive_inference
        || matches!(
            input.human_impact_review.coercion_risk,
            CoercionRisk::High | CoercionRisk::Unknown
        )
        || consent_missing
    {
        (
            ProactiveEventOutcome::FailClosed,
            "human_impact_gate_failed",
        )
    } else if expired {
        (ProactiveEventOutcome::Ignore, "event_expired")
    } else if terminal_response {
        (ProactiveEventOutcome::Ignore, "operator_response_terminal")
    } else if policy.require_evidence && !evidence_available {
        (ProactiveEventOutcome::FailClosed, "evidence_required")
    } else if !input.resource_budget_available {
        (
            ProactiveEventOutcome::PersistSilently,
            "resource_budget_unavailable",
        )
    } else if matches!(input.mode, ProactiveCommunicationMode::Disabled) {
        (
            ProactiveEventOutcome::PersistSilently,
            "proactive_mode_disabled",
        )
    } else if policy.suppress_at_high_fatigue
        && matches!(input.fatigue_level, OperatorFatigueLevel::High)
        && !matches!(input.urgency, ProactiveUrgency::Critical)
    {
        (
            ProactiveEventOutcome::PersistSilently,
            "fatigue_budget_exhausted",
        )
    } else if input.interruptions_in_window >= policy.max_interruptions_per_window {
        (
            ProactiveEventOutcome::IncludeInNextDigest,
            "interruption_budget_exhausted",
        )
    } else if policy.defer_noncritical_during_quiet_window
        && input.in_quiet_window
        && !matches!(input.urgency, ProactiveUrgency::Critical)
    {
        (ProactiveEventOutcome::IncludeInNextDigest, "quiet_window")
    } else if input.prior_attempts >= policy.max_delivery_attempts
        || matches!(input.prior_response, PriorOperatorResponse::Snoozed)
    {
        (
            ProactiveEventOutcome::IncludeInNextDigest,
            "delivery_deferred",
        )
    } else if matches!(input.action_class, ProactiveActionClass::ExternalSideEffect)
        || matches!(input.mode, ProactiveCommunicationMode::ApprovalRequired)
        || matches!(
            input.human_impact_review.coercion_risk,
            CoercionRisk::Elevated
        )
        || (matches!(
            input.action_class,
            ProactiveActionClass::ReversiblePreAuthorized
        ) && (!policy.allow_reversible_pre_authorized_actions
            || !matches!(
                input.human_impact_review.reversibility,
                ActionReversibility::Reversible
            )
            || !matches!(
                input.human_impact_review.consent_authority,
                ConsentAuthority::PolicyAllowed | ConsentAuthority::ScopedApproval
            )))
    {
        (
            ProactiveEventOutcome::RequestApproval,
            "explicit_approval_required",
        )
    } else if matches!(
        input.action_class,
        ProactiveActionClass::ReversiblePreAuthorized
    ) && policy.allow_reversible_pre_authorized_actions
        && matches!(
            input.human_impact_review.reversibility,
            ActionReversibility::Reversible
        )
        && matches!(
            input.human_impact_review.consent_authority,
            ConsentAuthority::PolicyAllowed | ConsentAuthority::ScopedApproval
        )
    {
        (
            ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction,
            "reversible_pre_authorized",
        )
    } else if matches!(input.action_class, ProactiveActionClass::Proposal) {
        (ProactiveEventOutcome::PrepareProposal, "proposal_only")
    } else if matches!(input.urgency, ProactiveUrgency::Background)
        || matches!(input.source_quality, ProactiveSourceQuality::Unverified)
        || effective_uncertainty > 0.5
    {
        (
            ProactiveEventOutcome::IncludeInNextDigest,
            "digest_preferred",
        )
    } else {
        (
            ProactiveEventOutcome::NotifyOnce,
            "fresh_bounded_notification",
        )
    };

    let delivery_authorized = matches!(
        outcome,
        ProactiveEventOutcome::NotifyOnce | ProactiveEventOutcome::RequestApproval
    );
    let action_authorized = matches!(
        outcome,
        ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction
    );
    let channel = match outcome {
        ProactiveEventOutcome::NotifyOnce | ProactiveEventOutcome::RequestApproval => {
            Some(policy.preferred_channel)
        }
        ProactiveEventOutcome::IncludeInNextDigest | ProactiveEventOutcome::PrepareProposal => {
            Some(policy.digest_channel)
        }
        _ => None,
    };
    let explanation = format!(
        "{} Event origin: {:?}; urgency: {:?}; source quality: {:?}; uncertainty: {:.2}; JouleWork budget class: {}; attempts: {}/{}; interruptions: {}/{}; outcome: {:?}.",
        input
            .human_impact_review
            .proactive_message_explanation(input.budget_class),
        input.origin,
        input.urgency,
        input.source_quality,
        effective_uncertainty,
        input.budget_class.as_str(),
        input.prior_attempts,
        policy.max_delivery_attempts,
        input.interruptions_in_window,
        policy.max_interruptions_per_window,
        outcome,
    );

    ProactiveCommunicationDisposition {
        schema_version: ProactiveCommunicationDisposition::SCHEMA_VERSION.to_string(),
        event_id: input.event_id.clone(),
        outcome,
        reason_code: reason_code.to_string(),
        explanation,
        evidence_available,
        channel,
        delivery_authorized,
        action_authorized,
        approval_granted: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveMessageLabel {
    Fact,
    Inference,
    Suggestion,
    Action,
    ApprovalRequest,
}

impl ProactiveMessageLabel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "Fact",
            Self::Inference => "Inference",
            Self::Suggestion => "Suggestion",
            Self::Action => "Action",
            Self::ApprovalRequest => "Approval request",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveMessage {
    pub template_version: String,
    pub event_id: String,
    pub label: ProactiveMessageLabel,
    pub channel: ProactiveChannel,
    pub text: String,
}

impl ProactiveMessage {
    pub const TEMPLATE_VERSION: &'static str = "arda.proactive-message.v1";
}

/// Render a transient operator message. The durable disposition intentionally
/// does not retain `concise_summary`; delivery owns any content-retention policy.
pub fn render_proactive_message(
    policy: &ProactiveCommunicationPolicy,
    input: &ProactiveCommunicationInput,
    disposition: &ProactiveCommunicationDisposition,
    concise_summary: &str,
) -> Option<ProactiveMessage> {
    let channel = disposition.channel?;
    if matches!(
        disposition.outcome,
        ProactiveEventOutcome::Ignore
            | ProactiveEventOutcome::PersistSilently
            | ProactiveEventOutcome::FailClosed
            | ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction
    ) {
        return None;
    }

    let label = if matches!(disposition.outcome, ProactiveEventOutcome::RequestApproval) {
        ProactiveMessageLabel::ApprovalRequest
    } else {
        match input.action_class {
            ProactiveActionClass::Proposal => ProactiveMessageLabel::Suggestion,
            ProactiveActionClass::ReversiblePreAuthorized
            | ProactiveActionClass::ExternalSideEffect => ProactiveMessageLabel::Action,
            ProactiveActionClass::Information => {
                if matches!(input.source_quality, ProactiveSourceQuality::Unverified) {
                    ProactiveMessageLabel::Inference
                } else {
                    ProactiveMessageLabel::Fact
                }
            }
            ProactiveActionClass::SensitiveInference => return None,
        }
    };
    let summary = normalize_message_fragment(concise_summary);
    let reason = normalize_message_fragment(
        input
            .human_impact_review
            .interruption_reason
            .as_deref()
            .unwrap_or("an operator-authored condition matched"),
    );
    let next = match disposition.outcome {
        ProactiveEventOutcome::RequestApproval => "Approve or decline when ready.",
        ProactiveEventOutcome::PrepareProposal => "Review the suggestion when convenient.",
        ProactiveEventOutcome::IncludeInNextDigest => "Saved for the next digest.",
        _ => "No immediate action is required.",
    };
    let text = format!(
        "{}: {} Why now: {} {}",
        label.as_str(),
        summary,
        reason,
        next
    );

    Some(ProactiveMessage {
        template_version: ProactiveMessage::TEMPLATE_VERSION.to_string(),
        event_id: disposition.event_id.clone(),
        label,
        channel,
        text: truncate_chars(&text, usize::from(policy.max_message_chars)),
    })
}

fn normalize_message_fragment(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 1 {
        return "…".chars().take(max_chars).collect();
    }
    let mut truncated: String = value.chars().take(max_chars - 1).collect();
    truncated.push('…');
    truncated
}
