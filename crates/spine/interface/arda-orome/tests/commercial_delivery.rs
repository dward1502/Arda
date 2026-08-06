use arda_core::company_ops::{
    ApprovalReceipt, CommercialAuthority, ConfidenceRange, ProposalDraft, ValueEstimate,
};
use arda_orome::commercial::{CommercialDeliveryReceipt, CommercialDeliveryState, CommercialDraft};
use arda_orome::provider::{DispatchReceipt, FleetScope};
use chrono::{Duration, TimeZone, Utc};
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
}

fn draft() -> CommercialDraft {
    CommercialDraft {
        proposal: ProposalDraft {
            proposal_id: Uuid::new_v4(),
            engagement_id: Uuid::new_v4(),
            title: "Paid discovery".into(),
            scope: "Bounded discovery".into(),
            price: ValueEstimate {
                currency: "USD".into(),
                range: ConfidenceRange {
                    low: 900.0,
                    expected: 1_000.0,
                    high: 1_100.0,
                    confidence: 0.8,
                },
                basis: "rate card".into(),
                evidence: vec![],
            },
            proposed_due_at: now() + Duration::days(7),
            audience: "client".into(),
            risk: "scope".into(),
            evidence: vec![],
            authority: CommercialAuthority::ProposalOnly,
            expires_at: now() + Duration::days(2),
        },
        source_context: vec!["crm:opportunity:1".into()],
        commitments: vec!["No work before approval".into()],
        approval_required: true,
    }
}

#[test]
fn external_send_requires_matching_approval_and_orome_external_scope() {
    let draft = draft();
    let mut approval = ApprovalReceipt {
        receipt_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        approved_by: "operator".into(),
        approved_at: now(),
        expires_at: now() + Duration::hours(1),
        approved_scope: draft.proposal.scope.clone(),
        approved_price: "USD 1000".into(),
        approved_due_at: now() + Duration::days(7),
    };
    assert!(draft.prepare_external_request(&approval, now()).is_err());
    approval.proposal_id = draft.proposal.proposal_id;
    let request = draft.prepare_external_request(&approval, now()).unwrap();
    assert_eq!(request.fleet_scope, FleetScope::External);
    assert!(request.approved);

    approval.approved_scope = "Expanded unreviewed scope".into();
    assert!(draft.prepare_external_request(&approval, now()).is_err());
    approval.approved_scope = draft.proposal.scope.clone();
    approval.approved_price = "USD 1250".into();
    assert!(draft.prepare_external_request(&approval, now()).is_err());
    approval.approved_price = "USD 1000".into();
    approval.approved_due_at += Duration::days(1);
    assert!(draft.prepare_external_request(&approval, now()).is_err());
}

#[test]
fn dispatch_truth_distinguishes_accepted_from_delivered_and_failed() {
    let accepted = CommercialDeliveryReceipt::from_dispatch(
        "proposal",
        DispatchReceipt {
            dispatched: true,
            attempts: 1,
            provider_id: "email".into(),
            ..Default::default()
        },
    );
    assert_eq!(accepted.state, CommercialDeliveryState::Accepted);
    let delivered = CommercialDeliveryReceipt::from_dispatch(
        "proposal",
        DispatchReceipt {
            dispatched: true,
            attempts: 1,
            provider_id: "email".into(),
            provider_message_id: Some("provider-1".into()),
            ..Default::default()
        },
    );
    assert_eq!(delivered.state, CommercialDeliveryState::Delivered);
    let failed = CommercialDeliveryReceipt::from_dispatch(
        "proposal",
        DispatchReceipt {
            attempts: 2,
            provider_id: "email".into(),
            error: Some("rejected".into()),
            ..Default::default()
        },
    );
    assert_eq!(failed.state, CommercialDeliveryState::Failed);
}
