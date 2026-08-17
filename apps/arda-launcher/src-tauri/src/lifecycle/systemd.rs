use crate::lifecycle::is_allowlisted_unit;
use crate::lifecycle::types::{
    ActiveState, Diagnostic, EnablementState, Freshness, ObservationMetadata,
    ObservationSourceKind, Observed, UnitObservation,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

const SYSTEMD_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_SYSTEMD_OUTPUT_BYTES: usize = 4 * 1024;
const MAX_SUB_STATE_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemdQueryError {
    Timeout,
    OutputLimit,
    Failed,
    Unavailable,
}

pub trait SystemdQuery {
    fn show_unit(
        &self,
        unit: &str,
        timeout: Duration,
        max_output_bytes: usize,
    ) -> Result<String, SystemdQueryError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemctlQuery;

impl SystemdQuery for SystemctlQuery {
    fn show_unit(
        &self,
        unit: &str,
        timeout: Duration,
        max_output_bytes: usize,
    ) -> Result<String, SystemdQueryError> {
        if !is_allowlisted_unit(unit) {
            return Err(SystemdQueryError::Failed);
        }

        let user = std::env::var("USER").map_err(|_| SystemdQueryError::Unavailable)?;
        let machine = format!("--machine={user}@.host");
        let mut child = Command::new("systemctl")
            .args([
                "--user",
                machine.as_str(),
                "show",
                unit,
                "--property=LoadState,UnitFileState,ActiveState,SubState,WatchdogUSec,WatchdogTimestampMonotonic",
                "--no-pager",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| SystemdQueryError::Unavailable)?;

        let stdout = child.stdout.take().ok_or(SystemdQueryError::Unavailable)?;
        let stderr = child.stderr.take().ok_or(SystemdQueryError::Unavailable)?;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, max_output_bytes));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, max_output_bytes));

        let status = match child
            .wait_timeout(timeout)
            .map_err(|_| SystemdQueryError::Unavailable)?
        {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(SystemdQueryError::Timeout);
            }
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| SystemdQueryError::Unavailable)??;
        let _stderr = stderr_reader
            .join()
            .map_err(|_| SystemdQueryError::Unavailable)??;
        if !status.success()
            && !stdout
                .windows(b"LoadState=".len())
                .any(|window| window == b"LoadState=")
        {
            return Err(SystemdQueryError::Failed);
        }
        String::from_utf8(stdout).map_err(|_| SystemdQueryError::Failed)
    }
}

