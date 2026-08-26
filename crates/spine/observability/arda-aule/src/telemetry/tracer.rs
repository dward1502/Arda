// sigil: TELEMETRY
use std::sync::OnceLock;

use {
    opentelemetry::trace::TracerProvider as OtelTracerProvider,
    opentelemetry_otlp::{WithExportConfig, WithHttpConfig},
    opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider},
    opentelemetry_sdk::Resource,
    tracing_opentelemetry::OpenTelemetryLayer,
};

use crate::telemetry::config::{OtlpProtocol, TelemetryConfig};

static TRACER_PROVIDER: OnceLock<Option<SdkTracerProvider>> = OnceLock::new();

pub(crate) fn tracer_provider() -> Option<&'static SdkTracerProvider> {
    TRACER_PROVIDER.get_or_init(tracer_provider_inner).as_ref()
}

fn tracer_provider_inner() -> Option<SdkTracerProvider> {
    let config = TelemetryConfig::current();
    let endpoint = config.otlp_endpoint.as_ref()?;
    let exporter = match config.otlp_protocol? {
        OtlpProtocol::Grpc => opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build(),
        OtlpProtocol::HttpProtobuf => opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .with_headers(config.http_headers.clone())
            .build(),
    }
    .ok()?;
    Some(
        SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(telemetry_resource())
            .build(),
    )
}

pub(crate) fn telemetry_resource() -> Resource {
    Resource::builder()
        .with_service_name(TelemetryConfig::current().service_name.clone())
        .build()
}

pub(crate) fn otel_layer() -> Option<OpenTelemetryLayer<tracing_subscriber::Registry, SdkTracer>> {
    tracer_provider()
        .map(|provider| tracing_opentelemetry::layer().with_tracer(provider.tracer("arda-runtime")))
}

pub(crate) fn shutdown_tracer() {
    if let Some(provider) = TRACER_PROVIDER
        .get()
        .and_then(|candidate| candidate.as_ref())
    {
        let _ = provider.shutdown();
    }
}
