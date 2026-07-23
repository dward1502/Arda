#![cfg(feature = "telemetry")]

use arda_aule::telemetry::{self, Destination, TelemetryEvent, SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

#[derive(Clone, Default)]
struct CaptureLayer {
    spans: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
    events: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
}

#[derive(Default)]
struct FieldVisitor(BTreeMap<String, String>);

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::span::Id,
        _ctx: Context<'_, S>,
    ) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        self.spans.lock().expect("span capture").push(visitor.0);
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        self.events.lock().expect("event capture").push(visitor.0);
    }
}

#[test]
fn telemetry_feature_exports_supported_event_api() {
    assert_eq!(telemetry::crate_namespace(), "arda-aule");
    assert_eq!(SCHEMA_VERSION, "arda.telemetry.v1");

    telemetry::emit(
        TelemetryEvent::new("telemetry.contract.test")
            .destination(Destination::Both)
            .attr("crate", "arda-aule")
            .attr("event", "contract_test"),
    );
}

#[test]
fn trace_destination_preserves_attributes_without_emitting_a_log() {
    let capture = CaptureLayer::default();
    let subscriber = tracing_subscriber::registry().with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        telemetry::emit(
            TelemetryEvent::new("telemetry.trace.test")
                .destination(Destination::Traces)
                .attr("provider", "local")
                .attr("attempt", 2),
        );
    });

    let spans = capture.spans.lock().expect("span capture");
    let serialized = format!("{spans:?}");
    assert!(serialized.contains("provider"));
    assert!(serialized.contains("local"));
    assert!(serialized.contains("attempt"));
    assert!(serialized.contains('2'));
    assert!(capture.events.lock().expect("event capture").is_empty());
}

#[test]
fn log_destination_preserves_attributes_without_creating_a_trace_span() {
    let capture = CaptureLayer::default();
    let subscriber = tracing_subscriber::registry().with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        telemetry::emit(
            TelemetryEvent::new("telemetry.log.test")
                .destination(Destination::Logs)
                .attr("provider", "remote")
                .attr("success", true),
        );
    });

    let events = capture.events.lock().expect("event capture");
    let serialized = format!("{events:?}");
    assert!(serialized.contains("provider"));
    assert!(serialized.contains("remote"));
    assert!(serialized.contains("success"));
    assert!(serialized.contains("true"));
    assert!(capture.spans.lock().expect("span capture").is_empty());
}

#[tokio::test]
async fn configured_opentelemetry_layer_builds_emits_and_shuts_down() {
    std::env::set_var("ARDA_OTLP_ENDPOINT", "http://127.0.0.1:9");
    let layer = telemetry::tracing_layer().expect("configured OpenTelemetry layer");
    let subscriber = tracing_subscriber::registry().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        telemetry::emit(
            TelemetryEvent::new("telemetry.otlp.lifecycle.test")
                .destination(Destination::Traces)
                .attr("lifecycle", "shutdown"),
        );
    });

    telemetry::shutdown();
    std::env::remove_var("ARDA_OTLP_ENDPOINT");
}
