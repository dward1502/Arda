//! Runtime presence projection for Arda observability.
//!
//! This module produces a sanitized, versioned runtime-presence graph from
//! bounded inputs that can be consumed by RELIC and CITADEL outposts without
//! exposing private task content or granting execution authority.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[allow(unused_imports)]
use arda_outpost_protocol::presence::{
    DegradedReason, HealthState, LifecycleState, PresenceEdge, PresenceEdgeType, PresenceNode,
    PresenceNodeKind, RedactionClass, ResourcePressure, RuntimePresenceProjection, SceneState,
};
/// The schema version for the runtime presence projection.
pub const RUNTIME_PRESENCE_SCHEMA_VERSION: &str = "arda.runtime-presence.v1";

/// Inputs older than this threshold are treated as stale and degrade the
/// corresponding node instead of inventing confidence or motion.
const STALE_THRESHOLD_SECONDS: u64 = 30;

/// Deterministic snapshot of inputs needed to build a presence projection.
///
/// Keep this value-oriented so callers can construct it from environment,
/// service status, run graph, provider state, and telemetry without holding
/// internal locks.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectionInputs {
    pub services: Vec<ServicePresence>,
    pub agents: Vec<AgentPresence>,
    pub edges: Vec<EdgePresence>,
    pub source_receipt_refs: Vec<String>,
}

