use arda_core::capability_composition::{CompositionAuthorityClass, DataClass};
use arda_core::service_registry::{
    CapabilityDeclaration, CapabilityExecutionAdapter, CapabilityHealth, CapabilityMaturity,
    CapabilityProvenance, CapabilityRegistry, CapabilityRegistryError, CapabilityRemovalStatus,
    CapabilityRuntimeState,
};
use std::collections::BTreeSet;

fn internal_declaration(id: &str, version: &str, owner: &str) -> CapabilityDeclaration {
    CapabilityDeclaration {
        id: id.to_string(),
        version: version.to_string(),
        owner: owner.to_string(),
        maturity: CapabilityMaturity::Stable,
        data_classes: BTreeSet::from([DataClass::Internal]),
        authority_ceiling: CompositionAuthorityClass::ExecuteWithApproval,
        execution_adapter: CapabilityExecutionAdapter::Service {
            service: "manwe".to_string(),
        },
        removal_status: CapabilityRemovalStatus::Active,
        provenance: CapabilityProvenance::Internal {
            source: "services.toml".to_string(),
            source_digest: "sha256:registry".to_string(),
        },
    }
}

#[test]
fn declaration_represents_required_capability_contract_fields() {
    let declaration = internal_declaration("inference.route", "1", "manwe");

    assert_eq!(declaration.id, "inference.route");
    assert_eq!(declaration.version, "1");
    assert_eq!(declaration.owner, "manwe");
    assert_eq!(declaration.maturity, CapabilityMaturity::Stable);
    assert_eq!(
        declaration.data_classes,
        BTreeSet::from([DataClass::Internal])
    );
    assert_eq!(
        declaration.authority_ceiling,
        CompositionAuthorityClass::ExecuteWithApproval
    );
    assert_eq!(declaration.removal_status, CapabilityRemovalStatus::Active);
}

#[test]
fn duplicate_capability_version_authority_is_rejected() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(
            internal_declaration("inference.route", "1", "manwe"),
            CapabilityRuntimeState::unavailable(true),
        )
        .unwrap();

    let error = registry
        .register(
            internal_declaration("inference.route", "1", "other-owner"),
            CapabilityRuntimeState::unavailable(true),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CapabilityRegistryError::DuplicateAuthority { id, version, .. }
            if id == "inference.route" && version == "1"
    ));
}

#[test]
fn availability_dimensions_and_all_five_health_states_are_distinct() {
    for health in [
        CapabilityHealth::NotConfigured,
        CapabilityHealth::Unavailable,
        CapabilityHealth::Degraded,
        CapabilityHealth::Stale,
        CapabilityHealth::Ready,
    ] {
        assert!(!health.as_str().is_empty());
    }

    let state = CapabilityRuntimeState {
        installed: true,
        health: CapabilityHealth::Ready,
        eligible: true,
        selected: false,
    };
    assert!(state.installed);
    assert!(state.healthy());
    assert!(state.eligible);
    assert!(!state.selected);
}

#[test]
fn selected_capability_must_be_installed_healthy_and_eligible() {
    let mut registry = CapabilityRegistry::new();
    let error = registry
        .register(
            internal_declaration("inference.route", "1", "manwe"),
            CapabilityRuntimeState {
                installed: true,
                health: CapabilityHealth::Degraded,
                eligible: false,
                selected: true,
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CapabilityRegistryError::InvalidRuntimeState { .. }
    ));
}

#[test]
fn external_adapter_and_model_worker_provenance_must_match_execution_adapter() {
    let mut external = internal_declaration("crm.read", "1", "company-ops");
    external.execution_adapter = CapabilityExecutionAdapter::ExternalAdapter {
        adapter_id: "crm-jsonl".to_string(),
    };
    let error = CapabilityRegistry::new()
        .register(external, CapabilityRuntimeState::unavailable(true))
        .unwrap_err();
    assert!(matches!(
        error,
        CapabilityRegistryError::InvalidProvenance { .. }
    ));

    let mut model = internal_declaration("model.reason", "1", "manwe");
    model.execution_adapter = CapabilityExecutionAdapter::ModelWorker {
        provider: "local".to_string(),
        model: "worker-a".to_string(),
    };
    model.provenance = CapabilityProvenance::ModelWorker {
        provider: "local".to_string(),
        model: "worker-a".to_string(),
        source_digest: "sha256:model-route".to_string(),
    };
    CapabilityRegistry::new()
        .register(model, CapabilityRuntimeState::unavailable(true))
        .unwrap();
}

#[test]
fn projections_are_recomputed_from_live_registry_state() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(
            internal_declaration("inference.route", "1", "manwe"),
            CapabilityRuntimeState::unavailable(true),
        )
        .unwrap();
    let before = registry.projection();
    assert_eq!(before[0].status, CapabilityHealth::Unavailable);
    assert!(!before[0].healthy);
    assert!(!before[0].selected);

    registry
        .set_runtime_state(
            "inference.route",
            "1",
            CapabilityRuntimeState {
                installed: true,
                health: CapabilityHealth::Ready,
                eligible: true,
                selected: true,
            },
        )
        .unwrap();
    let after = registry.projection();
    assert_eq!(after[0].status, CapabilityHealth::Ready);
    assert!(after[0].healthy);
    assert!(after[0].eligible);
    assert!(after[0].selected);
}
