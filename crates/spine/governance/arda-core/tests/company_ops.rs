use arda_core::capability_composition::{CapabilityComposition, CompositionScope, EgressTarget};
use arda_core::company_ops::{
    ApprovalReceipt, ClientDeliveryBundle, CommercialAuthority, CommercialEgress,
    CommercialLifecycleRecord, CommercialLifecycleState, CommercialLineage, CompanyOpsConfig,
    CompanyOpsError, ConfidenceRange, ContactReference, EngagementState, OperatorTimeBudget,
    OutcomeKind, OutcomeReceipt, PrivacyClass, ProductHypothesis, ProposalDraft, RevenueExperiment,
    ValueEstimate, COMMERCIAL_LIFECYCLE_SCHEMA_VERSION, COMPANY_OPERATIONS_CAPABILITY_ID,
};
use arda_core::run_graph::{ObjectiveId, Provenance, RunGraph, RunId};
use chrono::{Duration, TimeZone, Utc};
use std::collections::BTreeSet;
use std::path::PathBuf;
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
}

fn estimate() -> ValueEstimate {
    ValueEstimate {
        currency: "USD".into(),
        range: ConfidenceRange {
            low: 800.0,
            expected: 1_000.0,
            high: 1_400.0,
            confidence: 0.65,
        },
        basis: "two comparable engagements".into(),
        evidence: vec![],
    }
}

fn commercial_composition() -> CapabilityComposition {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec/capability-composition/v1/fixtures/valid-software-project.json");
    let mut composition =
        CapabilityComposition::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    composition.scope = CompositionScope::Business;
    composition
        .capabilities
        .required
        .insert(COMPANY_OPERATIONS_CAPABILITY_ID.to_string());
    composition
        .capabilities
        .forbidden
        .remove(COMPANY_OPERATIONS_CAPABILITY_ID);
    composition
}

fn commercial_run(composition: &CapabilityComposition) -> RunGraph {
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new(composition.lineage.run_id.clone()).unwrap(),
        objective_id: ObjectiveId::new(composition.lineage.objective_id.clone()).unwrap(),
        nodes: vec![],
        edges: vec![],
        provenance: Provenance {
            project_contract_digest: composition.lineage.project_contract_digest.clone(),
            created_by: "company-ops-test".into(),
            parent_receipts: vec![],
        },
    }
}

fn lifecycle(
    state: CommercialLifecycleState,
    approval_receipt_id: Option<Uuid>,
) -> CommercialLifecycleRecord {
    let composition = commercial_composition();
    let run = commercial_run(&composition);
    CommercialLifecycleRecord {
        schema_version: COMMERCIAL_LIFECYCLE_SCHEMA_VERSION.into(),
        record_id: Uuid::new_v4(),
        engagement_id: Uuid::new_v4(),
        subject_id: "commercial-subject-1".into(),
        state,
        lineage: CommercialLineage::from_composition(&composition, &run).unwrap(),
        business_scope: CompositionScope::Business,
        privacy: PrivacyClass::CommercialConfidential,
        evidence_receipt_ids: BTreeSet::from(["receipt:source".into()]),
        artifact_receipt_ids: BTreeSet::from(["artifact:deliverable".into()]),
        approval_receipt_id,
        egress: Some(CommercialEgress {
            target: EgressTarget::ExternalAdapter,
            destination: "accounting:client-ledger".into(),
        }),
        recorded_at: now(),
    }
}

#[test]
fn estimate_serialization_cannot_claim_realized_revenue() {
    let encoded = serde_json::to_value(estimate()).unwrap();
    assert!(encoded.get("range").is_some());
    assert!(encoded.get("realized_value").is_none());
    assert!(encoded.get("realized_revenue").is_none());
}

#[test]
fn canonical_company_operations_config_is_versioned_and_namespaced() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../config/business/company-operations.toml");
    let config = CompanyOpsConfig::load(path).unwrap();
    assert_eq!(config.mode, "operator_cockpit");
    assert_eq!(config.adapters.communications_owner, "arda-orome");
    assert!(config.storage.event_log.starts_with("data/business/"));
    assert!(config.scoring.include_operator_time);
}

