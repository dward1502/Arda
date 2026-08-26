//! Restart-safe governed assimilation state and bounded nightly evaluation policy.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::os::{fd::AsRawFd, unix::fs::PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssimilationState {
    Discovered,
    EvidenceCollected,
    NeedMatched,
    Isolated,
    TrialActive,
    Measured,
    ProposalReady,
    AwaitingGovernance,
    Accepted,
    Rejected,
    Deferred,
    Landed,
    AdapterRetained,
    Verified,
    RolledBack,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssimilationCandidate {
    pub candidate_id: String,
    pub adapter_id: String,
    pub state: AssimilationState,
    pub evidence: AssimilationEvidence,
    pub transition_count: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssimilationEvidence {
    pub canonical_source: Option<String>,
    pub license: Option<String>,
    pub source_digest: Option<String>,
    pub sbom_digest: Option<String>,
    pub objective_id: Option<String>,
    pub usage_receipt: Option<String>,
    pub security_classification: Option<String>,
    pub privacy_classification: Option<String>,
    pub implementation_comparison: Option<String>,
    pub patch_provenance: Option<String>,
    pub test_receipt: Option<String>,
    pub failure_receipt: Option<String>,
    pub removal_proof: Option<String>,
    pub rollback_proof: Option<String>,
    pub approval_reference: Option<String>,
    #[serde(default)]
    pub changes_dependency: bool,
    #[serde(default)]
    pub changes_data_access: bool,
    #[serde(default)]
    pub changes_network_access: bool,
    #[serde(default)]
    pub changes_architecture: bool,
}

impl AssimilationEvidence {
    fn merge(&mut self, update: AssimilationEvidence) {
        macro_rules! replace_some {
            ($($field:ident),+ $(,)?) => {
                $(if update.$field.is_some() { self.$field = update.$field; })+
            };
        }
        replace_some!(
            canonical_source,
            license,
            source_digest,
            sbom_digest,
            objective_id,
            usage_receipt,
            security_classification,
            privacy_classification,
            implementation_comparison,
            patch_provenance,
            test_receipt,
            failure_receipt,
            removal_proof,
            rollback_proof,
            approval_reference,
        );
        self.changes_dependency |= update.changes_dependency;
        self.changes_data_access |= update.changes_data_access;
        self.changes_network_access |= update.changes_network_access;
        self.changes_architecture |= update.changes_architecture;
    }

    fn consequential(&self) -> bool {
        self.changes_dependency
            || self.changes_data_access
            || self.changes_network_access
            || self.changes_architecture
    }
}

#[derive(Debug)]
pub struct AssimilationStore {
    path: PathBuf,
}

impl AssimilationStore {
    pub fn new(root: &Path) -> Self {
        Self {
            path: root.join("data/assimilation/candidates.jsonl"),
        }
    }

    pub fn ledger_path(&self) -> &Path {
        &self.path
    }

    pub fn discover(
        &self,
        candidate_id: &str,
        adapter_id: &str,
        observed_at: DateTime<Utc>,
    ) -> Result<AssimilationCandidate, AssimilationError> {
        self.discover_with_evidence(
            candidate_id,
            adapter_id,
            AssimilationEvidence::default(),
            observed_at,
        )
    }

    pub fn discover_with_evidence(
        &self,
        candidate_id: &str,
        adapter_id: &str,
        evidence: AssimilationEvidence,
        observed_at: DateTime<Utc>,
    ) -> Result<AssimilationCandidate, AssimilationError> {
        require_text("candidate_id", candidate_id)?;
        require_text("adapter_id", adapter_id)?;
        let mut file = self.open_locked()?;
        let projection = load_projection(&mut file)?;
        if let Some(existing) = projection.candidates.get(candidate_id) {
            if existing.adapter_id == adapter_id {
                return Ok(existing.clone());
            }
            return Err(AssimilationError::CandidateConflict(
                candidate_id.to_string(),
            ));
        }
        let candidate = AssimilationCandidate {
            candidate_id: candidate_id.to_string(),
            adapter_id: adapter_id.to_string(),
            state: AssimilationState::Discovered,
            evidence,
            transition_count: 0,
            updated_at: observed_at,
        };
        append_event(
            &mut file,
            projection.next_sequence,
            AssimilationEvent::Discovered {
                candidate: candidate.clone(),
            },
            observed_at,
        )?;
        Ok(candidate)
    }

    pub fn advance(
        &self,
        candidate_id: &str,
        target: AssimilationState,
        evidence: AssimilationEvidence,
        transitioned_at: DateTime<Utc>,
    ) -> Result<AssimilationCandidate, AssimilationError> {
        let mut file = self.open_locked()?;
        let projection = load_projection(&mut file)?;
        let current = projection
            .candidates
            .get(candidate_id)
            .ok_or_else(|| AssimilationError::NotFound(candidate_id.to_string()))?;
        if current.state == target {
            return Ok(current.clone());
        }
        if !transition_allowed(current.state, target) {
            return Err(AssimilationError::InvalidTransition {
                from: current.state,
                to: target,
            });
        }
        let mut updated = current.clone();
        updated.evidence.merge(evidence);
        validate_evidence(target, &updated.evidence)?;
        updated.state = target;
        updated.transition_count += 1;
        updated.updated_at = transitioned_at;
        append_event(
            &mut file,
            projection.next_sequence,
            AssimilationEvent::Advanced {
                candidate_id: candidate_id.to_string(),
                from: current.state,
                candidate: updated.clone(),
            },
            transitioned_at,
        )?;
        Ok(updated)
    }

    pub fn load_all(&self) -> Result<BTreeMap<String, AssimilationCandidate>, AssimilationError> {
        let mut file = match std::fs::OpenOptions::new().read(true).open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new())
            }
            Err(error) => return Err(error.into()),
        };
        let _lock = FileLock::shared(&file)?;
        Ok(load_projection(&mut file)?.candidates)
    }

    fn open_locked(&self) -> Result<LockedFile, AssimilationError> {
        let parent = self.path.parent().expect("assimilation path has parent");
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;
        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600))?;
        LockedFile::exclusive(file).map_err(AssimilationError::Io)
    }
}

