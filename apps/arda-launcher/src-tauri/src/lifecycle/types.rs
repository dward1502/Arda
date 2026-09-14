use chrono::{DateTime, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

pub const SYSTEM_LIFECYCLE_SCHEMA_VERSION: &str = "arda.system-lifecycle.v1";
const MAX_DIAGNOSTIC_CODE_BYTES: usize = 64;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleSchemaVersion {
    #[serde(rename = "arda.system-lifecycle.v1")]
    V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateState {
    Stopped,
    Starting,
    Healthy,
    Degraded,
    Failed,
    Stopping,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentClass {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnablementState {
    Enabled,
    Disabled,
    Static,
    Masked,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveState {
    Inactive,
    Activating,
    Active,
    Deactivating,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Unhealthy,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunningState {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSourceKind {
    Systemd,
    ProtocolProbe,
    Filesystem,
    Process,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationMetadata {
    pub source: ObservationSourceKind,
    pub source_id: String,
    pub observed_at: DateTime<Utc>,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observed<T> {
    pub value: T,
    pub observation: ObservationMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitObservation {
    pub owning_unit: String,
    pub enablement: Observed<EnablementState>,
    pub active_state: Observed<ActiveState>,
    pub sub_state: Observed<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryActionId {
    StartArdaSession,
    RetryHealthCheck,
    InspectComponent,
    RestartHermesGateway,
    RestartNativeHud,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    #[serde(deserialize_with = "deserialize_diagnostic_code")]
    code: String,
    #[serde(deserialize_with = "deserialize_diagnostic_message")]
    message: String,
}

impl Diagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Result<Self, &'static str> {
        let code = code.into();
        let message = message.into();
        if !bounded_string_is_valid(&code, MAX_DIAGNOSTIC_CODE_BYTES) {
            return Err("diagnostic code must contain 1..=64 bytes");
        }
        if !bounded_string_is_valid(&message, MAX_DIAGNOSTIC_MESSAGE_BYTES) {
            return Err("diagnostic message must contain 1..=256 bytes");
        }
        Ok(Self { code, message })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentObservation {
    pub component_id: String,
    pub class: ComponentClass,
    pub unit: UnitObservation,
    pub protocol_health: Observed<HealthState>,
    pub diagnostic: Option<Diagnostic>,
    pub recovery_action: Option<RecoveryActionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HudNativeObservation {
    pub availability: Observed<Availability>,
    pub running: Observed<RunningState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesGatewayObservation {
    pub availability: Observed<Availability>,
    pub protocol_health: Observed<HealthState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemLifecycleSnapshot {
    pub schema_version: LifecycleSchemaVersion,
    pub observed_at: DateTime<Utc>,
    pub aggregate_state: AggregateState,
    pub components: Vec<ComponentObservation>,
    pub hud_native: HudNativeObservation,
    pub hermes_gateway: HermesGatewayObservation,
}

fn deserialize_bounded_string<'de, D>(
    deserializer: D,
    field: &str,
    max_bytes: usize,
) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !bounded_string_is_valid(&value, max_bytes) {
        return Err(D::Error::custom(format!(
            "{field} must contain 1..={max_bytes} bytes"
        )));
    }
    Ok(value)
}

fn bounded_string_is_valid(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes
}

fn deserialize_diagnostic_code<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_string(deserializer, "diagnostic code", MAX_DIAGNOSTIC_CODE_BYTES)
}

fn deserialize_diagnostic_message<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_string(
        deserializer,
        "diagnostic message",
        MAX_DIAGNOSTIC_MESSAGE_BYTES,
    )
}

#[cfg(test)]
fn fixture_observation<T>(
    value: T,
    source: ObservationSourceKind,
    freshness: Freshness,
) -> Observed<T> {
    Observed {
        value,
        observation: ObservationMetadata {
            source,
            source_id: "fixture".to_string(),
            observed_at: DateTime::from_timestamp(1_700_000_000, 0).expect("fixture timestamp"),
            freshness,
        },
    }
}

#[cfg(test)]
impl ComponentObservation {
    pub(super) fn fixture(
        component_id: &str,
        class: ComponentClass,
        active_state: ActiveState,
        health: HealthState,
        freshness: Freshness,
    ) -> Self {
        Self {
            component_id: component_id.to_string(),
            class,
            unit: UnitObservation {
                owning_unit: format!("{component_id}.service"),
                enablement: fixture_observation(
                    EnablementState::Enabled,
                    ObservationSourceKind::Systemd,
                    freshness,
                ),
                active_state: fixture_observation(
                    active_state,
                    ObservationSourceKind::Systemd,
                    freshness,
                ),
                sub_state: fixture_observation(
                    "fixture".to_string(),
                    ObservationSourceKind::Systemd,
                    freshness,
                ),
            },
            protocol_health: fixture_observation(
                health,
                ObservationSourceKind::ProtocolProbe,
                freshness,
            ),
            diagnostic: Some(
                Diagnostic::new("fixture", "fixture diagnostic").expect("fixture diagnostic"),
            ),
            recovery_action: Some(RecoveryActionId::RetryHealthCheck),
        }
    }
}

#[cfg(test)]
impl SystemLifecycleSnapshot {
    fn fixture() -> Self {
        let observed_at = DateTime::from_timestamp(1_700_000_000, 0).expect("fixture timestamp");
        Self {
            schema_version: LifecycleSchemaVersion::V1,
            observed_at,
            aggregate_state: AggregateState::Healthy,
            components: vec![ComponentObservation::fixture(
                "arda-runtime",
                ComponentClass::Required,
                ActiveState::Active,
                HealthState::Healthy,
                Freshness::Fresh,
            )],
            hud_native: HudNativeObservation {
                availability: fixture_observation(
                    Availability::Available,
                    ObservationSourceKind::Filesystem,
                    Freshness::Fresh,
                ),
                running: fixture_observation(
                    RunningState::Running,
                    ObservationSourceKind::Process,
                    Freshness::Fresh,
                ),
            },
            hermes_gateway: HermesGatewayObservation {
                availability: fixture_observation(
                    Availability::Available,
                    ObservationSourceKind::ProtocolProbe,
                    Freshness::Fresh,
                ),
                protocol_health: fixture_observation(
                    HealthState::Healthy,
                    ObservationSourceKind::ProtocolProbe,
                    Freshness::Fresh,
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lifecycle_contract_is_strict_and_versioned() {
        let snapshot = SystemLifecycleSnapshot::fixture();
        let encoded = serde_json::to_value(snapshot).expect("serialize lifecycle snapshot");
        assert_eq!(encoded["schema_version"], SYSTEM_LIFECYCLE_SCHEMA_VERSION);

        let mut unknown_field = encoded.clone();
        unknown_field["unexpected"] = json!(true);
        assert!(serde_json::from_value::<SystemLifecycleSnapshot>(unknown_field).is_err());

        let mut wrong_version = encoded;
        wrong_version["schema_version"] = json!("arda.system-lifecycle.v2");
        assert!(serde_json::from_value::<SystemLifecycleSnapshot>(wrong_version).is_err());
    }

    #[test]
    fn lifecycle_contract_preserves_independent_hud_and_gateway_observations() {
        let snapshot = SystemLifecycleSnapshot::fixture();
        assert_eq!(
            snapshot.hud_native.availability.value,
            Availability::Available
        );
        assert_eq!(snapshot.hud_native.running.value, RunningState::Running);
        assert_eq!(
            snapshot.hermes_gateway.availability.value,
            Availability::Available
        );
        assert_eq!(
            snapshot.hermes_gateway.protocol_health.value,
            HealthState::Healthy
        );
    }

    #[test]
    fn diagnostic_payloads_are_bounded_and_recovery_actions_are_allowlisted() {
        let mut snapshot = serde_json::to_value(SystemLifecycleSnapshot::fixture())
            .expect("serialize lifecycle snapshot");
        snapshot["components"][0]["diagnostic"]["message"] = json!("x".repeat(257));
        assert!(serde_json::from_value::<SystemLifecycleSnapshot>(snapshot).is_err());

        let mut snapshot = serde_json::to_value(SystemLifecycleSnapshot::fixture())
            .expect("serialize lifecycle snapshot");
        snapshot["components"][0]["recovery_action"] = json!("run-arbitrary-shell");
        assert!(serde_json::from_value::<SystemLifecycleSnapshot>(snapshot).is_err());
    }

    #[test]
    fn diagnostic_values_cannot_be_constructed_outside_contract_bounds() {
        assert!(Diagnostic::new("", "message").is_err());
        assert!(Diagnostic::new("code", "").is_err());
        assert!(Diagnostic::new("code", "x".repeat(257)).is_err());

        let diagnostic = Diagnostic::new("unit-failed", "Unit entered failed state")
            .expect("bounded diagnostic");
        assert_eq!(diagnostic.code(), "unit-failed");
        assert_eq!(diagnostic.message(), "Unit entered failed state");
    }
}
