pub mod types;

use types::{
    ActiveState, AggregateState, ComponentClass, ComponentObservation, Freshness, HealthState,
};

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
    use super::reduce_aggregate_state;
    use super::types::{
        ActiveState, AggregateState, ComponentClass, ComponentObservation, Freshness, HealthState,
    };

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
}
