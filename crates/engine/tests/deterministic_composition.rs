use arda_core::capability_composition::{
    CapabilityComposition, CompositionAuthorityClass, DataClass,
};
use arda_core::run_graph::{CompositionTrigger, ObjectiveId, Provenance, RunGraph, RunId};
use arda_core::service_registry::{
    CapabilityDeclaration, CapabilityExecutionAdapter, CapabilityHealth, CapabilityMaturity,
    CapabilityProvenance, CapabilityRegistry, CapabilityRemovalStatus, CapabilityRuntimeState,
};
use arda_engine::observability::CapabilityCompositionObservation;
use arda_engine::runs::{
    compose_run_capabilities, CompositionExecutionError, RunEventKind, RunStore,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn composition() -> CapabilityComposition {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/capability-composition/v1/fixtures/valid-software-project.json");
    let mut composition =
        CapabilityComposition::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    composition.capabilities.required = BTreeSet::from(["verification".to_string()]);
    composition.capabilities.forbidden.clear();
    composition.roles.clear();
    composition.validate().unwrap();
    composition
}

fn graph_for(composition: &CapabilityComposition) -> RunGraph {
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.to_string(),
        run_id: RunId::new(composition.lineage.run_id.clone()).unwrap(),
        objective_id: ObjectiveId::new(composition.lineage.objective_id.clone()).unwrap(),
        nodes: vec![],
        edges: vec![],
        provenance: Provenance {
            project_contract_digest: composition.lineage.project_contract_digest.clone(),
            created_by: "composition-engine-test".to_string(),
            parent_receipts: vec![],
        },
    }
}

fn declaration(version: &str) -> CapabilityDeclaration {
    CapabilityDeclaration {
        id: "verification".to_string(),
        version: version.to_string(),
        owner: format!("verification-v{version}"),
        maturity: CapabilityMaturity::Stable,
        data_classes: BTreeSet::from([DataClass::Internal]),
        authority_ceiling: CompositionAuthorityClass::ReadOnly,
        execution_adapter: CapabilityExecutionAdapter::Service {
            service: format!("verification-v{version}"),
        },
        removal_status: CapabilityRemovalStatus::Active,
        provenance: CapabilityProvenance::Internal {
            source: "engine-test".to_string(),
            source_digest: format!("sha256:verification-v{version}"),
        },
    }
}

fn ready_registry() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    for version in ["1", "2"] {
        registry
            .register(
                declaration(version),
                CapabilityRuntimeState {
                    installed: true,
                    health: CapabilityHealth::Ready,
                    eligible: true,
                    selected: false,
                },
            )
            .unwrap();
    }
    registry
}

#[test]
fn composition_is_durable_and_re_evaluates_only_at_an_explicit_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let composition = composition();
    let graph = graph_for(&composition);
    let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    let mut registry = ready_registry();

    let initial = compose_run_capabilities(
        &store,
        &graph,
        &composition,
        &mut registry,
        &BTreeSet::new(),
        CompositionTrigger::Initial,
    )
    .unwrap();
    assert_eq!(initial.receipt.selected_capabilities[0].version, "2");
    assert_eq!(
        store
            .read_composition_receipt()
            .unwrap()
            .unwrap()
            .digest()
            .unwrap(),
        initial.receipt_digest
    );
    assert!(store
        .composition_receipt_archive_path(&initial.receipt_digest)
        .is_file());
    assert!(registry.get("verification", "2").unwrap().runtime.selected);
    assert!(!registry.get("verification", "1").unwrap().runtime.selected);

    assert!(matches!(
        compose_run_capabilities(
            &store,
            &graph,
            &composition,
            &mut registry,
            &BTreeSet::new(),
            CompositionTrigger::Initial,
        ),
        Err(CompositionExecutionError::ReevaluationBoundaryRequired)
    ));

    assert!(matches!(
        compose_run_capabilities(
            &store,
            &graph,
            &composition,
            &mut registry,
            &BTreeSet::new(),
            CompositionTrigger::HealthChanged,
        ),
        Err(CompositionExecutionError::BoundaryNotObserved(
            CompositionTrigger::HealthChanged
        ))
    ));

    registry
        .set_runtime_state(
            "verification",
            "2",
            CapabilityRuntimeState {
                installed: true,
                health: CapabilityHealth::Unavailable,
                eligible: false,
                selected: false,
            },
        )
        .unwrap();
    let reevaluated = compose_run_capabilities(
        &store,
        &graph,
        &composition,
        &mut registry,
        &BTreeSet::new(),
        CompositionTrigger::HealthChanged,
    )
    .unwrap();

    assert_eq!(reevaluated.receipt.selected_capabilities[0].version, "1");
    assert_eq!(
        reevaluated.receipt.prior_receipt_digest.as_deref(),
        Some(initial.receipt_digest.as_str())
    );
    assert_ne!(reevaluated.receipt_digest, initial.receipt_digest);
    assert!(store
        .composition_receipt_archive_path(&reevaluated.receipt_digest)
        .is_file());
    assert!(registry.get("verification", "1").unwrap().runtime.selected);
    assert!(!registry.get("verification", "2").unwrap().runtime.selected);

    let recovered = store.recover().unwrap();
    assert_eq!(recovered.events.len(), 2);
    assert!(matches!(
        &recovered.events[0].kind,
        RunEventKind::CapabilityCompositionSelected {
            composition_digest,
            trigger: CompositionTrigger::Initial,
            ..
        } if composition_digest == &initial.receipt.composition_digest
    ));
    let observation =
        CapabilityCompositionObservation::from_run_event(&recovered.events[1]).unwrap();
    assert_eq!(observation.trigger, CompositionTrigger::HealthChanged);
    assert_eq!(observation.receipt_digest, reevaluated.receipt_digest);
    assert_eq!(observation.selected_capability_count, 1);
}

#[test]
fn explicit_re_evaluation_cannot_skip_initial_composition() {
    let temp = tempfile::tempdir().unwrap();
    let composition = composition();
    let graph = graph_for(&composition);
    let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    let mut registry = ready_registry();

    assert!(matches!(
        compose_run_capabilities(
            &store,
            &graph,
            &composition,
            &mut registry,
            &BTreeSet::new(),
            CompositionTrigger::Failure,
        ),
        Err(CompositionExecutionError::InitialCompositionRequired)
    ));
}
