// sigil: ANKH
//! Service registry contract definitions and governance baselines.
//!
//! This module defines the core contract structure for the Annunimas service registry,
//! including governance requirements and continuity baselines.
//!
//! # Types
//! - [`AnnunimasServiceRegistryContract`]: Main contract structure
//! - [`GovernanceBaseline`]: Governance policy requirements
//! - [`ContinuityBaseline`]: Continuity and state requirements
//!
//! # Functions
//! - [`contract()`]: Returns the default service registry contract

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnnunimasServiceRegistryContract {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
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

pub fn contract() -> AnnunimasServiceRegistryContract {
    AnnunimasServiceRegistryContract {
        crate_name: "annunimas-service-registry",
        realm: "operations",
        productizable: true,
        state_export_path: "core/state/annunimas-service-registry.json",
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
