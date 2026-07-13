// sigil: ANKH
use crate::contract::contract;
use crate::council::{CouncilBrief, CouncilQuery};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnnunimasCouncilStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub seats_total: usize,
}

pub fn status() -> AnnunimasCouncilStatus {
    let base = contract();
    let governance_ready = base.governance.triad_required
        && base.governance.bacon_lite_required
        && base.governance.joulework_required
        && base.governance.love_equation_required
        && base.governance.soterion_trace_required
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;
    AnnunimasCouncilStatus {
        crate_name: "annunimas-council",
        realm: base.realm,
        productizable: base.productizable,
        state_export_path: base.state_export_path,
        governance_ready,
        seats_total: 7,
    }
}

pub fn build_brief(query: &CouncilQuery) -> CouncilBrief {
    CouncilBrief::from_query(query)
}
