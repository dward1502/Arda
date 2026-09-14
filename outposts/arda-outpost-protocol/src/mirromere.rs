//! Strict, presentation-only contract shared by Mirromere renderers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const MIRROMERE_SURFACE_SCHEMA_VERSION: &str = "arda.mirromere.surface.v1";
pub const MIRROMERE_MAX_SLOTS: usize = 12;
pub const MIRROMERE_MAX_TEXT_BYTES: usize = 1024;
pub const MIRROMERE_MAX_PURPOSE_BYTES: usize = 256;
pub const MIRROMERE_MAX_ACCESSIBILITY_BYTES: usize = 512;
pub const MIRROMERE_MAX_VECTOR_SAMPLES: usize = 256;
pub const MIRROMERE_MAX_TRANSITION_MS: u16 = 2_000;
pub const MIRROMERE_MAX_ATTENTION_BUDGET: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereSurfaceProjection {
    pub schema_version: String,
    pub surface_id: String,
    pub outpost_id: String,
    pub display_role: MirromereDisplayRole,
    pub source_mode: MirromereSourceMode,
    pub scene: MirromereScene,
    pub slots: Vec<MirromereSlot>,
    pub evidence: Vec<MirromereEvidenceReference>,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub freshness: MirromereFreshness,
    pub availability: MirromereAvailability,
    pub privacy: MirromerePrivacyPolicy,
    pub allowed_interactions: Vec<MirromereInteractionId>,
    pub accessibility: MirromereAccessibility,
    pub transition: MirromereTransitionPolicy,
}

