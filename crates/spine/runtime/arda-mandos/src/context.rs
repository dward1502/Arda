// sigil: REPAIR
use crate::evidence::EvidenceRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ORACLE_SCHEMA_VERSION: &str = "arda.mandos.v1";
pub const MAX_RECENT_VERDICTS: usize = 10;

pub const DEFAULT_MAX_REASONING_DEPTH: usize = 8;
pub const DEFAULT_MAX_REASONING_NODES: usize = 256;
pub const DEFAULT_MAX_REASONING_BYTES: usize = 512 * 1024;

pub type ReasoningNodeId = String;
pub type ReasoningEvidenceId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_bytes: usize,
}

impl Default for ReasoningLimits {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_REASONING_DEPTH,
            max_nodes: DEFAULT_MAX_REASONING_NODES,
            max_bytes: DEFAULT_MAX_REASONING_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningNodeKind {
    Claim,
    Evidence,
    Objection,
    Assumption,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEdgeType {
    Supports,
    ObjectsTo,
    Assumes,
    DependsOn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningNode {
    pub id: ReasoningNodeId,
    pub kind: ReasoningNodeKind,
    /// Concise, operator-facing rationale. Private model traces are deliberately not represented.
    pub public_rationale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_id: Option<ReasoningEvidenceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReasoningEdge {
    pub parent_id: ReasoningNodeId,
    pub child_id: ReasoningNodeId,
    pub edge_type: ReasoningEdgeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub evidence_count: usize,
    pub byte_count: usize,
    pub max_depth: usize,
    pub public_rationales: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningLimitKind {
    Depth,
    Nodes,
    Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningContextError {
    EmptyRationale,
    MissingEvidenceReference,
    DanglingNodeReference {
        node_id: String,
    },
    DanglingEvidenceReference {
        evidence_id: String,
    },
    CycleDetected {
        parent_id: String,
        child_id: String,
    },
    InvalidNodeId {
        node_id: String,
    },
    InvalidEvidenceId {
        evidence_id: String,
    },
    LimitExceeded {
        kind: ReasoningLimitKind,
        actual: usize,
        maximum: usize,
    },
}

impl fmt::Display for ReasoningContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRationale => write!(formatter, "reasoning rationale must not be empty"),
            Self::MissingEvidenceReference => {
                write!(formatter, "reasoning evidence node must reference evidence")
            }
            Self::DanglingNodeReference { node_id } => {
                write!(
                    formatter,
                    "reasoning edge references missing node '{node_id}'"
                )
            }
            Self::DanglingEvidenceReference { evidence_id } => write!(
                formatter,
                "reasoning node references missing evidence '{evidence_id}'"
            ),
            Self::CycleDetected {
                parent_id,
                child_id,
            } => write!(
                formatter,
                "reasoning edge {parent_id} -> {child_id} would create a cycle"
            ),
            Self::InvalidNodeId { node_id } => {
                write!(formatter, "reasoning node ID '{node_id}' is not stable")
            }
            Self::InvalidEvidenceId { evidence_id } => {
                write!(
                    formatter,
                    "reasoning evidence ID '{evidence_id}' is not stable"
                )
            }
            Self::LimitExceeded {
                kind,
                actual,
                maximum,
            } => write!(
                formatter,
                "reasoning {kind:?} limit exceeded: {actual} > {maximum}"
            ),
        }
    }
}

impl std::error::Error for ReasoningContextError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningContext {
    limits: ReasoningLimits,
    nodes: BTreeMap<ReasoningNodeId, ReasoningNode>,
    edges: BTreeSet<ReasoningEdge>,
    evidence: BTreeMap<ReasoningEvidenceId, EvidenceRef>,
}

impl ReasoningContext {
    pub fn new(limits: ReasoningLimits) -> Self {
        Self {
            limits,
            nodes: BTreeMap::new(),
            edges: BTreeSet::new(),
            evidence: BTreeMap::new(),
        }
    }

    pub fn limits(&self) -> ReasoningLimits {
        self.limits
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn nodes(&self) -> &BTreeMap<ReasoningNodeId, ReasoningNode> {
        &self.nodes
    }

    pub fn node(&self, id: &str) -> Option<&ReasoningNode> {
        self.nodes.get(id)
    }

    pub fn edges(&self) -> &BTreeSet<ReasoningEdge> {
        &self.edges
    }

    pub fn evidence(&self) -> &BTreeMap<ReasoningEvidenceId, EvidenceRef> {
        &self.evidence
    }

    pub fn register_evidence(
        &mut self,
        evidence: EvidenceRef,
    ) -> Result<ReasoningEvidenceId, ReasoningContextError> {
        let evidence_id = evidence.digest.clone();
        if self.evidence.contains_key(&evidence_id) {
            return Ok(evidence_id);
        }
        self.evidence.insert(evidence_id.clone(), evidence);
        if let Err(error) = self.validate_size() {
            self.evidence.remove(&evidence_id);
            return Err(error);
        }
        Ok(evidence_id)
    }

    pub fn add_node(
        &mut self,
        kind: ReasoningNodeKind,
        public_rationale: impl Into<String>,
        evidence_id: Option<&str>,
    ) -> Result<ReasoningNodeId, ReasoningContextError> {
        let public_rationale = public_rationale.into();
        if public_rationale.trim().is_empty() {
            return Err(ReasoningContextError::EmptyRationale);
        }
        if kind == ReasoningNodeKind::Evidence && evidence_id.is_none() {
            return Err(ReasoningContextError::MissingEvidenceReference);
        }
        if let Some(evidence_id) = evidence_id {
            if !self.evidence.contains_key(evidence_id) {
                return Err(ReasoningContextError::DanglingEvidenceReference {
                    evidence_id: evidence_id.to_string(),
                });
            }
        }
        let id = stable_node_id(kind, &public_rationale, evidence_id);
        let node = ReasoningNode {
            id: id.clone(),
            kind,
            public_rationale,
            evidence_id: evidence_id.map(ToOwned::to_owned),
        };
        if self.nodes.get(&id) == Some(&node) {
            return Ok(id);
        }
        let actual = self.nodes.len() + 1;
        if actual > self.limits.max_nodes {
            return Err(ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Nodes,
                actual,
                maximum: self.limits.max_nodes,
            });
        }
        self.nodes.insert(id.clone(), node);
        if let Err(error) = self.validate_size() {
            self.nodes.remove(&id);
            return Err(error);
        }
        Ok(id)
    }

    pub fn add_edge(
        &mut self,
        parent_id: &str,
        child_id: &str,
        edge_type: ReasoningEdgeType,
    ) -> Result<(), ReasoningContextError> {
        for node_id in [parent_id, child_id] {
            if !self.nodes.contains_key(node_id) {
                return Err(ReasoningContextError::DanglingNodeReference {
                    node_id: node_id.to_string(),
                });
            }
        }
        let edge = ReasoningEdge {
            parent_id: parent_id.to_string(),
            child_id: child_id.to_string(),
            edge_type,
        };
        if !self.edges.insert(edge.clone()) {
            return Ok(());
        }
        if self.path_exists(child_id, parent_id) {
            self.edges.remove(&edge);
            return Err(ReasoningContextError::CycleDetected {
                parent_id: parent_id.to_string(),
                child_id: child_id.to_string(),
            });
        }
        if let Err(error) = self.validate_depth().and_then(|_| self.validate_size()) {
            self.edges.remove(&edge);
            return Err(error);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ReasoningContextError> {
        if self.nodes.len() > self.limits.max_nodes {
            return Err(ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Nodes,
                actual: self.nodes.len(),
                maximum: self.limits.max_nodes,
            });
        }
        for edge in &self.edges {
            for node_id in [&edge.parent_id, &edge.child_id] {
                if !self.nodes.contains_key(node_id) {
                    return Err(ReasoningContextError::DanglingNodeReference {
                        node_id: node_id.clone(),
                    });
                }
            }
        }
        for (map_id, node) in &self.nodes {
            let expected_id = stable_node_id(
                node.kind,
                &node.public_rationale,
                node.evidence_id.as_deref(),
            );
            if map_id != &node.id || node.id != expected_id {
                return Err(ReasoningContextError::InvalidNodeId {
                    node_id: node.id.clone(),
                });
            }
            if node.kind == ReasoningNodeKind::Evidence && node.evidence_id.is_none() {
                return Err(ReasoningContextError::MissingEvidenceReference);
            }
            if let Some(evidence_id) = &node.evidence_id {
                if !self.evidence.contains_key(evidence_id) {
                    return Err(ReasoningContextError::DanglingEvidenceReference {
                        evidence_id: evidence_id.clone(),
                    });
                }
            }
        }
        for (evidence_id, evidence) in &self.evidence {
            if evidence_id != &evidence.digest {
                return Err(ReasoningContextError::InvalidEvidenceId {
                    evidence_id: evidence_id.clone(),
                });
            }
        }
        self.detect_cycle()?;
        self.validate_depth()?;
        self.validate_size()
    }

    pub fn traverse(&self) -> Result<Vec<&ReasoningNode>, ReasoningContextError> {
        self.validate()?;
        let child_ids: BTreeSet<_> = self
            .edges
            .iter()
            .map(|edge| edge.child_id.as_str())
            .collect();
        let mut roots: Vec<_> = self
            .nodes
            .values()
            .filter(|node| !child_ids.contains(node.id.as_str()))
            .collect();
        roots.sort_by(node_order);

        let mut visited = BTreeSet::new();
        let mut traversal = Vec::with_capacity(self.nodes.len());
        for root in roots {
            self.visit(&root.id, &mut visited, &mut traversal);
        }
        Ok(traversal)
    }

    pub fn summary(&self) -> Result<ReasoningSummary, ReasoningContextError> {
        let traversal = self.traverse()?;
        Ok(ReasoningSummary {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            evidence_count: self.evidence.len(),
            byte_count: self.byte_count(),
            max_depth: self.current_max_depth(),
            public_rationales: traversal
                .into_iter()
                .map(|node| node.public_rationale.clone())
                .collect(),
        })
    }

    pub fn redacted_for_export(&self) -> Self {
        let mut redacted = self.clone();
        for evidence in redacted.evidence.values_mut() {
            *evidence = evidence.clone().redacted_for_export();
        }
        redacted
    }

    fn visit<'a>(
        &'a self,
        node_id: &str,
        visited: &mut BTreeSet<String>,
        traversal: &mut Vec<&'a ReasoningNode>,
    ) {
        if !visited.insert(node_id.to_string()) {
            return;
        }
        if let Some(node) = self.nodes.get(node_id) {
            traversal.push(node);
        }
        let mut children: Vec<_> = self
            .edges
            .iter()
            .filter(|edge| edge.parent_id == node_id)
            .collect();
        children.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| node_order_by_id(&self.nodes, &left.child_id, &right.child_id))
        });
        for edge in children {
            self.visit(&edge.child_id, visited, traversal);
        }
    }