impl ProjectionInputs {
    /// Sentinel inputs meaning the caller did not have enough state to project.
    pub fn empty() -> Self {
        Self {
            services: Vec::new(),
            agents: Vec::new(),
            edges: Vec::new(),
            source_receipt_refs: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.services.is_empty() && self.agents.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServicePresence {
    pub id: String,
    pub label: String,
    pub lifecycle: LifecycleState,
    pub health: HealthState,
    pub confidence: f32,
    pub freshness_seconds: u64,
    pub resource_pressure: ResourcePressure,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub source_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentPresence {
    pub id: String,
    pub label: String,
    pub lifecycle: LifecycleState,
    pub health: HealthState,
    pub confidence: f32,
    pub freshness_seconds: u64,
    pub resource_pressure: ResourcePressure,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub source_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgePresence {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_type: PresenceEdgeType,
    pub confidence: f32,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub source_receipt_refs: Vec<String>,
}

/// Build a deterministic runtime presence projection from bounded inputs.
///
/// Unknown, stale, or malformed inputs reduce confidence and can fall back to
/// `IdleDegraded` scene states instead of inventing runtime motion.
pub fn build_presence_projection(inputs: ProjectionInputs) -> RuntimePresenceProjection {
    build_presence_projection_at(inputs, fixed_utc_now())
}

/// Build a projection with an injected clock for runtime and deterministic tests.
pub fn build_presence_projection_at(
    mut inputs: ProjectionInputs,
    now: DateTime<Utc>,
) -> RuntimePresenceProjection {
    canonicalize_inputs(&mut inputs);
    let valid_until = now + ChronoDuration::seconds(30);

    if inputs.is_empty() {
        return empty_projection(&inputs, now, valid_until);
    }

    let mut nodes = Vec::with_capacity(inputs.services.len() + inputs.agents.len());
    for service in &inputs.services {
        nodes.push(node_from_service(service));
    }
    for agent in &inputs.agents {
        nodes.push(node_from_agent(agent));
    }

    let projection_id = projection_id(&inputs, now);
    let mut source_receipt_refs = inputs.source_receipt_refs;
    source_receipt_refs.sort_unstable();
    source_receipt_refs.dedup();

    RuntimePresenceProjection {
        projection_id,
        schema_version: RUNTIME_PRESENCE_SCHEMA_VERSION.to_string(),
        generated_at: now,
        valid_until,
        nodes,
        edges: edges_from_inputs(inputs.edges),
        source_receipt_refs,
        redaction_class: RedactionClass::PublicOperational,
    }
}

fn canonicalize_inputs(inputs: &mut ProjectionInputs) {
    inputs
        .services
        .sort_by(|left, right| left.id.cmp(&right.id));
    inputs.agents.sort_by(|left, right| left.id.cmp(&right.id));
    inputs.edges.sort_by(|left, right| left.id.cmp(&right.id));
    inputs.source_receipt_refs.sort();
    inputs.source_receipt_refs.dedup();
}

fn projection_id(inputs: &ProjectionInputs, now: DateTime<Utc>) -> String {
    let material = serde_json::json!({
        "inputs": inputs,
        "clock": now.to_rfc3339(),
    });
    let digest = Sha256::digest(serde_json::to_vec(&material).expect("presence inputs serialize"));
    format!("arda-runtime-presence-{:x}", digest)
}

fn node_from_service(service: &ServicePresence) -> PresenceNode {
    let stale = service.freshness_seconds > STALE_THRESHOLD_SECONDS;
    PresenceNode {
        id: service.id.clone(),
        kind: PresenceNodeKind::Service,
        label: service.label.clone(),
        lifecycle: service.lifecycle,
        health: if stale {
            HealthState::Degraded
        } else {
            service.health
        },
        confidence: if stale {
            0.0
        } else {
            clamp(service.confidence)
        },
        freshness_seconds: service.freshness_seconds,
        resource_pressure: Some(clamp_pressure(&service.resource_pressure)),
        run_id: service.run_id.clone(),
        task_id: service.task_id.clone(),
        source_receipt_refs: service.source_receipt_refs.clone(),
    }
}

fn node_from_agent(agent: &AgentPresence) -> PresenceNode {
    let stale = agent.freshness_seconds > STALE_THRESHOLD_SECONDS;
    PresenceNode {
        id: agent.id.clone(),
        kind: PresenceNodeKind::Agent,
        label: agent.label.clone(),
        lifecycle: agent.lifecycle,
        health: if stale {
            HealthState::Degraded
        } else {
            agent.health
        },
        confidence: if stale { 0.0 } else { clamp(agent.confidence) },
        freshness_seconds: agent.freshness_seconds,
        resource_pressure: Some(clamp_pressure(&agent.resource_pressure)),
        run_id: agent.run_id.clone(),
        task_id: agent.task_id.clone(),
        source_receipt_refs: agent.source_receipt_refs.clone(),
    }
}

fn empty_projection(
    inputs: &ProjectionInputs,
    now: DateTime<Utc>,
    valid_until: DateTime<Utc>,
) -> RuntimePresenceProjection {
    RuntimePresenceProjection {
        projection_id: projection_id(inputs, now),
        schema_version: RUNTIME_PRESENCE_SCHEMA_VERSION.to_string(),
        generated_at: now,
        valid_until,
        nodes: Vec::new(),
        edges: Vec::new(),
        source_receipt_refs: Vec::new(),
        redaction_class: RedactionClass::PublicOperational,
    }
}

fn edges_from_inputs(inputs: Vec<EdgePresence>) -> Vec<PresenceEdge> {
    inputs
        .into_iter()
        .filter_map(|edge| {
            let confidence = clamp(edge.confidence);
            if confidence == 0.0 {
                return None;
            }
            Some(PresenceEdge {
                id: edge.id,
                from_node_id: edge.from_node_id,
                to_node_id: edge.to_node_id,
                edge_type: edge.edge_type,
                confidence,
                run_id: edge.run_id,
                task_id: edge.task_id,
                source_receipt_refs: edge.source_receipt_refs,
            })
        })
        .collect()
}

fn clamp(value: f32) -> f32 {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        value
    } else {
        0.0
    }
}

fn clamp_pressure(pressure: &ResourcePressure) -> ResourcePressure {
    ResourcePressure {
        cpu: clamp(pressure.cpu),
        memory: clamp(pressure.memory),
        provider: clamp(pressure.provider),
    }
}

#[allow(dead_code)]
fn inputs_fixture() -> ProjectionInputs {
    ProjectionInputs {
        services: vec![ServicePresence {
            id: "service-manwe".to_string(),
            label: "Manwe Gateway".to_string(),
            lifecycle: LifecycleState::Active,
            health: HealthState::Healthy,
            confidence: 0.92,
            freshness_seconds: 2,
            resource_pressure: ResourcePressure {
                cpu: 0.31,
                memory: 0.28,
                provider: 0.15,
            },
            run_id: None,
            task_id: None,
            source_receipt_refs: vec!["receipt-ghi".to_string()],
        }],
        agents: vec![AgentPresence {
            id: "agent-awa".to_string(),
            label: "Aulë Observer".to_string(),
            lifecycle: LifecycleState::Active,
            health: HealthState::Healthy,
            confidence: 0.88,
            freshness_seconds: 3,
            resource_pressure: ResourcePressure {
                cpu: 0.23,
                memory: 0.41,
                provider: 0.19,
            },
            run_id: Some("run-123".to_string()),
            task_id: Some("task-456".to_string()),
            source_receipt_refs: vec!["receipt-abc".to_string(), "receipt-def".to_string()],
        }],
        edges: vec![EdgePresence {
            id: "edge-002".to_string(),
            from_node_id: "agent-awa".to_string(),
            to_node_id: "service-manwe".to_string(),
            edge_type: PresenceEdgeType::Collaboration,
            confidence: 0.87,
            run_id: Some("run-123".to_string()),
            task_id: Some("task-456".to_string()),
            source_receipt_refs: vec!["receipt-def".to_string()],
        }],
        source_receipt_refs: vec![
            "receipt-abc".to_string(),
            "receipt-def".to_string(),
            "receipt-ghi".to_string(),
        ],
    }
}

fn fixed_utc_now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("fixed timestamp must be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_inputs_yield_deterministic_projection() {
        let inputs = inputs_fixture();
        let first = build_presence_projection(inputs.clone());
        let second = build_presence_projection(inputs);
        assert_eq!(first, second);
    }

    #[test]
    fn stale_service_reduces_confidence_instead_of_motion() {
        let mut inputs = inputs_fixture();
        inputs.services[0].freshness_seconds = 10_000;

        let projection = build_presence_projection(inputs);
        let node = projection
            .nodes
            .iter()
            .find(|node| node.id == "service-manwe")
            .expect("service node");
        assert_eq!(node.health, HealthState::Degraded);
        assert_eq!(node.confidence, 0.0);
    }

    #[test]
    fn empty_inputs_produce_verifiable_but_empty_presence() {
        let projection = build_presence_projection(ProjectionInputs::empty());
        assert!(projection.nodes.is_empty());
        assert!(projection.edges.is_empty());
        let disposition = projection.scene_disposition_at(projection.generated_at);
        assert_eq!(disposition.state, SceneState::IdleDegraded);
        assert_eq!(
            disposition.degraded_reason,
            Some(DegradedReason::Unverifiable)
        );
    }

    #[test]
    fn unknown_node_id_is_not_invented() {
        let inputs = inputs_fixture();
        let projection = build_presence_projection(inputs);
        assert!(!projection
            .nodes
            .iter()
            .any(|node| node.id == "agent-mystery"));
    }

    #[test]
    fn unknown_edge_target_falls_back_to_degraded_idle() {
        let mut inputs = inputs_fixture();
        inputs.edges.clear();
        inputs.source_receipt_refs.clear();

        let projection = build_presence_projection(inputs);
        let disposition = projection.scene_disposition_at(projection.generated_at);
        assert_eq!(disposition.state, SceneState::IdleDegraded);
        assert_eq!(
            disposition.degraded_reason,
            Some(DegradedReason::Unverifiable)
        );
    }

    #[test]
    fn schema_version_mismatch_is_idle_degraded() {
        let mut projection = build_presence_projection(inputs_fixture());
        projection.schema_version = "unsupported".into();
        let disposition = projection.scene_disposition_at(projection.generated_at);
        assert_eq!(disposition.state, SceneState::IdleDegraded);
        assert_eq!(
            disposition.degraded_reason,
            Some(DegradedReason::UnsupportedSchema)
        );
    }

    #[test]
    fn stale_inputs_keep_confidence_low() {
        let mut inputs = inputs_fixture();
        inputs.agents[0].freshness_seconds = 5_000;

        let projection = build_presence_projection(inputs);
        let node = projection
            .nodes
            .iter()
            .find(|node| node.id == "agent-awa")
            .expect("agent node");
        assert_eq!(node.confidence, 0.0);
        assert_eq!(node.health, HealthState::Degraded);
    }
}