impl MirromereSurfaceProjection {
    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), MirromereSurfaceValidationError> {
        if self.schema_version != MIRROMERE_SURFACE_SCHEMA_VERSION {
            return Err(MirromereSurfaceValidationError::UnsupportedSchema);
        }
        non_empty(&self.surface_id)?;
        reject_unsafe(&self.surface_id)?;
        non_empty(&self.outpost_id)?;
        reject_unsafe(&self.outpost_id)?;
        if self.generated_at > self.expires_at {
            return Err(MirromereSurfaceValidationError::InvalidValidityWindow);
        }
        if now > self.expires_at {
            return Err(MirromereSurfaceValidationError::Expired);
        }
        if self.slots.is_empty() || self.slots.len() > MIRROMERE_MAX_SLOTS {
            return Err(MirromereSurfaceValidationError::TooManySlots);
        }
        if self.evidence.is_empty() {
            return Err(MirromereSurfaceValidationError::MissingEvidence);
        }
        if self.privacy.privacy_class.rank() > self.privacy.visibility_ceiling.rank() {
            return Err(MirromereSurfaceValidationError::PrivacyEscalation);
        }
        if self.transition.duration_ms > MIRROMERE_MAX_TRANSITION_MS
            || self.transition.attention_budget > MIRROMERE_MAX_ATTENTION_BUDGET
        {
            return Err(MirromereSurfaceValidationError::InvalidAttentionPolicy);
        }
        bounded(&self.scene.application_id, MIRROMERE_MAX_PURPOSE_BYTES)?;
        reject_unsafe(&self.scene.application_id)?;
        bounded(&self.scene.application_version, 64)?;
        bounded(&self.scene.purpose, MIRROMERE_MAX_PURPOSE_BYTES)?;
        reject_unsafe(&self.scene.purpose)?;
        bounded(
            &self.accessibility.description,
            MIRROMERE_MAX_ACCESSIBILITY_BYTES,
        )?;
        reject_unsafe(&self.accessibility.description)?;

        let mut slot_ids = HashSet::new();
        for slot in &self.slots {
            bounded(&slot.id, 64)?;
            if !slot_ids.insert(slot.id.as_str()) {
                return Err(MirromereSurfaceValidationError::DuplicateSlot);
            }
            slot.validate()?;
        }
        for evidence in &self.evidence {
            bounded(&evidence.source_id, 128)?;
            bounded(&evidence.evidence_ref, 256)?;
        }
        let mut interactions = HashSet::new();
        if !self
            .allowed_interactions
            .iter()
            .all(|interaction| interactions.insert(*interaction))
        {
            return Err(MirromereSurfaceValidationError::DuplicateInteraction);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereDisplayRole {
    HudAperture,
    NativeOutpost,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereSourceMode {
    Runtime,
    Fixture,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereScene {
    pub scene_id: MirromereSceneId,
    pub application_id: String,
    pub application_version: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MirromereSceneId {
    #[serde(rename = "ambient.idle")]
    AmbientIdle,
    #[serde(rename = "system.starting")]
    SystemStarting,
    #[serde(rename = "system.degraded")]
    SystemDegraded,
    #[serde(rename = "conversation.presence")]
    ConversationPresence,
    #[serde(rename = "continuity.handoff-ready")]
    ContinuityHandoffReady,
    #[serde(rename = "research.focus")]
    ResearchFocus,
    #[serde(rename = "privacy.veil")]
    PrivacyVeil,
    #[serde(rename = "offline.local")]
    OfflineLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereSlot {
    pub id: String,
    pub content: MirromereSlotContent,
}

impl MirromereSlot {
    fn validate(&self) -> Result<(), MirromereSurfaceValidationError> {
        match &self.content {
            MirromereSlotContent::Status { label, state } => {
                bounded(label, 128)?;
                bounded(state, 128)?;
                reject_unsafe(label)?;
                reject_unsafe(state)?;
            }
            MirromereSlotContent::Text { text } => {
                bounded(text, MIRROMERE_MAX_TEXT_BYTES)?;
                reject_unsafe(text)?;
            }
            MirromereSlotContent::MediaRef {
                asset_id,
                digest,
                mime_type,
            } => {
                bounded(asset_id, 128)?;
                if !valid_sha256(digest) || !allowed_mime(mime_type) {
                    return Err(MirromereSurfaceValidationError::UnsafeContent);
                }
            }
            MirromereSlotContent::VectorField { field, samples } => {
                if samples.is_empty()
                    || samples.len() > MIRROMERE_MAX_VECTOR_SAMPLES
                    || samples
                        .iter()
                        .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
                {
                    return Err(MirromereSurfaceValidationError::InvalidVectorField);
                }
                let _ = field;
            }
            MirromereSlotContent::ConversationPresence {
                participant_ref,
                phase: _,
            } => bounded(participant_ref, 128)?,
            MirromereSlotContent::AppView { view_id } => {
                if !matches!(
                    view_id.as_str(),
                    "system_status" | "continuity" | "research_focus"
                ) {
                    return Err(MirromereSurfaceValidationError::UnregisteredAppView);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MirromereSlotContent {
    Status {
        label: String,
        state: String,
    },
    Text {
        text: String,
    },
    MediaRef {
        asset_id: String,
        digest: String,
        mime_type: String,
    },
    VectorField {
        field: MirromereVectorFieldKind,
        samples: Vec<f32>,
    },
    ConversationPresence {
        participant_ref: String,
        phase: MirromerePresencePhase,
    },
    AppView {
        view_id: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereVectorFieldKind {
    Vector,
    Radar,
    Wave,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromerePresencePhase {
    Listening,
    Thinking,
    Responding,
    Waiting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereEvidenceReference {
    pub source_id: String,
    pub evidence_ref: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereFreshness {
    Fresh,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromerePrivacyPolicy {
    pub privacy_class: MirromerePrivacyClass,
    pub visibility_ceiling: MirromereVisibilityCeiling,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromerePrivacyClass {
    PublicAmbient,
    SharedRoom,
    OperatorPrivate,
}

impl MirromerePrivacyClass {
    fn rank(self) -> u8 {
        match self {
            Self::PublicAmbient => 0,
            Self::SharedRoom => 1,
            Self::OperatorPrivate => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereVisibilityCeiling {
    PublicAmbient,
    SharedRoom,
    OperatorPrivate,
}

impl MirromereVisibilityCeiling {
    fn rank(self) -> u8 {
        match self {
            Self::PublicAmbient => 0,
            Self::SharedRoom => 1,
            Self::OperatorPrivate => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MirromereInteractionId {
    InspectProvenance,
    ContinueHandoff,
    DismissAttention,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereAccessibility {
    pub description: String,
    pub reduced_motion: MirromereReducedMotion,
    pub urgency: MirromereUrgency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereReducedMotion {
    Freeze,
    Simplify,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereUrgency {
    Ambient,
    Normal,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MirromereTransitionPolicy {
    pub style: MirromereTransitionStyle,
    pub duration_ms: u16,
    pub attention_budget: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirromereTransitionStyle {
    Cut,
    Fade,
    Sweep,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MirromereSurfaceValidationError {
    #[error("unsupported Mirromere schema")]
    UnsupportedSchema,
    #[error("Mirromere surface expired")]
    Expired,
    #[error("invalid Mirromere validity window")]
    InvalidValidityWindow,
    #[error("Mirromere privacy exceeds visibility ceiling")]
    PrivacyEscalation,
    #[error("Mirromere surface has too many or no slots")]
    TooManySlots,
    #[error("Mirromere surface is missing evidence")]
    MissingEvidence,
    #[error("Mirromere content is unsafe")]
    UnsafeContent,
    #[error("Mirromere value is empty or exceeds its bound")]
    InvalidBoundedValue,
    #[error("Mirromere slot ids must be unique")]
    DuplicateSlot,
    #[error("Mirromere interactions must be unique")]
    DuplicateInteraction,
    #[error("Mirromere vector field is invalid")]
    InvalidVectorField,
    #[error("Mirromere app view is not registered")]
    UnregisteredAppView,
    #[error("Mirromere attention policy is invalid")]
    InvalidAttentionPolicy,
}

fn non_empty(value: &str) -> Result<(), MirromereSurfaceValidationError> {
    bounded(value, 128)
}

fn bounded(value: &str, max_bytes: usize) -> Result<(), MirromereSurfaceValidationError> {
    if value.trim().is_empty() || value.len() > max_bytes {
        Err(MirromereSurfaceValidationError::InvalidBoundedValue)
    } else {
        Ok(())
    }
}

fn reject_unsafe(value: &str) -> Result<(), MirromereSurfaceValidationError> {
    let lower = value.to_ascii_lowercase();
    if value.contains('<')
        || value.contains('>')
        || lower.contains("javascript:")
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("rm -rf")
        || lower.contains("sh -c")
    {
        Err(MirromereSurfaceValidationError::UnsafeContent)
    } else {
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn allowed_mime(value: &str) -> bool {
    matches!(
        value,
        "image/png" | "image/jpeg" | "video/mp4" | "audio/mpeg"
    )
}
