pub mod commands;
pub mod health;
pub mod systemd;
pub mod types;

use chrono::{DateTime, Utc};
use health::{
    observe_health, HealthClient, HealthObservationResult, HealthProbeSpec, ReqwestHealthClient,
};
use systemd::{observe_unit, SystemctlQuery, SystemdQuery};
use types::{
    ActiveState, AggregateState, ComponentClass, ComponentObservation, Diagnostic, Freshness,
    HealthState, ObservationMetadata, ObservationSourceKind, Observed, RecoveryActionId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentSpec {
    pub component_id: &'static str,
    pub class: ComponentClass,
    pub unit: &'static str,
    pub health_probe: Option<HealthProbeSpec>,
    pub recovery_action: RecoveryActionId,
}

const REQUIRED_COMPONENTS: [ComponentSpec; 2] = [
    ComponentSpec {
        component_id: "arda-runtime",
        class: ComponentClass::Required,
        unit: "arda.service",
        health_probe: Some(HealthProbeSpec::json_bool(
            "http://127.0.0.1:7171/healthz",
            "ok",
            true,
        )),
        recovery_action: RecoveryActionId::StartArdaSession,
    },
    ComponentSpec {
        component_id: "hermes-gateway",
        class: ComponentClass::Required,
        unit: "hermes-gateway.service",
        health_probe: None,
        recovery_action: RecoveryActionId::RestartHermesGateway,
    },
];

pub fn required_component_specs() -> &'static [ComponentSpec] {
    &REQUIRED_COMPONENTS
}

pub(super) fn is_allowlisted_unit(unit: &str) -> bool {
    REQUIRED_COMPONENTS.iter().any(|spec| spec.unit == unit)
}

pub(super) fn is_allowlisted_health_url(url: &str) -> bool {
    REQUIRED_COMPONENTS
        .iter()
        .filter_map(|spec| spec.health_probe)
        .any(|probe| probe.url == url)
}

pub fn observe_component<S: SystemdQuery, H: HealthClient>(
    spec: &ComponentSpec,
    systemd: &S,
    health: &H,
    observed_at: DateTime<Utc>,
) -> ComponentObservation {
    let unit = observe_unit(systemd, spec.unit, observed_at);
    let protocol = spec
        .health_probe
        .as_ref()
        .map(|probe| observe_health(health, probe, observed_at))
        .unwrap_or_else(|| unavailable_health(spec.component_id, observed_at));
    let diagnostic = unit.diagnostic.or(protocol.diagnostic);

    ComponentObservation {
        component_id: spec.component_id.to_string(),
        class: spec.class,
        unit: unit.observation,
        protocol_health: protocol.observation,
        diagnostic,
        recovery_action: Some(spec.recovery_action),
    }
}

pub fn observe_required_components(observed_at: DateTime<Utc>) -> Vec<ComponentObservation> {
    let systemd = SystemctlQuery;
    let health = ReqwestHealthClient;
    required_component_specs()
        .iter()
        .map(|spec| observe_component(spec, &systemd, &health, observed_at))
        .collect()
}

fn unavailable_health(component_id: &str, observed_at: DateTime<Utc>) -> HealthObservationResult {
    HealthObservationResult {
        observation: Observed {
            value: HealthState::Unavailable,
            observation: ObservationMetadata {
                source: ObservationSourceKind::ProtocolProbe,
                source_id: format!("{component_id}:not-configured"),
                observed_at,
                freshness: Freshness::Fresh,
            },
        },
        diagnostic: Some(
            Diagnostic::new(
                "health-unavailable",
                "No allowlisted protocol health probe is configured",
            )
            .expect("static diagnostic is bounded"),
        ),
    }
}