#[test]
fn draft_requires_matching_unexpired_operator_approval_to_become_commitment() {
    let proposal_id = Uuid::new_v4();
    let engagement_id = Uuid::new_v4();
    let draft = ProposalDraft {
        proposal_id,
        engagement_id,
        title: "Paid discovery".into(),
        scope: "draft scope".into(),
        price: estimate(),
        proposed_due_at: now() + Duration::days(7),
        audience: "client".into(),
        risk: "scope uncertainty".into(),
        evidence: vec![],
        authority: CommercialAuthority::ProposalOnly,
        expires_at: now() + Duration::days(2),
    };
    let wrong = ApprovalReceipt {
        receipt_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        approved_by: "operator".into(),
        approved_at: now(),
        expires_at: now() + Duration::days(1),
        approved_scope: "approved scope".into(),
        approved_price: "USD 1000".into(),
        approved_due_at: now() + Duration::days(7),
    };
    assert_eq!(
        draft.clone().into_commitment(wrong, now()),
        Err(CompanyOpsError::ApprovalMismatch)
    );

    let approval = ApprovalReceipt {
        proposal_id,
        ..ApprovalReceipt {
            receipt_id: Uuid::new_v4(),
            proposal_id,
            approved_by: "operator".into(),
            approved_at: now(),
            expires_at: now() + Duration::days(1),
            approved_scope: "approved scope".into(),
            approved_price: "USD 1000".into(),
            approved_due_at: now() + Duration::days(7),
        }
    };
    let commitment = draft.into_commitment(approval.clone(), now()).unwrap();
    assert_eq!(commitment.approval_receipt_id, approval.receipt_id);
    assert_eq!(commitment.scope, "approved scope");
}

#[test]
fn contact_telemetry_is_redacted() {
    let contact = ContactReference {
        contact_id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        display_label: "Private client".into(),
        private_locator: Some("person@example.invalid".into()),
        privacy: PrivacyClass::ContactRestricted,
        adapter_provenance: None,
    };
    let telemetry = serde_json::to_string(&contact.for_general_telemetry()).unwrap();
    assert!(!telemetry.contains("Private client"));
    assert!(!telemetry.contains("example.invalid"));
    assert!(telemetry.contains("\"redacted\":true"));
}

#[test]
fn realized_value_is_receipted_and_pipeline_stages_are_distinct() {
    let outcome = OutcomeReceipt {
        receipt_id: Uuid::new_v4(),
        engagement_id: Uuid::new_v4(),
        experiment_id: None,
        kind: OutcomeKind::Paid,
        recorded_at: now(),
        summary: "payment settled".into(),
        delivery_cost: None,
        operator_assessment: "accepted".into(),
        evidence: vec![],
        reviewed: true,
    };
    let realized =
        arda_core::company_ops::RealizedValue::from_outcome("USD", 1_000.0, &outcome).unwrap();
    assert_eq!(realized.outcome_receipt_id, outcome.receipt_id);
    assert_ne!(EngagementState::Proposed, EngagementState::Won);
    assert_ne!(EngagementState::Invoiced, EngagementState::Paid);
}

#[test]
fn experiment_requires_approval_before_becoming_workbench_objective() {
    let approval = ApprovalReceipt {
        receipt_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        approved_by: "operator".into(),
        approved_at: now(),
        expires_at: now() + Duration::hours(1),
        approved_scope: "bounded build".into(),
        approved_price: "USD 100".into(),
        approved_due_at: now() + Duration::days(2),
    };
    let time = OperatorTimeBudget {
        expected_hours: ConfidenceRange {
            low: 1.0,
            expected: 2.0,
            high: 3.0,
            confidence: 0.7,
        },
        maximum_hours: 4.0,
        expires_at: now() + Duration::days(2),
    };
    let experiment = RevenueExperiment {
        experiment_id: Uuid::new_v4(),
        hypothesis: ProductHypothesis {
            hypothesis_id: Uuid::new_v4(),
            customer_problem: "slow status reconstruction".into(),
            proposed_offer: "paid cockpit setup".into(),
            target_audience: "small teams".into(),
            evidence: vec![],
            expected_value: estimate(),
            build_time: time.clone(),
            expires_at: now() + Duration::days(2),
        },
        success_threshold: "one paid trial".into(),
        stop_condition: "four hours exhausted".into(),
        maximum_spend: estimate(),
        maximum_operator_time: time,
        authority: CommercialAuthority::ProposalOnly,
        approval_receipt_id: None,
        decision: None,
    };
    assert_eq!(
        experiment.into_workbench_objective(
            &approval,
            "project-1",
            vec!["test passes".into()],
            "one module",
            now()
        ),
        Err(CompanyOpsError::ExperimentApprovalRequired)
    );
    let approved = RevenueExperiment {
        authority: CommercialAuthority::ExplicitOperatorApproval,
        approval_receipt_id: Some(approval.receipt_id),
        ..experiment
    };
    let objective = approved
        .into_workbench_objective(
            &approval,
            "project-1",
            vec!["test passes".into()],
            "one module",
            now(),
        )
        .unwrap();
    assert_eq!(objective.authority, CommercialAuthority::ReviewRequired);
    assert_eq!(objective.approval_receipt_id, approval.receipt_id);
}

