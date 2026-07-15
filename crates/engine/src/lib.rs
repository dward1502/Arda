//! arda-engine: the single dependency surface the `arda` daemon (and later the
//! launcher / HUD bridge) uses to reach system services. Re-exports the core
//! spine so callers import from `arda_engine` rather than reaching into the
//! vendored crates directly.

pub mod manwe;

pub mod supervisor;
pub mod harness;
pub mod registry;
pub use arda_service_registry as service_registry;

use tracing::info;

/// Boot the Arda engine. Currently a placeholder that verifies the spine crates
/// are linked. Real service wiring lands here.
pub fn boot() -> anyhow::Result<()> {
    info!("arda-engine boot: linked spine");
    Ok(())
}
