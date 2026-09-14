//! Linux Foundation A2A semantic adapter for the enrolled Arda node mesh.
//!
//! Oromë owns the typed handoff envelope and correlation receipt. The engine
//! owns placement and durable attempts; Hermes/A2A owns the network wire.

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const NODE_IDENTITY_SCHEMA_VERSION: &str = "arda.node-identity.v1";
pub const NODE_ENROLLMENT_SCHEMA_VERSION: &str = "arda.node-enrollment.v1";
pub const CAPABILITY_OBSERVATION_SCHEMA_VERSION: &str = "arda.node-capability-observation.v1";
pub const WORK_ENVELOPE_SCHEMA_VERSION: &str = "arda.work-envelope.v1";
pub const HANDOFF_RECEIPT_SCHEMA_VERSION: &str = "arda.a2a-handoff-receipt.v1";
pub const MESH_PROJECTION_SCHEMA_VERSION: &str = "arda.a2a-mesh-projection.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeIdentity {
    pub schema_version: String,
    pub node_id: String,
    pub agent_id: String,
    pub trust_domain: String,
    pub enrollment_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEnrollment {
    pub schema_version: String,
    pub identity: NodeIdentity,
    pub agent_card_url: String,
    /// Environment variable name only. Credentials are never persisted.
    pub bearer_env: String,
    pub allowed_capabilities: Vec<String>,
    pub allowed_data_domains: Vec<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePressureObservation {
    pub cpu: f32,
    pub memory: f32,
    pub gpu: Option<f32>,
    pub queue_depth: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityObservation {
    pub schema_version: String,
    pub observation_id: String,
    pub node_id: String,
    pub capabilities: Vec<String>,
    pub pressure: ResourcePressureObservation,
    pub observed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkEnvelope {
    pub schema_version: String,
    pub envelope_id: String,
    pub objective_id: String,
    pub run_id: String,
    pub worker_id: String,
    pub capability: String,
    pub data_domain: String,
    pub payload: Value,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub nonce: String,
    pub route_trace: Vec<String>,
    pub max_hops: usize,
}

impl WorkEnvelope {
    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), A2aMeshError> {
        if self.schema_version != WORK_ENVELOPE_SCHEMA_VERSION
            || self.envelope_id.trim().is_empty()
            || self.objective_id.trim().is_empty()
            || self.run_id.trim().is_empty()
            || self.worker_id.trim().is_empty()
            || self.capability.trim().is_empty()
            || self.data_domain.trim().is_empty()
            || self.nonce.trim().is_empty()
            || self.issued_at >= self.expires_at
        {
            return Err(A2aMeshError::InvalidContract);
        }
        if now >= self.expires_at {
            return Err(A2aMeshError::ExpiredEnvelope);
        }
        if self.route_trace.len() >= self.max_hops {
            return Err(A2aMeshError::HopLimitExceeded);
        }
        Ok(())
    }

    /// Map the canonical Arda work envelope to the A2A v1.0 JSON-RPC binding.
    pub fn to_a2a_send_message(&self, target_node: &str) -> Result<Value, A2aMeshError> {
        self.validate_at(Utc::now().min(self.expires_at - chrono::Duration::nanoseconds(1)))?;
        Ok(json!({
            "jsonrpc": "2.0",
            "id": self.envelope_id,
            "method": "SendMessage",
            "params": {
                "message": {
                    "role": "ROLE_USER",
                    "parts": [{
                        "data": self,
                        "mediaType": "application/vnd.arda.work-envelope.v1+json"
                    }],
                    "messageId": self.envelope_id,
                    "contextId": self.run_id,
                    "metadata": {
                        "ardaTargetNode": target_node,
                        "ardaObjectiveId": self.objective_id,
                        "ardaWorkerId": self.worker_id,
                        "ardaExpiresAt": self.expires_at,
                        "ardaRouteTrace": self.route_trace,
                    }
                }
            }
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct A2aHandoffReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub envelope_id: String,
    pub objective_id: String,
    pub run_id: String,
    pub worker_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub a2a_task_id: String,
    pub a2a_context_id: String,
    pub status: String,
    pub dispatched_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutedPeer {
    pub enrollment: NodeEnrollment,
    pub observation: CapabilityObservation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshPeerProjection {
    pub node_id: String,
    pub agent_id: String,
    pub trust_domain: String,
    pub availability: String,
    pub capabilities: Vec<String>,
    pub pressure: Option<ResourcePressureObservation>,
    pub enrollment_expires_at: DateTime<Utc>,
    pub observation_expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshProjection {
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub peers: Vec<MeshPeerProjection>,
    pub dispatches_claimed: usize,
    pub receipts: Vec<A2aHandoffReceipt>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum A2aMeshError {
    #[error("invalid mesh contract")]
    InvalidContract,
    #[error("work envelope expired")]
    ExpiredEnvelope,
    #[error("route hop limit exceeded")]
    HopLimitExceeded,
    #[error("no eligible enrolled peer")]
    NoEligiblePeer,
    #[error("message replay detected")]
    ReplayDetected,
    #[error("registry I/O failed")]
    RegistryIo,
    #[error("registry row is invalid")]
    InvalidRegistryRow,
    #[error("node is not enrolled")]
    NodeNotEnrolled,
    #[error("completion correlation is invalid")]
    ForgedCompletion,
    #[error("authentication credential is unavailable")]
    AuthenticationUnavailable,
    #[error("A2A transport failed")]
    TransportFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum RegistryEvent {
    NodeEnrolled {
        recorded_at: DateTime<Utc>,
        enrollment: NodeEnrollment,
    },
    NodeRevoked {
        recorded_at: DateTime<Utc>,
        node_id: String,
        reason: String,
    },
    CapabilityObserved {
        recorded_at: DateTime<Utc>,
        observation: CapabilityObservation,
    },
    DispatchClaimed {
        recorded_at: DateTime<Utc>,
        envelope_id: String,
        nonce: String,
    },
    HandoffCompleted {
        recorded_at: DateTime<Utc>,
        receipt: A2aHandoffReceipt,
    },
}

#[derive(Debug)]
pub struct MeshRegistry {
    path: PathBuf,
    enrollments: BTreeMap<String, NodeEnrollment>,
    observations: BTreeMap<String, CapabilityObservation>,
    claimed_envelopes: BTreeSet<String>,
    claimed_nonces: BTreeSet<String>,
    receipts: Vec<A2aHandoffReceipt>,
}

impl MeshRegistry {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, A2aMeshError> {
        let path = path.as_ref().to_path_buf();
        let mut registry = Self {
            path,
            enrollments: BTreeMap::new(),
            observations: BTreeMap::new(),
            claimed_envelopes: BTreeSet::new(),
            claimed_nonces: BTreeSet::new(),
            receipts: Vec::new(),
        };
        if registry.path.exists() {
            let file = File::open(&registry.path).map_err(|_| A2aMeshError::RegistryIo)?;
            for line in BufReader::new(file).lines() {
                let line = line.map_err(|_| A2aMeshError::RegistryIo)?;
                if line.trim().is_empty() {
                    continue;
                }
                let event: RegistryEvent =
                    serde_json::from_str(&line).map_err(|_| A2aMeshError::InvalidRegistryRow)?;
                registry.apply(event);
            }
        }
        Ok(registry)
    }

    pub fn enroll(
        &mut self,
        enrollment: NodeEnrollment,
        now: DateTime<Utc>,
    ) -> Result<(), A2aMeshError> {
        validate_enrollment(&enrollment, now)?;
        if let Some(existing) = self.enrollments.get(&enrollment.identity.node_id) {
            let stable_identity_matches = existing.identity.agent_id
                == enrollment.identity.agent_id
                && existing.identity.trust_domain == enrollment.identity.trust_domain;
            let epoch_is_monotonic =
                enrollment.identity.enrollment_epoch >= existing.identity.enrollment_epoch;
            let revoked_epoch_rotated = existing.revoked_at.is_none()
                || enrollment.identity.enrollment_epoch > existing.identity.enrollment_epoch;
            if !stable_identity_matches || !epoch_is_monotonic || !revoked_epoch_rotated {
                return Err(A2aMeshError::InvalidContract);
            }
        }
        self.append(RegistryEvent::NodeEnrolled {
            recorded_at: now,
            enrollment,
        })
    }

    pub fn revoke(
        &mut self,
        node_id: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), A2aMeshError> {
        if !self.enrollments.contains_key(node_id) || reason.trim().is_empty() {
            return Err(A2aMeshError::NodeNotEnrolled);
        }
        self.append(RegistryEvent::NodeRevoked {
            recorded_at: now,
            node_id: node_id.to_owned(),
            reason: reason.to_owned(),
        })
    }

    pub fn publish_observation(
        &mut self,
        observation: CapabilityObservation,
        now: DateTime<Utc>,
    ) -> Result<(), A2aMeshError> {
        if observation.schema_version != CAPABILITY_OBSERVATION_SCHEMA_VERSION
            || observation.observation_id.trim().is_empty()
            || !self.enrollments.contains_key(&observation.node_id)
            || observation.observed_at >= observation.expires_at
            || now >= observation.expires_at
            || !valid_pressure(&observation.pressure)
        {
            return Err(A2aMeshError::InvalidContract);
        }
        self.append(RegistryEvent::CapabilityObserved {
            recorded_at: now,
            observation,
        })
    }

    pub fn route(
        &self,
        envelope: &WorkEnvelope,
        now: DateTime<Utc>,
    ) -> Result<RoutedPeer, A2aMeshError> {
        envelope.validate_at(now)?;
        self.enrollments
            .values()
            .filter(|enrollment| enrollment.revoked_at.is_none())
            .filter(|enrollment| now < enrollment.expires_at)
            .filter(|enrollment| {
                enrollment
                    .allowed_capabilities
                    .contains(&envelope.capability)
                    && enrollment
                        .allowed_data_domains
                        .contains(&envelope.data_domain)
                    && !envelope.route_trace.contains(&enrollment.identity.node_id)
            })
            .filter_map(|enrollment| {
                let observation = self.observations.get(&enrollment.identity.node_id)?;
                (now < observation.expires_at
                    && observation.capabilities.contains(&envelope.capability))
                .then(|| RoutedPeer {
                    enrollment: enrollment.clone(),
                    observation: observation.clone(),
                })
            })
            .min_by(|left, right| {
                pressure_score(&left.observation.pressure)
                    .total_cmp(&pressure_score(&right.observation.pressure))
                    .then_with(|| {
                        left.enrollment
                            .identity
                            .node_id
                            .cmp(&right.enrollment.identity.node_id)
                    })
            })
            .ok_or(A2aMeshError::NoEligiblePeer)
    }

    pub fn claim_dispatch(
        &mut self,
        envelope: &WorkEnvelope,
        now: DateTime<Utc>,
    ) -> Result<(), A2aMeshError> {
        envelope.validate_at(now)?;
        if self.claimed_envelopes.contains(&envelope.envelope_id)
            || self.claimed_nonces.contains(&envelope.nonce)
        {
            return Err(A2aMeshError::ReplayDetected);
        }
        self.append(RegistryEvent::DispatchClaimed {
            recorded_at: now,
            envelope_id: envelope.envelope_id.clone(),
            nonce: envelope.nonce.clone(),
        })
    }

    pub fn record_receipt(
        &mut self,
        receipt: A2aHandoffReceipt,
        now: DateTime<Utc>,
    ) -> Result<(), A2aMeshError> {
        if receipt.schema_version != HANDOFF_RECEIPT_SCHEMA_VERSION
            || receipt.receipt_id.trim().is_empty()
            || receipt.completed_at < receipt.dispatched_at
        {
            return Err(A2aMeshError::InvalidContract);
        }
        self.append(RegistryEvent::HandoffCompleted {
            recorded_at: now,
            receipt,
        })
    }

    pub fn projection(&self, now: DateTime<Utc>) -> MeshProjection {
        let peers = self
            .enrollments
            .values()
            .map(|enrollment| {
                let observation = self.observations.get(&enrollment.identity.node_id);
                let availability = if enrollment.revoked_at.is_some() {
                    "revoked"
                } else if now >= enrollment.expires_at {
                    "expired"
                } else if observation.is_some_and(|value| now < value.expires_at) {
                    "online"
                } else {
                    "offline"
                };
                MeshPeerProjection {
                    node_id: enrollment.identity.node_id.clone(),
                    agent_id: enrollment.identity.agent_id.clone(),
                    trust_domain: enrollment.identity.trust_domain.clone(),
                    availability: availability.to_owned(),
                    capabilities: observation
                        .map(|value| value.capabilities.clone())
                        .unwrap_or_default(),
                    pressure: observation.map(|value| value.pressure.clone()),
                    enrollment_expires_at: enrollment.expires_at,
                    observation_expires_at: observation.map(|value| value.expires_at),
                    revoked_at: enrollment.revoked_at,
                }
            })
            .collect();
        MeshProjection {
            schema_version: MESH_PROJECTION_SCHEMA_VERSION.to_owned(),
            generated_at: now,
            peers,
            dispatches_claimed: self.claimed_envelopes.len(),
            receipts: self.receipts.clone(),
        }
    }

    fn append(&mut self, event: RegistryEvent) -> Result<(), A2aMeshError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| A2aMeshError::RegistryIo)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|_| A2aMeshError::RegistryIo)?;
        file.lock_exclusive()
            .map_err(|_| A2aMeshError::RegistryIo)?;
        self.reload()?;
        match &event {
            RegistryEvent::DispatchClaimed {
                envelope_id, nonce, ..
            } if self.claimed_envelopes.contains(envelope_id)
                || self.claimed_nonces.contains(nonce) =>
            {
                FileExt::unlock(&file).map_err(|_| A2aMeshError::RegistryIo)?;
                return Err(A2aMeshError::ReplayDetected);
            }
            RegistryEvent::NodeEnrolled { enrollment, .. }
                if self
                    .enrollments
                    .get(&enrollment.identity.node_id)
                    .is_some_and(|existing| {
                        existing.identity.agent_id != enrollment.identity.agent_id
                            || existing.identity.trust_domain != enrollment.identity.trust_domain
                            || enrollment.identity.enrollment_epoch
                                < existing.identity.enrollment_epoch
                            || (existing.revoked_at.is_some()
                                && enrollment.identity.enrollment_epoch
                                    <= existing.identity.enrollment_epoch)
                    }) =>
            {
                FileExt::unlock(&file).map_err(|_| A2aMeshError::RegistryIo)?;
                return Err(A2aMeshError::InvalidContract);
            }
            _ => {}
        }
        serde_json::to_writer(&mut file, &event).map_err(|_| A2aMeshError::RegistryIo)?;
        file.write_all(b"\n")
            .and_then(|_| file.sync_data())
            .map_err(|_| A2aMeshError::RegistryIo)?;
        FileExt::unlock(&file).map_err(|_| A2aMeshError::RegistryIo)?;
        self.apply(event);
        Ok(())
    }

    fn reload(&mut self) -> Result<(), A2aMeshError> {
        self.enrollments.clear();
        self.observations.clear();
        self.claimed_envelopes.clear();
        self.claimed_nonces.clear();
        self.receipts.clear();
        if !self.path.exists() {
            return Ok(());
        }
        let file = File::open(&self.path).map_err(|_| A2aMeshError::RegistryIo)?;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|_| A2aMeshError::RegistryIo)?;
            if line.trim().is_empty() {
                continue;
            }
            let event: RegistryEvent =
                serde_json::from_str(&line).map_err(|_| A2aMeshError::InvalidRegistryRow)?;
            self.apply(event);
        }
        Ok(())
    }

    fn apply(&mut self, event: RegistryEvent) {
        match event {
            RegistryEvent::NodeEnrolled { enrollment, .. } => {
                self.enrollments
                    .insert(enrollment.identity.node_id.clone(), enrollment);
            }
            RegistryEvent::NodeRevoked {
                node_id,
                recorded_at,
                ..
            } => {
                if let Some(enrollment) = self.enrollments.get_mut(&node_id) {
                    enrollment.revoked_at = Some(recorded_at);
                }
            }
            RegistryEvent::CapabilityObserved { observation, .. } => {
                self.observations
                    .insert(observation.node_id.clone(), observation);
            }
            RegistryEvent::DispatchClaimed {
                envelope_id, nonce, ..
            } => {
                self.claimed_envelopes.insert(envelope_id);
                self.claimed_nonces.insert(nonce);
            }
            RegistryEvent::HandoffCompleted { receipt, .. } => self.receipts.push(receipt),
        }
    }
}

fn validate_enrollment(
    enrollment: &NodeEnrollment,
    now: DateTime<Utc>,
) -> Result<(), A2aMeshError> {
    let identity = &enrollment.identity;
    if enrollment.schema_version != NODE_ENROLLMENT_SCHEMA_VERSION
        || identity.schema_version != NODE_IDENTITY_SCHEMA_VERSION
        || identity.node_id.trim().is_empty()
        || identity.agent_id.trim().is_empty()
        || identity.trust_domain.trim().is_empty()
        || identity.enrollment_epoch == 0
        || enrollment.agent_card_url.trim().is_empty()
        || enrollment.bearer_env.trim().is_empty()
        || enrollment.allowed_capabilities.is_empty()
        || enrollment.allowed_data_domains.is_empty()
        || enrollment.issued_at >= enrollment.expires_at
        || now >= enrollment.expires_at
        || enrollment.revoked_at.is_some()
    {
        return Err(A2aMeshError::InvalidContract);
    }
    Ok(())
}

fn valid_pressure(pressure: &ResourcePressureObservation) -> bool {
    let valid = |value: f32| value.is_finite() && (0.0..=1.0).contains(&value);
    valid(pressure.cpu) && valid(pressure.memory) && pressure.gpu.is_none_or(valid)
}

fn pressure_score(pressure: &ResourcePressureObservation) -> f32 {
    let gpu = pressure.gpu.unwrap_or(0.0);
    pressure.cpu + pressure.memory + gpu + (pressure.queue_depth as f32 * 0.01)
}