#[test]
fn client_delivery_requires_receipts_boundaries_and_export_only_invoicing() {
    let commitment_id = Uuid::new_v4();
    let mut delivery_lifecycle =
        lifecycle(CommercialLifecycleState::Deliverable, Some(Uuid::new_v4()));
    delivery_lifecycle.subject_id = commitment_id.to_string();
    let workbench_run_id = delivery_lifecycle.lineage.run_id.as_str().to_string();
    let valid = ClientDeliveryBundle {
        commitment_id,
        workbench_run_id,
        deliverables: vec!["artifact".into()],
        acceptance_evidence: vec!["tests pass".into()],
        change_requests: vec![],
        overrun_warning: None,
        handoff_boundary: "artifact handoff".into(),
        support_boundary: "seven days".into(),
        invoice_export_only: true,
        lifecycle: delivery_lifecycle,
    };
    assert_eq!(valid.validate(), Ok(()));
    let invalid = ClientDeliveryBundle {
        invoice_export_only: false,
        ..valid
    };
    assert_eq!(
        invalid.validate(),
        Err(CompanyOpsError::InvalidDeliveryBundle)
    );
}

#[test]
fn commercial_lifecycle_maps_every_state_to_stable_project_run_lineage() {
    let composition = commercial_composition();
    let run = commercial_run(&composition);
    let lineage = CommercialLineage::from_composition(&composition, &run).unwrap();
    assert_eq!(lineage.project_id, composition.lineage.project_id);
    assert_eq!(lineage.run_id.as_str(), composition.lineage.run_id);
    assert_eq!(
        lineage.objective_id.as_str(),
        composition.lineage.objective_id
    );

    for state in [
        CommercialLifecycleState::Opportunity,
        CommercialLifecycleState::Quote,
        CommercialLifecycleState::Deliverable,
        CommercialLifecycleState::Acceptance,
        CommercialLifecycleState::Invoice,
        CommercialLifecycleState::Settlement,
        CommercialLifecycleState::AccountingExport,
    ] {
        let approval = state.requires_external_approval().then(Uuid::new_v4);
        let mut record = lifecycle(state, approval);
        if !state.requires_external_approval() {
            record.egress = None;
        }
        assert_eq!(record.validate(), Ok(()), "state {state:?}");
    }
}

#[test]
fn commercial_activation_is_business_only_and_explicitly_selected() {
    let mut composition = commercial_composition();
    let run = commercial_run(&composition);
    assert!(CommercialLineage::from_composition(&composition, &run).is_ok());

    let mut mismatched_run = run.clone();
    mismatched_run.provenance.project_contract_digest = "sha256:wrong-project".into();
    assert_eq!(
        CommercialLineage::from_composition(&composition, &mismatched_run),
        Err(CompanyOpsError::CommercialLineageMismatch)
    );

    composition.scope = CompositionScope::Personal;
    assert_eq!(
        CommercialLineage::from_composition(&composition, &run),
        Err(CompanyOpsError::CompanyOperationsNotSelected)
    );
    composition.scope = CompositionScope::Business;
    composition
        .capabilities
        .required
        .remove(COMPANY_OPERATIONS_CAPABILITY_ID);
    assert_eq!(
        CommercialLineage::from_composition(&composition, &run),
        Err(CompanyOpsError::CompanyOperationsNotSelected)
    );
}

#[test]
fn external_commercial_states_require_exact_approval_and_explicit_egress() {
    let approval_id = Uuid::new_v4();
    let mut invoice = lifecycle(CommercialLifecycleState::Invoice, None);
    assert_eq!(
        invoice.validate(),
        Err(CompanyOpsError::MissingApprovalReceipt)
    );
    invoice.approval_receipt_id = Some(approval_id);
    invoice.egress = None;
    assert_eq!(
        invoice.validate(),
        Err(CompanyOpsError::MissingCommercialEgress)
    );
    invoice.egress = Some(CommercialEgress {
        target: EgressTarget::LocalDevice,
        destination: "accounting:client-ledger".into(),
    });
    assert_eq!(
        invoice.validate(),
        Err(CompanyOpsError::InvalidCommercialEgress)
    );
}
