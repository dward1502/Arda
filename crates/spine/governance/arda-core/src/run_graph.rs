use crate::capability_composition::{
    CapabilityComposition, CapabilityCompositionError, EgressTarget, RouteMode,
};
use crate::service_registry::{
    CapabilityExecutionAdapter, CapabilityHealth, CapabilityMaturity, CapabilityRecord,
    CapabilityRegistry, CapabilityRemovalStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, RunGraphError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(RunGraphError::EmptyIdentifier(stringify!($name)));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(RunId);
string_id!(NodeId);
string_id!(EdgeId);
string_id!(ObjectiveId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    Pending,
    Ready,
    Blocked,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Superseded,
}

impl NodeState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Ready | Self::Blocked | Self::Cancelled)
                | (
                    Self::Ready,
                    Self::Running | Self::Blocked | Self::Cancelled | Self::Superseded
                )
                | (
                    Self::Blocked,
                    Self::Ready | Self::Cancelled | Self::Superseded
                )
                | (
                    Self::Running,
                    Self::Succeeded | Self::Failed | Self::Cancelled
                )
                | (
                    Self::Failed,
                    Self::Ready | Self::Cancelled | Self::Superseded
                )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Inspect,
    Retrieve,
    Research,
    Plan,
    Approval,
    Execute,
    Verify,
    Review,
    Compensate,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    ReadOnly,
    HumanApproval,
    ExecuteWithApproval,
    Verify,
    CompensateWithApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRole {
    PlannerProposer,
    Implementer,
    IndependentVerifier,
    SecurityPrivacyCritic,
    ImplementationRiskCritic,
    LocalSummaryClassification,
    Adjudicator,
    DeterministicTool,
    HumanApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRouteClass {
    Local,
    Hosted,
    Deterministic,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePolicy {
    WorkerReport,
    ProjectNativeChecks,
    DeterministicReceipt,
    HumanDecisionReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerExecutionSpec {
    pub role: WorkerRole,
    pub worker_id: String,
    pub route_id: String,
    pub route_class: WorkerRouteClass,
    pub prompt_digest: String,
    pub allowed_toolsets: BTreeSet<String>,
    pub dependencies: Vec<NodeId>,
    pub deadline_unix_ms: u128,
    pub output_contract: String,
    pub evidence_policy: EvidencePolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub max_joules: f64,
    pub max_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPolicy {
    pub max_attempts: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointMetadata {
    pub sequence: u64,
    pub recovery_token: Option<String>,
    pub checkpoint_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub project_contract_digest: String,
    pub created_by: String,
    #[serde(default)]
    pub parent_receipts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub state: NodeState,
    pub authority: AuthorityClass,
    pub budget: Budget,
    pub retry: RetryPolicy,
    pub timeout_ms: u64,
    pub idempotency_key: String,
    pub input_digest: Option<String>,
    pub output_digest: Option<String>,
    #[serde(default)]
    pub parent_receipts: Vec<String>,
    #[serde(default)]
    pub checkpoint: CheckpointMetadata,
    /// P3 worker contract. `None` preserves readable P1/P2 checkpoints; every
    /// newly planned multi-worker node must persist this contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker: Option<WorkerExecutionSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub parent_receipt: Option<String>,
}

impl RunEdge {
    pub fn new(
        id: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Result<Self, RunGraphError> {
        Ok(Self {
            id: EdgeId::new(id)?,
            from: NodeId::new(from)?,
            to: NodeId::new(to)?,
            parent_receipt: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunGraph {
    pub schema_version: String,
    pub run_id: RunId,
    pub objective_id: ObjectiveId,
    pub nodes: Vec<RunNode>,
    pub edges: Vec<RunEdge>,
    pub provenance: Provenance,
}

impl RunGraph {
    pub const SCHEMA_VERSION: &'static str = "arda.run-graph.v1";

    pub fn from_json_str(raw: &str) -> Result<Self, RunGraphError> {
        let graph: Self = serde_json::from_str(raw)
            .map_err(|error| RunGraphError::InvalidJson(error.to_string()))?;
        graph.validate()?;
        Ok(graph)
    }

    pub fn validate(&self) -> Result<(), RunGraphError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(RunGraphError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }

        let mut nodes = HashMap::with_capacity(self.nodes.len());
        let mut idempotency_keys = HashSet::with_capacity(self.nodes.len());
        for node in &self.nodes {
            if nodes.insert(node.id.clone(), node).is_some() {
                return Err(RunGraphError::DuplicateNode(node.id.clone()));
            }
            if node.idempotency_key.trim().is_empty() {
                return Err(RunGraphError::EmptyIdempotencyKey(node.id.clone()));
            }
            if !idempotency_keys.insert(node.idempotency_key.clone()) {
                return Err(RunGraphError::DuplicateIdempotencyKey(
                    node.idempotency_key.clone(),
                ));
            }
            if node.timeout_ms == 0
                || node.retry.max_attempts == 0
                || !node.budget.max_joules.is_finite()
                || !node.budget.max_cost_usd.is_finite()
                || node.budget.max_joules < 0.0
                || node.budget.max_cost_usd < 0.0
            {
                return Err(RunGraphError::InvalidExecutionBounds(node.id.clone()));
            }
            if let Some(worker) = &node.worker {
                if worker.worker_id.trim().is_empty()
                    || worker.route_id.trim().is_empty()
                    || worker.output_contract.trim().is_empty()
                    || worker.deadline_unix_ms == 0
                    || worker.allowed_toolsets.iter().any(|toolset| {
                        toolset.is_empty()
                            || !toolset.chars().all(|character| {
                                character == '-'
                                    || character == '_'
                                    || character.is_ascii_alphanumeric()
                            })
                    })
                    || !is_sha256_digest(&worker.prompt_digest)
                {
                    return Err(RunGraphError::InvalidWorkerContract(node.id.clone()));
                }
                let role_matches = match worker.role {
                    WorkerRole::PlannerProposer => node.kind == NodeKind::Plan,
                    WorkerRole::Implementer => {
                        node.kind == NodeKind::Execute
                            && node.authority == AuthorityClass::ExecuteWithApproval
                    }
                    WorkerRole::IndependentVerifier => {
                        node.kind == NodeKind::Verify
                            && node.authority == AuthorityClass::Verify
                            && worker.evidence_policy == EvidencePolicy::ProjectNativeChecks
                    }
                    WorkerRole::SecurityPrivacyCritic => {
                        matches!(node.kind, NodeKind::Inspect | NodeKind::Review)
                            && node.authority == AuthorityClass::ReadOnly
                    }
                    WorkerRole::ImplementationRiskCritic => {
                        matches!(node.kind, NodeKind::Inspect | NodeKind::Review)
                            && node.authority == AuthorityClass::ReadOnly
                    }
                    WorkerRole::LocalSummaryClassification => {
                        matches!(node.kind, NodeKind::Inspect | NodeKind::Review)
                            && node.authority == AuthorityClass::ReadOnly
                    }
                    WorkerRole::Adjudicator => {
                        node.kind == NodeKind::Review && node.authority == AuthorityClass::Verify
                    }
                    WorkerRole::DeterministicTool => {
                        worker.route_class == WorkerRouteClass::Deterministic
                            && worker.evidence_policy == EvidencePolicy::DeterministicReceipt
                    }
                    WorkerRole::HumanApproval => {
                        node.kind == NodeKind::Approval
                            && node.authority == AuthorityClass::HumanApproval
                            && worker.route_class == WorkerRouteClass::Human
                            && worker.evidence_policy == EvidencePolicy::HumanDecisionReceipt
                    }
                };
                if !role_matches {
                    return Err(RunGraphError::WorkerRoleMismatch(node.id.clone()));
                }
            }
        }

        let mut edge_ids = HashSet::with_capacity(self.edges.len());
        let mut indegree: HashMap<NodeId, usize> =
            self.nodes.iter().map(|node| (node.id.clone(), 0)).collect();
        let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for edge in &self.edges {
            if !edge_ids.insert(edge.id.clone()) {
                return Err(RunGraphError::DuplicateEdge(edge.id.clone()));
            }
            if !nodes.contains_key(&edge.from) {
                return Err(RunGraphError::MissingNode(edge.from.clone()));
            }
            if !nodes.contains_key(&edge.to) {
                return Err(RunGraphError::MissingNode(edge.to.clone()));
            }
            *indegree.get_mut(&edge.to).expect("validated node") += 1;
            children
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }

        for node in &self.nodes {
            let Some(worker) = &node.worker else {
                continue;
            };
            let declared = worker.dependencies.iter().cloned().collect::<HashSet<_>>();
            let actual = self
                .edges
                .iter()
                .filter(|edge| edge.to == node.id)
                .map(|edge| edge.from.clone())
                .collect::<HashSet<_>>();
            if declared.len() != worker.dependencies.len() || declared != actual {
                return Err(RunGraphError::WorkerDependencyMismatch(node.id.clone()));
            }
        }

        for node in &self.nodes {
            if matches!(
                node.authority,
                AuthorityClass::ExecuteWithApproval | AuthorityClass::CompensateWithApproval
            ) {
                let approved = self.edges.iter().any(|edge| {
                    edge.to == node.id
                        && edge.parent_receipt.is_some()
                        && nodes
                            .get(&edge.from)
                            .is_some_and(|parent| parent.kind == NodeKind::Approval)
                });
                if !approved {
                    return Err(RunGraphError::MissingApprovalParent {
                        node: node.id.clone(),
                    });
                }
            }
        }

        let mut queue: VecDeque<NodeId> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop_front() {
            visited += 1;
            for child in children.get(&id).into_iter().flatten() {
                let degree = indegree.get_mut(child).expect("validated node");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(RunGraphError::Cycle);
        }

        Ok(())
    }

    pub fn transition_node(
        &mut self,
        node_id: &NodeId,
        next: NodeState,
    ) -> Result<(), RunGraphError> {
        let node = self
            .nodes
            .iter_mut()
            .find(|node| &node.id == node_id)
            .ok_or_else(|| RunGraphError::MissingNode(node_id.clone()))?;
        if !node.state.can_transition_to(next) {
            return Err(RunGraphError::InvalidTransition {
                node: node.id.clone(),
                from: node.state,
                to: next,
            });
        }
        node.state = next;
        Ok(())
    }

    pub fn matches_composition_lineage(&self, composition: &CapabilityComposition) -> bool {
        self.run_id.as_str() == composition.lineage.run_id
            && self.objective_id.as_str() == composition.lineage.objective_id
            && self.provenance.project_contract_digest
                == composition.lineage.project_contract_digest
    }

    pub fn deterministic_composition(
        &self,
        composition: &CapabilityComposition,
        registry: &CapabilityRegistry,
        model_recommendations: &BTreeSet<String>,
        trigger: CompositionTrigger,
        prior_receipt_digest: Option<String>,
    ) -> Result<CapabilityCompositionReceipt, DeterministicCompositionError> {
        composition
            .validate()
            .map_err(DeterministicCompositionError::InvalidComposition)?;
        if !self.matches_composition_lineage(composition) {
            return Err(DeterministicCompositionError::LineageMismatch);
        }

        let mut required: BTreeMap<String, BTreeSet<String>> = composition
            .capabilities
            .required
            .iter()
            .map(|capability| {
                (
                    capability.clone(),
                    BTreeSet::from(["signed_request".to_string()]),
                )
            })
            .collect();
        for (role_id, role) in &composition.roles {
            for capability in &role.required_capabilities {
                required
                    .entry(capability.clone())
                    .or_default()
                    .insert(format!("required_role:{role_id}"));
            }
        }

        let records = registry.records().collect::<Vec<_>>();
        let mut selected = Vec::with_capacity(required.len());
        let mut selected_keys = BTreeSet::new();
        let mut selected_reasons: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();

        for (capability_id, sources) in &required {
            if composition.capabilities.forbidden.contains(capability_id) {
                return Err(
                    DeterministicCompositionError::UnsatisfiedRequiredCapability {
                        capability_id: capability_id.clone(),
                        reasons: vec!["forbidden_by_signed_request".to_string()],
                    },
                );
            }
            let mut candidates = records
                .iter()
                .copied()
                .filter(|record| record.declaration.id == *capability_id)
                .map(|record| (record, hard_constraint_reasons(record, composition)))
                .collect::<Vec<_>>();
            let mut eligible = candidates
                .iter()
                .filter(|(_, reasons)| reasons.is_empty())
                .map(|(record, _)| *record)
                .collect::<Vec<_>>();
            eligible.sort_by(|left, right| {
                candidate_score(right, composition, model_recommendations)
                    .cmp(&candidate_score(left, composition, model_recommendations))
                    .then_with(|| right.declaration.version.cmp(&left.declaration.version))
                    .then_with(|| right.declaration.owner.cmp(&left.declaration.owner))
            });
            let Some(chosen) = eligible.first().copied() else {
                let mut reasons = candidates
                    .drain(..)
                    .flat_map(|(_, reasons)| reasons)
                    .collect::<Vec<_>>();
                if reasons.is_empty() {
                    reasons.push("not_registered".to_string());
                }
                reasons.sort();
                reasons.dedup();
                return Err(
                    DeterministicCompositionError::UnsatisfiedRequiredCapability {
                        capability_id: capability_id.clone(),
                        reasons,
                    },
                );
            };
            let key = (
                chosen.declaration.id.clone(),
                chosen.declaration.version.clone(),
            );
            selected_keys.insert(key.clone());
            let mut reasons = sources.iter().cloned().collect::<Vec<_>>();
            if recommendation_matches(chosen, model_recommendations) {
                reasons.push("model_recommended_after_hard_constraints".to_string());
            }
            if route_preference_matches(chosen, composition.route_preferences.mode) {
                reasons.push("selected_route_preference".to_string());
            } else {
                reasons.push("selected_preference_unavailable".to_string());
            }
            reasons.sort();
            selected_reasons.insert(key, reasons);
            selected.push(SelectedCapability {
                id: chosen.declaration.id.clone(),
                version: chosen.declaration.version.clone(),
                owner: chosen.declaration.owner.clone(),
            });
        }

        selected.sort_by(|left, right| {
            (&left.id, &left.version, &left.owner).cmp(&(&right.id, &right.version, &right.owner))
        });
        let mut decisions = records
            .iter()
            .map(|record| {
                let key = (
                    record.declaration.id.clone(),
                    record.declaration.version.clone(),
                );
                let selected = selected_keys.contains(&key);
                let mut reasons = if selected {
                    selected_reasons.get(&key).cloned().unwrap_or_default()
                } else if !required.contains_key(&record.declaration.id) {
                    vec!["not_required_by_signed_contract_or_role".to_string()]
                } else {
                    let hard = hard_constraint_reasons(record, composition);
                    if hard.is_empty() {
                        vec!["alternate_version_not_selected".to_string()]
                    } else {
                        hard
                    }
                };
                if !selected && recommendation_matches(record, model_recommendations) {
                    reasons.push(if required.contains_key(&record.declaration.id) {
                        "model_recommendation_not_selected_after_hard_constraints".to_string()
                    } else {
                        "model_recommendation_ignored_not_required".to_string()
                    });
                }
                reasons.sort();
                reasons.dedup();
                CapabilityCompositionDecision {
                    capability: SelectedCapability {
                        id: record.declaration.id.clone(),
                        version: record.declaration.version.clone(),
                        owner: record.declaration.owner.clone(),
                    },
                    selected,
                    reasons,
                }
            })
            .collect::<Vec<_>>();
        decisions.sort_by(|left, right| {
            (
                &left.capability.id,
                &left.capability.version,
                &left.capability.owner,
            )
                .cmp(&(
                    &right.capability.id,
                    &right.capability.version,
                    &right.capability.owner,
                ))
        });

        Ok(CapabilityCompositionReceipt {
            schema_version: CapabilityCompositionReceipt::SCHEMA_VERSION.to_string(),
            run_id: self.run_id.as_str().to_string(),
            composition_digest: composition
                .digest()
                .map_err(DeterministicCompositionError::InvalidComposition)?,
            registry_constraint_digest: registry_constraint_digest(registry)?,
            trigger,
            prior_receipt_digest,
            model_recommendations: model_recommendations.clone(),
            selected_capabilities: selected,
            decisions,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionTrigger {
    Initial,
    Failure,
    HealthChanged,
    OperatorAmendment,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedCapability {
    pub id: String,
    pub version: String,
    pub owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCompositionDecision {
    pub capability: SelectedCapability,
    pub selected: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCompositionReceipt {
    pub schema_version: String,
    pub run_id: String,
    pub composition_digest: String,
    pub registry_constraint_digest: String,
    pub trigger: CompositionTrigger,
    pub prior_receipt_digest: Option<String>,
    pub model_recommendations: BTreeSet<String>,
    pub selected_capabilities: Vec<SelectedCapability>,
    pub decisions: Vec<CapabilityCompositionDecision>,
}

impl CapabilityCompositionReceipt {
    pub const SCHEMA_VERSION: &'static str = "arda.capability-composition-receipt.v1";

    pub fn digest(&self) -> Result<String, DeterministicCompositionError> {
        let bytes = serde_json::to_vec(self).map_err(|error| {
            DeterministicCompositionError::ReceiptSerialization(error.to_string())
        })?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DeterministicCompositionError {
    #[error("invalid capability composition: {0}")]
    InvalidComposition(#[source] CapabilityCompositionError),
    #[error("capability composition lineage does not match the run graph")]
    LineageMismatch,
    #[error("required capability {capability_id} has no eligible implementation: {reasons:?}")]
    UnsatisfiedRequiredCapability {
        capability_id: String,
        reasons: Vec<String>,
    },
    #[error("failed to serialize capability composition receipt: {0}")]
    ReceiptSerialization(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CapabilityLocation {
    Local,
    Hosted,
}

fn capability_location(record: &CapabilityRecord) -> CapabilityLocation {
    match &record.declaration.execution_adapter {
        CapabilityExecutionAdapter::Service { .. }
        | CapabilityExecutionAdapter::ExternalAdapter { .. } => CapabilityLocation::Local,
        CapabilityExecutionAdapter::ModelWorker { provider, .. }
            if provider.eq_ignore_ascii_case("local") =>
        {
            CapabilityLocation::Local
        }
        CapabilityExecutionAdapter::ModelWorker { .. } => CapabilityLocation::Hosted,
    }
}

fn hard_constraint_reasons(
    record: &CapabilityRecord,
    composition: &CapabilityComposition,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !record.runtime.installed {
        reasons.push("not_installed".to_string());
    }
    if record.runtime.health != CapabilityHealth::Ready {
        reasons.push(format!("health:{}", record.runtime.health.as_str()));
    }
    if !record.runtime.eligible {
        reasons.push("not_eligible".to_string());
    }
    if record.declaration.removal_status == CapabilityRemovalStatus::Removed {
        reasons.push("removed".to_string());
    }
    if !composition
        .authority
        .authority_ceiling
        .permits(record.declaration.authority_ceiling)
    {
        reasons.push("authority_above_signed_ceiling".to_string());
    }
    if !record
        .declaration
        .data_classes
        .is_subset(&composition.sensitivity.data_classes)
    {
        reasons.push("data_class_not_permitted".to_string());
    }
    let location = capability_location(record);
    match composition.route_preferences.mode {
        RouteMode::LocalOnly if location != CapabilityLocation::Local => {
            reasons.push("local_only_route".to_string());
        }
        RouteMode::HostedOnly if location != CapabilityLocation::Hosted => {
            reasons.push("hosted_only_route".to_string());
        }
        _ => {}
    }
    if let CapabilityExecutionAdapter::ModelWorker { provider, .. } =
        &record.declaration.execution_adapter
    {
        if !composition.route_preferences.allowed_providers.is_empty()
            && !composition
                .route_preferences
                .allowed_providers
                .contains(provider)
        {
            reasons.push("provider_not_allowed".to_string());
        }
        if location == CapabilityLocation::Hosted
            && !composition
                .sensitivity
                .permitted_egress
                .contains(&EgressTarget::HostedProvider)
        {
            reasons.push("hosted_egress_not_permitted".to_string());
        }
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn maturity_rank(maturity: CapabilityMaturity) -> u8 {
    match maturity {
        CapabilityMaturity::Experimental => 0,
        CapabilityMaturity::Preview => 1,
        CapabilityMaturity::Stable => 2,
    }
}

fn route_preference_matches(record: &CapabilityRecord, mode: RouteMode) -> bool {
    let location = capability_location(record);
    match mode {
        RouteMode::LocalOnly | RouteMode::PreferLocal => location == CapabilityLocation::Local,
        RouteMode::PreferHosted | RouteMode::HostedOnly => location == CapabilityLocation::Hosted,
    }
}

fn recommendation_matches(record: &CapabilityRecord, recommendations: &BTreeSet<String>) -> bool {
    recommendations.contains(&record.declaration.id)
        || recommendations.contains(&format!(
            "{}@{}",
            record.declaration.id, record.declaration.version
        ))
}

fn candidate_score(
    record: &CapabilityRecord,
    composition: &CapabilityComposition,
    recommendations: &BTreeSet<String>,
) -> (bool, bool, u8) {
    (
        route_preference_matches(record, composition.route_preferences.mode),
        recommendation_matches(record, recommendations),
        maturity_rank(record.declaration.maturity),
    )
}

fn registry_constraint_digest(
    registry: &CapabilityRegistry,
) -> Result<String, DeterministicCompositionError> {
    let snapshot = registry
        .records()
        .map(|record| {
            serde_json::json!({
                "declaration": &record.declaration,
                "installed": record.runtime.installed,
                "health": record.runtime.health,
                "eligible": record.runtime.eligible,
            })
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&snapshot)
        .map_err(|error| DeterministicCompositionError::ReceiptSerialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RunGraphError {
    #[error("invalid run graph JSON: {0}")]
    InvalidJson(String),
    #[error("{0} cannot be empty")]
    EmptyIdentifier(&'static str),
    #[error("unsupported run graph schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("duplicate node id: {0:?}")]
    DuplicateNode(NodeId),
    #[error("duplicate edge id: {0:?}")]
    DuplicateEdge(EdgeId),
    #[error("missing node: {0:?}")]
    MissingNode(NodeId),
    #[error("duplicate idempotency key: {0}")]
    DuplicateIdempotencyKey(String),
    #[error("node {0:?} has an empty idempotency key")]
    EmptyIdempotencyKey(NodeId),
    #[error("node {0:?} has invalid budget, timeout, or retry bounds")]
    InvalidExecutionBounds(NodeId),
    #[error("node {0:?} has an invalid persisted worker contract")]
    InvalidWorkerContract(NodeId),
    #[error("node {0:?} worker role does not match kind, authority, or evidence policy")]
    WorkerRoleMismatch(NodeId),
    #[error("node {0:?} worker dependencies do not match incoming run-graph edges")]
    WorkerDependencyMismatch(NodeId),
    #[error("node {node:?} requires an approval parent with a receipt")]
    MissingApprovalParent { node: NodeId },
    #[error("initial executable graph contains a cycle")]
    Cycle,
    #[error("node {node:?} cannot transition from {from:?} to {to:?}")]
    InvalidTransition {
        node: NodeId,
        from: NodeState,
        to: NodeState,
    },
}
