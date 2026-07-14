// sigil: ANKH
use crate::contract::{contract, GovernanceBaseline, ToolHarnessRole};
use crate::types::{
    plan_invocation, HarnessError, InvocationEnvelope, InvocationPlan, ToolMetadata,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ArdaToolHarnessStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub role: ToolHarnessRole,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub pipeline_layers: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct GovernanceValidation {
    pub ready: bool,
    pub required_validators_total: usize,
}

pub fn status() -> ArdaToolHarnessStatus {
    let base = contract();
    let governance_ready = validate_governance_baseline(base.governance).ready
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;
    ArdaToolHarnessStatus {
        crate_name: "arda-tool-harness",
        realm: base.realm,
        productizable: base.productizable,
        role: base.role,
        state_export_path: base.state_export_path,
        governance_ready,
        pipeline_layers: &[
            "validate_input",
            "policy_gate",
            "budget_guard",
            "idempotency_gate",
            "execute",
            "normalize_output",
            "observe",
            "record",
        ],
    }
}

pub fn validate_invocation(
    metadata: &ToolMetadata,
    envelope: &InvocationEnvelope,
) -> Result<(), HarnessError> {
    metadata.validate()?;
    envelope.validate(metadata.side_effect_class.is_mutating())?;
    Ok(())
}

pub fn build_invocation_plan(
    metadata: &ToolMetadata,
    envelope: &InvocationEnvelope,
) -> Result<InvocationPlan, HarnessError> {
    validate_invocation(metadata, envelope)?;
    Ok(plan_invocation(metadata))
}

pub fn validate_governance_baseline(governance: GovernanceBaseline) -> GovernanceValidation {
    let required_validators_total = [
        governance.triad_required,
        governance.bacon_lite_required,
        governance.joulework_required,
        governance.love_equation_required,
        governance.soterion_trace_required,
    ]
    .into_iter()
    .filter(|required| *required)
    .count();

    GovernanceValidation {
        ready: required_validators_total == 5,
        required_validators_total,
    }
}
