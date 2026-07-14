//! Annunimas ORACLE Module
//!
//! Truth confidence scoring for the learning loop.

pub mod context;
pub mod notify;
pub mod pageindex;
pub mod reasoning;
pub mod scoring;
pub mod service;
pub mod transport;

pub use context::ReasoningContext;
pub use notify::OracleNotifier;
pub use pageindex::PageIndex;
pub use reasoning::{
    GateReasoning, GateResult, LoveEquationGuard, OracleEngine, OracleQuery, QueryType, TriadGates,
    Verdict, VerdictGovernance, VerdictOutcome,
};
pub use scoring::{DefaultTruthScorer, TruthScorer, TruthScoringResult};
pub use service::{OracleRuntimePaths, OracleService, ORACLE_RUNTIME_SCHEMA_VERSION};
pub use transport::{expand_home, OracleDaemon, OracleDaemonConfig};
