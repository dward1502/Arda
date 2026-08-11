//! Clean-room presentation adapter for the external RELIC renderer.
//!
//! The adapter projects only the already-sanitized runtime-presence contract.
//! It carries provenance into every form and makes degraded/accessibility
//! behavior explicit so a renderer cannot turn a timer into implied activity.

use arda_outpost_protocol::presence::{
    DegradedReason, HealthState, LifecycleState, PresenceNode, PresenceNodeKind, SceneState,
};
use serde::{Deserialize, Serialize};

use crate::CachedPresence;

pub const RELIC_SCENE_ADAPTER_SCHEMA_VERSION: &str = "arda.relic.scene-adapter.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Geometry {
    Gate,
    Prism,
    Lattice,
    Halo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    None,
    Steady,
    Pulse,
    Fade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneForm {
    pub id: String,
    pub label: String,
    pub geometry: Geometry,
    pub color: String,
    pub motion: Motion,
    pub lifecycle: LifecycleState,
    pub health: HealthState,
    pub confidence: f32,
    pub freshness_seconds: u64,
    pub source_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityProfile {
    pub reduced_motion: bool,
    pub reduced_brightness: bool,
    pub high_contrast: bool,
    pub no_audio: bool,
    pub text_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegendEntry {
    pub key: String,
    pub meaning: String,
    pub geometry: Geometry,
    pub color: String,
    pub motion: Motion,
}

impl Default for AccessibilityProfile {
    fn default() -> Self {
        Self {
            reduced_motion: false,
            reduced_brightness: false,
            high_contrast: false,
            no_audio: true,
            text_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RelicSceneAdapterState {
    pub schema_version: String,
    pub projection_id: String,
    pub generated_at: String,
    pub valid_until: String,
    pub scene_state: SceneState,
    pub degraded_reason: Option<DegradedReason>,
    pub status_text: String,
    pub forms: Vec<SceneForm>,
    pub legend: Vec<LegendEntry>,
    pub brightness: f32,
    pub accessibility: AccessibilityProfile,
}

impl CachedPresence {
    /// Produce the renderer-facing state without adding activity, identity, or
    /// content that is absent from the sanitized projection.
    pub fn to_relic_scene_state(
        &self,
        accessibility: AccessibilityProfile,
    ) -> RelicSceneAdapterState {
        let disposition = self.scene;
        let forms = if disposition.state == SceneState::Active && !accessibility.text_only {
            self.snapshot
                .nodes
                .iter()
                .map(|node| form_from_node(node, &accessibility))
                .collect()
        } else {
            Vec::new()
        };
        let status_text = status_text(
            disposition.state,
            disposition.degraded_reason,
            self.age_seconds,
        );

        RelicSceneAdapterState {
            schema_version: RELIC_SCENE_ADAPTER_SCHEMA_VERSION.into(),
            projection_id: self.snapshot.projection_id.clone(),
            generated_at: self.snapshot.generated_at.to_rfc3339(),
            valid_until: self.snapshot.valid_until.to_rfc3339(),
            scene_state: disposition.state,
            degraded_reason: disposition.degraded_reason,
            status_text,
            forms,
            legend: legend(),
            brightness: if accessibility.reduced_brightness {
                0.55
            } else {
                1.0
            },
            accessibility,
        }
    }
}

fn form_from_node(node: &PresenceNode, accessibility: &AccessibilityProfile) -> SceneForm {
    let geometry = match node.kind {
        PresenceNodeKind::Agent => Geometry::Prism,
        PresenceNodeKind::Service => Geometry::Lattice,
        PresenceNodeKind::Realm => Geometry::Halo,
    };
    let motion = if accessibility.reduced_motion {
        Motion::None
    } else {
        match (node.lifecycle, node.health) {
            (LifecycleState::Failed, _) | (_, HealthState::Failed) => Motion::Fade,
            (LifecycleState::WaitingApproval, _) => Motion::Pulse,
            (LifecycleState::Active, HealthState::Healthy) => Motion::Steady,
            _ => Motion::None,
        }
    };
    let color = match node.health {
        HealthState::Healthy => "#46d7ff",
        HealthState::Degraded | HealthState::Unknown => "#ffd166",
        HealthState::Failed => "#ff5c7a",
    };
    let color = if accessibility.high_contrast {
        match node.health {
            HealthState::Healthy => "#ffffff",
            HealthState::Degraded | HealthState::Unknown => "#ffea00",
            HealthState::Failed => "#ff003c",
        }
    } else {
        color
    };

    SceneForm {
        id: node.id.clone(),
        label: node.label.clone(),
        geometry,
        color: color.into(),
        motion,
        lifecycle: node.lifecycle,
        health: node.health,
        confidence: node.confidence,
        freshness_seconds: node.freshness_seconds,
        source_receipt_refs: node.source_receipt_refs.clone(),
    }
}

fn legend() -> Vec<LegendEntry> {
    vec![
        LegendEntry {
            key: "healthy_active".into(),
            meaning: "fresh healthy runtime presence".into(),
            geometry: Geometry::Lattice,
            color: "#46d7ff".into(),
            motion: Motion::Steady,
        },
        LegendEntry {
            key: "waiting_approval".into(),
            meaning: "runtime is waiting for an approval boundary".into(),
            geometry: Geometry::Prism,
            color: "#ffd166".into(),
            motion: Motion::Pulse,
        },
        LegendEntry {
            key: "failed".into(),
            meaning: "runtime failure reported by the source projection".into(),
            geometry: Geometry::Halo,
            color: "#ff5c7a".into(),
            motion: Motion::Fade,
        },
        LegendEntry {
            key: "idle_degraded".into(),
            meaning: "no fresh verifiable projection is displayable".into(),
            geometry: Geometry::Halo,
            color: "#6b7280".into(),
            motion: Motion::None,
        },
    ]
}

fn status_text(state: SceneState, reason: Option<DegradedReason>, age_seconds: i64) -> String {
    match state {
        SceneState::Active => format!("live presence · snapshot age {age_seconds}s"),
        SceneState::IdleDegraded => match reason {
            Some(reason) => format!("idle degraded · {reason:?}"),
            None => "idle degraded".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_outpost_protocol::presence::{
        PresenceEdge, RedactionClass, ResourcePressure, RuntimePresenceProjection,
    };
    use chrono::{Duration, Utc};

    fn cached() -> CachedPresence {
        let now = Utc::now();
        CachedPresence {
            snapshot_sequence: 7,
            snapshot: RuntimePresenceProjection {
                projection_id: "projection-7".into(),
                schema_version: "arda.runtime-presence.v1".into(),
                generated_at: now,
                valid_until: now + Duration::seconds(30),
                nodes: vec![PresenceNode {
                    id: "agent-athena".into(),
                    kind: PresenceNodeKind::Agent,
                    label: "Athena".into(),
                    lifecycle: LifecycleState::WaitingApproval,
                    health: HealthState::Healthy,
                    confidence: 0.9,
                    freshness_seconds: 2,
                    resource_pressure: Some(ResourcePressure {
                        cpu: 0.1,
                        memory: 0.2,
                        provider: 0.3,
                    }),
                    run_id: None,
                    task_id: None,
                    source_receipt_refs: vec!["receipt:7".into()],
                }],
                edges: Vec::<PresenceEdge>::new(),
                source_receipt_refs: vec!["receipt:7".into()],
                redaction_class: RedactionClass::PublicOperational,
            },
            age_seconds: 2,
            scene: arda_outpost_protocol::presence::SceneDisposition {
                state: SceneState::Active,
                degraded_reason: None,
            },
        }
    }

    #[test]
    fn adapter_preserves_provenance_and_maps_waiting_approval() {
        let state = cached().to_relic_scene_state(AccessibilityProfile::default());
        assert_eq!(state.schema_version, RELIC_SCENE_ADAPTER_SCHEMA_VERSION);
        assert_eq!(state.forms[0].geometry, Geometry::Prism);
        assert_eq!(state.forms[0].motion, Motion::Pulse);
        assert_eq!(state.forms[0].source_receipt_refs, vec!["receipt:7"]);
        assert_eq!(state.legend.len(), 4);
        assert!(state.accessibility.no_audio);
    }

    #[test]
    fn reduced_motion_and_text_only_emit_no_moving_forms() {
        let state = cached().to_relic_scene_state(AccessibilityProfile {
            reduced_motion: true,
            reduced_brightness: true,
            high_contrast: true,
            no_audio: true,
            text_only: true,
        });
        assert!(state.forms.is_empty());
        assert_eq!(state.brightness, 0.55);
        assert!(state.status_text.contains("live presence"));
    }
}
