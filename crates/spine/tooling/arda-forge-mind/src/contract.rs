// sigil: ANKH
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactPolicy {
    PrototypeAllowed,
    ProductionOnly,
    ResearchOnly,
    VerificationRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeContract {
    pub version: String,
    pub governance_level: u8, // 1-5
    pub artifact_policy: ArtifactPolicy,
    pub memory_checkpoint: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ForgeMindRole {
    BlueprintWorkflow,
    PlanningLayer,
    BuildExecutionAuthority,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArdaForgeMindContract {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub role: ForgeMindRole,
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

pub fn contract() -> ArdaForgeMindContract {
    ArdaForgeMindContract {
        crate_name: "arda-forge-mind",
        realm: "operations",
        productizable: true,
        role: ForgeMindRole::BlueprintWorkflow,
        state_export_path: "core/state/arda-forge-mind.json",
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
