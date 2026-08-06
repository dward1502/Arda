use arda_core::company_ops::{
    ApprovalReceipt, ClientDeliveryBundle, CommercialAuthority, CompanyOpsConfig, CompanyOpsError,
    ConfidenceRange, ContactReference, EngagementState, OperatorTimeBudget, OutcomeKind,
    OutcomeReceipt, PrivacyClass, ProductHypothesis, ProposalDraft, RevenueExperiment,
    ValueEstimate,
};
use chrono::{Duration, TimeZone, Utc};
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
    let valid = ClientDeliveryBundle {
        commitment_id: Uuid::new_v4(),
        workbench_run_id: "run-1".into(),
        deliverables: vec!["artifact".into()],
        acceptance_evidence: vec!["tests pass".into()],
        change_requests: vec![],
        overrun_warning: None,
        handoff_boundary: "artifact handoff".into(),
        support_boundary: "seven days".into(),
        invoice_export_only: true,
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
