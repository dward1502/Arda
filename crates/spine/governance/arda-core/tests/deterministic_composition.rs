use arda_core::capability_composition::{
    CapabilityComposition, CompositionAuthorityClass, DataClass, EgressTarget, RouteMode,
};
use arda_core::run_graph::{
    CompositionTrigger, DeterministicCompositionError, ObjectiveId, Provenance, RunGraph, RunId,
};
use arda_core::service_registry::{
    CapabilityDeclaration, CapabilityExecutionAdapter, CapabilityHealth, CapabilityMaturity,
    CapabilityProvenance, CapabilityRegistry, CapabilityRemovalStatus, CapabilityRuntimeState,
};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

fn fixture(name: &str) -> CapabilityComposition {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec/capability-composition/v1/fixtures")
        .join(name);
    CapabilityComposition::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap()
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
            created_by: "deterministic-composition-test".to_string(),
            parent_receipts: vec![],
        },
    }
}

fn declaration(
    id: &str,
    authority_ceiling: CompositionAuthorityClass,
    data_classes: &[DataClass],
    execution_adapter: CapabilityExecutionAdapter,
    provenance: CapabilityProvenance,
) -> CapabilityDeclaration {
    CapabilityDeclaration {
        id: id.to_string(),
        version: "1".to_string(),
        owner: format!("owner:{id}"),
        maturity: CapabilityMaturity::Stable,
        data_classes: data_classes.iter().copied().collect(),
        authority_ceiling,
        execution_adapter,
        removal_status: CapabilityRemovalStatus::Active,
        provenance,
    }
}

fn service_capability(
    id: &str,
    authority: CompositionAuthorityClass,
    data_classes: &[DataClass],
) -> CapabilityDeclaration {
    declaration(
        id,
        authority,
        data_classes,
        CapabilityExecutionAdapter::Service {
            service: format!("service:{id}"),
        },
        CapabilityProvenance::Internal {
            source: "test-registry".to_string(),
            source_digest: format!("sha256:{id}"),
        },
    )
}

fn external_adapter_capability(
    id: &str,
    authority: CompositionAuthorityClass,
    data_classes: &[DataClass],
) -> CapabilityDeclaration {
    declaration(
        id,
        authority,
        data_classes,
        CapabilityExecutionAdapter::ExternalAdapter {
            adapter_id: id.to_string(),
        },
        CapabilityProvenance::ExternalAdapter {
            adapter_id: id.to_string(),
            adapter_version: "1".to_string(),
            source_digest: format!("sha256:{id}"),
        },
    )
}

fn model_capability(id: &str, provider: &str) -> CapabilityDeclaration {
    declaration(
        id,
        CompositionAuthorityClass::Propose,
        &[DataClass::Internal],
        CapabilityExecutionAdapter::ModelWorker {
            provider: provider.to_string(),
            model: "worker".to_string(),
        },
        CapabilityProvenance::ModelWorker {
            provider: provider.to_string(),
            model: "worker".to_string(),
            source_digest: format!("sha256:{id}"),
        },
    )
}

