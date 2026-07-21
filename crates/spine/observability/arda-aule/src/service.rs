// sigil: ANKH
//! Observability readiness probe for the Arda `arda-aule` crate.
//!
//! This module exposes status/build_brief utilities tied to the
//! observability contract, not governance-only council semantics.

use crate::contract::contract;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ArdaAuleStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub observability_ready: bool,
}

pub fn status() -> ArdaAuleStatus {
    let base = contract();
    let governance_ready = base.governance.triad_required
        && base.governance.bacon_lite_required
        && base.governance.joulework_required
        && base.governance.love_equation_required
        && base.governance.soterion_trace_required
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;
    ArdaAuleStatus {
        crate_name: "arda-aule",
        realm: base.realm,
        productizable: base.productizable,
        state_export_path: base.state_export_path,
        governance_ready,
        observability_ready: governance_ready,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservabilityBrief {
    pub crate_name: &'static str,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
}

impl ObservabilityBrief {
    pub fn from_status(status: &ArdaAuleStatus) -> Self {
        Self {
            crate_name: status.crate_name,
            state_export_path: status.state_export_path,
            governance_ready: status.governance_ready,
        }
    }
}
