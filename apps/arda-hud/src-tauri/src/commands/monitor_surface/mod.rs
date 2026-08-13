pub mod browser_capture;
pub mod contract;
pub mod monitor_surface;
pub mod pty_capture;
pub mod registry;
pub mod typed;

pub use monitor_surface::{
    claim_monitor_slot, push_surface_payload, refresh_monitor_slot_lease, release_monitor_slot,
    MonitorSurfaceState,
};

pub use browser_capture::{
    click_browser_capture, get_browser_capture_frame, get_browser_capture_status,
    key_browser_capture, navigate_browser_capture, scroll_browser_capture, start_browser_capture,
    stop_browser_capture, type_browser_capture, BrowserCaptureState,
};

pub use pty_capture::{
    get_pty_capture_status, start_pty_capture, stop_pty_capture, write_pty_capture, PtyCaptureState,
};

pub use typed::{
    claim_monitor_surface, get_monitor_surface_registry, patch_monitor_surface_playback,
    refresh_monitor_surface_lease, release_monitor_surface, restore_monitor_surface_registry,
    TypedMonitorSurfaceState,
};

#[cfg(test)]
mod browser_capture_tests;
#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod typed_tests;
