// sigil: ANKH
use crate::contract::{contract, GovernanceBaseline, SignalGridRole};
use crate::pipeline::{
    crisis_tier_for, route_plan, CommentSignal, CrisisTier, RouteDecision, SignalGridPolicy,
    SignalRoutePlan,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnnunimasSignalGridStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub role: SignalGridRole,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub crisis_tiers_total: usize,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct GovernanceValidation {
    pub ready: bool,
    pub required_validators_total: usize,
}

pub fn status() -> AnnunimasSignalGridStatus {
    let base = contract();
    let governance_ready = validate_governance_baseline(base.governance).ready
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;
    AnnunimasSignalGridStatus {
        crate_name: "annunimas-signal-grid",
        realm: base.realm,
        productizable: base.productizable,
        role: base.role,
        state_export_path: base.state_export_path,
        governance_ready,
        crisis_tiers_total: 3,
    }
}

pub fn route_signal(signal: CommentSignal) -> RouteDecision {
    signal.route()
}

pub fn plan_route(signal: CommentSignal, policy: SignalGridPolicy) -> SignalRoutePlan {
    route_plan(signal, policy)
}

pub fn crisis_tier(negative_spike: bool, legal_threat: bool) -> CrisisTier {
    crisis_tier_for(SignalGridPolicy {
        negative_spike,
        legal_threat,
        export_state: true,
    })
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
