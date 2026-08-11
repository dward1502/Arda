use arda_core::capability_composition::{CapabilityComposition, CompositionScope, EgressTarget};
use arda_core::company_ops::{
    ApprovalReceipt, CommercialAuthority, CommercialEgress, CommercialLifecycleRecord,
    CommercialLifecycleState, CommercialLineage, PrivacyClass, COMMERCIAL_LIFECYCLE_SCHEMA_VERSION,
    COMPANY_OPERATIONS_CAPABILITY_ID,
};
use arda_core::run_graph::{ObjectiveId, Provenance, RunGraph, RunId};
use arda_engine::adapters::{
    CompanyAdapterCapability, CompanyAdapterError, CompanyAdapterOperation, CompanyAdapterRequest,
    ReferenceCrmAdapter,
};
use chrono::{Duration, TimeZone, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
}

fn lifecycle(
    state: CommercialLifecycleState,
    subject_id: &str,
    approval_receipt_id: Option<Uuid>,
) -> CommercialLifecycleRecord {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/capability-composition/v1/fixtures/valid-software-project.json");
    let mut composition =
        CapabilityComposition::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    composition.scope = CompositionScope::Business;
    composition
        .capabilities
        .required
        .insert(COMPANY_OPERATIONS_CAPABILITY_ID.into());
    let run = RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new(composition.lineage.run_id.clone()).unwrap(),
        objective_id: ObjectiveId::new(composition.lineage.objective_id.clone()).unwrap(),
        nodes: vec![],
        edges: vec![],
        provenance: Provenance {
            project_contract_digest: composition.lineage.project_contract_digest.clone(),
            created_by: "company-adapter-test".into(),
            parent_receipts: vec![],
        },
    };
    let requires_approval = state.requires_external_approval();
    CommercialLifecycleRecord {
        schema_version: COMMERCIAL_LIFECYCLE_SCHEMA_VERSION.into(),
        record_id: Uuid::new_v4(),
        engagement_id: Uuid::new_v4(),
        subject_id: subject_id.into(),
        state,
        lineage: CommercialLineage::from_composition(&composition, &run).unwrap(),
        business_scope: CompositionScope::Business,
        privacy: PrivacyClass::CommercialConfidential,
        evidence_receipt_ids: BTreeSet::from(["receipt:adapter-source".into()]),
        artifact_receipt_ids: BTreeSet::new(),
        approval_receipt_id,
        egress: requires_approval.then(|| CommercialEgress {
            target: EgressTarget::ExternalAdapter,
            destination: "accounting:client-ledger".into(),
        }),
        recorded_at: now(),
    }
}

#[test]
fn reference_crm_is_read_only_deduplicated_and_provenanced() {
    let adapter = ReferenceCrmAdapter::read_only("reference-crm", "1.0.0");
    let resources = adapter
        .normalize(
            "organization",
            now(),
            vec![
                ("org-2".into(), "Second".into(), BTreeMap::new()),
                ("org-1".into(), "First".into(), BTreeMap::new()),
            ],
        )
        .unwrap();
    assert_eq!(resources[0].external_id, "org-1");
    assert!(resources
        .iter()
        .all(|resource| resource.provenance.read_only));
    let duplicate = adapter.normalize(
        "organization",
        now(),
        vec![
            ("org-1".into(), "First".into(), BTreeMap::new()),
            ("org-1".into(), "Duplicate".into(), BTreeMap::new()),
        ],
    );
    assert_eq!(
        duplicate,
        Err(CompanyAdapterError::DuplicateOrMissingExternalId)
    );
}

#[test]
fn outbound_capabilities_are_denied_without_explicit_approval() {
    let allowlist = BTreeSet::from([CompanyAdapterCapability::AccountingExportWrite]);
    let request = CompanyAdapterRequest {
        schema_version: "arda.company-adapter.v1".into(),
        request_id: "request-1".into(),
        operation: CompanyAdapterOperation::AccountingExportWrite,
        resource_id: "invoice-1".into(),
        idempotency_key: "invoice-1".into(),
        authority: CommercialAuthority::ProposalOnly,
        approval: None,
        lifecycle: lifecycle(
            CommercialLifecycleState::AccountingExport,
            "invoice-1",
            None,
        ),
    };
    assert_eq!(
        request.validate(&allowlist, now()),
        Err(CompanyAdapterError::ApprovalRequired)
    );

    let approval = ApprovalReceipt {
        receipt_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        approved_by: "operator".into(),
        approved_at: now(),
        expires_at: now() + Duration::hours(1),
        approved_scope: "accounting_export_write:invoice-1".into(),
        approved_price: "USD 1000".into(),
        approved_due_at: now() + Duration::days(7),
    };
    let approved = CompanyAdapterRequest {
        authority: CommercialAuthority::ExplicitOperatorApproval,
        lifecycle: lifecycle(
            CommercialLifecycleState::AccountingExport,
            "invoice-1",
            Some(approval.receipt_id),
        ),
        approval: Some(approval),
        ..request
    };
    assert_eq!(approved.validate(&allowlist, now()), Ok(()));

    let mut wrong_approval_lineage = approved.clone();
    wrong_approval_lineage.lifecycle.approval_receipt_id = Some(Uuid::new_v4());
    assert_eq!(
        wrong_approval_lineage.validate(&allowlist, now()),
        Err(CompanyAdapterError::CommercialLineageMismatch)
    );

    let wrong_resource = CompanyAdapterRequest {
        resource_id: "invoice-2".into(),
        ..approved
    };
    assert_eq!(
        wrong_resource.validate(&allowlist, now()),
        Err(CompanyAdapterError::ApprovalScopeMismatch)
    );
}

#[test]
fn protocol_exposes_all_planned_read_adapter_capabilities() {
    let operations = [
        CompanyAdapterOperation::OrganizationsRead,
        CompanyAdapterOperation::ContactsRead,
        CompanyAdapterOperation::OpportunitiesRead,
        CompanyAdapterOperation::ActivitiesRead,
        CompanyAdapterOperation::CalendarActivitiesRead,
        CompanyAdapterOperation::EmailContextRead,
        CompanyAdapterOperation::ProjectIssuesRead,
    ];
    let allowlist = operations
        .iter()
        .map(|operation| operation.capability())
        .collect();
    for (index, operation) in operations.into_iter().enumerate() {
        let request = CompanyAdapterRequest {
            schema_version: "arda.company-adapter.v1".into(),
            request_id: format!("read-{index}"),
            operation,
            resource_id: "*".into(),
            idempotency_key: format!("read-{index}:v1"),
            authority: CommercialAuthority::ReadOnly,
            approval: None,
            lifecycle: lifecycle(
                CommercialLifecycleState::Opportunity,
                "opportunity-read",
                None,
            ),
        };
        assert_eq!(request.validate(&allowlist, now()), Ok(()));
    }
}
