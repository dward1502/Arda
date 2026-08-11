use super::{
    apply_transition_once, AppendOutcome, ResourceLedgerError, ResourceMeasurementSource,
    ResourceUsageDraft, RunEventDraft, RunEventKind, RunStore, RunStoreError, TransitionOutcome,
};
use arda_core::run_graph::{NodeId, NodeState, RunGraph};
use chrono::{Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CANONICAL_GOVERNANCE_OWNER: &str = "arda_engine::runs::ArdaEngineGovernanceEnforcer";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryDecision {
    pub receipt_digest: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernanceAdvisories {
    pub love: AdvisoryDecision,
    pub triad: AdvisoryDecision,
    pub bacon_lite: AdvisoryDecision,
    pub readiness: AdvisoryDecision,
    pub realm_policy: AdvisoryDecision,
    pub joulework: AdvisoryDecision,
}

impl GovernanceAdvisories {
    fn digests(&self) -> BTreeMap<String, String> {
        [
            ("love", &self.love),
            ("triad", &self.triad),
            ("bacon_lite", &self.bacon_lite),
            ("readiness", &self.readiness),
            ("realm_policy", &self.realm_policy),
            ("joulework", &self.joulework),
        ]
        .into_iter()
        .map(|(name, decision)| (name.to_string(), decision.receipt_digest.clone()))
        .collect()
    }

    fn failed_names(&self) -> Vec<String> {
        [
            ("love", &self.love),
            ("triad", &self.triad),
            ("bacon_lite", &self.bacon_lite),
            ("readiness", &self.readiness),
            ("realm_policy", &self.realm_policy),
            ("joulework", &self.joulework),
        ]
        .into_iter()
        .filter(|(_, decision)| !decision.passed)
        .map(|(name, _)| name.to_string())
        .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDemand {
    pub provider_id: Option<String>,
    pub local_joulework: f64,
    pub hosted_cost_usd: f64,
    pub hosted_requests: u64,
    pub reasoning_pressure: f64,
}

impl ResourceDemand {
    fn validate(&self) -> Result<(), GovernanceEnforcementError> {
        if !self.local_joulework.is_finite()
            || !self.hosted_cost_usd.is_finite()
            || !self.reasoning_pressure.is_finite()
            || self.local_joulework < 0.0
            || self.hosted_cost_usd < 0.0
            || !(0.0..=1.0).contains(&self.reasoning_pressure)
        {
            return Err(GovernanceEnforcementError::InvalidResourceDemand);
        }
        if self.hosted_requests > 0 && self.provider_id.is_none() {
            return Err(GovernanceEnforcementError::ProviderRequired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernanceEvaluationRequest {
    pub decision_key: String,
    pub node_id: NodeId,
    pub requested_transition: NodeState,
    pub action: serde_json::Value,
    pub advisories: GovernanceAdvisories,
    pub approval_required: bool,
    pub approval_granted: bool,
    pub demand: ResourceDemand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetDisposition {
    WithinBudget,
    ApprovalRequired,
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetDecision {
    pub disposition: BudgetDisposition,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalGovernanceVerdict {
    Allowed,
    Blocked,
    ApprovalRequired,
    BudgetExhausted,
}

impl CanonicalGovernanceVerdict {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Blocked => "blocked",
            Self::ApprovalRequired => "approval_required",
            Self::BudgetExhausted => "budget_exhausted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernanceReceipt {
    pub schema_version: String,
    pub canonical_owner: String,
    pub decision_key: String,
    pub run_id: String,
    pub node_id: NodeId,
    pub evaluation_input_digest: String,
    pub action_digest: String,
    pub advisory_receipt_digests: BTreeMap<String, String>,
    pub verdict: CanonicalGovernanceVerdict,
    pub approval_required: bool,
    pub approval_granted: bool,
    pub budget: BudgetDecision,
    pub transition: NodeState,
    pub transition_idempotency_key: String,
    pub explanations: Vec<String>,
}

impl GovernanceReceipt {
    pub const SCHEMA_VERSION: &'static str = "arda.governance-enforcement-receipt.v1";

    pub fn digest(&self) -> Result<String, GovernanceEnforcementError> {
        let bytes = serde_json::to_vec(self).map_err(GovernanceEnforcementError::Serialize)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub fn validate_presentation(
        &self,
        presenter: &str,
        claimed_verdict: CanonicalGovernanceVerdict,
        claimed_approval_required: bool,
    ) -> Result<(), GovernanceEnforcementError> {
        if claimed_verdict != self.verdict || claimed_approval_required != self.approval_required {
            return Err(GovernanceEnforcementError::ConflictingPresentation {
                presenter: presenter.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedGovernanceDecision {
    pub receipt: GovernanceReceipt,
    pub receipt_digest: String,
    pub receipt_sequence: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GovernanceExecutionOutcome {
    pub decision: RecordedGovernanceDecision,
    pub governance_event_sequence: u64,
    pub transition: TransitionOutcome,
    pub resource_reservation: Option<AppendOutcome>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeGovernorBudgetPolicy {
    pub contract: RuntimeGovernorContract,
    pub user_plan: UserPlanBudget,
    pub routing_load_shed: RoutingPressureBudget,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderBudget>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeGovernorContract {
    pub schema_version: String,
    pub authority: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserPlanBudget {
    pub monthly_spend_usd_soft_cap: f64,
    pub monthly_spend_usd_hard_cap: f64,
    pub daily_spend_usd_soft_cap: f64,
    pub daily_spend_usd_hard_cap: f64,
    pub local_joulework_daily_soft_cap: f64,
    pub local_joulework_daily_hard_cap: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutingPressureBudget {
    pub reasoning_minute_pressure_soft: f64,
    pub reasoning_minute_pressure_hard: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderBudget {
    pub monthly_requests_soft_cap: u64,
    pub monthly_requests_hard_cap: u64,
}

impl RuntimeGovernorBudgetPolicy {
    pub const SCHEMA_VERSION: &'static str = "arda.runtime-governor-budget.v1";

    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self, GovernanceEnforcementError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| GovernanceEnforcementError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&raw)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, GovernanceEnforcementError> {
        let policy: Self = toml::from_str(raw).map_err(GovernanceEnforcementError::Toml)?;
        policy.validate()?;
        Ok(policy)
    }

    fn validate(&self) -> Result<(), GovernanceEnforcementError> {
        if self.contract.schema_version != Self::SCHEMA_VERSION
            || self.contract.authority != "runtime_governor_budget_policy"
        {
            return Err(GovernanceEnforcementError::InvalidBudgetPolicy);
        }
        let soft_hard = [
            (
                self.user_plan.monthly_spend_usd_soft_cap,
                self.user_plan.monthly_spend_usd_hard_cap,
            ),
            (
                self.user_plan.daily_spend_usd_soft_cap,
                self.user_plan.daily_spend_usd_hard_cap,
            ),
            (
                self.user_plan.local_joulework_daily_soft_cap,
                self.user_plan.local_joulework_daily_hard_cap,
            ),
            (
                self.routing_load_shed.reasoning_minute_pressure_soft,
                self.routing_load_shed.reasoning_minute_pressure_hard,
            ),
        ];
        if soft_hard.iter().any(|(soft, hard)| {
            !soft.is_finite() || !hard.is_finite() || *soft < 0.0 || soft > hard
        }) || self
            .providers
            .values()
            .any(|provider| provider.monthly_requests_soft_cap > provider.monthly_requests_hard_cap)
        {
            return Err(GovernanceEnforcementError::InvalidBudgetPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ArdaEngineGovernanceEnforcer {
    policy: RuntimeGovernorBudgetPolicy,
}

impl ArdaEngineGovernanceEnforcer {
    pub fn new(policy: RuntimeGovernorBudgetPolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate_route_budget(
        &self,
        store: &RunStore,
        demand: &ResourceDemand,
    ) -> Result<BudgetDecision, GovernanceEnforcementError> {
        demand.validate()?;
        let now = Utc::now();
        let day_start = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .single()
            .ok_or(GovernanceEnforcementError::Clock)?
            .timestamp_millis() as u128;
        let month_start = Utc
            .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
            .single()
            .ok_or(GovernanceEnforcementError::Clock)?
            .timestamp_millis() as u128;
        let daily = store.resource_rollup_since(day_start, None)?;
        let monthly = store.resource_rollup_since(month_start, None)?;
        let provider = match demand.provider_id.as_deref() {
            Some(provider_id) => Some((
                self.policy.providers.get(provider_id).ok_or_else(|| {
                    GovernanceEnforcementError::UnconfiguredProvider(provider_id.to_string())
                })?,
                store.resource_rollup_since(month_start, Some(provider_id))?,
            )),
            None => None,
        };

        let mut hard = Vec::new();
        let mut soft = Vec::new();
        classify_cap(
            "daily_spend_usd",
            daily.hosted_cost_usd + demand.hosted_cost_usd,
            self.policy.user_plan.daily_spend_usd_soft_cap,
            self.policy.user_plan.daily_spend_usd_hard_cap,
            &mut soft,
            &mut hard,
        );
        classify_cap(
            "monthly_spend_usd",
            monthly.hosted_cost_usd + demand.hosted_cost_usd,
            self.policy.user_plan.monthly_spend_usd_soft_cap,
            self.policy.user_plan.monthly_spend_usd_hard_cap,
            &mut soft,
            &mut hard,
        );
        classify_cap(
            "local_joulework_daily",
            daily.local_joulework + demand.local_joulework,
            self.policy.user_plan.local_joulework_daily_soft_cap,
            self.policy.user_plan.local_joulework_daily_hard_cap,
            &mut soft,
            &mut hard,
        );
        classify_cap(
            "reasoning_pressure",
            demand.reasoning_pressure,
            self.policy.routing_load_shed.reasoning_minute_pressure_soft,
            self.policy.routing_load_shed.reasoning_minute_pressure_hard,
            &mut soft,
            &mut hard,
        );
        if let Some((provider_policy, provider_rollup)) = provider {
            classify_cap_u64(
                "provider_monthly_requests",
                provider_rollup.hosted_requests + demand.hosted_requests,
                provider_policy.monthly_requests_soft_cap,
                provider_policy.monthly_requests_hard_cap,
                &mut soft,
                &mut hard,
            );
        }
        if !hard.is_empty() {
            Ok(BudgetDecision {
                disposition: BudgetDisposition::Exhausted,
                reasons: hard,
            })
        } else if !soft.is_empty() {
            Ok(BudgetDecision {
                disposition: BudgetDisposition::ApprovalRequired,
                reasons: soft,
            })
        } else {
            Ok(BudgetDecision {
                disposition: BudgetDisposition::WithinBudget,
                reasons: Vec::new(),
            })
        }
    }

    pub fn evaluate_and_record(
        &self,
        store: &RunStore,
        request: &GovernanceEvaluationRequest,
    ) -> Result<RecordedGovernanceDecision, GovernanceEnforcementError> {
        if request.decision_key.trim().is_empty() {
            return Err(GovernanceEnforcementError::EmptyDecisionKey);
        }
        request.demand.validate()?;
        let input_bytes =
            serde_json::to_vec(request).map_err(GovernanceEnforcementError::Serialize)?;
        let evaluation_input_digest = format!("sha256:{:x}", Sha256::digest(input_bytes));
        if let Some((index, existing)) = read_governance_receipts(store)?
            .into_iter()
            .enumerate()
            .find(|(_, receipt)| receipt.decision_key == request.decision_key)
        {
            if existing.evaluation_input_digest != evaluation_input_digest {
                return Err(GovernanceEnforcementError::DuplicateReceiptConflict {
                    key: request.decision_key.clone(),
                });
            }
            let receipt_digest = existing.digest()?;
            return Ok(RecordedGovernanceDecision {
                receipt: existing,
                receipt_digest,
                receipt_sequence: index as u64 + 1,
            });
        }
        let budget = self.evaluate_route_budget(store, &request.demand)?;
        let failed_advisories = request.advisories.failed_names();
        let mut explanations = Vec::new();
        let verdict = if !failed_advisories.is_empty() {
            explanations.push(format!("advisory veto: {}", failed_advisories.join(",")));
            CanonicalGovernanceVerdict::Blocked
        } else if budget.disposition == BudgetDisposition::Exhausted {
            explanations.extend(budget.reasons.clone());
            CanonicalGovernanceVerdict::BudgetExhausted
        } else if (request.approval_required
            || budget.disposition == BudgetDisposition::ApprovalRequired)
            && !request.approval_granted
        {
            explanations.extend(budget.reasons.clone());
            CanonicalGovernanceVerdict::ApprovalRequired
        } else {
            CanonicalGovernanceVerdict::Allowed
        };
        let approval_required =
            request.approval_required || budget.disposition == BudgetDisposition::ApprovalRequired;
        let transition = if verdict == CanonicalGovernanceVerdict::Allowed {
            request.requested_transition
        } else {
            NodeState::Blocked
        };
        let action_bytes =
            serde_json::to_vec(&request.action).map_err(GovernanceEnforcementError::Serialize)?;
        let action_digest = format!("sha256:{:x}", Sha256::digest(action_bytes));
        let receipt = GovernanceReceipt {
            schema_version: GovernanceReceipt::SCHEMA_VERSION.to_string(),
            canonical_owner: CANONICAL_GOVERNANCE_OWNER.to_string(),
            decision_key: request.decision_key.clone(),
            run_id: store.run_id().as_str().to_string(),
            node_id: request.node_id.clone(),
            evaluation_input_digest,
            action_digest,
            advisory_receipt_digests: request.advisories.digests(),
            verdict,
            approval_required,
            approval_granted: request.approval_granted,
            budget,
            transition,
            transition_idempotency_key: format!("{}:transition", request.decision_key),
            explanations,
        };
        let receipt_digest = receipt.digest()?;
        let receipt_sequence = append_governance_receipt(store, &receipt)?;
        Ok(RecordedGovernanceDecision {
            receipt,
            receipt_digest,
            receipt_sequence,
        })
    }

    pub fn apply_recorded_transition(
        &self,
        store: &RunStore,
        graph: &mut RunGraph,
        decision: RecordedGovernanceDecision,
        demand: &ResourceDemand,
    ) -> Result<GovernanceExecutionOutcome, GovernanceEnforcementError> {
        let persisted = read_governance_receipts(store)?;
        if !persisted.iter().any(|receipt| receipt == &decision.receipt) {
            return Err(GovernanceEnforcementError::UnrecordedReceipt);
        }
        let event = store.append(RunEventDraft {
            node_id: decision.receipt.node_id.clone(),
            idempotency_key: format!("{}:governance-event", decision.receipt.decision_key),
            kind: RunEventKind::GovernanceEvaluated {
                action_digest: decision.receipt.action_digest.clone(),
                verdict: decision.receipt.verdict.as_str().to_string(),
                approval_required: decision.receipt.approval_required,
                transition: decision.receipt.transition,
            },
            receipt_digest: Some(decision.receipt_digest.clone()),
        })?;
        let governance_event_sequence = match event {
            AppendOutcome::Appended { sequence } | AppendOutcome::AlreadyApplied { sequence } => {
                sequence
            }
        };
        let resource_reservation = if decision.receipt.verdict
            == CanonicalGovernanceVerdict::Allowed
        {
            Some(store.append_resource_usage(ResourceUsageDraft {
                idempotency_key: format!("{}:budget-reservation", decision.receipt.decision_key),
                source: ResourceMeasurementSource::DefaultFallback,
                provider_id: demand.provider_id.clone(),
                local_joulework: demand.local_joulework,
                hosted_cost_usd: demand.hosted_cost_usd,
                hosted_requests: demand.hosted_requests,
                supersedes: None,
            })?)
        } else {
            None
        };
        let transition = apply_transition_once(
            store,
            graph,
            &decision.receipt.node_id,
            decision.receipt.transition,
            decision.receipt.transition_idempotency_key.clone(),
            Some(decision.receipt_digest.clone()),
        )?;
        Ok(GovernanceExecutionOutcome {
            decision,
            governance_event_sequence,
            transition,
            resource_reservation,
        })
    }

    pub fn enforce_transition(
        &self,
        store: &RunStore,
        graph: &mut RunGraph,
        request: &GovernanceEvaluationRequest,
    ) -> Result<GovernanceExecutionOutcome, GovernanceEnforcementError> {
        let decision = self.evaluate_and_record(store, request)?;
        self.apply_recorded_transition(store, graph, decision, &request.demand)
    }
}

fn append_governance_receipt(
    store: &RunStore,
    receipt: &GovernanceReceipt,
) -> Result<u64, GovernanceEnforcementError> {
    let receipts = read_governance_receipts(store)?;
    if let Some((index, existing)) = receipts
        .iter()
        .enumerate()
        .find(|(_, existing)| existing.decision_key == receipt.decision_key)
    {
        if existing != receipt {
            return Err(GovernanceEnforcementError::DuplicateReceiptConflict {
                key: receipt.decision_key.clone(),
            });
        }
        return Ok(index as u64 + 1);
    }
    let path = store.governance_receipts_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GovernanceEnforcementError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut bytes = serde_json::to_vec(receipt).map_err(GovernanceEnforcementError::Serialize)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| GovernanceEnforcementError::Io {
            path: path.clone(),
            source,
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|source| GovernanceEnforcementError::Io { path, source })?;
    Ok(receipts.len() as u64 + 1)
}

pub fn read_governance_receipts(
    store: &RunStore,
) -> Result<Vec<GovernanceReceipt>, GovernanceEnforcementError> {
    let path = store.governance_receipts_path();
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => return Err(GovernanceEnforcementError::Io { path, source }),
    };
    if !raw.is_empty() && !raw.ends_with('\n') {
        return Err(GovernanceEnforcementError::CorruptReceiptTail);
    }
    raw.lines()
        .enumerate()
        .map(|(index, line)| {
            let receipt: GovernanceReceipt = serde_json::from_str(line).map_err(|error| {
                GovernanceEnforcementError::CorruptReceipt {
                    line: index + 1,
                    message: error.to_string(),
                }
            })?;
            if receipt.schema_version != GovernanceReceipt::SCHEMA_VERSION {
                return Err(GovernanceEnforcementError::UnsupportedReceiptVersion(
                    receipt.schema_version,
                ));
            }
            Ok(receipt)
        })
        .collect()
}

fn classify_cap(
    name: &str,
    value: f64,
    soft_cap: f64,
    hard_cap: f64,
    soft: &mut Vec<String>,
    hard: &mut Vec<String>,
) {
    if value > hard_cap {
        hard.push(format!("{name}_hard_cap_exhausted"));
    } else if value > soft_cap {
        soft.push(format!("{name}_soft_cap_requires_approval"));
    }
}

fn classify_cap_u64(
    name: &str,
    value: u64,
    soft_cap: u64,
    hard_cap: u64,
    soft: &mut Vec<String>,
    hard: &mut Vec<String>,
) {
    if value > hard_cap {
        hard.push(format!("{name}_hard_cap_exhausted"));
    } else if value > soft_cap {
        soft.push(format!("{name}_soft_cap_requires_approval"));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GovernanceEnforcementError {
    #[error("governance decision key cannot be empty")]
    EmptyDecisionKey,
    #[error("resource demand is invalid")]
    InvalidResourceDemand,
    #[error("hosted requests require a provider")]
    ProviderRequired,
    #[error("provider {0:?} has no configured budget and cannot be selected")]
    UnconfiguredProvider(String),
    #[error("runtime governor budget policy is invalid")]
    InvalidBudgetPolicy,
    #[error("failed to parse runtime governor budget policy: {0}")]
    Toml(toml::de::Error),
    #[error("governance I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize governance receipt: {0}")]
    Serialize(serde_json::Error),
    #[error("governance receipt tail is not newline-terminated")]
    CorruptReceiptTail,
    #[error("corrupt governance receipt at line {line}: {message}")]
    CorruptReceipt { line: usize, message: String },
    #[error("unsupported governance receipt version: {0}")]
    UnsupportedReceiptVersion(String),
    #[error("governance receipt key {key:?} conflicts with an existing receipt")]
    DuplicateReceiptConflict { key: String },
    #[error("governance transition requires a durably recorded receipt")]
    UnrecordedReceipt,
    #[error("{presenter} presented a decision that conflicts with canonical governance")]
    ConflictingPresentation { presenter: String },
    #[error("system clock could not produce a valid budget window")]
    Clock,
    #[error("run store failed during governance enforcement: {0}")]
    Store(#[from] RunStoreError),
    #[error("resource ledger failed during governance enforcement: {0}")]
    Resource(#[from] ResourceLedgerError),
}
