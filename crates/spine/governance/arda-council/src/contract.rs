// sigil: ANKH
//! Blueprint contract definition for the Arda Council agent.
//!
//! Defines the governance and continuity baselines that all new sovereign
//! agents must implement to ensure consistency across the system.
//!
//! # Example
//! ```rust
//! use arda_council::contract::{contract, ArdaCouncilContract};
//!
//! let c: ArdaCouncilContract = contract();
//! assert_eq!(c.realm, "command");
//! assert!(c.productizable);
//! assert_eq!(c.state_export_path, "core/state/arda-council.json");
//! ```

use serde::Serialize;

/// The contract that defines required governance and continuity baselines.
#[derive(Debug, Clone, Serialize)]
pub struct ArdaCouncilContract {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance: GovernanceBaseline,
    pub continuity: ContinuityBaseline,
}

/// Governance requirements that must be satisfied by any sovereign agent.
#[derive(Debug, Clone, Serialize)]
pub struct GovernanceBaseline {
    pub triad_required: bool,
    pub bacon_lite_required: bool,
    pub joulework_required: bool,
    pub love_equation_required: bool,
    pub soterion_trace_required: bool,
}

/// Continuity requirements for state preservation and recovery.
#[derive(Debug, Clone, Serialize)]
pub struct ContinuityBaseline {
    pub task_ledger_linked: bool,
    pub memory_checkpoint_expected: bool,
    pub arda_visibility_defined: bool,
}

/// Returns the canonical contract for the Arda Council blueprint.
pub fn contract() -> ArdaCouncilContract {
    ArdaCouncilContract {
        crate_name: "arda-council",
        realm: "command",
        productizable: true,
        state_export_path: "core/state/arda-council.json",
        governance: GovernanceBaseline {
            triad_required: true,
            bacon_lite_required: true,
            joulework_required: true,
            love_equation_required: true,
            soterion_trace_required: true,
        },
        continuity: ContinuityBaseline {
            task_ledger_linked: true,
            memory_checkpoint_expected: true,
            arda_visibility_defined: true,
        },
    }
}
