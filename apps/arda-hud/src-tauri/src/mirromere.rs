use arda_launcher::lifecycle::commands::lifecycle_status;
use arda_launcher::lifecycle::types::AggregateState;
use arda_outpost_protocol::{
    MirromereAccessibility, MirromereAvailability, MirromereDisplayRole,
    MirromereEvidenceReference, MirromereFreshness, MirromereInteractionId, MirromerePresencePhase,
    MirromerePrivacyClass, MirromerePrivacyPolicy, MirromereReducedMotion, MirromereScene,
    MirromereSceneId, MirromereSlot, MirromereSlotContent, MirromereSourceMode,
    MirromereSurfaceProjection, MirromereTransitionPolicy, MirromereTransitionStyle,
    MirromereUrgency, MirromereVectorFieldKind, MirromereVisibilityCeiling,
    MIRROMERE_SURFACE_SCHEMA_VERSION,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const DEFAULT_HARNESS_URL: &str = "http://127.0.0.1:7878";
const SOURCE_STALE_AFTER_SECONDS: i64 = 30;
const SURFACE_TTL_SECONDS: i64 = 30;

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
    project_mirromere_surface_at(input, Utc::now()).map_err(|error| error.to_string())
}
