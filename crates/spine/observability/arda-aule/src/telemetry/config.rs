// sigil: TELEMETRY
use std::sync::OnceLock;

use crate::telemetry::semantic::SERVICE_NAME;

static TELEMETRY_CONFIG: OnceLock<TelemetryConfig> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub(crate) struct TelemetryConfig {
    pub otlp_endpoint: Option<String>,
    pub service_name: String,
}

impl TelemetryConfig {
    pub fn from_env() -> Self {
        let otlp_endpoint = std::env::var("ARDA_OTLP_ENDPOINT")
            .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
            .ok();
        let service_name =
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| SERVICE_NAME.to_string());

        Self {
            otlp_endpoint,
            service_name,
        }
    }

    pub fn current() -> &'static Self {
        TELEMETRY_CONFIG.get_or_init(Self::from_env)
    }
}