fn transition_allowed(from: AssimilationState, to: AssimilationState) -> bool {
    use AssimilationState::*;
    matches!(
        (from, to),
        (Discovered, EvidenceCollected)
            | (EvidenceCollected, NeedMatched)
            | (NeedMatched, Isolated)
            | (Isolated, TrialActive)
            | (TrialActive, Measured)
            | (Measured, ProposalReady)
            | (ProposalReady, AwaitingGovernance)
            | (AwaitingGovernance, Accepted | Rejected | Deferred)
            | (Accepted, Landed | AdapterRetained)
            | (Landed | AdapterRetained, Verified | RolledBack | Removed)
            | (Verified, RolledBack | Removed)
            | (RolledBack, Removed)
            | (Rejected | Deferred, Removed)
    )
}

fn validate_evidence(
    state: AssimilationState,
    evidence: &AssimilationEvidence,
) -> Result<(), AssimilationError> {
    use AssimilationState::*;
    let require = |field: &'static str, value: &Option<String>| {
        if value
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            Ok(())
        } else {
            Err(AssimilationError::MissingEvidence(field))
        }
    };
    if !matches!(state, Discovered) {
        require("canonical_source", &evidence.canonical_source)?;
        require("license", &evidence.license)?;
        require("source_digest", &evidence.source_digest)?;
        require("sbom_digest", &evidence.sbom_digest)?;
        for value in [&evidence.source_digest, &evidence.sbom_digest] {
            let valid = value.as_deref().is_some_and(|digest| {
                digest.strip_prefix("sha256:").is_some_and(|hex| {
                    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
            });
            if !valid {
                return Err(AssimilationError::InvalidDigest);
            }
        }
    }
    if matches!(
        state,
        NeedMatched
            | Isolated
            | TrialActive
            | Measured
            | ProposalReady
            | AwaitingGovernance
            | Accepted
            | Rejected
            | Deferred
            | Landed
            | AdapterRetained
            | Verified
            | RolledBack
            | Removed
    ) {
        require("objective_id", &evidence.objective_id)?;
    }
    if matches!(
        state,
        Measured
            | ProposalReady
            | AwaitingGovernance
            | Accepted
            | Landed
            | AdapterRetained
            | Verified
            | RolledBack
            | Removed
    ) {
        require("usage_receipt", &evidence.usage_receipt)?;
    }
    if matches!(
        state,
        ProposalReady
            | AwaitingGovernance
            | Accepted
            | Landed
            | AdapterRetained
            | Verified
            | RolledBack
            | Removed
    ) {
        for (field, value) in [
            ("security_classification", &evidence.security_classification),
            ("privacy_classification", &evidence.privacy_classification),
            (
                "implementation_comparison",
                &evidence.implementation_comparison,
            ),
            ("patch_provenance", &evidence.patch_provenance),
            ("test_receipt", &evidence.test_receipt),
            ("failure_receipt", &evidence.failure_receipt),
            ("removal_proof", &evidence.removal_proof),
            ("rollback_proof", &evidence.rollback_proof),
        ] {
            require(field, value)?;
        }
    }
    if matches!(state, Accepted | Landed | Verified) && evidence.consequential() {
        require("approval_reference", &evidence.approval_reference)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NightlyIntent {
    RefreshApprovedSource,
    CompareMeasuredGap,
    RunIsolatedReadOnlyFixture,
    GenerateReport,
    PrepareBoundedPatch,
    RunTests,
    PrepareAdoptionProposal,
    ScrapeArbitraryCode,
    InstallDependency,
    ExpandNetworkOrSecretAccess,
    MutatePrivateData,
    PromoteObservationToTask,
    MergeConsequentialPatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NightlyEvaluationPolicy {
    pub approved_sources: BTreeSet<String>,
    pub allow_bounded_patch: bool,
    pub allow_tests: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NightlyIntentRequest {
    pub intent: NightlyIntent,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NightlyEvaluationPlan {
    pub allowed: Vec<NightlyIntent>,
    pub denied: Vec<(NightlyIntent, String)>,
}

pub fn evaluate_nightly_intents(
    policy: &NightlyEvaluationPolicy,
    requests: &[NightlyIntentRequest],
) -> NightlyEvaluationPlan {
    let mut plan = NightlyEvaluationPlan::default();
    for request in requests {
        let denial = match request.intent {
            NightlyIntent::RefreshApprovedSource
                if request
                    .source
                    .as_ref()
                    .is_none_or(|source| !policy.approved_sources.contains(source)) =>
            {
                Some("source is not approved".to_string())
            }
            NightlyIntent::PrepareBoundedPatch if !policy.allow_bounded_patch => {
                Some("bounded patch preparation is disabled".to_string())
            }
            NightlyIntent::RunTests if !policy.allow_tests => {
                Some("test execution is disabled".to_string())
            }
            NightlyIntent::ScrapeArbitraryCode
            | NightlyIntent::InstallDependency
            | NightlyIntent::ExpandNetworkOrSecretAccess
            | NightlyIntent::MutatePrivateData
            | NightlyIntent::PromoteObservationToTask
            | NightlyIntent::MergeConsequentialPatch => {
                Some("intent exceeds nightly evaluation authority".to_string())
            }
            _ => None,
        };
        if let Some(reason) = denial {
            plan.denied.push((request.intent, reason));
        } else {
            plan.allowed.push(request.intent);
        }
    }
    plan
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AssimilationEvent {
    Discovered {
        candidate: AssimilationCandidate,
    },
    Advanced {
        candidate_id: String,
        from: AssimilationState,
        candidate: AssimilationCandidate,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssimilationEnvelope {
    schema_version: String,
    sequence: u64,
    recorded_at: DateTime<Utc>,
    event: AssimilationEvent,
}

#[derive(Default)]
struct AssimilationProjection {
    candidates: BTreeMap<String, AssimilationCandidate>,
    next_sequence: u64,
}

fn load_projection(file: &mut std::fs::File) -> Result<AssimilationProjection, AssimilationError> {
    file.seek(SeekFrom::Start(0))?;
    let mut projection = AssimilationProjection::default();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let envelope: AssimilationEnvelope =
            serde_json::from_str(&line).map_err(|error| AssimilationError::CorruptEntry {
                line: index + 1,
                message: error.to_string(),
            })?;
        if envelope.schema_version != "arda.assimilation-ledger.v1" {
            return Err(AssimilationError::UnsupportedVersion(
                envelope.schema_version,
            ));
        }
        let expected = projection.next_sequence + 1;
        if envelope.sequence != expected {
            return Err(AssimilationError::SequenceGap {
                expected,
                actual: envelope.sequence,
            });
        }
        projection.next_sequence = envelope.sequence;
        match envelope.event {
            AssimilationEvent::Discovered { candidate } => {
                if projection
                    .candidates
                    .insert(candidate.candidate_id.clone(), candidate.clone())
                    .is_some()
                {
                    return Err(AssimilationError::CandidateConflict(candidate.candidate_id));
                }
            }
            AssimilationEvent::Advanced {
                candidate_id,
                from,
                candidate,
            } => {
                let current = projection
                    .candidates
                    .get(&candidate_id)
                    .ok_or_else(|| AssimilationError::NotFound(candidate_id.clone()))?;
                if current.state != from || !transition_allowed(from, candidate.state) {
                    return Err(AssimilationError::InvalidTransition {
                        from: current.state,
                        to: candidate.state,
                    });
                }
                validate_evidence(candidate.state, &candidate.evidence)?;
                projection.candidates.insert(candidate_id, candidate);
            }
        }
    }
    Ok(projection)
}

fn append_event(
    file: &mut LockedFile,
    current_sequence: u64,
    event: AssimilationEvent,
    recorded_at: DateTime<Utc>,
) -> Result<(), AssimilationError> {
    serde_json::to_writer(
        &mut *file,
        &AssimilationEnvelope {
            schema_version: "arda.assimilation-ledger.v1".to_string(),
            sequence: current_sequence + 1,
            recorded_at,
            event,
        },
    )?;
    writeln!(file)?;
    file.sync_all()?;
    Ok(())
}

fn require_text(field: &'static str, value: &str) -> Result<(), AssimilationError> {
    if value.trim().is_empty() {
        Err(AssimilationError::MissingEvidence(field))
    } else {
        Ok(())
    }
}

struct FileLock {
    fd: std::os::fd::RawFd,
}

impl FileLock {
    fn shared(file: &std::fs::File) -> std::io::Result<Self> {
        Self::acquire(file, libc::LOCK_SH)
    }

    fn acquire(file: &std::fs::File, operation: libc::c_int) -> std::io::Result<Self> {
        let fd = file.as_raw_fd();
        // SAFETY: `fd` belongs to the live file held by the caller.
        if unsafe { libc::flock(fd, operation) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { fd })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // SAFETY: the locked file outlives this guard.
        let _ = unsafe { libc::flock(self.fd, libc::LOCK_UN) };
    }
}

struct LockedFile {
    file: std::fs::File,
    _lock: FileLock,
}

impl LockedFile {
    fn exclusive(file: std::fs::File) -> std::io::Result<Self> {
        let lock = FileLock::acquire(&file, libc::LOCK_EX)?;
        Ok(Self { file, _lock: lock })
    }
}

impl std::ops::Deref for LockedFile {
    type Target = std::fs::File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl std::ops::DerefMut for LockedFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

impl Write for LockedFile {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.file.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssimilationError {
    #[error("assimilation I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("assimilation serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("corrupt assimilation entry at line {line}: {message}")]
    CorruptEntry { line: usize, message: String },
    #[error("unsupported assimilation ledger version: {0}")]
    UnsupportedVersion(String),
    #[error("assimilation sequence gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    #[error("assimilation candidate {0} not found")]
    NotFound(String),
    #[error("assimilation candidate {0} conflicts with durable state")]
    CandidateConflict(String),
    #[error("invalid assimilation transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: AssimilationState,
        to: AssimilationState,
    },
    #[error("assimilation transition is missing required evidence: {0}")]
    MissingEvidence(&'static str),
    #[error("assimilation source and SBOM evidence must use SHA-256 digests")]
    InvalidDigest,
}
