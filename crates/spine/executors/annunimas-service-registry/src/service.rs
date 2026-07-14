// sigil: ANKH
//! Service status and system integration definitions.
//!
//! This module provides status reporting and integration points for the Annunimas
//! service registry, including governance compliance checks and startup ordering.
//!
//! # Types
//! - [`AnnunimasServiceRegistryStatus`]: Current service registry status
//!
//! # Functions
//! - [`status()`]: Returns the current registry status
//! - [`startup_order()`]: Calculates service startup sequence

use crate::contract::contract;
use crate::registry::ServiceRegistry;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnnunimasServiceRegistryStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub deterministic_startup_supported: bool,
}

pub fn status() -> AnnunimasServiceRegistryStatus {
    let base = contract();
    let governance_ready = base.governance.triad_required
        && base.governance.bacon_lite_required
        && base.governance.joulework_required
        && base.governance.love_equation_required
        && base.governance.soterion_trace_required
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;
    AnnunimasServiceRegistryStatus {
        crate_name: "annunimas-service-registry",
        realm: base.realm,
        productizable: base.productizable,
        state_export_path: base.state_export_path,
        governance_ready,
        deterministic_startup_supported: true,
    }
}

pub fn startup_order(registry: &ServiceRegistry) -> Result<Vec<String>, String> {
    registry.startup_order()
}