fn ready_registry(declarations: Vec<CapabilityDeclaration>) -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    for declaration in declarations {
        registry
            .register(
                declaration,
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

fn selected_ids(receipt: &arda_core::run_graph::CapabilityCompositionReceipt) -> HashSet<&str> {
    receipt
        .selected_capabilities
        .iter()
        .map(|capability| capability.id.as_str())
        .collect()
}

#[test]
fn personal_reminder_selects_only_personal_operations_and_phone_notification() {
    let composition = fixture("valid-personal-objective.json");
    let graph = graph_for(&composition);
    let registry = ready_registry(vec![
        service_capability(
            "personal_operations",
            CompositionAuthorityClass::Plan,
            &[DataClass::Health],
        ),
        service_capability(
            "phone_notification",
            CompositionAuthorityClass::Propose,
            &[DataClass::Health],
        ),
        service_capability(
            "workbench",
            CompositionAuthorityClass::Propose,
            &[DataClass::Health],
        ),
        service_capability(
            "council",
            CompositionAuthorityClass::Propose,
            &[DataClass::Health],
        ),
        service_capability(
            "payment",
            CompositionAuthorityClass::Propose,
            &[DataClass::Financial],
        ),
        service_capability(
            "health_device",
            CompositionAuthorityClass::Propose,
            &[DataClass::Health],
        ),
        service_capability(
            "manwe",
            CompositionAuthorityClass::Propose,
            &[DataClass::Health],
        ),
        service_capability(
            "external_write",
            CompositionAuthorityClass::Propose,
            &[DataClass::Health],
        ),
        service_capability(
            "private_data_egress",
            CompositionAuthorityClass::Propose,
            &[DataClass::Private],
        ),
    ]);
    let recommendations = BTreeSet::from([
        "workbench".to_string(),
        "council".to_string(),
        "payment".to_string(),
        "health_device".to_string(),
        "manwe".to_string(),
        "external_write".to_string(),
        "private_data_egress".to_string(),
    ]);

    let receipt = graph
        .deterministic_composition(
            &composition,
            &registry,
            &recommendations,
            CompositionTrigger::Initial,
            None,
        )
        .unwrap();

    assert_eq!(
        selected_ids(&receipt),
        HashSet::from(["personal_operations", "phone_notification"])
    );
    for capability in [
        "workbench",
        "council",
        "payment",
        "health_device",
        "manwe",
        "external_write",
        "private_data_egress",
    ] {
        let decision = receipt
            .decisions
            .iter()
            .find(|decision| decision.capability.id == capability)
            .unwrap();
        assert!(!decision.selected);
        assert!(decision
            .reasons
            .contains(&"model_recommendation_ignored_not_required".to_string()));
    }
}

#[test]
fn coding_objective_selects_only_signed_project_execution_capabilities() {
    let mut composition = fixture("valid-software-project.json");
    composition
        .capabilities
        .required
        .insert("project_contract".to_string());
    composition.validate().unwrap();
    let graph = graph_for(&composition);
    let registry = ready_registry(vec![
        service_capability(
            "project_contract",
            CompositionAuthorityClass::Plan,
            &[DataClass::Internal],
        ),
        external_adapter_capability(
            "hermes",
            CompositionAuthorityClass::ExecuteWithApproval,
            &[DataClass::Internal],
        ),
        service_capability(
            "verification",
            CompositionAuthorityClass::ReadOnly,
            &[DataClass::Internal],
        ),
        service_capability(
            "artifact_receipt",
            CompositionAuthorityClass::ReadOnly,
            &[DataClass::Internal],
        ),
        service_capability(
            "payment",
            CompositionAuthorityClass::Propose,
            &[DataClass::Financial],
        ),
        service_capability(
            "economic",
            CompositionAuthorityClass::Propose,
            &[DataClass::Internal],
        ),
    ]);

    let receipt = graph
        .deterministic_composition(
            &composition,
            &registry,
            &BTreeSet::from(["payment".to_string(), "economic".to_string()]),
            CompositionTrigger::Initial,
            None,
        )
        .unwrap();

    assert_eq!(
        selected_ids(&receipt),
        HashSet::from([
            "project_contract",
            "hermes",
            "verification",
            "artifact_receipt",
        ])
    );
    assert!(!selected_ids(&receipt).contains("payment"));
    assert!(!selected_ids(&receipt).contains("economic"));
    assert!(receipt.digest().unwrap().starts_with("sha256:"));
}

#[test]
fn prefer_local_is_a_policy_preference_not_a_guarantee() {
    let mut composition = fixture("valid-software-project.json");
    composition.capabilities.required = BTreeSet::from(["hosted_worker".to_string()]);
    composition.capabilities.forbidden.clear();
    composition.roles.clear();
    composition.route_preferences.mode = RouteMode::PreferLocal;
    composition.route_preferences.allowed_providers =
        BTreeSet::from(["hosted_fallback".to_string()]);
    composition.sensitivity.permitted_egress = BTreeSet::from([EgressTarget::HostedProvider]);
    composition.validate().unwrap();
    let graph = graph_for(&composition);
    let registry = ready_registry(vec![model_capability("hosted_worker", "hosted_fallback")]);

    let receipt = graph
        .deterministic_composition(
            &composition,
            &registry,
            &BTreeSet::new(),
            CompositionTrigger::Initial,
            None,
        )
        .unwrap();

    assert_eq!(selected_ids(&receipt), HashSet::from(["hosted_worker"]));
    assert!(receipt.decisions[0]
        .reasons
        .contains(&"selected_preference_unavailable".to_string()));
}

#[test]
fn local_only_is_a_hard_constraint_before_model_recommendation() {
    let mut composition = fixture("valid-software-project.json");
    composition.capabilities.required = BTreeSet::from(["hosted_worker".to_string()]);
    composition.capabilities.forbidden.clear();
    composition.roles.clear();
    composition.route_preferences.mode = RouteMode::LocalOnly;
    composition.route_preferences.allowed_providers =
        BTreeSet::from(["hosted_fallback".to_string()]);
    composition.sensitivity.permitted_egress = BTreeSet::from([EgressTarget::HostedProvider]);
    composition.validate().unwrap();
    let graph = graph_for(&composition);
    let registry = ready_registry(vec![model_capability("hosted_worker", "hosted_fallback")]);

    assert!(matches!(
        graph.deterministic_composition(
            &composition,
            &registry,
            &BTreeSet::from(["hosted_worker".to_string()]),
            CompositionTrigger::Initial,
            None,
        ),
        Err(DeterministicCompositionError::UnsatisfiedRequiredCapability { capability_id, .. })
            if capability_id == "hosted_worker"
    ));
}
