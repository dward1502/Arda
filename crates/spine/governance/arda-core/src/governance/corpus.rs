// Phase 0 stub for the GovernanceCorpus + GovernanceGate surface.
// Implementations live in their own crates in Phase 2 (philosopher-chain refactor).
// See human/03-Knowledge/plans/rust-tips-incorporation-plan.md §3 and §5.2.

use serde::{Deserialize, Serialize};

/// Governance gate routing taxonomy.
///
/// `GateSelector` (Phase 3) chooses one of these per validation request.
/// In Phase 2 the philosopher chain emits the chosen variant onto
/// `Decision.gate_used` (proposed v0.2 contract field — see
/// `spec/agent-state-contract.md` §7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceGate {
    None,
    Regex,
    Single,
    Triad,
    Chain,
    Corporate,
}

/// A pluggable governance corpus.
///
/// Phase 2 lands the trait surface; Phase 2b lands concrete impls
/// (Philosophical, Company, Regulatory, BrandVoice) each in their own
/// crate behind Cargo feature flags. The trait deliberately carries no
/// methods yet — naming the type is the precondition for the chain
/// refactor; the method surface is decided alongside the first concrete
/// impl so the abstraction is shaped by a real use case.
pub trait GovernanceCorpus {
    fn name(&self) -> &str;
}
