use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

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