fn read_bounded(mut reader: impl Read, max_bytes: usize) -> Result<Vec<u8>, SystemdQueryError> {
    let mut kept = Vec::new();
    let mut exceeded = false;
    let mut buffer = [0_u8; 512];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| SystemdQueryError::Unavailable)?;
        if count == 0 {
            break;
        }
        let remaining = max_bytes.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..count.min(remaining)]);
        exceeded |= count > remaining;
    }
    if exceeded {
        Err(SystemdQueryError::OutputLimit)
    } else {
        Ok(kept)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitObservationResult {
    pub observation: UnitObservation,
    pub diagnostic: Option<Diagnostic>,
    pub watchdog_configured: bool,
    pub watchdog_timestamp_monotonic: u64,
}

pub fn observe_unit<S: SystemdQuery>(
    query: &S,
    unit: &str,
    observed_at: DateTime<Utc>,
) -> UnitObservationResult {
    match query.show_unit(unit, SYSTEMD_TIMEOUT, MAX_SYSTEMD_OUTPUT_BYTES) {
        Ok(raw) => observe_unit_from_properties(unit, &raw, observed_at),
        Err(error) => unavailable_unit(unit, error, observed_at),
    }
}

fn observe_unit_from_properties(
    unit: &str,
    raw: &str,
    observed_at: DateTime<Utc>,
) -> UnitObservationResult {
    let properties: BTreeMap<_, _> = raw
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    let load_state = properties.get("LoadState").copied().unwrap_or("unknown");
    let missing = load_state == "not-found";
    let active_state = if missing {
        ActiveState::Unknown
    } else {
        parse_active_state(properties.get("ActiveState").copied().unwrap_or("unknown"))
    };
    let enablement = if missing {
        EnablementState::Unknown
    } else {
        parse_enablement(
            properties
                .get("UnitFileState")
                .copied()
                .unwrap_or("unknown"),
        )
    };
    let sub_state = properties.get("SubState").copied().unwrap_or("unknown");
    let sub_state = if sub_state.len() <= MAX_SUB_STATE_BYTES {
        sub_state
    } else {
        "unknown"
    };
    let metadata = metadata(unit, observed_at, Freshness::Fresh);
    let diagnostic = if missing {
        Some(diagnostic(
            "unit-missing",
            "Allowlisted systemd unit is not installed",
        ))
    } else if active_state == ActiveState::Failed {
        Some(diagnostic(
            "unit-failed",
            "Allowlisted systemd unit is failed",
        ))
    } else {
        None
    };

    UnitObservationResult {
        observation: UnitObservation {
            owning_unit: unit.to_string(),
            enablement: Observed {
                value: enablement,
                observation: metadata.clone(),
            },
            active_state: Observed {
                value: active_state,
                observation: metadata.clone(),
            },
            sub_state: Observed {
                value: sub_state.to_string(),
                observation: metadata,
            },
        },
        diagnostic,
        watchdog_configured: properties
            .get("WatchdogUSec")
            .is_some_and(|value| !matches!(*value, "0" | "0s" | "")),
        watchdog_timestamp_monotonic: properties
            .get("WatchdogTimestampMonotonic")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
    }
}

fn unavailable_unit(
    unit: &str,
    error: SystemdQueryError,
    observed_at: DateTime<Utc>,
) -> UnitObservationResult {
    let (code, message) = match error {
        SystemdQueryError::Timeout => ("unit-query-timeout", "Systemd query timed out"),
        SystemdQueryError::OutputLimit => {
            ("unit-query-oversize", "Systemd query exceeded output limit")
        }
        SystemdQueryError::Failed => ("unit-query-failed", "Systemd query failed"),
        SystemdQueryError::Unavailable => {
            ("unit-query-unavailable", "Systemd query is unavailable")
        }
    };
    let metadata = metadata(unit, observed_at, Freshness::Unknown);
    UnitObservationResult {
        observation: UnitObservation {
            owning_unit: unit.to_string(),
            enablement: Observed {
                value: EnablementState::Unknown,
                observation: metadata.clone(),
            },
            active_state: Observed {
                value: ActiveState::Unknown,
                observation: metadata.clone(),
            },
            sub_state: Observed {
                value: "unknown".to_string(),
                observation: metadata,
            },
        },
        diagnostic: Some(diagnostic(code, message)),
        watchdog_configured: false,
        watchdog_timestamp_monotonic: 0,
    }
}

fn metadata(unit: &str, observed_at: DateTime<Utc>, freshness: Freshness) -> ObservationMetadata {
    ObservationMetadata {
        source: ObservationSourceKind::Systemd,
        source_id: unit.to_string(),
        observed_at,
        freshness,
    }
}

fn parse_active_state(value: &str) -> ActiveState {
    match value {
        "inactive" => ActiveState::Inactive,
        "activating" => ActiveState::Activating,
        "active" => ActiveState::Active,
        "deactivating" => ActiveState::Deactivating,
        "failed" => ActiveState::Failed,
        _ => ActiveState::Unknown,
    }
}

fn parse_enablement(value: &str) -> EnablementState {
    match value {
        "enabled" | "enabled-runtime" => EnablementState::Enabled,
        "disabled" => EnablementState::Disabled,
        "static" | "indirect" | "generated" | "transient" => EnablementState::Static,
        "masked" | "masked-runtime" => EnablementState::Masked,
        _ => EnablementState::Unknown,
    }
}

fn diagnostic(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(code, message).expect("static systemd diagnostic is bounded")
}

#[cfg(test)]
mod tests {
    use super::{observe_unit, SystemctlQuery, SystemdQuery, SystemdQueryError};
    use crate::lifecycle::types::{ActiveState, EnablementState, Freshness};
    use chrono::{DateTime, Utc};
    use std::time::Duration;

    struct FixtureQuery(Result<&'static str, SystemdQueryError>);

    impl SystemdQuery for FixtureQuery {
        fn show_unit(
            &self,
            _unit: &str,
            _timeout: Duration,
            _max_output_bytes: usize,
        ) -> Result<String, SystemdQueryError> {
            self.0.clone().map(str::to_string)
        }
    }

    fn observed_at() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("fixture timestamp")
    }

    #[test]
    fn missing_unit_is_explicitly_unknown() {
        let result = observe_unit(
            &FixtureQuery(Ok(
                "LoadState=not-found\nUnitFileState=\nActiveState=inactive\nSubState=dead\n",
            )),
            "arda.service",
            observed_at(),
        );

        assert_eq!(result.observation.active_state.value, ActiveState::Unknown);
        assert_eq!(
            result.observation.enablement.value,
            EnablementState::Unknown
        );
        assert_eq!(
            result.observation.active_state.observation.freshness,
            Freshness::Fresh
        );
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("unit-missing")
        );
    }

    #[test]
    fn inactive_unit_preserves_enablement_and_substate() {
        let result = observe_unit(
            &FixtureQuery(Ok(
                "LoadState=loaded\nUnitFileState=disabled\nActiveState=inactive\nSubState=dead\n",
            )),
            "arda.service",
            observed_at(),
        );

        assert_eq!(result.observation.active_state.value, ActiveState::Inactive);
        assert_eq!(
            result.observation.enablement.value,
            EnablementState::Disabled
        );
        assert_eq!(result.observation.sub_state.value, "dead");
        assert!(result.diagnostic.is_none());
    }

    #[test]
    fn failed_unit_has_bounded_diagnostic() {
        let result = observe_unit(
            &FixtureQuery(Ok(
                "LoadState=loaded\nUnitFileState=enabled\nActiveState=failed\nSubState=failed\n",
            )),
            "hermes-gateway.service",
            observed_at(),
        );

        assert_eq!(result.observation.active_state.value, ActiveState::Failed);
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("unit-failed")
        );
        assert!(result
            .diagnostic
            .as_ref()
            .is_some_and(|item| item.message().len() <= 256));
    }

    #[test]
    fn timeout_is_unavailable_without_command_output() {
        let result = observe_unit(
            &FixtureQuery(Err(SystemdQueryError::Timeout)),
            "arda.service",
            observed_at(),
        );

        assert_eq!(result.observation.active_state.value, ActiveState::Unknown);
        assert_eq!(
            result.observation.active_state.observation.freshness,
            Freshness::Unknown
        );
        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("unit-query-timeout")
        );
    }

    #[test]
    fn oversized_output_is_unavailable_without_preserving_output() {
        let result = observe_unit(
            &FixtureQuery(Err(SystemdQueryError::OutputLimit)),
            "arda.service",
            observed_at(),
        );

        assert_eq!(
            result.diagnostic.as_ref().map(|item| item.code()),
            Some("unit-query-oversize")
        );
        assert_eq!(result.observation.sub_state.value, "unknown");
    }

    #[test]
    fn production_query_rejects_non_allowlisted_units_before_execution() {
        let error = SystemctlQuery
            .show_unit("ssh.service", Duration::from_secs(1), 128)
            .expect_err("non-allowlisted unit");
        assert_eq!(error, SystemdQueryError::Failed);
    }
}
