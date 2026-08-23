use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const HOMEOSTASIS_RECEIPT_SCHEMA_VERSION: &str = "arda.homeostasis-recovery-receipt.v1";
pub const ORGANISM_HEALTH_SCHEMA_VERSION: &str = "arda.organism-health.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganismHealthState {
    Ready,
    Degraded,
    IntentionalOffline,
    Unobserved,
    Unreachable,
    ServiceDown,
    RoutingDrift,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectHealthEvidence {
    pub node_id: String,
    pub enrollment_status: String,
    pub observed_at_unix_ms: u64,
    pub heartbeat_at_unix_ms: Option<u64>,
    pub endpoint_reachable: Option<bool>,
    pub service_active: Option<bool>,
    pub minimal_work_succeeded: Option<bool>,
    pub queue_pressure: Option<f64>,
    pub resource_pressure: Option<f64>,
    pub memory_available: Option<bool>,
    pub configured_route: Option<String>,
    pub observed_route: Option<String>,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganismHealth {
    pub schema_version: String,
    pub node_id: String,
    pub state: OrganismHealthState,
    pub ready_for_new_work: bool,
    pub observed_at_unix_ms: u64,
    pub reasons: Vec<String>,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservationLimits {
    pub max_concurrency: u32,
    pub max_retries: u32,
    pub max_elapsed_ms: u64,
    pub max_context_tokens: u64,
    pub max_output_tokens: u64,
    pub max_cost_microunits: u64,
    pub max_cpu_ratio: f64,
    pub max_gpu_ratio: f64,
    pub max_ram_ratio: f64,
    pub max_thermal_ratio: f64,
    pub max_power_ratio: f64,
    pub max_network_bytes: u64,
    pub max_storage_bytes: u64,
    pub max_operator_attention_units: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservationObservation {
    pub concurrency: u32,
    pub retries: u32,
    pub elapsed_ms: u64,
    pub context_tokens: u64,
    pub output_tokens: u64,
    pub cost_microunits: u64,
    pub cpu_ratio: f64,
    pub gpu_ratio: f64,
    pub ram_ratio: f64,
    pub thermal_ratio: f64,
    pub power_ratio: f64,
    pub network_bytes: u64,
    pub storage_bytes: u64,
    pub operator_attention_units: u32,
    pub optional_work: bool,
    pub consequential_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservationDisposition {
    Continue,
    Degrade,
    ShedOptional,
    Pause,
    RequestReview,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConservationDecision {
    pub disposition: ConservationDisposition,
    pub exceeded_limits: Vec<String>,
    pub explanation: String,
}

pub fn evaluate_conservation(
    limits: &ConservationLimits,
    observation: &ConservationObservation,
) -> ConservationDecision {
    let mut exceeded: Vec<String> = Vec::new();
    if observation.concurrency > limits.max_concurrency {
        exceeded.push("concurrency".into());
    }
    if observation.retries > limits.max_retries {
        exceeded.push("retries".into());
    }
    if observation.elapsed_ms > limits.max_elapsed_ms {
        exceeded.push("time".into());
    }
    if observation.context_tokens > limits.max_context_tokens {
        exceeded.push("context_tokens".into());
    }
    if observation.output_tokens > limits.max_output_tokens {
        exceeded.push("output_tokens".into());
    }
    if observation.cost_microunits > limits.max_cost_microunits {
        exceeded.push("cost".into());
    }
    if observation.cpu_ratio > limits.max_cpu_ratio {
        exceeded.push("cpu".into());
    }
    if observation.gpu_ratio > limits.max_gpu_ratio {
        exceeded.push("gpu".into());
    }
    if observation.ram_ratio > limits.max_ram_ratio {
        exceeded.push("ram".into());
    }
    if observation.thermal_ratio > limits.max_thermal_ratio {
        exceeded.push("thermal".into());
    }
    if observation.power_ratio > limits.max_power_ratio {
        exceeded.push("power".into());
    }
    if observation.network_bytes > limits.max_network_bytes {
        exceeded.push("network".into());
    }
    if observation.storage_bytes > limits.max_storage_bytes {
        exceeded.push("storage".into());
    }
    if observation.operator_attention_units > limits.max_operator_attention_units {
        exceeded.push("operator_attention".into());
    }
    let disposition = if exceeded.is_empty() {
        ConservationDisposition::Continue
    } else if observation.retries > limits.max_retries {
        ConservationDisposition::Stop
    } else if observation.consequential_action {
        ConservationDisposition::RequestReview
    } else if observation.optional_work {
        ConservationDisposition::ShedOptional
    } else if exceeded.iter().any(|name| {
        matches!(
            name.as_str(),
            "ram" | "thermal" | "power" | "storage" | "operator_attention"
        )
    }) {
        ConservationDisposition::Pause
    } else {
        ConservationDisposition::Degrade
    };
    ConservationDecision {
        disposition,
        explanation: if exceeded.is_empty() {
            "all bounded resource and attention budgets remain within policy".into()
        } else {
            format!(
                "policy selected {disposition:?} because these bounded budgets were exceeded: {}",
                exceeded.join(", ")
            )
        },
        exceeded_limits: exceeded,
    }
}

pub fn synthesize_health(
    evidence: &DirectHealthEvidence,
    now_unix_ms: u64,
    stale_after_ms: u64,
) -> OrganismHealth {
    let mut reasons = Vec::new();
    let state = if evidence.enrollment_status == "offline" {
        reasons.push("node is configured intentional-offline".to_string());
        OrganismHealthState::IntentionalOffline
    } else if evidence.observed_at_unix_ms > now_unix_ms
        || evidence
            .heartbeat_at_unix_ms
            .is_some_and(|at| at > now_unix_ms)
    {
        reasons.push("health evidence has a future timestamp".to_string());
        OrganismHealthState::Unknown
    } else if evidence
        .heartbeat_at_unix_ms
        .is_none_or(|at| now_unix_ms.saturating_sub(at) > stale_after_ms)
    {
        reasons.push("no fresh node heartbeat is available".to_string());
        OrganismHealthState::Unobserved
    } else if evidence.endpoint_reachable == Some(false) {
        reasons.push("configured endpoint is unreachable".to_string());
        OrganismHealthState::Unreachable
    } else if evidence.service_active == Some(false) {
        reasons.push("owning service is down".to_string());
        OrganismHealthState::ServiceDown
    } else if matches!(
        (&evidence.configured_route, &evidence.observed_route),
        (Some(configured), Some(observed)) if configured != observed
    ) {
        reasons.push("observed route differs from configured route".to_string());
        OrganismHealthState::RoutingDrift
    } else if evidence.endpoint_reachable.is_none()
        || evidence.service_active.is_none()
        || evidence.minimal_work_succeeded.is_none()
        || evidence.memory_available.is_none()
    {
        reasons.push("required direct evidence is missing".to_string());
        OrganismHealthState::Unknown
    } else if evidence.minimal_work_succeeded == Some(false)
        || evidence.memory_available == Some(false)
        || evidence.resource_pressure.is_some_and(|p| p >= 0.90)
        || evidence.queue_pressure.is_some_and(|p| p >= 0.90)
    {
        if evidence.minimal_work_succeeded == Some(false) {
            reasons.push("minimal work probe failed".to_string());
        }
        if evidence.memory_available == Some(false) {
            reasons.push("canonical memory is unavailable".to_string());
        }
        if evidence.resource_pressure.is_some_and(|p| p >= 0.90) {
            reasons.push("resource pressure reached the hard conservation threshold".to_string());
        }
        if evidence.queue_pressure.is_some_and(|p| p >= 0.90) {
            reasons.push("queue pressure reached the hard conservation threshold".to_string());
        }
        OrganismHealthState::Degraded
    } else {
        reasons.push(
            "fresh heartbeat, service, endpoint, minimal work, and memory evidence agree"
                .to_string(),
        );
        OrganismHealthState::Ready
    };
    OrganismHealth {
        schema_version: ORGANISM_HEALTH_SCHEMA_VERSION.to_string(),
        node_id: evidence.node_id.clone(),
        state,
        ready_for_new_work: state == OrganismHealthState::Ready,
        observed_at_unix_ms: now_unix_ms,
        reasons,
        source_refs: evidence.source_refs.clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl AttemptState {
    fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityEnvelope {
    pub approval_class: String,
    pub allowed_capabilities: Vec<String>,
    pub allowed_data_domains: Vec<String>,
    pub egress_allowed: bool,
}

impl AuthorityEnvelope {
    fn permits(&self, target: &Self) -> bool {
        target.approval_class == self.approval_class
            && (!target.egress_allowed || self.egress_allowed)
            && target
                .allowed_capabilities
                .iter()
                .all(|capability| self.allowed_capabilities.contains(capability))
            && target
                .allowed_data_domains
                .iter()
                .all(|domain| self.allowed_data_domains.contains(domain))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterruptedAttempt {
    pub run_id: String,
    pub attempt_id: String,
    pub work_id: String,
    pub worker_id: String,
    pub node_id: String,
    pub state: AttemptState,
    pub idempotency_key: String,
    pub external_side_effect: bool,
    pub side_effect_idempotent: bool,
    pub terminal_receipt_ref: Option<String>,
    pub authority: AuthorityEnvelope,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryTarget {
    pub worker_id: String,
    pub node_id: String,
    pub health: OrganismHealthState,
    pub authority: AuthorityEnvelope,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDisposition {
    PreserveTerminal,
    Reassign,
    Pause,
    MarkUnknown,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRequest {
    pub recovery_key: String,
    pub interrupted_health: OrganismHealthState,
    pub attempt: InterruptedAttempt,
    pub target: Option<RecoveryTarget>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub recorded_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeostasisReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub recovery_key: String,
    pub input_digest: String,
    pub run_id: String,
    pub attempt_id: String,
    pub work_id: String,
    pub interrupted_node_id: String,
    pub target_node_id: Option<String>,
    pub disposition: RecoveryDisposition,
    pub completed_evidence_preserved: bool,
    pub duplicate_mutation_allowed: bool,
    pub authority_preserved: bool,
    pub explanations: Vec<String>,
    pub source_refs: Vec<String>,
    pub recorded_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
pub struct HomeostasisStore {
    ledger_path: PathBuf,
    lock_path: PathBuf,
}

impl HomeostasisStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let directory = root.as_ref().join("data/homeostasis");
        Self {
            ledger_path: directory.join("recovery-receipts.jsonl"),
            lock_path: directory.join("recovery-receipts.lock"),
        }
    }

    pub fn reconcile(
        &self,
        request: &RecoveryRequest,
    ) -> Result<HomeostasisReceipt, HomeostasisError> {
        validate_request(request)?;
        if let Some(parent) = self.ledger_path.parent() {
            fs::create_dir_all(parent).map_err(|source| HomeostasisError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.lock_path)
            .map_err(|source| HomeostasisError::Io {
                path: self.lock_path.clone(),
                source,
            })?;
        lock.lock_exclusive()
            .map_err(|source| HomeostasisError::Io {
                path: self.lock_path.clone(),
                source,
            })?;
        let existing = self.receipts_unlocked()?;
        let input_digest = digest(request)?;
        if let Some(receipt) = existing
            .iter()
            .find(|receipt| receipt.recovery_key == request.recovery_key)
        {
            if receipt.input_digest == input_digest {
                return Ok(receipt.clone());
            }
            return Err(HomeostasisError::ReplayConflict(
                request.recovery_key.clone(),
            ));
        }
        let receipt = decide(request, input_digest);
        let mut bytes = serde_json::to_vec(&receipt).map_err(HomeostasisError::Serialize)?;
        bytes.push(b'\n');
        let mut ledger = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .map_err(|source| HomeostasisError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
        ledger
            .write_all(&bytes)
            .and_then(|_| ledger.sync_all())
            .map_err(|source| HomeostasisError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
        Ok(receipt)
    }

    pub fn receipts(&self) -> Result<Vec<HomeostasisReceipt>, HomeostasisError> {
        self.receipts_unlocked()
    }

    fn receipts_unlocked(&self) -> Result<Vec<HomeostasisReceipt>, HomeostasisError> {
        let raw = match fs::read_to_string(&self.ledger_path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(HomeostasisError::Io {
                    path: self.ledger_path.clone(),
                    source,
                })
            }
        };
        if !raw.is_empty() && !raw.ends_with('\n') {
            return Err(HomeostasisError::CorruptLedger(
                "non-newline-terminated tail".into(),
            ));
        }
        raw.lines()
            .enumerate()
            .map(|(index, line)| {
                let receipt: HomeostasisReceipt = serde_json::from_str(line).map_err(|error| {
                    HomeostasisError::CorruptLedger(format!("line {}: {error}", index + 1))
                })?;
                if receipt.schema_version != HOMEOSTASIS_RECEIPT_SCHEMA_VERSION {
                    return Err(HomeostasisError::CorruptLedger(format!(
                        "line {} has unsupported schema",
                        index + 1
                    )));
                }
                Ok(receipt)
            })
            .collect()
    }
}

fn validate_request(request: &RecoveryRequest) -> Result<(), HomeostasisError> {
    if request.recovery_key.trim().is_empty()
        || request.attempt.run_id.trim().is_empty()
        || request.attempt.attempt_id.trim().is_empty()
        || request.attempt.work_id.trim().is_empty()
        || request.attempt.idempotency_key.trim().is_empty()
        || request.retry_count > request.max_retries.saturating_add(1)
    {
        return Err(HomeostasisError::InvalidRequest);
    }
    Ok(())
}

fn decide(request: &RecoveryRequest, input_digest: String) -> HomeostasisReceipt {
    let attempt = &request.attempt;
    let mut explanations = Vec::new();
    let completed_evidence_preserved =
        attempt.state.terminal() || attempt.terminal_receipt_ref.is_some();
    let (disposition, target_node_id, authority_preserved) = if completed_evidence_preserved {
        explanations
            .push("durable terminal evidence wins; recovery cannot repeat completed work".into());
        (RecoveryDisposition::PreserveTerminal, None, true)
    } else if !matches!(
        request.interrupted_health,
        OrganismHealthState::Unobserved
            | OrganismHealthState::Unreachable
            | OrganismHealthState::ServiceDown
            | OrganismHealthState::Degraded
    ) {
        explanations
            .push("interrupted attempt is not backed by a recoverable degradation state".into());
        (RecoveryDisposition::Pause, None, true)
    } else if attempt.external_side_effect && !attempt.side_effect_idempotent {
        explanations.push("non-idempotent external side effect has no terminal evidence; attempt is unknown until reviewed or compensated".into());
        (RecoveryDisposition::MarkUnknown, None, true)
    } else if request.retry_count >= request.max_retries {
        explanations.push("bounded retry budget is exhausted".into());
        (RecoveryDisposition::Stop, None, true)
    } else if let Some(target) = &request.target {
        if target.health != OrganismHealthState::Ready {
            explanations.push("candidate target is not directly evidenced ready".into());
            (RecoveryDisposition::Pause, None, true)
        } else if !attempt.authority.permits(&target.authority) {
            explanations.push(
                "candidate target would widen or alter the attempt authority envelope".into(),
            );
            (RecoveryDisposition::Stop, None, false)
        } else {
            explanations.push("eligible work may resume on a directly evidenced ready target with the same bounded authority and idempotency key".into());
            (
                RecoveryDisposition::Reassign,
                Some(target.node_id.clone()),
                true,
            )
        }
    } else {
        explanations
            .push("no eligible ready target is available; optional work remains paused".into());
        (RecoveryDisposition::Pause, None, true)
    };
    let mut source_refs = attempt.source_refs.clone();
    if let Some(target) = &request.target {
        source_refs.extend(target.source_refs.clone());
    }
    source_refs.sort();
    source_refs.dedup();
    let receipt_id = format!("homeostasis:{}", &input_digest[7..23]);
    HomeostasisReceipt {
        schema_version: HOMEOSTASIS_RECEIPT_SCHEMA_VERSION.into(),
        receipt_id,
        recovery_key: request.recovery_key.clone(),
        input_digest,
        run_id: attempt.run_id.clone(),
        attempt_id: attempt.attempt_id.clone(),
        work_id: attempt.work_id.clone(),
        interrupted_node_id: attempt.node_id.clone(),
        target_node_id,
        disposition,
        completed_evidence_preserved,
        duplicate_mutation_allowed: false,
        authority_preserved,
        explanations,
        source_refs,
        recorded_at_unix_ms: request.recorded_at_unix_ms,
    }
}

fn digest<T: Serialize>(value: &T) -> Result<String, HomeostasisError> {
    let bytes = serde_json::to_vec(value).map_err(HomeostasisError::Serialize)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[derive(Debug, thiserror::Error)]
pub enum HomeostasisError {
    #[error("invalid homeostasis recovery request")]
    InvalidRequest,
    #[error("recovery key {0:?} was replayed with conflicting evidence")]
    ReplayConflict(String),
    #[error("corrupt homeostasis ledger: {0}")]
    CorruptLedger(String),
    #[error("homeostasis I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("homeostasis serialization failed: {0}")]
    Serialize(serde_json::Error),
}
