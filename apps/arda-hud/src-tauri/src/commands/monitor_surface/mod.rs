pub mod contract;
pub mod monitor_surface;
pub mod registry;
pub mod typed;

pub use monitor_surface::{
    claim_monitor_slot, push_surface_payload, refresh_monitor_slot_lease, release_monitor_slot,
    MonitorSurfaceState,
};

pub use typed::{
    claim_monitor_surface, get_monitor_surface_registry, refresh_monitor_surface_lease,
    release_monitor_surface, restore_monitor_surface_registry, TypedMonitorSurfaceState,
};

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod typed_tests;
