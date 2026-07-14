//! arda-engine: the single dependency surface the `arda` daemon (and later the
//! launcher / HUD bridge) uses to reach Annunimas systems. Re-exports the core
//! spine so callers import from `arda_engine` rather than reaching into the
//! vendored crates directly.

pub mod harness;
pub mod registry;
pub mod supervisor;

pub use annunimas_charon as charon;
pub use annunimas_core as core;
pub use annunimas_onboarding as onboarding;
pub use annunimas_service_registry as service_registry;

use tracing::info;

/// Boot the Arda engine. Currently a placeholder that verifies the spine crates
/// are linked. Real service wiring lands here.
pub fn boot() -> anyhow::Result<()> {
    info!("arda-engine boot: linked Annunimas spine (core, charon, service_registry, onboarding)");
    Ok(())
}
