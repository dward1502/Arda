// sigil: ANKH
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum SignalGridRole {
    ProjectionSurface,
    PolicyLayer,
    TransportRouter,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnunimasSignalGridContract {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub role: SignalGridRole,
    pub state_export_path: &'static str,
    pub governance: GovernanceBaseline,
    pub continuity: ContinuityBaseline,
}

#[derive(Debug, Clone, Serialize)]
pub struct GovernanceBaseline {
    pub triad_required: bool,
    pub bacon_lite_required: bool,
    pub joulework_required: bool,
    pub love_equation_required: bool,
    pub soterion_trace_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinuityBaseline {
    pub task_ledger_linked: bool,
    pub memory_checkpoint_expected: bool,
    pub arda_visibility_defined: bool,
}

pub fn contract() -> AnnunimasSignalGridContract {
    AnnunimasSignalGridContract {
        crate_name: "annunimas-signal-grid",
        realm: "communications",
        productizable: true,
        role: SignalGridRole::ProjectionSurface,
        state_export_path: "core/state/annunimas-signal-grid.json",
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
