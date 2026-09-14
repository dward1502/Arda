use crate::lifecycle::is_allowlisted_health_url;
use crate::lifecycle::types::{
    Diagnostic, Freshness, HealthState, ObservationMetadata, ObservationSourceKind, Observed,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_HEALTH_BODY_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthProbeSpec {
    pub url: &'static str,
    expected_field: &'static str,
    expected_bool: bool,
}

impl HealthProbeSpec {
    pub const fn json_bool(
        url: &'static str,
        expected_field: &'static str,
        expected_bool: bool,
    ) -> Self {
        Self {
            url,
            expected_field,
            expected_bool,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthProbeError {
    Timeout,
    OutputLimit,
    Transport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthResponse {
    status: u16,
    body: Vec<u8>,
}

impl HealthResponse {
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self { status, body }
    }
}

pub trait HealthClient {
    fn get(
        &self,
        url: &str,
        timeout: Duration,
        max_body_bytes: usize,
    ) -> Result<HealthResponse, HealthProbeError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReqwestHealthClient;

impl HealthClient for ReqwestHealthClient {
    fn get(
        &self,
        url: &str,
        timeout: Duration,
        max_body_bytes: usize,
    ) -> Result<HealthResponse, HealthProbeError> {
        if !is_allowlisted_health_url(url) {
            return Err(HealthProbeError::Transport);
        }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .build()
            .map_err(|_| HealthProbeError::Transport)?;
        let response = client.get(url).send().map_err(|error| {
            if error.is_timeout() {
                HealthProbeError::Timeout
            } else {
                HealthProbeError::Transport
            }
        })?;
        let status = response.status().as_u16();
        let mut body = Vec::new();
        response
            .take(max_body_bytes as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|_| HealthProbeError::Transport)?;
        if body.len() > max_body_bytes {
            return Err(HealthProbeError::OutputLimit);
        }
        Ok(HealthResponse { status, body })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthObservationResult {
    pub observation: Observed<HealthState>,
    pub diagnostic: Option<Diagnostic>,
}

pub fn observe_health<H: HealthClient>(
    client: &H,
    probe: &HealthProbeSpec,
    observed_at: DateTime<Utc>,
) -> HealthObservationResult {
    match client.get(probe.url, HEALTH_TIMEOUT, MAX_HEALTH_BODY_BYTES) {
        Ok(response) => observe_response(probe, response, observed_at),
        Err(error) => unavailable_health(probe, error, observed_at),
    }
}

fn observe_response(
    probe: &HealthProbeSpec,
    response: HealthResponse,
    observed_at: DateTime<Utc>,
) -> HealthObservationResult {
    if !(200..300).contains(&response.status) {
        return result(
            probe,
            observed_at,
            HealthState::Unhealthy,
            Some(diagnostic(
                "health-http-status",
                "Health endpoint returned a non-success status",
            )),
        );
    }
    let payload: Value = match serde_json::from_slice(&response.body) {
        Ok(payload) => payload,
        Err(_) => {
            return result(
                probe,
                observed_at,
                HealthState::Unhealthy,
                Some(diagnostic(
                    "health-malformed",
                    "Health endpoint returned malformed JSON",
                )),
            )
        }
    };
    if payload.get(probe.expected_field).and_then(Value::as_bool) == Some(probe.expected_bool) {
        result(probe, observed_at, HealthState::Healthy, None)
    } else {
        result(
            probe,
            observed_at,
            HealthState::Unhealthy,
            Some(diagnostic(
                "health-unhealthy",
                "Health endpoint did not satisfy its contract",
            )),
        )
    }
}

fn unavailable_health(
    probe: &HealthProbeSpec,
    error: HealthProbeError,
    observed_at: DateTime<Utc>,
) -> HealthObservationResult {
    let (code, message) = match error {
        HealthProbeError::Timeout => ("health-timeout", "Health endpoint timed out"),
        HealthProbeError::OutputLimit => {
            ("health-oversize", "Health response exceeded output limit")
        }
        HealthProbeError::Transport => ("health-unavailable", "Health endpoint is unavailable"),
    };
    result(
        probe,
        observed_at,
        HealthState::Unavailable,
        Some(diagnostic(code, message)),
    )
}

fn result(
    probe: &HealthProbeSpec,
    observed_at: DateTime<Utc>,
    state: HealthState,
    diagnostic: Option<Diagnostic>,
) -> HealthObservationResult {
    HealthObservationResult {
        observation: Observed {
            value: state,
            observation: ObservationMetadata {
                source: ObservationSourceKind::ProtocolProbe,
                source_id: probe.url.to_string(),
                observed_at,
                freshness: Freshness::Fresh,
            },
        },
        diagnostic,
    }
}

fn diagnostic(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(code, message).expect("static health diagnostic is bounded")
}

#[cfg(test)]
mod tests {
    use super::{
        observe_health, HealthClient, HealthProbeError, HealthProbeSpec, HealthResponse,
        ReqwestHealthClient,
    };
    use crate::lifecycle::types::{Freshness, HealthState};
    use chrono::{DateTime, Utc};
    use std::time::Duration;

    struct FixtureClient(Result<HealthResponse, HealthProbeError>);

    impl HealthClient for FixtureClient {
        fn get(
            &self,
            _url: &str,
            _timeout: Duration,
            _max_body_bytes: usize,
        ) -> Result<HealthResponse, HealthProbeError> {
            self.0.clone()
        }
    }

    fn observed_at() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("fixture timestamp")
    }

    fn probe() -> HealthProbeSpec {
        HealthProbeSpec::json_bool("http://127.0.0.1:7171/healthz", "ok", true)
    }

    #[test]
    fn active_but_unhealthy_endpoint_is_not_protocol_healthy() {
        let result = observe_health(
            &FixtureClient(Ok(HealthResponse::new(503, br#"{"ok":false}"#.to_vec()))),
            &probe(),
            observed_at(),
        );

        assert_eq!(result.observation.value, HealthState::Unhealthy);
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("health-http-status")
        );
    }

    #[test]
    fn timeout_is_explicitly_unavailable() {
        let result = observe_health(
            &FixtureClient(Err(HealthProbeError::Timeout)),
            &probe(),
            observed_at(),
        );

        assert_eq!(result.observation.value, HealthState::Unavailable);
        assert_eq!(result.observation.observation.freshness, Freshness::Fresh);
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("health-timeout")
        );
    }

    #[test]
    fn malformed_payload_is_unhealthy() {
        let result = observe_health(
            &FixtureClient(Ok(HealthResponse::new(200, b"not-json".to_vec()))),
            &probe(),
            observed_at(),
        );

        assert_eq!(result.observation.value, HealthState::Unhealthy);
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("health-malformed")
        );
    }

    #[test]
    fn expected_json_payload_is_healthy() {
        let result = observe_health(
            &FixtureClient(Ok(HealthResponse::new(200, br#"{"ok":true}"#.to_vec()))),
            &probe(),
            observed_at(),
        );

        assert_eq!(result.observation.value, HealthState::Healthy);
        assert!(result.diagnostic.is_none());
    }

    #[test]
    fn oversized_response_is_unavailable_without_preserving_body() {
        let result = observe_health(
            &FixtureClient(Err(HealthProbeError::OutputLimit)),
            &probe(),
            observed_at(),
        );

        assert_eq!(result.observation.value, HealthState::Unavailable);
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("health-oversize")
        );
    }

    #[test]
    fn production_client_rejects_non_allowlisted_urls_before_network_io() {
        let error = ReqwestHealthClient
            .get("http://127.0.0.1:9/health", Duration::from_secs(1), 128)
            .expect_err("non-allowlisted URL");
        assert_eq!(error, HealthProbeError::Transport);
    }
}