/// Reduce independently collected component observations into launcher state.
///
/// Transition states are derived from process state, but an active process is
/// healthy only when its separately observed protocol health is fresh and
/// healthy.
pub fn reduce_aggregate_state(components: &[ComponentObservation]) -> AggregateState {
    let required: Vec<_> = components
        .iter()
        .filter(|component| component.class == ComponentClass::Required)
        .collect();

    if required.is_empty() {
        return AggregateState::Unknown;
    }

    if required.iter().any(|component| {
        component.unit.active_state.observation.freshness != Freshness::Fresh
            || component.protocol_health.observation.freshness != Freshness::Fresh
    }) {
        return AggregateState::Unknown;
    }

    if required
        .iter()
        .any(|component| component.unit.active_state.value == ActiveState::Failed)
        || required
            .iter()
            .any(|component| component.protocol_health.value == HealthState::Unhealthy)
    {
        return AggregateState::Failed;
    }

    if required
        .iter()
        .any(|component| component.unit.active_state.value == ActiveState::Activating)
    {
        return AggregateState::Starting;
    }

    if required
        .iter()
        .any(|component| component.unit.active_state.value == ActiveState::Deactivating)
    {
        return AggregateState::Stopping;
    }

    if required
        .iter()
        .all(|component| component.unit.active_state.value == ActiveState::Inactive)
    {
        return AggregateState::Stopped;
    }

    let required_healthy = required.iter().all(|component| {
        component.unit.active_state.value == ActiveState::Active
            && component.protocol_health.value == HealthState::Healthy
    });
    if !required_healthy {
        return AggregateState::Unknown;
    }

    let optional_degraded = components
        .iter()
        .filter(|component| component.class == ComponentClass::Optional)
        .any(|component| {
            component.unit.active_state.observation.freshness != Freshness::Fresh
                || component.protocol_health.observation.freshness != Freshness::Fresh
                || component.unit.active_state.value != ActiveState::Active
                || component.protocol_health.value != HealthState::Healthy
        });

    if optional_degraded {
        AggregateState::Degraded
    } else {
        AggregateState::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::health::{HealthClient, HealthProbeError, HealthResponse};
    use super::observe_component;
    use super::reduce_aggregate_state;
    use super::required_component_specs;
    use super::systemd::{SystemdQuery, SystemdQueryError};
    use super::types::{
        ActiveState, AggregateState, ComponentClass, ComponentObservation, Freshness, HealthState,
    };
    use chrono::{DateTime, Utc};
    use std::time::Duration;

    struct ActiveSystemd;

    impl SystemdQuery for ActiveSystemd {
        fn show_unit(
            &self,
            _unit: &str,
            _timeout: Duration,
            _max_output_bytes: usize,
        ) -> Result<String, SystemdQueryError> {
            Ok(
                "LoadState=loaded\nUnitFileState=enabled\nActiveState=active\nSubState=running\n"
                    .to_string(),
            )
        }
    }

    struct UnhealthyHealth;

    impl HealthClient for UnhealthyHealth {
        fn get(
            &self,
            _url: &str,
            _timeout: Duration,
            _max_body_bytes: usize,
        ) -> Result<HealthResponse, HealthProbeError> {
            Ok(HealthResponse::new(503, br#"{"ok":false}"#.to_vec()))
        }
    }

    fn observed_at() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("fixture timestamp")
    }

    fn component(
        component_id: &str,
        class: ComponentClass,
        active_state: ActiveState,
        health: HealthState,
        freshness: Freshness,
    ) -> ComponentObservation {
        ComponentObservation::fixture(component_id, class, active_state, health, freshness)
    }

    #[test]
    fn aggregate_state_reduction_covers_lifecycle_truth_table() {
        let cases = [
            (
                "stopped",
                vec![component(
                    "arda-runtime",
                    ComponentClass::Required,
                    ActiveState::Inactive,
                    HealthState::Unavailable,
                    Freshness::Fresh,
                )],
                AggregateState::Stopped,
            ),
            (
                "starting",
                vec![component(
                    "arda-runtime",
                    ComponentClass::Required,
                    ActiveState::Activating,
                    HealthState::Unknown,
                    Freshness::Fresh,
                )],
                AggregateState::Starting,
            ),
            (
                "required failure",
                vec![component(
                    "hermes-gateway",
                    ComponentClass::Required,
                    ActiveState::Active,
                    HealthState::Unhealthy,
                    Freshness::Fresh,
                )],
                AggregateState::Failed,
            ),
            (
                "optional degradation",
                vec![
                    component(
                        "arda-runtime",
                        ComponentClass::Required,
                        ActiveState::Active,
                        HealthState::Healthy,
                        Freshness::Fresh,
                    ),
                    component(
                        "arda-relic",
                        ComponentClass::Optional,
                        ActiveState::Failed,
                        HealthState::Unavailable,
                        Freshness::Fresh,
                    ),
                ],
                AggregateState::Degraded,
            ),
            (
                "stale required observation",
                vec![component(
                    "arda-runtime",
                    ComponentClass::Required,
                    ActiveState::Active,
                    HealthState::Healthy,
                    Freshness::Stale,
                )],
                AggregateState::Unknown,
            ),
            (
                "all healthy",
                vec![
                    component(
                        "arda-runtime",
                        ComponentClass::Required,
                        ActiveState::Active,
                        HealthState::Healthy,
                        Freshness::Fresh,
                    ),
                    component(
                        "arda-relic",
                        ComponentClass::Optional,
                        ActiveState::Active,
                        HealthState::Healthy,
                        Freshness::Fresh,
                    ),
                ],
                AggregateState::Healthy,
            ),
            (
                "stopping",
                vec![component(
                    "arda-runtime",
                    ComponentClass::Required,
                    ActiveState::Deactivating,
                    HealthState::Unknown,
                    Freshness::Fresh,
                )],
                AggregateState::Stopping,
            ),
            (
                "active process with unknown protocol health",
                vec![component(
                    "hermes-gateway",
                    ComponentClass::Required,
                    ActiveState::Active,
                    HealthState::Unknown,
                    Freshness::Fresh,
                )],
                AggregateState::Unknown,
            ),
            (
                "mixed required process states",
                vec![
                    component(
                        "arda-runtime",
                        ComponentClass::Required,
                        ActiveState::Active,
                        HealthState::Healthy,
                        Freshness::Fresh,
                    ),
                    component(
                        "hermes-gateway",
                        ComponentClass::Required,
                        ActiveState::Inactive,
                        HealthState::Unavailable,
                        Freshness::Fresh,
                    ),
                ],
                AggregateState::Unknown,
            ),
            ("no observations", vec![], AggregateState::Unknown),
        ];

        for (name, components, expected) in cases {
            assert_eq!(
                reduce_aggregate_state(&components),
                expected,
                "case: {name}"
            );
        }
    }

    #[test]
    fn required_component_allowlist_matches_verified_local_authorities() {
        let specs = required_component_specs();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].component_id, "arda-runtime");
        assert_eq!(specs[0].unit, "arda.service");
        assert!(specs[0].health_probe.is_some());
        assert_eq!(specs[1].component_id, "hermes-gateway");
        assert_eq!(specs[1].unit, "hermes-gateway.service");
        assert!(specs[1].health_probe.is_none());
        assert!(specs
            .iter()
            .all(|spec| spec.class == ComponentClass::Required));
        assert!(specs.iter().all(
            |spec| !spec.component_id.contains("varda") && !spec.component_id.contains("relic")
        ));
    }

    #[test]
    fn active_process_with_unhealthy_endpoint_is_not_healthy() {
        let component = observe_component(
            &required_component_specs()[0],
            &ActiveSystemd,
            &UnhealthyHealth,
            observed_at(),
        );

        assert_eq!(component.unit.active_state.value, ActiveState::Active);
        assert_eq!(component.protocol_health.value, HealthState::Unhealthy);
        assert_eq!(reduce_aggregate_state(&[component]), AggregateState::Failed);
    }

    #[test]
    fn active_gateway_without_verified_protocol_probe_remains_unavailable() {
        let component = observe_component(
            &required_component_specs()[1],
            &ActiveSystemd,
            &UnhealthyHealth,
            observed_at(),
        );

        assert_eq!(component.unit.active_state.value, ActiveState::Active);
        assert_eq!(component.protocol_health.value, HealthState::Unavailable);
        assert_eq!(
            reduce_aggregate_state(&[component]),
            AggregateState::Unknown
        );
    }
}
