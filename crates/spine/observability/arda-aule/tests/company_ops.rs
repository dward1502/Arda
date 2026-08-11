#![cfg(feature = "full-cli")]

use arda_aule::company_ops::{
    build_projection, AppendOutcome, CompanyOpsEvent, CompanyOpsEventKind, CompanyOpsStore,
};
use arda_core::company_ops::{
    ClientEngagement, CommercialAuthority, ConfidenceRange, EngagementState, OperatorTimeBudget,
    Opportunity, OutcomeKind, OutcomeReceipt, PrivacyClass, RealizedValue, ValueEstimate,
};
use chrono::{Duration, TimeZone, Utc};
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
}

fn opportunity(id: Uuid, title: &str, expected: f64) -> Opportunity {
    Opportunity {
        opportunity_id: id,
        organization_id: Uuid::new_v4(),
        title: title.into(),
        stage: EngagementState::Qualified,
        expected_value: ValueEstimate {
            currency: "USD".into(),
            range: ConfidenceRange {
                low: expected * 0.5,
                expected,
                high: expected * 1.5,
                confidence: 0.7,
            },
            basis: "cited comparable".into(),
            evidence: vec![],
        },
        operator_time: OperatorTimeBudget {
            expected_hours: ConfidenceRange {
                low: 1.0,
                expected: 2.0,
                high: 3.0,
                confidence: 0.8,
            },
            maximum_hours: 4.0,
            expires_at: now() + Duration::days(7),
        },
        evidence: vec![],
        expires_at: now() + Duration::days(2),
    }
}

#[test]
fn append_is_idempotent_and_replay_is_deterministic() {
    let root = tempfile::tempdir().unwrap();
    let store = CompanyOpsStore::new(root.path());
    let first = CompanyOpsEvent::new(
        "crm:opportunity:1:v1",
        now() + Duration::minutes(2),
        CompanyOpsEventKind::OpportunityObserved(opportunity(
            Uuid::from_u128(2),
            "Second",
            2_000.0,
        )),
    );
    let second = CompanyOpsEvent::new(
        "crm:opportunity:2:v1",
        now() + Duration::minutes(1),
        CompanyOpsEventKind::OpportunityObserved(opportunity(Uuid::from_u128(1), "First", 1_000.0)),
    );
    assert_eq!(store.append(&first).unwrap(), AppendOutcome::Appended);
    assert_eq!(store.append(&second).unwrap(), AppendOutcome::Appended);
    assert_eq!(store.append(&first).unwrap(), AppendOutcome::Duplicate);

    let replay = store.load().unwrap();
    assert_eq!(replay.len(), 2);
    let projection_a = build_projection(&replay, now());
    let projection_b = build_projection(&store.load().unwrap(), now());
    assert_eq!(projection_a, projection_b);
    assert!(projection_a.scored_opportunities[0].score.uncertainty > 0.0);
    projection_a.write_canonical(root.path()).unwrap();
    assert!(root
        .path()
        .join("data/business/opportunities.json")
        .exists());
}

#[test]
fn projections_do_not_contain_contact_locators_or_personal_health_data() {
    let projection = build_projection(&[], now());
    let encoded = serde_json::to_string(&projection).unwrap();
    assert!(!encoded.contains("private_locator"));
    assert!(!encoded.contains("health"));
    assert!(!encoded.contains("personal_restricted"));
}

#[test]
fn engagement_and_receipted_realized_value_reach_the_canonical_projection() {
    let outcome = OutcomeReceipt {
        receipt_id: Uuid::from_u128(20),
        engagement_id: Uuid::from_u128(10),
        experiment_id: None,
        kind: OutcomeKind::Paid,
        recorded_at: now(),
        summary: "payment settled".into(),
        delivery_cost: None,
        operator_assessment: "reviewed".into(),
        evidence: vec![],
        reviewed: true,
    };
    let engagement = ClientEngagement {
        engagement_id: outcome.engagement_id,
        organization_id: Uuid::from_u128(1),
        title: "Paid discovery".into(),
        state: EngagementState::Paid,
        expected_value: opportunity(Uuid::from_u128(2), "forecast", 1_000.0).expected_value,
        realized_value: Some(RealizedValue::from_outcome("USD", 1_000.0, &outcome).unwrap()),
        authority: CommercialAuthority::ReadOnly,
        privacy: PrivacyClass::CommercialConfidential,
    };
    let projection = build_projection(
        &[
            CompanyOpsEvent::new(
                "crm:engagement:10:v1",
                now(),
                CompanyOpsEventKind::EngagementObserved(engagement.clone()),
            ),
            CompanyOpsEvent::new(
                "outcome:20:v1",
                now(),
                CompanyOpsEventKind::OutcomeRecorded(outcome),
            ),
        ],
        now(),
    );
    assert_eq!(projection.engagements, vec![engagement]);
    assert_eq!(
        projection.engagements[0]
            .realized_value
            .as_ref()
            .unwrap()
            .amount,
        1_000.0
    );
}

#[test]
fn reviewed_outcomes_feed_back_into_related_opportunity_scoring() {
    let organization_id = Uuid::from_u128(30);
    let engagement_id = Uuid::from_u128(31);
    let mut candidate = opportunity(Uuid::from_u128(32), "Reviewed pilot", 1_000.0);
    candidate.organization_id = organization_id;
    let engagement = ClientEngagement {
        engagement_id,
        organization_id,
        title: "Reviewed pilot engagement".into(),
        state: EngagementState::Paid,
        expected_value: candidate.expected_value.clone(),
        realized_value: None,
        authority: CommercialAuthority::ReadOnly,
        privacy: PrivacyClass::CommercialConfidential,
    };
    let reviewed = OutcomeReceipt {
        receipt_id: Uuid::from_u128(33),
        engagement_id,
        experiment_id: None,
        kind: OutcomeKind::Paid,
        recorded_at: now(),
        summary: "payment settled".into(),
        delivery_cost: None,
        operator_assessment: "reviewed and reusable".into(),
        evidence: vec![],
        reviewed: true,
    };
    let common_events = |engagement: ClientEngagement, candidate: Opportunity| {
        vec![
            CompanyOpsEvent::new(
                "crm:engagement:31:v1",
                now(),
                CompanyOpsEventKind::EngagementObserved(engagement),
            ),
            CompanyOpsEvent::new(
                "crm:opportunity:32:v1",
                now(),
                CompanyOpsEventKind::OpportunityObserved(candidate),
            ),
        ]
    };
    let baseline = build_projection(&common_events(engagement.clone(), candidate.clone()), now());
    let mut events = common_events(engagement, candidate);
    events.push(CompanyOpsEvent::new(
        "outcome:33:v1",
        now(),
        CompanyOpsEventKind::OutcomeRecorded(reviewed),
    ));
    let with_outcome = build_projection(&events, now());
    assert_eq!(
        baseline.scored_opportunities[0]
            .score
            .reviewed_outcome_signal,
        0.0
    );
    assert_eq!(
        with_outcome.scored_opportunities[0]
            .score
            .reviewed_outcome_signal,
        1.0
    );
    assert!(
        with_outcome.scored_opportunities[0].score.total
            > baseline.scored_opportunities[0].score.total
    );
}
