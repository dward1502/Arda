// sigil: ANKH
//! Observability contract definition for the Arda observability home crate.

use serde::Serialize;

/// The contract that defines required governance and continuity baselines.
#[derive(Debug, Clone, Serialize)]
pub struct ArdaAuleContract {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance: GovernanceBaseline,
    pub continuity: ContinuityBaseline,
}

/// Governance requirements that must be satisfied by observability surfaces.
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

/// Returns the canonical contract for the Arda observability crate.
pub fn contract() -> ArdaAuleContract {
    ArdaAuleContract {
        crate_name: "arda-aule",
        realm: "observability",
        productizable: true,
        state_export_path: "core/state/arda-aule.json",
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
