// sigil: TELEMETRY
//! OpenTelemetry substrate for the Arda observability stack.
//!
//! When the `telemetry` feature is enabled, this module provides:
//! - structured tracer initialization from env/config
//! - shutdown/guard helpers for clean process exit
//! - semantic event helpers for runtime/engine surfaces
//!
//! The module is exported as `arda_aule::telemetry` only when the crate's
//! `telemetry` feature is enabled.

use std::borrow::Cow;

pub mod config;
mod events;
mod tracer;

pub(crate) mod semantic {
    pub const SERVICE_NAME: &'static str = "arda-runtime";
}

/// Semantic schema carried by events emitted through this module.
pub const SCHEMA_VERSION: &str = "arda.telemetry.v1";

/// Build identifier appended to structured event payloads.
pub(crate) const ARDA_TELEMETRY_BUILD: &'static str = "arda.telemetry.substrate.v1";

/// Crate-local event namespace. Consumers should use this value for
/// `crate` structured fields so dashboards can filter across crates.
pub fn crate_namespace() -> &'static str {
    "arda-aule"
}

/// Emit one structured telemetry event through the selected trace/log
/// destination. Event attributes are preserved as structured JSON.
pub fn emit(event: TelemetryEvent) {
    events::emit(event);
}

/// Build the OpenTelemetry tracing layer when an OTLP endpoint is configured.
pub fn tracing_layer() -> Option<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::SdkTracer,
    >,
> {
    tracer::otel_layer()
}

/// Flush and shut down the configured tracer provider.
pub fn shutdown() {
    tracer::shutdown_tracer();
}

/// Return a process-lifetime guard that flushes telemetry when dropped.
pub fn shutdown_guard() -> ShutdownGuard {
    ShutdownGuard
}

pub struct ShutdownGuard;

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        shutdown();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Emit an OpenTelemetry-compatible tracing span.
    Traces,
    /// Emit a root event to the configured `tracing` log layers.
    Logs,
    /// Emit both the tracing span and the local `tracing` log event.
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
