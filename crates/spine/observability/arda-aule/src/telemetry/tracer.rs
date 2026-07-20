// sigil: TELEMETRY
use std::sync::OnceLock;

use tracing::Span;
#[cfg(feature = "telemetry")]
use {
    opentelemetry::KeyValue,
    opentelemetry::trace::TracerProvider as OtelTracerProvider,
    opentelemetry_sdk::trace::{self, TracerProvider as SdkTracerProvider},
    opentelemetry_sdk::Resource,
    tracing_opentelemetry::OpenTelemetryLayer,
};

use crate::telemetry::config::TelemetryConfig;

static TRACER_PROVIDER: OnceLock<Option<SdkTracerProvider>> = OnceLock::new();

pub(crate) fn tracer_provider() -> Option<&'static dyn OtelTracerProvider> {
    #[cfg(feature = "telemetry")]
    {
        TRACER_PROVIDER.get_or_init(tracer_provider_inner).as_ref().map(|p| p as &dyn OtelTracerProvider)
    }
    #[cfg(not(feature = "telemetry"))]
    None
}

#[cfg(feature = "telemetry")]
fn tracer_provider_inner() -> Option<SdkTracerProvider> {
    let config = TelemetryConfig::current();
    if config.otlp_endpoint.is_none() {
        return None;
    }

    let provider = opentelemetry_otlp::new_exchange()
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_endpoint(config.otlp_endpoint.clone().expect("endpoint present"))
        .install_batch(opentelemetry_sdk::runtime::Toolkit::Tokio)
        .ok()?;
    Some(provider)
}

#[cfg(feature = "telemetry")]
pub(crate) fn telemetry_resource() -> Resource {
    Resource::new([KeyValue::new("service.name", TelemetryConfig::current().service_name.clone())])
}

#[cfg(feature = "telemetry")]
pub(crate) fn otel_layer() -> Option<OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry::trace::Tracer>> {
    tracer_provider().map(|provider| OpenTelemetryLayer::new(provider.tracer("ardea-runtime")))
}

#[cfg(feature = "telemetry")]
pub(crate) fn shutdown_tracer() {
    if let Some(provider) = TRACER_PROVIDER.get().and_then(|candidate| candidate.as_ref()) {
        let _ = provider.shutdown();
    }
}

pub(crate) fn instrument_span(span: &Span, event: &crate::telemetry::TelemetryEvent) {
    for (key, value) in &event.attributes {
        span.record(*key, tracing::field::display(value));
    }
    if let Some(current_explicit) = std::env::var("ARDA_TELEMETRY_EMIT_HEADLINE").ok().filter(|v| v != "0") {
        tracing::event!(parent: span.id(), tracing::Level::DEBUG, schema = %event.schema_headline(), console_only = true, "{}", current_explicit);
    }
}
