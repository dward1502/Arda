// sigil: TELEMETRY
//! OpenTelemetry substrate for the Arda observability stack.
//!
//! When the `telemetry` feature is enabled, this module provides:
//! - structured tracer initialization from env/config
//! - shutdown/guard helpers for clean process exit
//! - semantic event helpers for runtime/engine surfaces
//!
//! All public items are gated behind `cfg(feature = "telemetry")` so
//! consumers can import `ardea_telemetry` unconditionally; the actual
//! runtime wiring compiles away when the feature is off.

use std::borrow::Cow;

#[cfg(feature = "telemetry")]
pub mod config;
#[cfg(feature = "telemetry")]
pub mod events;
#[cfg(feature = "telemetry")]
pub mod tracer;

pub(crate) mod semantic {
    pub const SERVICE_NAME: &'static str = "arda-runtime";
    pub const SCHEMA_VERSION: &'static str = "arda.telemetry.v1";
}

/// Build identifier appended to structured event payloads.
pub(crate) const ARDA_TELEMETRY_BUILD: &'static str = "arda.telemetry.substrate.v1";

/// Crate-local event namespace. Consumers should use this value for
/// `crate` structured fields so dashboards can filter across crates.
pub fn crate_namespace() -> &'static str {
    "arda-aule"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Traces,
    Logs,
    Both,
}

impl Destination {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Destination::Traces => "traces",
            Destination::Logs => "logs",
            Destination::Both => "traces+logs",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub name: Cow<'static, str>,
    pub destination: Destination,
    pub attributes: Vec<(&'static str, String)>,
}

impl TelemetryEvent {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            destination: Destination::Both,
            attributes: Vec::new(),
        }
    }

    pub fn destination(mut self, destination: Destination) -> Self {
        self.destination = destination;
        self
    }

    pub fn attr(mut self, key: &'static str, value: impl ToString) -> Self {
        self.attributes.push((key, value.to_string()));
        self
    }

    pub(crate) fn schema_headline(&self) -> String {
        format!(
            "{}|{}|{}",
            ARDA_TELEMETRY_BUILD,
            self.destination.as_str(),
            self.name
        )
    }
}
