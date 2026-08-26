// sigil: TELEMETRY
use std::{collections::HashMap, path::Path, sync::OnceLock};

use crate::telemetry::semantic::SERVICE_NAME;

static TELEMETRY_CONFIG: OnceLock<TelemetryConfig> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OtlpProtocol {
    Grpc,
    HttpProtobuf,
}

pub(crate) fn parse_protocol(value: Option<&str>) -> Option<OtlpProtocol> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("grpc") => Some(OtlpProtocol::Grpc),
        Some("http/protobuf") => Some(OtlpProtocol::HttpProtobuf),
        Some(_) => None,
    }
}

pub(crate) fn read_http_headers(credentials_directory: Option<&Path>) -> HashMap<String, String> {
    let Some(directory) = credentials_directory else {
        return HashMap::new();
    };
    let Ok(value) = std::fs::read_to_string(directory.join("langfuse-otlp-authorization")) else {
        return HashMap::new();
    };
    let value = value.trim();
    if value.is_empty() {
        return HashMap::new();
    }
    HashMap::from([("Authorization".to_string(), value.to_string())])
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TelemetryConfig {
    pub otlp_endpoint: Option<String>,
    pub otlp_protocol: Option<OtlpProtocol>,
    pub http_headers: HashMap<String, String>,
    pub service_name: String,
}

impl TelemetryConfig {
    pub fn from_env() -> Self {
        let otlp_endpoint = std::env::var("ARDA_OTLP_ENDPOINT")
            .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
            .ok();
        let protocol = std::env::var("ARDA_OTLP_PROTOCOL")
            .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL"))
            .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL"))
            .ok();
        let credentials_directory = std::env::var_os("CREDENTIALS_DIRECTORY");
        let service_name =
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| SERVICE_NAME.to_string());

        Self {
            otlp_endpoint,
            otlp_protocol: parse_protocol(protocol.as_deref()),
            http_headers: read_http_headers(credentials_directory.as_deref().map(Path::new)),
            service_name,
        }
    }

    pub fn current() -> &'static Self {
        TELEMETRY_CONFIG.get_or_init(Self::from_env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn protocol_selection_defaults_to_grpc_and_accepts_http_protobuf() {
        assert_eq!(parse_protocol(None), Some(OtlpProtocol::Grpc));
        assert_eq!(parse_protocol(Some("grpc")), Some(OtlpProtocol::Grpc));
        assert_eq!(
            parse_protocol(Some("http/protobuf")),
            Some(OtlpProtocol::HttpProtobuf)
        );
        assert_eq!(parse_protocol(Some("http/json")), None);
        assert_eq!(parse_protocol(Some("unknown")), None);
    }

    #[test]
    fn authorization_credential_is_loaded_as_an_http_header() {
        let root = tempfile::tempdir().expect("credential directory");
        fs::write(
            root.path().join("langfuse-otlp-authorization"),
            "Basic test-placeholder\n",
        )
        .expect("write credential fixture");

        let headers = read_http_headers(Some(root.path()));

        assert_eq!(
            headers.get("Authorization").map(String::as_str),
            Some("Basic test-placeholder")
        );
        assert!(read_http_headers(None).is_empty());
    }
}
