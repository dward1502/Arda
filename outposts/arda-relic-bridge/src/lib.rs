//! Read-only RELIC bridge for the versioned runtime-presence projection.
//!
//! This crate owns validation and last-valid caching only. It has no mutation
//! path back into Arda and does not contain renderer or kiosk policy.

use std::sync::Arc;

use arda_outpost_protocol::presence::{
    DegradedReason, RedactionClass, RuntimePresenceProjection, SceneDisposition, SceneState,
    RUNTIME_PRESENCE_SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceivedPresence {
    pub snapshot_sequence: u64,
    pub snapshot: RuntimePresenceProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedPresence {
    pub snapshot_sequence: u64,
    pub snapshot: RuntimePresenceProjection,
    pub age_seconds: i64,
    pub scene: SceneDisposition,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BridgeError {
    #[error("invalid presence payload: {0}")]
    InvalidPayload(String),
    #[error("snapshot sequence {received} is not newer than {last}")]
    NonMonotonicSequence { received: u64, last: u64 },
    #[error("unsupported presence schema: {0}")]
    UnsupportedSchema(String),
    #[error("presence redaction class is not display-safe")]
    InvalidRedactionClass,
    #[error("presence projection has no verifiable source receipts")]
    Unverifiable,
}

#[derive(Debug, Clone, Default)]
pub struct PresenceBridge {
    cache: Arc<RwLock<Option<ReceivedPresence>>>,
}

impl PresenceBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept_json(
        &self,
        payload: &[u8],
        now: DateTime<Utc>,
    ) -> Result<CachedPresence, BridgeError> {
        let received: ReceivedPresence = serde_json::from_slice(payload)
            .map_err(|error| BridgeError::InvalidPayload(error.to_string()))?;
        self.accept(received, now)
    }

    pub fn accept(
        &self,
        received: ReceivedPresence,
        now: DateTime<Utc>,
    ) -> Result<CachedPresence, BridgeError> {
        validate_projection(&received.snapshot)?;
        if let Some(previous) = self.cache.read().as_ref() {
            if received.snapshot_sequence <= previous.snapshot_sequence {
                return Err(BridgeError::NonMonotonicSequence {
                    received: received.snapshot_sequence,
                    last: previous.snapshot_sequence,
                });
            }
        }
        *self.cache.write() = Some(received);
        self.current(now)
            .ok_or_else(|| BridgeError::InvalidPayload("cache write did not persist".into()))
    }

    pub fn current(&self, now: DateTime<Utc>) -> Option<CachedPresence> {
        self.cache.read().as_ref().map(|received| CachedPresence {
            snapshot_sequence: received.snapshot_sequence,
            snapshot: received.snapshot.clone(),
            age_seconds: (now - received.snapshot.generated_at).num_seconds().max(0),
            scene: received.snapshot.scene_disposition_at(now),
        })
    }

    pub fn last_valid_at(&self) -> Option<DateTime<Utc>> {
        self.cache
            .read()
            .as_ref()
            .map(|received| received.snapshot.generated_at)
    }

    pub fn is_idle_degraded(&self, now: DateTime<Utc>) -> bool {
        self.current(now)
            .map(|cached| cached.scene.state == SceneState::IdleDegraded)
            .unwrap_or(true)
    }
}

fn validate_projection(snapshot: &RuntimePresenceProjection) -> Result<(), BridgeError> {
    if snapshot.schema_version != RUNTIME_PRESENCE_SCHEMA_VERSION {
        return Err(BridgeError::UnsupportedSchema(
            snapshot.schema_version.clone(),
        ));
    }
    if snapshot.generated_at > snapshot.valid_until {
        return Err(BridgeError::InvalidPayload(
            "generated_at is after valid_until".into(),
        ));
    }
    if !matches!(
        snapshot.redaction_class,
        RedactionClass::PublicOperational | RedactionClass::PrivateMetadataRemoved
    ) {
        return Err(BridgeError::InvalidRedactionClass);
    }
    if snapshot
        .scene_disposition_at(snapshot.generated_at)
        .degraded_reason
        == Some(DegradedReason::Unverifiable)
    {
        return Err(BridgeError::Unverifiable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_outpost_protocol::presence::{
        HealthState, LifecycleState, PresenceNode, PresenceNodeKind, ResourcePressure,
    };
    use chrono::Duration;

    fn received(sequence: u64, generated_at: DateTime<Utc>) -> ReceivedPresence {
        ReceivedPresence {
            snapshot_sequence: sequence,
            snapshot: RuntimePresenceProjection {
                projection_id: format!("projection-{sequence}"),
                schema_version: RUNTIME_PRESENCE_SCHEMA_VERSION.into(),
                generated_at,
                valid_until: generated_at + Duration::seconds(30),
                nodes: vec![PresenceNode {
                    id: "service:manwe".into(),
                    kind: PresenceNodeKind::Service,
                    label: "Manwe".into(),
                    lifecycle: LifecycleState::Active,
                    health: HealthState::Healthy,
                    confidence: 1.0,
                    freshness_seconds: 1,
                    resource_pressure: Some(ResourcePressure {
                        cpu: 0.1,
                        memory: 0.1,
                        provider: 0.0,
                    }),
                    run_id: None,
                    task_id: None,
                    source_receipt_refs: vec!["receipt:1".into()],
                }],
                edges: vec![],
                source_receipt_refs: vec!["receipt:1".into()],
                redaction_class: RedactionClass::PublicOperational,
            },
        }
    }

    #[test]
    fn accepts_newer_snapshot_and_exposes_age() {
        let now = DateTime::parse_from_rfc3339("2026-08-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let bridge = PresenceBridge::new();
        let cached = bridge
            .accept(received(1, now - Duration::seconds(5)), now)
            .unwrap();
        assert_eq!(cached.snapshot_sequence, 1);
        assert_eq!(cached.age_seconds, 5);
        assert_eq!(cached.scene.state, SceneState::Active);
        assert_eq!(bridge.last_valid_at(), Some(now - Duration::seconds(5)));
    }

    #[test]
    fn rejects_repeated_sequence_and_expires_last_valid_snapshot() {
        let now = Utc::now();
        let bridge = PresenceBridge::new();
        bridge.accept(received(4, now), now).unwrap();
        let error = bridge.accept(received(4, now), now).unwrap_err();
        assert_eq!(
            error,
            BridgeError::NonMonotonicSequence {
                received: 4,
                last: 4
            }
        );
        assert!(bridge.is_idle_degraded(now + Duration::seconds(31)));
    }

    #[test]
    fn rejects_unknown_json_fields_and_missing_receipts() {
        let now = Utc::now();
        let bridge = PresenceBridge::new();
        let payload = br#"{"snapshot_sequence":1,"snapshot":{},"extra":true}"#;
        assert!(matches!(
            bridge.accept_json(payload, now),
            Err(BridgeError::InvalidPayload(_))
        ));

        let mut invalid = received(1, now);
        invalid.snapshot.source_receipt_refs.clear();
        assert_eq!(
            bridge.accept(invalid, now).unwrap_err(),
            BridgeError::Unverifiable
        );
    }
}