    fn path_exists(&self, start: &str, target: &str) -> bool {
        let mut pending = vec![start];
        let mut visited = BTreeSet::new();
        while let Some(node_id) = pending.pop() {
            if node_id == target {
                return true;
            }
            if visited.insert(node_id) {
                pending.extend(
                    self.edges
                        .iter()
                        .filter(|edge| edge.parent_id == node_id)
                        .map(|edge| edge.child_id.as_str()),
                );
            }
        }
        false
    }

    fn detect_cycle(&self) -> Result<(), ReasoningContextError> {
        for edge in &self.edges {
            let without_current = self
                .edges
                .iter()
                .filter(|candidate| *candidate != edge)
                .any(|candidate| {
                    candidate.parent_id == edge.child_id
                        && (candidate.child_id == edge.parent_id
                            || self.path_exists(&candidate.child_id, &edge.parent_id))
                });
            if edge.parent_id == edge.child_id || without_current {
                return Err(ReasoningContextError::CycleDetected {
                    parent_id: edge.parent_id.clone(),
                    child_id: edge.child_id.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_depth(&self) -> Result<(), ReasoningContextError> {
        let actual = self.current_max_depth();
        if actual > self.limits.max_depth {
            return Err(ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Depth,
                actual,
                maximum: self.limits.max_depth,
            });
        }
        Ok(())
    }

    fn current_max_depth(&self) -> usize {
        let child_ids: BTreeSet<_> = self
            .edges
            .iter()
            .map(|edge| edge.child_id.as_str())
            .collect();
        self.nodes
            .keys()
            .filter(|id| !child_ids.contains(id.as_str()))
            .map(|root| self.depth_from(root, &mut BTreeSet::new()))
            .max()
            .unwrap_or(0)
    }

    fn depth_from(&self, node_id: &str, path: &mut BTreeSet<String>) -> usize {
        if !path.insert(node_id.to_string()) {
            return self.limits.max_depth.saturating_add(1);
        }
        let depth = self
            .edges
            .iter()
            .filter(|edge| edge.parent_id == node_id)
            .map(|edge| 1usize.saturating_add(self.depth_from(&edge.child_id, path)))
            .max()
            .unwrap_or(0);
        path.remove(node_id);
        depth
    }

    fn validate_size(&self) -> Result<(), ReasoningContextError> {
        let actual = self.byte_count();
        if actual > self.limits.max_bytes {
            return Err(ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Bytes,
                actual,
                maximum: self.limits.max_bytes,
            });
        }
        Ok(())
    }

    fn byte_count(&self) -> usize {
        serde_json::to_vec(self).map_or(usize::MAX, |serialized| serialized.len())
    }
}

impl Default for ReasoningContext {
    fn default() -> Self {
        Self::new(ReasoningLimits::default())
    }
}

fn stable_node_id(
    kind: ReasoningNodeKind,
    public_rationale: &str,
    evidence_id: Option<&str>,
) -> ReasoningNodeId {
    let mut hasher = Sha256::new();
    hasher.update(match kind {
        ReasoningNodeKind::Claim => b"claim".as_slice(),
        ReasoningNodeKind::Evidence => b"evidence".as_slice(),
        ReasoningNodeKind::Objection => b"objection".as_slice(),
        ReasoningNodeKind::Assumption => b"assumption".as_slice(),
    });
    hasher.update([0]);
    hasher.update(public_rationale.as_bytes());
    hasher.update([0]);
    if let Some(evidence_id) = evidence_id {
        hasher.update(evidence_id.as_bytes());
    }
    format!("reasoning:sha256:{:x}", hasher.finalize())
}

fn node_order(left: &&ReasoningNode, right: &&ReasoningNode) -> std::cmp::Ordering {
    left.kind
        .cmp(&right.kind)
        .then_with(|| left.public_rationale.cmp(&right.public_rationale))
        .then_with(|| left.id.cmp(&right.id))
}

fn node_order_by_id(
    nodes: &BTreeMap<ReasoningNodeId, ReasoningNode>,
    left: &str,
    right: &str,
) -> std::cmp::Ordering {
    match (nodes.get(left), nodes.get(right)) {
        (Some(left), Some(right)) => node_order(&left, &right),
        _ => left.cmp(right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceRef, EvidenceStance};
    use chrono::{DateTime, Utc};

    fn observed_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-22T12:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc)
    }

    fn evidence(source_id: &str) -> EvidenceRef {
        EvidenceRef::supplied(
            source_id,
            format!("fixture://{source_id}"),
            observed_at(),
            format!("source content for {source_id}"),
        )
        .with_claim("the proposal is supported", EvidenceStance::Supporting)
    }

    #[test]
    fn branching_claims_objections_and_dependencies_traverse_deterministically() {
        fn build(reverse: bool) -> ReasoningContext {
            let mut context = ReasoningContext::new(ReasoningLimits::default());
            let claim = context
                .add_node(ReasoningNodeKind::Claim, "Approve the proposal", None)
                .expect("claim");
            let objection = context
                .add_node(
                    ReasoningNodeKind::Objection,
                    "Rollback evidence is incomplete",
                    None,
                )
                .expect("objection");
            let assumption = context
                .add_node(
                    ReasoningNodeKind::Assumption,
                    "The maintenance window remains available",
                    None,
                )
                .expect("assumption");
            let dependency = context
                .add_node(
                    ReasoningNodeKind::Claim,
                    "A human reviewer is available",
                    None,
                )
                .expect("dependency");

            let mut edges = vec![
                (claim.clone(), objection, ReasoningEdgeType::ObjectsTo),
                (claim.clone(), assumption, ReasoningEdgeType::Assumes),
                (claim, dependency, ReasoningEdgeType::DependsOn),
            ];
            if reverse {
                edges.reverse();
            }
            for (parent, child, edge_type) in edges {
                context
                    .add_edge(&parent, &child, edge_type)
                    .expect("bounded acyclic edge");
            }
            context
        }

        let first = build(false);
        let second = build(true);
        let first_ids: Vec<_> = first
            .traverse()
            .expect("valid traversal")
            .into_iter()
            .map(|node| node.id.clone())
            .collect();
        let second_ids: Vec<_> = second
            .traverse()
            .expect("valid traversal")
            .into_iter()
            .map(|node| node.id.clone())
            .collect();

        assert_eq!(first_ids, second_ids);
        assert_eq!(first.summary().expect("summary").node_count, 4);
        assert_eq!(first.summary().expect("summary").max_depth, 1);
    }

    #[test]
    fn evidence_nodes_require_registered_stable_references() {
        let mut context = ReasoningContext::new(ReasoningLimits::default());
        let source = evidence("report-a");
        let evidence_id = context
            .register_evidence(source.clone())
            .expect("registered evidence");
        let node_id = context
            .add_node(
                ReasoningNodeKind::Evidence,
                "Report A supports the proposal",
                Some(evidence_id.as_str()),
            )
            .expect("evidence node");

        assert_eq!(
            context.node(&node_id).expect("node").evidence_id,
            Some(evidence_id)
        );
        assert_eq!(context.evidence().len(), 1);

        let error = context
            .add_node(
                ReasoningNodeKind::Evidence,
                "Missing report",
                Some("sha256:not-registered"),
            )
            .expect_err("dangling evidence must be rejected");
        assert!(matches!(
            error,
            ReasoningContextError::DanglingEvidenceReference { .. }
        ));

        assert!(matches!(
            context
                .add_node(ReasoningNodeKind::Evidence, "Unbacked evidence claim", None,)
                .expect_err("evidence nodes require evidence IDs"),
            ReasoningContextError::MissingEvidenceReference
        ));
    }

    #[test]
    fn cycles_are_rejected_before_traversal() {
        let mut context = ReasoningContext::new(ReasoningLimits::default());
        let first = context
            .add_node(ReasoningNodeKind::Claim, "First", None)
            .expect("first");
        let second = context
            .add_node(ReasoningNodeKind::Claim, "Second", None)
            .expect("second");
        context
            .add_edge(&first, &second, ReasoningEdgeType::DependsOn)
            .expect("first edge");

        let error = context
            .add_edge(&second, &first, ReasoningEdgeType::DependsOn)
            .expect_err("cycle must be rejected");
        assert!(matches!(error, ReasoningContextError::CycleDetected { .. }));
        assert_eq!(context.traverse().expect("acyclic traversal").len(), 2);

        context.edges.insert(ReasoningEdge {
            parent_id: second,
            child_id: first,
            edge_type: ReasoningEdgeType::DependsOn,
        });
        assert!(matches!(
            context.validate(),
            Err(ReasoningContextError::CycleDetected { .. })
        ));
    }

    #[test]
    fn validation_detects_dangling_evidence_in_deserialized_graphs() {
        let mut context = ReasoningContext::new(ReasoningLimits::default());
        let evidence_id = context
            .register_evidence(evidence("report-a"))
            .expect("registered evidence");
        context
            .add_node(
                ReasoningNodeKind::Evidence,
                "Report A supports the proposal",
                Some(&evidence_id),
            )
            .expect("evidence node");
        context.evidence.clear();

        assert!(matches!(
            context.validate(),
            Err(ReasoningContextError::DanglingEvidenceReference { .. })
        ));
        assert!(context.traverse().is_err());
    }

    #[test]
    fn validation_rejects_tampered_stable_ids() {
        let mut context = ReasoningContext::new(ReasoningLimits::default());
        let stable_id = context
            .add_node(ReasoningNodeKind::Claim, "Stable claim", None)
            .expect("claim");
        let mut node = context.nodes.remove(&stable_id).expect("stored node");
        node.id = "reasoning:tampered".to_string();
        context.nodes.insert(node.id.clone(), node);

        assert!(matches!(
            context.validate(),
            Err(ReasoningContextError::InvalidNodeId { .. })
        ));
    }

    #[test]
    fn node_depth_and_byte_limits_are_enforced() {
        let mut node_limited = ReasoningContext::new(ReasoningLimits {
            max_nodes: 1,
            ..ReasoningLimits::default()
        });
        node_limited
            .add_node(ReasoningNodeKind::Claim, "first", None)
            .expect("within node limit");
        assert!(matches!(
            node_limited
                .add_node(ReasoningNodeKind::Claim, "second", None)
                .expect_err("node limit"),
            ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Nodes,
                ..
            }
        ));

        let mut depth_limited = ReasoningContext::new(ReasoningLimits {
            max_depth: 1,
            ..ReasoningLimits::default()
        });
        let root = depth_limited
            .add_node(ReasoningNodeKind::Claim, "root", None)
            .expect("root");
        let child = depth_limited
            .add_node(ReasoningNodeKind::Claim, "child", None)
            .expect("child");
        let grandchild = depth_limited
            .add_node(ReasoningNodeKind::Claim, "grandchild", None)
            .expect("grandchild");
        depth_limited
            .add_edge(&root, &child, ReasoningEdgeType::DependsOn)
            .expect("within depth limit");
        assert!(matches!(
            depth_limited
                .add_edge(&child, &grandchild, ReasoningEdgeType::DependsOn)
                .expect_err("depth limit"),
            ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Depth,
                ..
            }
        ));

        let mut byte_limited = ReasoningContext::new(ReasoningLimits {
            max_bytes: 8,
            ..ReasoningLimits::default()
        });
        assert!(matches!(
            byte_limited
                .add_node(ReasoningNodeKind::Claim, "more than eight bytes", None)
                .expect_err("byte limit"),
            ReasoningContextError::LimitExceeded {
                kind: ReasoningLimitKind::Bytes,
                ..
            }
        ));
    }

    #[test]
    fn serialized_summary_contains_public_rationales_not_private_model_traces() {
        let mut context = ReasoningContext::new(ReasoningLimits::default());
        context
            .add_node(
                ReasoningNodeKind::Claim,
                "Concise operator-facing rationale",
                None,
            )
            .expect("public rationale");

        let serialized = serde_json::to_string(&context).expect("serialize context");
        assert!(serialized.contains("public_rationale"));
        assert!(!serialized.contains("private_trace"));
        assert!(!serialized.contains("chain_of_thought"));
        assert_eq!(
            context.summary().expect("summary").public_rationales,
            vec!["Concise operator-facing rationale"]
        );
        assert_eq!(
            context.summary().expect("summary").byte_count,
            serde_json::to_vec(&context)
                .expect("serialized graph")
                .len()
        );
    }
}
