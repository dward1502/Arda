//! Annunimas ORACLE Module
//!
//! Truth confidence scoring for the learning loop.

pub mod context;
pub mod evidence;
pub mod notify;
pub mod pageindex;
pub mod reasoning;
pub mod scoring;
pub mod service;
pub mod transport;

pub use context::{
    ReasoningContext, ReasoningContextError, ReasoningEdge, ReasoningEdgeType, ReasoningEvidenceId,
    ReasoningLimitKind, ReasoningLimits, ReasoningNode, ReasoningNodeId, ReasoningNodeKind,
    ReasoningSummary, DEFAULT_MAX_REASONING_BYTES, DEFAULT_MAX_REASONING_DEPTH,
    DEFAULT_MAX_REASONING_NODES,
};
pub use evidence::{
    EvidenceAssessment, EvidenceDisposition, EvidenceFreshness, EvidenceIndependence,
    EvidenceIntegrity, EvidenceKind, EvidenceRef, EvidenceSignal, EvidenceSignalKind,
    EvidenceStance, EVIDENCE_FRESHNESS_WINDOW_DAYS,
};
pub use notify::OracleNotifier;
pub use pageindex::{
    IndexingDisposition, IndexingReport, PageIndex, PageNodeMeta, PageTree, SearchResult, TocEntry,
};
pub use reasoning::{
    GateResult, LoveEquationGuard, OracleEngine, OraclePolicy, OracleQuery, OracleQueryError,
    PolicyVeto, PolicyVetoKind, QueryType, TriadGates, Verdict, VerdictCondition,
    VerdictConditionKind, VerdictGovernance, VerdictOutcome, DEFAULT_ORACLE_POLICY_ID,
    DEFAULT_ORACLE_POLICY_VERSION, MAX_QUERY_CONTEXT_ITEMS, MAX_QUERY_CONTEXT_ITEM_BYTES,
    MAX_QUERY_ID_BYTES, MAX_QUERY_REQUESTER_BYTES, MAX_QUERY_TASK_BYTES,
};
pub use scoring::{DefaultTruthScorer, TruthScorer, TruthScoringResult};
pub use service::{OracleRuntimePaths, OracleService, ORACLE_RUNTIME_SCHEMA_VERSION};
pub use transport::{expand_home, OracleDaemon, OracleDaemonConfig};

#[cfg(test)]
pub(crate) static PLUTUS_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
