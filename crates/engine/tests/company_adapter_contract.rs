use arda_core::company_ops::{ApprovalReceipt, CommercialAuthority};
use arda_engine::adapters::{
    CompanyAdapterCapability, CompanyAdapterError, CompanyAdapterOperation, CompanyAdapterRequest,
    ReferenceCrmAdapter,
};
use chrono::{Duration, TimeZone, Utc};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
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
    };
    assert_eq!(
        request.validate(&allowlist, now()),
        Err(CompanyAdapterError::ApprovalRequired)
    );

    let approved = CompanyAdapterRequest {
        authority: CommercialAuthority::ExplicitOperatorApproval,
        approval: Some(ApprovalReceipt {
            receipt_id: Uuid::new_v4(),
            proposal_id: Uuid::new_v4(),
            approved_by: "operator".into(),
            approved_at: now(),
            expires_at: now() + Duration::hours(1),
            approved_scope: "accounting_export_write:invoice-1".into(),
            approved_price: "USD 1000".into(),
            approved_due_at: now() + Duration::days(7),
        }),
        ..request
    };
    assert_eq!(approved.validate(&allowlist, now()), Ok(()));

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
        };
        assert_eq!(request.validate(&allowlist, now()), Ok(()));
    }
}
