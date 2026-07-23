// sigil: TELEMETRY
use crate::telemetry::{TelemetryEvent, SCHEMA_VERSION};

pub(crate) fn emit(event: TelemetryEvent) {
    let attributes = serde_json::to_string(&event.attributes).unwrap_or_else(|_| "[]".to_string());

    match event.destination {
        crate::telemetry::Destination::Traces => emit_trace(&event, &attributes),
        crate::telemetry::Destination::Logs => emit_log(&event, &attributes),
        crate::telemetry::Destination::Both => {
            emit_trace(&event, &attributes);
            emit_log(&event, &attributes);
        }
    }
}

fn emit_trace(event: &TelemetryEvent, attributes: &str) {
    let span = tracing::info_span!(
        target: "arda.telemetry.traces",
        "arda.telemetry.event",
        otel.name = %event.name,
        telemetry.name = %event.name,
        telemetry.destination = event.destination.as_str(),
        telemetry.schema_version = SCHEMA_VERSION,
        telemetry.schema = %event.schema_headline(),
        telemetry.attributes = %attributes,
    );
    span.in_scope(|| {});
}

fn emit_log(event: &TelemetryEvent, attributes: &str) {
    tracing::event!(
        target: "arda.telemetry.logs",
        parent: None,
        tracing::Level::INFO,
        telemetry.name = %event.name,
        telemetry.destination = event.destination.as_str(),
        telemetry.schema_version = SCHEMA_VERSION,
        telemetry.schema = %event.schema_headline(),
        telemetry.attributes = %attributes,
        "Arda telemetry event"
    );
}
