use arda_launcher::lifecycle::commands::lifecycle_status;
use arda_launcher::lifecycle::types::AggregateState;
use arda_outpost_protocol::{
    MirromereAccessibility, MirromereAvailability, MirromereDisplayRole,
    MirromereEvidenceReference, MirromereFreshness, MirromereInteractionId, MirromerePresencePhase,
    MirromerePrivacyClass, MirromerePrivacyPolicy, MirromereReducedMotion, MirromereScene,
    MirromereSceneId, MirromereSlot, MirromereSlotContent, MirromereSourceMode,
    MirromereSurfaceProjection, MirromereSurfaceValidationError, MirromereTransitionPolicy,
    MirromereTransitionStyle, MirromereUrgency, MirromereVectorFieldKind,
    MirromereVisibilityCeiling, MIRROMERE_SURFACE_SCHEMA_VERSION,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;

const DEFAULT_HARNESS_URL: &str = "http://127.0.0.1:7878";
const SOURCE_STALE_AFTER_SECONDS: i64 = 30;
const SURFACE_TTL_SECONDS: i64 = 30;
const MIRROMERE_INTERACTION_RECEIPT_SCHEMA_VERSION: &str = "arda.mirromere.interaction-receipt.v1";
const MIRROMERE_INTERACTION_RECEIPT_LIMIT: usize = 128;
static MIRROMERE_INTERACTION_RECEIPT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereInteractionRequest {
    pub surface: MirromereSurfaceProjection,
    pub interaction_id: MirromereInteractionId,
    pub requested_at: DateTime<Utc>,
    pub explicit_operator_action: bool,
    pub presented_privacy_class: MirromerePrivacyClass,
    pub visibility_ceiling: MirromereVisibilityCeiling,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereInteractionOutcome {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereInteractionStatus {
    Requested,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MirromereInteractionReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub surface_id: String,
    pub scene_id: MirromereSceneId,
    pub interaction_id: MirromereInteractionId,
    pub requested_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub outcome: MirromereInteractionOutcome,
    pub status: MirromereInteractionStatus,
    pub requires_operator_action: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirromereInteractionPolicyDecision {
    accepted: bool,
    reason: &'static str,
    requires_operator_action: bool,
}

#[derive(Default)]
pub struct MirromereInteractionReceiptState {
    receipts: Mutex<VecDeque<MirromereInteractionReceipt>>,
    issued_surfaces: Mutex<HashMap<String, MirromereSurfaceProjection>>,
}

impl MirromereInteractionReceiptState {
    fn remember_surface(&self, surface: MirromereSurfaceProjection) -> Result<(), String> {
        self.issued_surfaces
            .lock()
            .map_err(|_| "Mirromere issued-surface state is unavailable".to_string())?
            .insert(surface.surface_id.clone(), surface);
        Ok(())
    }

    pub fn record(
        &self,
        request: MirromereInteractionRequest,
        now: DateTime<Utc>,
    ) -> Result<MirromereInteractionReceipt, String> {
        let authoritative_surface = self
            .issued_surfaces
            .lock()
            .map_err(|_| "Mirromere issued-surface state is unavailable".to_string())?
            .get(&request.surface.surface_id)
            .cloned();
        let decision = if authoritative_surface.as_ref() == Some(&request.surface) {
            evaluate_mirromere_interaction(&request, now)
        } else {
            MirromereInteractionPolicyDecision {
                accepted: false,
                reason: "surface_not_current",
                requires_operator_action: interaction_requires_operator_action(
                    request.interaction_id,
                ),
            }
        };
        let sequence = MIRROMERE_INTERACTION_RECEIPT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let receipt = MirromereInteractionReceipt {
            schema_version: MIRROMERE_INTERACTION_RECEIPT_SCHEMA_VERSION.to_string(),
            receipt_id: format!(
                "mirromere-interaction-{}-{sequence}",
                now.timestamp_micros()
            ),
            surface_id: request.surface.surface_id.clone(),
            scene_id: request.surface.scene.scene_id,
            interaction_id: request.interaction_id,
            requested_at: request.requested_at,
            recorded_at: now,
            outcome: if decision.accepted {
                MirromereInteractionOutcome::Accepted
            } else {
                MirromereInteractionOutcome::Rejected
            },
            status: if decision.accepted {
                MirromereInteractionStatus::Requested
            } else {
                MirromereInteractionStatus::Rejected
            },
            requires_operator_action: decision.requires_operator_action,
            reason: decision.reason.to_string(),
        };
        let mut receipts = self
            .receipts
            .lock()
            .map_err(|_| "Mirromere interaction receipt state is unavailable".to_string())?;
        receipts.push_back(receipt.clone());
        while receipts.len() > MIRROMERE_INTERACTION_RECEIPT_LIMIT {
            receipts.pop_front();
        }
        Ok(receipt)
    }

    #[cfg(test)]
    fn snapshot(&self) -> Result<Vec<MirromereInteractionReceipt>, String> {
        self.receipts
            .lock()
            .map(|receipts| receipts.iter().cloned().collect())
            .map_err(|_| "Mirromere interaction receipt state is unavailable".to_string())
    }
}

fn interaction_requires_operator_action(interaction_id: MirromereInteractionId) -> bool {
    matches!(
        interaction_id,
        MirromereInteractionId::ContinueHandoff | MirromereInteractionId::DismissAttention
    )
}

fn registered_scene_interactions(scene_id: MirromereSceneId) -> &'static [MirromereInteractionId] {
    use MirromereInteractionId::{ContinueHandoff, DismissAttention, InspectProvenance};
    match scene_id {
        MirromereSceneId::AmbientIdle
        | MirromereSceneId::SystemStarting
        | MirromereSceneId::ConversationPresence
        | MirromereSceneId::ResearchFocus => &[InspectProvenance],
        MirromereSceneId::SystemDegraded => &[InspectProvenance, DismissAttention],
        MirromereSceneId::ContinuityHandoffReady => &[InspectProvenance, ContinueHandoff],
        MirromereSceneId::PrivacyVeil | MirromereSceneId::OfflineLocal => &[],
    }
}

fn evaluate_mirromere_interaction(
    request: &MirromereInteractionRequest,
    now: DateTime<Utc>,
) -> MirromereInteractionPolicyDecision {
    let requires_operator_action = interaction_requires_operator_action(request.interaction_id);
    if request.presented_privacy_class != request.surface.privacy.privacy_class
        || request.visibility_ceiling != request.surface.privacy.visibility_ceiling
    {
        return MirromereInteractionPolicyDecision {
            accepted: false,
            reason: "privacy_mismatch",
            requires_operator_action,
        };
    }
    if let Err(error) = request.surface.validate_at(now) {
        return MirromereInteractionPolicyDecision {
            accepted: false,
            reason: match error {
                MirromereSurfaceValidationError::Expired => "expired_surface",
                MirromereSurfaceValidationError::PrivacyEscalation => "privacy_mismatch",
                _ => "invalid_surface",
            },
            requires_operator_action,
        };
    }
    if request.requested_at < request.surface.generated_at
        || request.requested_at > request.surface.expires_at
        || request.requested_at > now + ChronoDuration::seconds(5)
    {
        return MirromereInteractionPolicyDecision {
            accepted: false,
            reason: "invalid_request_time",
            requires_operator_action,
        };
    }
    if !registered_scene_interactions(request.surface.scene.scene_id)
        .contains(&request.interaction_id)
    {
        return MirromereInteractionPolicyDecision {
            accepted: false,
            reason: "interaction_not_registered_for_scene",
            requires_operator_action,
        };
    }
    if !request
        .surface
        .allowed_interactions
        .contains(&request.interaction_id)
    {
        return MirromereInteractionPolicyDecision {
            accepted: false,
            reason: "interaction_not_registered_on_surface",
            requires_operator_action,
        };
    }
    if requires_operator_action && !request.explicit_operator_action {
        return MirromereInteractionPolicyDecision {
            accepted: false,
            reason: "explicit_operator_action_required",
            requires_operator_action,
        };
    }
    MirromereInteractionPolicyDecision {
        accepted: true,
        reason: "request_recorded",
        requires_operator_action,
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAggregateState {
    Stopped,
    Starting,
    Healthy,
    Degraded,
    Failed,
    Stopping,
    Unknown,
}

impl From<AggregateState> for LifecycleAggregateState {
    fn from(value: AggregateState) -> Self {
        match value {
            AggregateState::Stopped => Self::Stopped,
            AggregateState::Starting => Self::Starting,
            AggregateState::Healthy => Self::Healthy,
            AggregateState::Degraded => Self::Degraded,
            AggregateState::Failed => Self::Failed,
            AggregateState::Stopping => Self::Stopping,
            AggregateState::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LifecycleProjectionReference {
    pub aggregate_state: LifecycleAggregateState,
    pub observed_at: DateTime<Utc>,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityFreshness {
    Fresh,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityPrivacyClass {
    PublicRoom,
    SharedRoom,
    OperatorPrivate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandoffState {
    Requested,
    Prepared,
    Accepted,
    Active,
    Declined,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContinuityProjectionReference {
    pub generated_at: DateTime<Utc>,
    pub freshness: ContinuityFreshness,
    pub active: bool,
    pub privacy_class: Option<ContinuityPrivacyClass>,
    pub handoff_id: Option<String>,
    pub handoff_state: Option<HandoffState>,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereProjectionSourceMode {
    Runtime,
    Fixture,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MirromereProjectionInput {
    pub display_role: MirromereDisplayRole,
    pub source_mode: MirromereProjectionSourceMode,
    pub lifecycle: Option<LifecycleProjectionReference>,
    pub continuity: Option<ContinuityProjectionReference>,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MirromereProjectionError {
    #[error("fixture source mode is test-only and cannot enter the runtime projection command")]
    FixtureModeRejected,
    #[error("projected Mirromere surface is invalid: {0}")]
    InvalidSurface(String),
}

pub fn project_mirromere_surface_at(
    input: MirromereProjectionInput,
    now: DateTime<Utc>,
) -> Result<MirromereSurfaceProjection, MirromereProjectionError> {
    if input.source_mode != MirromereProjectionSourceMode::Runtime {
        return Err(MirromereProjectionError::FixtureModeRejected);
    }

    let lifecycle_stale = input.lifecycle.as_ref().is_some_and(|lifecycle| {
        now.signed_duration_since(lifecycle.observed_at)
            .num_seconds()
            > SOURCE_STALE_AFTER_SECONDS
    });
    let continuity_stale = input.continuity.as_ref().is_some_and(|continuity| {
        continuity.freshness == ContinuityFreshness::Stale
            || now
                .signed_duration_since(continuity.generated_at)
                .num_seconds()
                > SOURCE_STALE_AFTER_SECONDS
    });
    let unavailable = input.lifecycle.is_none();
    let continuity_unavailable = input
        .continuity
        .as_ref()
        .is_some_and(|continuity| continuity.freshness == ContinuityFreshness::Unavailable);

    let lifecycle_state = input
        .lifecycle
        .as_ref()
        .map(|lifecycle| lifecycle.aggregate_state)
        .unwrap_or(LifecycleAggregateState::Unknown);
    let private_for_display = input
        .continuity
        .as_ref()
        .and_then(|continuity| continuity.privacy_class)
        .is_some_and(|privacy| {
            privacy.rank() > visibility_rank(display_ceiling(input.display_role))
        });

    let scene_id = if unavailable {
        MirromereSceneId::OfflineLocal
    } else if private_for_display {
        MirromereSceneId::PrivacyVeil
    } else if continuity_unavailable {
        MirromereSceneId::SystemDegraded
    } else if lifecycle_stale || continuity_stale {
        MirromereSceneId::SystemDegraded
    } else {
        match lifecycle_state {
            LifecycleAggregateState::Stopped | LifecycleAggregateState::Stopping => {
                MirromereSceneId::OfflineLocal
            }
            LifecycleAggregateState::Starting => MirromereSceneId::SystemStarting,
            LifecycleAggregateState::Degraded
            | LifecycleAggregateState::Failed
            | LifecycleAggregateState::Unknown => MirromereSceneId::SystemDegraded,
            LifecycleAggregateState::Healthy => continuity_scene(input.continuity.as_ref()),
        }
    };

    let freshness = if unavailable {
        MirromereFreshness::Unavailable
    } else if continuity_unavailable {
        MirromereFreshness::Unavailable
    } else if lifecycle_stale || continuity_stale {
        MirromereFreshness::Stale
    } else {
        MirromereFreshness::Fresh
    };
    let availability = if unavailable
        || matches!(
            lifecycle_state,
            LifecycleAggregateState::Stopped | LifecycleAggregateState::Stopping
        ) {
        MirromereAvailability::Unavailable
    } else {
        MirromereAvailability::Available
    };

    let privacy_ceiling = display_ceiling(input.display_role);
    let privacy_class = if scene_id == MirromereSceneId::PrivacyVeil {
        MirromerePrivacyClass::PublicAmbient
    } else {
        input
            .continuity
            .as_ref()
            .and_then(|continuity| continuity.privacy_class)
            .map(Into::into)
            .unwrap_or(MirromerePrivacyClass::PublicAmbient)
    };
    let evidence = evidence_references(&input, now, private_for_display);
    let surface = MirromereSurfaceProjection {
        schema_version: MIRROMERE_SURFACE_SCHEMA_VERSION.to_string(),
        surface_id: match input.display_role {
            MirromereDisplayRole::HudAperture => "mirromere-hud-preview".to_string(),
            MirromereDisplayRole::NativeOutpost => "mirromere-native-outpost".to_string(),
        },
        outpost_id: match input.display_role {
            MirromereDisplayRole::HudAperture => "hud:monitor_3".to_string(),
            MirromereDisplayRole::NativeOutpost => "native:selected-display".to_string(),
        },
        display_role: input.display_role,
        source_mode: MirromereSourceMode::Runtime,
        scene: scene(scene_id),
        slots: slots(scene_id),
        evidence,
        generated_at: now,
        expires_at: now + ChronoDuration::seconds(SURFACE_TTL_SECONDS),
        freshness,
        availability,
        privacy: MirromerePrivacyPolicy {
            privacy_class,
            visibility_ceiling: privacy_ceiling,
        },
        allowed_interactions: interactions(scene_id),
        accessibility: accessibility(scene_id),
        transition: transition(scene_id),
    };
    surface
        .validate_at(now)
        .map_err(|error| MirromereProjectionError::InvalidSurface(error.to_string()))?;
    Ok(surface)
}

fn continuity_scene(continuity: Option<&ContinuityProjectionReference>) -> MirromereSceneId {
    let Some(continuity) = continuity else {
        return MirromereSceneId::AmbientIdle;
    };
    if matches!(
        continuity.handoff_state,
        Some(HandoffState::Requested | HandoffState::Prepared)
    ) && continuity.handoff_id.is_some()
    {
        MirromereSceneId::ContinuityHandoffReady
    } else if continuity.active {
        MirromereSceneId::ConversationPresence
    } else {
        MirromereSceneId::AmbientIdle
    }
}

fn display_ceiling(role: MirromereDisplayRole) -> MirromereVisibilityCeiling {
    match role {
        MirromereDisplayRole::HudAperture => MirromereVisibilityCeiling::PublicAmbient,
        MirromereDisplayRole::NativeOutpost => MirromereVisibilityCeiling::OperatorPrivate,
    }
}

impl ContinuityPrivacyClass {
    fn rank(self) -> u8 {
        match self {
            Self::PublicRoom => 0,
            Self::SharedRoom => 1,
            Self::OperatorPrivate => 2,
        }
    }
}

fn visibility_rank(value: MirromereVisibilityCeiling) -> u8 {
    match value {
        MirromereVisibilityCeiling::PublicAmbient => 0,
        MirromereVisibilityCeiling::SharedRoom => 1,
        MirromereVisibilityCeiling::OperatorPrivate => 2,
    }
}

impl From<ContinuityPrivacyClass> for MirromerePrivacyClass {
    fn from(value: ContinuityPrivacyClass) -> Self {
        match value {
            ContinuityPrivacyClass::PublicRoom => Self::PublicAmbient,
            ContinuityPrivacyClass::SharedRoom => Self::SharedRoom,
            ContinuityPrivacyClass::OperatorPrivate => Self::OperatorPrivate,
        }
    }
}

fn evidence_references(
    input: &MirromereProjectionInput,
    now: DateTime<Utc>,
    private_for_display: bool,
) -> Vec<MirromereEvidenceReference> {
    let mut evidence = Vec::new();
    if let Some(lifecycle) = &input.lifecycle {
        evidence.push(MirromereEvidenceReference {
            source_id: "arda.system-lifecycle.v1".to_string(),
            evidence_ref: lifecycle.evidence_ref.clone(),
            observed_at: lifecycle.observed_at,
        });
    } else {
        evidence.push(MirromereEvidenceReference {
            source_id: "arda.system-lifecycle.v1".to_string(),
            evidence_ref: "system-lifecycle://unavailable".to_string(),
            observed_at: now,
        });
    }
    if let Some(continuity) = &input.continuity {
        evidence.push(MirromereEvidenceReference {
            source_id: "arda.continuity-projection.v1".to_string(),
            evidence_ref: if private_for_display {
                "continuity://projection/withheld".to_string()
            } else {
                continuity.evidence_ref.clone()
            },
            observed_at: continuity.generated_at,
        });
    }
    evidence
}

fn scene(scene_id: MirromereSceneId) -> MirromereScene {
    let (application_id, purpose) = match scene_id {
        MirromereSceneId::AmbientIdle => ("arda.mirromere.ambient", "Calm local readiness"),
        MirromereSceneId::SystemStarting => ("arda.mirromere.system", "Local system starting"),
        MirromereSceneId::SystemDegraded => (
            "arda.mirromere.system",
            "Bounded degraded system indication",
        ),
        MirromereSceneId::ConversationPresence => {
            ("arda.mirromere.continuity", "Conversation presence")
        }
        MirromereSceneId::ContinuityHandoffReady => {
            ("arda.mirromere.continuity", "Authenticated handoff ready")
        }
        MirromereSceneId::ResearchFocus => ("arda.mirromere.research", "Research focus"),
        MirromereSceneId::PrivacyVeil => ("arda.mirromere.privacy", "Private content veiled"),
        MirromereSceneId::OfflineLocal => ("arda.mirromere.offline", "Local source unavailable"),
    };
    MirromereScene {
        scene_id,
        application_id: application_id.to_string(),
        application_version: "1.0.0".to_string(),
        purpose: purpose.to_string(),
    }
}

fn slots(scene_id: MirromereSceneId) -> Vec<MirromereSlot> {
    let content = match scene_id {
        MirromereSceneId::AmbientIdle => MirromereSlotContent::VectorField {
            field: MirromereVectorFieldKind::Wave,
            samples: vec![0.0, 0.2, 0.0, -0.2, 0.0],
        },
        MirromereSceneId::SystemStarting => MirromereSlotContent::VectorField {
            field: MirromereVectorFieldKind::Radar,
            samples: vec![0.1, 0.3, 0.5, 0.3, 0.1],
        },
        MirromereSceneId::SystemDegraded => MirromereSlotContent::Status {
            label: "LOCAL MESH".to_string(),
            state: "DEGRADED".to_string(),
        },
        MirromereSceneId::ConversationPresence => MirromereSlotContent::ConversationPresence {
            participant_ref: "operator:authenticated".to_string(),
            phase: MirromerePresencePhase::Listening,
        },
        MirromereSceneId::ContinuityHandoffReady => MirromereSlotContent::AppView {
            view_id: "continuity".to_string(),
        },
        MirromereSceneId::ResearchFocus => MirromereSlotContent::AppView {
            view_id: "research_focus".to_string(),
        },
        MirromereSceneId::PrivacyVeil => MirromereSlotContent::VectorField {
            field: MirromereVectorFieldKind::Wave,
            samples: vec![0.0, 0.05, 0.0, -0.05, 0.0],
        },
        MirromereSceneId::OfflineLocal => MirromereSlotContent::Status {
            label: "LOCAL".to_string(),
            state: "UNAVAILABLE".to_string(),
        },
    };
    vec![MirromereSlot {
        id: "primary".to_string(),
        content,
    }]
}

fn interactions(scene_id: MirromereSceneId) -> Vec<MirromereInteractionId> {
    match scene_id {
        MirromereSceneId::ContinuityHandoffReady => vec![
            MirromereInteractionId::InspectProvenance,
            MirromereInteractionId::ContinueHandoff,
        ],
        MirromereSceneId::PrivacyVeil | MirromereSceneId::OfflineLocal => Vec::new(),
        MirromereSceneId::SystemDegraded => vec![
            MirromereInteractionId::InspectProvenance,
            MirromereInteractionId::DismissAttention,
        ],
        _ => vec![MirromereInteractionId::InspectProvenance],
    }
}

fn accessibility(scene_id: MirromereSceneId) -> MirromereAccessibility {
    let (description, urgency) = match scene_id {
        MirromereSceneId::SystemDegraded => {
            ("Local services are degraded.", MirromereUrgency::Normal)
        }
        MirromereSceneId::ContinuityHandoffReady => (
            "An authenticated handoff is ready.",
            MirromereUrgency::Normal,
        ),
        MirromereSceneId::PrivacyVeil => (
            "Private content is hidden on this display.",
            MirromereUrgency::Ambient,
        ),
        MirromereSceneId::OfflineLocal => (
            "The local projection source is unavailable.",
            MirromereUrgency::Normal,
        ),
        _ => ("Mirromere ambient presentation.", MirromereUrgency::Ambient),
    };
    MirromereAccessibility {
        description: description.to_string(),
        reduced_motion: MirromereReducedMotion::Freeze,
        urgency,
    }
}

fn transition(scene_id: MirromereSceneId) -> MirromereTransitionPolicy {
    MirromereTransitionPolicy {
        style: if matches!(scene_id, MirromereSceneId::SystemDegraded) {
            MirromereTransitionStyle::Sweep
        } else {
            MirromereTransitionStyle::Fade
        },
        duration_ms: 600,
        attention_budget: if matches!(scene_id, MirromereSceneId::SystemDegraded) {
            2
        } else {
            1
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HarnessContinuityProjection {
    schema_version: String,
    generated_at: DateTime<Utc>,
    active: bool,
    session_lineage_id: Option<String>,
    current_session_id: Option<String>,
    surface_id: Option<String>,
    privacy_class: Option<String>,
    freshness: String,
    handoff_id: Option<String>,
    handoff_state: Option<String>,
    action_ids: Vec<String>,
    private_refs_withheld: bool,
    topic_refs: Vec<String>,
    commitment_refs: Vec<String>,
    memory_scope_refs: Vec<String>,
}

impl HarnessContinuityProjection {
    fn into_reference(self) -> Result<ContinuityProjectionReference, String> {
        if self.schema_version != "arda.continuity-projection.v1" {
            return Err("unsupported continuity projection schema".to_string());
        }
        let freshness = match self.freshness.as_str() {
            "fresh" => ContinuityFreshness::Fresh,
            "stale" => ContinuityFreshness::Stale,
            "unavailable" => ContinuityFreshness::Unavailable,
            _ => return Err("invalid continuity freshness".to_string()),
        };
        let privacy_class = match self.privacy_class.as_deref() {
            Some("public_room") | None => Some(ContinuityPrivacyClass::PublicRoom),
            Some("shared_room") => Some(ContinuityPrivacyClass::SharedRoom),
            Some("private_room" | "personal_device") => {
                Some(ContinuityPrivacyClass::OperatorPrivate)
            }
            Some(_) => return Err("invalid continuity privacy class".to_string()),
        };
        let handoff_state = match self.handoff_state.as_deref() {
            None => None,
            Some("requested") => Some(HandoffState::Requested),
            Some("prepared") => Some(HandoffState::Prepared),
            Some("accepted") => Some(HandoffState::Accepted),
            Some("active") => Some(HandoffState::Active),
            Some("declined") => Some(HandoffState::Declined),
            Some("expired") => Some(HandoffState::Expired),
            Some("failed") => Some(HandoffState::Failed),
            Some(_) => return Err("invalid continuity handoff state".to_string()),
        };
        if self.private_refs_withheld
            && (!self.topic_refs.is_empty()
                || !self.commitment_refs.is_empty()
                || !self.memory_scope_refs.is_empty())
        {
            return Err("private continuity references were not withheld".to_string());
        }
        let identity = self
            .handoff_id
            .as_deref()
            .or(self.current_session_id.as_deref())
            .or(self.session_lineage_id.as_deref())
            .unwrap_or("current")
            .to_string();
        let _bounded_transport_fields = (self.surface_id, self.action_ids);
        Ok(ContinuityProjectionReference {
            generated_at: self.generated_at,
            freshness,
            active: self.active,
            privacy_class,
            handoff_id: self.handoff_id,
            handoff_state,
            evidence_ref: format!("continuity://projection/{identity}"),
        })
    }
}

async fn load_continuity_reference() -> ContinuityProjectionReference {
    let now = Utc::now();
    let operator_id = match std::env::var("ARDA_OPERATOR_ID") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return unavailable_continuity(now),
    };
    let base = std::env::var("ARDA_HARNESS_URL")
        .unwrap_or_else(|_| DEFAULT_HARNESS_URL.to_string())
        .trim_end_matches('/')
        .to_string();
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(_) => return unavailable_continuity(now),
    };
    let response = match client
        .get(format!("{base}/v1/continuity/projection"))
        .header("x-arda-operator-id", operator_id.trim())
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => response,
        _ => return unavailable_continuity(now),
    };
    match response.json::<HarnessContinuityProjection>().await {
        Ok(projection) => projection
            .into_reference()
            .unwrap_or_else(|_| unavailable_continuity(now)),
        Err(_) => unavailable_continuity(now),
    }
}

fn unavailable_continuity(now: DateTime<Utc>) -> ContinuityProjectionReference {
    ContinuityProjectionReference {
        generated_at: now,
        freshness: ContinuityFreshness::Unavailable,
        active: false,
        privacy_class: None,
        handoff_id: None,
        handoff_state: None,
        evidence_ref: "continuity://unavailable".to_string(),
    }
}

#[tauri::command]
pub async fn get_mirromere_surface(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    display_role: MirromereDisplayRole,
) -> Result<MirromereSurfaceProjection, String> {
    let lifecycle = tauri::async_runtime::spawn_blocking(lifecycle_status)
        .await
        .map_err(|error| format!("lifecycle observation failed: {error}"))?;
    let lifecycle_reference = LifecycleProjectionReference {
        aggregate_state: lifecycle.aggregate_state.into(),
        observed_at: lifecycle.observed_at,
        evidence_ref: format!("system-lifecycle://{}", lifecycle.observed_at.to_rfc3339()),
    };
    let input = MirromereProjectionInput {
        display_role,
        source_mode: MirromereProjectionSourceMode::Runtime,
        lifecycle: Some(lifecycle_reference),
        continuity: Some(load_continuity_reference().await),
    };
    let surface =
        project_mirromere_surface_at(input, Utc::now()).map_err(|error| error.to_string())?;
    state.remember_surface(surface.clone())?;
    Ok(surface)
}

#[tauri::command]
pub fn request_mirromere_interaction(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    request: MirromereInteractionRequest,
) -> Result<MirromereInteractionReceipt, String> {
    state.record(request, Utc::now())
}

#[cfg(test)]
mod mirromere_interaction_tests {
    use super::*;
    use serde_json::json;

    fn surface_at(
        now: DateTime<Utc>,
        lifecycle_state: LifecycleAggregateState,
        continuity: Option<ContinuityProjectionReference>,
    ) -> MirromereSurfaceProjection {
        project_mirromere_surface_at(
            MirromereProjectionInput {
                display_role: MirromereDisplayRole::NativeOutpost,
                source_mode: MirromereProjectionSourceMode::Runtime,
                lifecycle: Some(LifecycleProjectionReference {
                    aggregate_state: lifecycle_state,
                    observed_at: now,
                    evidence_ref: "lifecycle://test".to_string(),
                }),
                continuity,
            },
            now,
        )
        .expect("surface")
    }

    fn request(
        surface: MirromereSurfaceProjection,
        interaction_id: MirromereInteractionId,
        explicit_operator_action: bool,
        requested_at: DateTime<Utc>,
    ) -> MirromereInteractionRequest {
        MirromereInteractionRequest {
            presented_privacy_class: surface.privacy.privacy_class,
            visibility_ceiling: surface.privacy.visibility_ceiling,
            surface,
            interaction_id,
            requested_at,
            explicit_operator_action,
        }
    }

    #[test]
    fn strict_request_rejects_unknown_scene_id() {
        let now = DateTime::parse_from_rfc3339("2026-08-19T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let surface = surface_at(now, LifecycleAggregateState::Healthy, None);
        let mut value = serde_json::to_value(request(
            surface,
            MirromereInteractionId::InspectProvenance,
            false,
            now,
        ))
        .unwrap();
        value["surface"]["scene"]["scene_id"] = json!("unknown.scene");
        assert!(serde_json::from_value::<MirromereInteractionRequest>(value).is_err());
    }

    #[test]
    fn rejects_unregistered_privacy_mismatch_and_expired_requests() {
        let now = DateTime::parse_from_rfc3339("2026-08-19T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let surface = surface_at(now, LifecycleAggregateState::Healthy, None);
        let unregistered = evaluate_mirromere_interaction(
            &request(
                surface.clone(),
                MirromereInteractionId::ContinueHandoff,
                true,
                now,
            ),
            now,
        );
        assert_eq!(unregistered.reason, "interaction_not_registered_for_scene");

        let mut privacy = request(
            surface.clone(),
            MirromereInteractionId::InspectProvenance,
            false,
            now,
        );
        privacy.presented_privacy_class = MirromerePrivacyClass::SharedRoom;
        assert_eq!(
            evaluate_mirromere_interaction(&privacy, now).reason,
            "privacy_mismatch"
        );

        let expired_at = now + ChronoDuration::seconds(SURFACE_TTL_SECONDS + 1);
        assert_eq!(
            evaluate_mirromere_interaction(
                &request(
                    surface,
                    MirromereInteractionId::InspectProvenance,
                    false,
                    now,
                ),
                expired_at,
            )
            .reason,
            "expired_surface"
        );
    }

    #[test]
    fn records_requested_not_success_and_requires_explicit_mutation() {
        let now = DateTime::parse_from_rfc3339("2026-08-19T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let store = MirromereInteractionReceiptState::default();
        let ambient = surface_at(now, LifecycleAggregateState::Healthy, None);
        store.remember_surface(ambient.clone()).unwrap();
        let inspect = store
            .record(
                request(
                    ambient,
                    MirromereInteractionId::InspectProvenance,
                    false,
                    now,
                ),
                now,
            )
            .unwrap();
        assert_eq!(inspect.outcome, MirromereInteractionOutcome::Accepted);
        assert_eq!(inspect.status, MirromereInteractionStatus::Requested);

        let handoff = surface_at(
            now,
            LifecycleAggregateState::Healthy,
            Some(ContinuityProjectionReference {
                generated_at: now,
                freshness: ContinuityFreshness::Fresh,
                active: false,
                privacy_class: Some(ContinuityPrivacyClass::OperatorPrivate),
                handoff_id: Some("handoff-test".to_string()),
                handoff_state: Some(HandoffState::Prepared),
                evidence_ref: "continuity://test".to_string(),
            }),
        );
        store.remember_surface(handoff.clone()).unwrap();
        let rejected = store
            .record(
                request(
                    handoff.clone(),
                    MirromereInteractionId::ContinueHandoff,
                    false,
                    now,
                ),
                now,
            )
            .unwrap();
        assert_eq!(rejected.outcome, MirromereInteractionOutcome::Rejected);
        assert_eq!(rejected.reason, "explicit_operator_action_required");

        let accepted = store
            .record(
                request(handoff, MirromereInteractionId::ContinueHandoff, true, now),
                now,
            )
            .unwrap();
        assert_eq!(accepted.status, MirromereInteractionStatus::Requested);

        let degraded = surface_at(now, LifecycleAggregateState::Degraded, None);
        store.remember_surface(degraded.clone()).unwrap();
        let dismiss = store
            .record(
                request(
                    degraded,
                    MirromereInteractionId::DismissAttention,
                    false,
                    now,
                ),
                now,
            )
            .unwrap();
        assert_eq!(dismiss.outcome, MirromereInteractionOutcome::Rejected);
        assert_eq!(dismiss.reason, "explicit_operator_action_required");
    }

    #[test]
    fn receipt_store_is_bounded_to_128_entries() {
        let now = DateTime::parse_from_rfc3339("2026-08-19T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let store = MirromereInteractionReceiptState::default();
        let surface = surface_at(now, LifecycleAggregateState::Healthy, None);
        store.remember_surface(surface.clone()).unwrap();
        for _ in 0..140 {
            store
                .record(
                    request(
                        surface.clone(),
                        MirromereInteractionId::InspectProvenance,
                        false,
                        now,
                    ),
                    now,
                )
                .unwrap();
        }
        assert_eq!(store.snapshot().unwrap().len(), 128);
    }

    #[test]
    fn rejects_surface_not_issued_by_backend() {
        let now = DateTime::parse_from_rfc3339("2026-08-19T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let store = MirromereInteractionReceiptState::default();
        let receipt = store
            .record(
                request(
                    surface_at(now, LifecycleAggregateState::Healthy, None),
                    MirromereInteractionId::InspectProvenance,
                    false,
                    now,
                ),
                now,
            )
            .unwrap();
        assert_eq!(receipt.outcome, MirromereInteractionOutcome::Rejected);
        assert_eq!(receipt.reason, "surface_not_current");
    }
}
