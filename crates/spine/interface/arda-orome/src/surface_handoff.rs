//! Strict, transcript-free surface handoff contract shared by Hermes and Arda.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const SURFACE_HANDOFF_SCHEMA_VERSION: &str = "arda.surface-handoff.v1";
const MAX_ID: usize = 256;
const MAX_REASON: usize = 512;
const MAX_REFS: usize = 32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DataDomain {
    System,
    Personal,
    Business,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    PublicRoom,
    SharedRoom,
    PrivateRoom,
    PersonalDevice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsentState {
    Requested,
    Granted,
    Declined,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HandoffConsent {
    pub state: ConsentState,
    pub requesting_actor: String,
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

impl HandoffState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Requested,
                Self::Prepared | Self::Declined | Self::Expired | Self::Failed
            ) | (
                Self::Prepared,
                Self::Accepted | Self::Declined | Self::Expired | Self::Failed
            ) | (Self::Accepted, Self::Active | Self::Expired | Self::Failed)
        )
    }

    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Active | Self::Declined | Self::Expired | Self::Failed
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SurfaceHandoff {
    pub schema_version: String,
    pub handoff_id: String,
    pub operator_ref: String,
    pub session_lineage_id: String,
    pub current_session_id: String,
    pub source_surface_id: String,
    pub destination_surface_id: String,
    #[serde(default)]
    pub topic_refs: Vec<String>,
    #[serde(default)]
    pub commitment_refs: Vec<String>,
    #[serde(default)]
    pub memory_scope_refs: Vec<String>,
    pub authorized_domains: Vec<DataDomain>,
    pub requested_domains: Vec<DataDomain>,
    pub privacy_class: PrivacyClass,
    pub consent: HandoffConsent,
    pub state: HandoffState,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub accepted_at: Option<DateTime<Utc>>,
    pub idempotency_key: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SurfaceHandoffError {
    #[error("unsupported surface handoff schema")]
    UnsupportedSchema,
    #[error("missing required identity: {0}")]
    MissingIdentity(&'static str),
    #[error("surface handoff field is out of bounds: {0}")]
    OutOfBounds(&'static str),
    #[error("surface handoff is expired")]
    Expired,
    #[error("requested data domain exceeds authorized domains")]
    DomainEscalation,
    #[error("surface handoff replay identity changed")]
    ReplayMismatch,
    #[error("illegal surface handoff transition")]
    IllegalTransition,
    #[error("invalid handoff consent or acceptance state")]
    InvalidConsent,
}

impl SurfaceHandoff {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), SurfaceHandoffError> {
        if self.schema_version != SURFACE_HANDOFF_SCHEMA_VERSION {
            return Err(SurfaceHandoffError::UnsupportedSchema);
        }
        for (label, value) in [
            ("handoff_id", self.handoff_id.as_str()),
            ("operator_ref", self.operator_ref.as_str()),
            ("session_lineage_id", self.session_lineage_id.as_str()),
            ("current_session_id", self.current_session_id.as_str()),
            ("source_surface_id", self.source_surface_id.as_str()),
            (
                "destination_surface_id",
                self.destination_surface_id.as_str(),
            ),
            ("requesting_actor", self.consent.requesting_actor.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(SurfaceHandoffError::MissingIdentity(label));
            }
            if value.len() > MAX_ID {
                return Err(SurfaceHandoffError::OutOfBounds(label));
            }
        }
        if self.expires_at <= self.issued_at || self.expires_at <= now {
            return Err(SurfaceHandoffError::Expired);
        }
        if self.requested_domains.is_empty()
            || self.authorized_domains.is_empty()
            || self
                .requested_domains
                .iter()
                .any(|domain| !self.authorized_domains.contains(domain))
        {
            return Err(SurfaceHandoffError::DomainEscalation);
        }
        validate_refs("topic_refs", &self.topic_refs)?;
        validate_refs("commitment_refs", &self.commitment_refs)?;
        validate_refs("memory_scope_refs", &self.memory_scope_refs)?;
        validate_refs("receipt_refs", &self.receipt_refs)?;
        if self
            .reason
            .as_ref()
            .is_some_and(|value| value.len() > MAX_REASON)
            || self
                .error
                .as_ref()
                .is_some_and(|value| value.len() > MAX_REASON)
        {
            return Err(SurfaceHandoffError::OutOfBounds("reason_or_error"));
        }
        let replay = self.idempotency_key.strip_prefix("sha256:");
        if !replay.is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(SurfaceHandoffError::OutOfBounds("idempotency_key"));
        }
        if matches!(self.state, HandoffState::Accepted | HandoffState::Active)
            && (!matches!(self.consent.state, ConsentState::Granted) || self.accepted_at.is_none())
        {
            return Err(SurfaceHandoffError::InvalidConsent);
        }
        Ok(())
    }

    pub fn validate_replay(&self, replay: &Self) -> Result<(), SurfaceHandoffError> {
        if self.handoff_id != replay.handoff_id
            || self.operator_ref != replay.operator_ref
            || self.session_lineage_id != replay.session_lineage_id
            || self.current_session_id != replay.current_session_id
            || self.source_surface_id != replay.source_surface_id
            || self.destination_surface_id != replay.destination_surface_id
            || self.idempotency_key != replay.idempotency_key
        {
            return Err(SurfaceHandoffError::ReplayMismatch);
        }
        Ok(())
    }

    pub fn validate_transition(&self, next: &Self) -> Result<(), SurfaceHandoffError> {
        self.validate_replay(next)?;
        if !self.state.can_transition_to(next.state) {
            return Err(SurfaceHandoffError::IllegalTransition);
        }
        Ok(())
    }
}

fn validate_refs(label: &'static str, refs: &[String]) -> Result<(), SurfaceHandoffError> {
    if refs.len() > MAX_REFS
        || refs
            .iter()
            .any(|reference| reference.trim().is_empty() || reference.len() > MAX_ID)
    {
        return Err(SurfaceHandoffError::OutOfBounds(label));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json::{json, Value};

    fn now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 17, 20, 0, 0).unwrap()
    }

    fn valid_value() -> Value {
        json!({
            "schema_version": "arda.surface-handoff.v1",
            "handoff_id": "handoff-01",
            "operator_ref": "operator:local-primary",
            "session_lineage_id": "lineage-discord-01",
            "current_session_id": "20260809_014842_aaaaa1c1",
            "source_surface_id": "discord:private-chat",
            "destination_surface_id": "desktop:arda-hud",
            "topic_refs": ["topic:phase-2"],
            "commitment_refs": ["commitment:complete-phase-2"],
            "memory_scope_refs": ["vaire:scope:system-continuity"],
            "authorized_domains": ["system"],
            "requested_domains": ["system"],
            "privacy_class": "personal_device",
            "consent": {
                "state": "requested",
                "requesting_actor": "operator:local-primary"
            },
            "state": "requested",
            "issued_at": "2026-08-17T19:59:00Z",
            "expires_at": "2026-08-17T20:05:00Z",
            "accepted_at": null,
            "idempotency_key": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "reason": "continue on desktop",
            "error": null,
            "receipt_refs": []
        })
    }

    #[test]
    fn valid_request_deserializes_and_validates() {
        let request: SurfaceHandoff = serde_json::from_value(valid_value()).unwrap();
        request.validate(now()).unwrap();
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = valid_value();
        value["transcript"] = json!("must not cross this boundary");
        assert!(serde_json::from_value::<SurfaceHandoff>(value).is_err());
    }

    #[test]
    fn missing_operator_or_session_identity_is_rejected() {
        for key in ["operator_ref", "session_lineage_id", "current_session_id"] {
            let mut value = valid_value();
            value[key] = json!("");
            let request: SurfaceHandoff = serde_json::from_value(value).unwrap();
            assert!(matches!(
                request.validate(now()),
                Err(SurfaceHandoffError::MissingIdentity(_))
            ));
        }
    }

    #[test]
    fn expired_request_is_rejected() {
        let mut value = valid_value();
        value["expires_at"] = json!("2026-08-17T19:59:59Z");
        let request: SurfaceHandoff = serde_json::from_value(value).unwrap();
        assert_eq!(request.validate(now()), Err(SurfaceHandoffError::Expired));
    }

    #[test]
    fn requested_domain_cannot_escalate_authorized_domain() {
        let mut value = valid_value();
        value["requested_domains"] = json!(["system", "business"]);
        let request: SurfaceHandoff = serde_json::from_value(value).unwrap();
        assert_eq!(
            request.validate(now()),
            Err(SurfaceHandoffError::DomainEscalation)
        );
    }

    #[test]
    fn altered_replay_key_is_rejected() {
        let original: SurfaceHandoff = serde_json::from_value(valid_value()).unwrap();
        let mut altered = original.clone();
        altered.idempotency_key =
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
        assert_eq!(
            original.validate_replay(&altered),
            Err(SurfaceHandoffError::ReplayMismatch)
        );
    }

    #[test]
    fn transitions_must_follow_the_state_machine() {
        assert!(HandoffState::Requested.can_transition_to(HandoffState::Prepared));
        assert!(HandoffState::Prepared.can_transition_to(HandoffState::Accepted));
        assert!(HandoffState::Accepted.can_transition_to(HandoffState::Active));
        assert!(!HandoffState::Requested.can_transition_to(HandoffState::Active));
        assert!(!HandoffState::Declined.can_transition_to(HandoffState::Active));
    }

    #[test]
    fn canonical_request_round_trips() {
        let request: SurfaceHandoff = serde_json::from_value(valid_value()).unwrap();
        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: SurfaceHandoff = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, request);
    }
}
