use arda_core::capability_composition::ProactiveCommunicationMode;
use arda_core::governance_gates::{
    ActionReversibility, CoercionRisk, ConsentAuthority, HumanFacingActionReview,
    JouleWorkBudgetClass,
};
use arda_core::proactive_communication::{
    OperatorFatigueLevel, PriorOperatorResponse, ProactiveActionClass, ProactiveChannel,
    ProactiveCommunicationInput, ProactiveCommunicationPolicy, ProactiveEventOrigin,
    ProactiveEventOutcome, ProactiveSourceQuality, ProactiveUrgency,
};
use arda_engine::personal_ops::{
    DeliveryPermit, ProactiveCycleError, ProactiveCycleStore, ProactiveEvaluationStatus,
};
use chrono::{Duration, TimeZone, Utc};
use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

fn policy() -> ProactiveCommunicationPolicy {
    ProactiveCommunicationPolicy {
        max_delivery_attempts: 1,
        max_interruptions_per_window: 3,
        defer_noncritical_during_quiet_window: true,
        suppress_at_high_fatigue: true,
        require_evidence: true,
        allow_reversible_pre_authorized_actions: true,
        preferred_channel: ProactiveChannel::OperatorSession,
        digest_channel: ProactiveChannel::Digest,
        max_message_chars: 280,
    }
}

fn input(event_id: &str) -> ProactiveCommunicationInput {
    let evaluated_at = Utc.with_ymd_and_hms(2026, 8, 9, 12, 0, 0).unwrap();
    ProactiveCommunicationInput {
        event_id: event_id.into(),
        mode: ProactiveCommunicationMode::Allowed,
        observed_at: evaluated_at - Duration::minutes(2),
        expires_at: evaluated_at + Duration::minutes(10),
        evaluated_at,
        origin: ProactiveEventOrigin::OperatorRoutine,
        source_quality: ProactiveSourceQuality::OperatorAuthored,
        uncertainty: 0.05,
        urgency: ProactiveUrgency::TimeSensitive,
        action_class: ProactiveActionClass::Information,
        budget_class: JouleWorkBudgetClass::Routine,
        resource_budget_available: true,
        in_quiet_window: false,
        fatigue_level: OperatorFatigueLevel::Low,
        interruptions_in_window: 0,
        prior_attempts: 0,
        prior_response: PriorOperatorResponse::None,
        human_impact_review: HumanFacingActionReview {
            schema_version: HumanFacingActionReview::SCHEMA_VERSION.into(),
            semantic: HumanFacingActionReview::SEMANTIC.into(),
            affected_parties: vec!["operator".into()],
            reversibility: ActionReversibility::Reversible,
            interruption_reason: Some("operator-authored condition matched".into()),
            consent_authority: ConsentAuthority::OperatorAuthored,
            uncertainty: 0.05,
            coercion_risk: CoercionRisk::Low,
        },
        evidence_refs: vec![format!("personal-event:{event_id}")],
    }
}

#[test]
fn overdue_reminder_is_delivered_once_then_acknowledged_without_replay() {
    let temp = TempDir::new().unwrap();
    let store = ProactiveCycleStore::new(temp.path());
    let request = input("overdue-routine");

    let first = store.evaluate_once(&policy(), &request).unwrap();
    assert_eq!(first.status, ProactiveEvaluationStatus::Recorded);
    assert_eq!(first.disposition.outcome, ProactiveEventOutcome::NotifyOnce);

    let permit = store.delivery_permit(&request.event_id).unwrap();
    let DeliveryPermit::Ready {
        idempotency_key, ..
    } = permit
    else {
        panic!("fresh reminder must be ready for one delivery")
    };
    assert!(store
        .record_delivery(
            &request.event_id,
            &idempotency_key,
            "orome-message:1",
            request.evaluated_at,
        )
        .unwrap());
    assert!(!store
        .record_delivery(
            &request.event_id,
            &idempotency_key,
            "orome-message:1",
            request.evaluated_at,
        )
        .unwrap());
    assert!(matches!(
        store.delivery_permit(&request.event_id).unwrap(),
        DeliveryPermit::AlreadyDelivered { .. }
    ));

    assert!(store
        .record_operator_response(
            &request.event_id,
            PriorOperatorResponse::Acknowledged,
            request.evaluated_at + Duration::minutes(1),
        )
        .unwrap());
    assert!(matches!(
        store.delivery_permit(&request.event_id).unwrap(),
        DeliveryPermit::SuppressedByOperatorResponse(PriorOperatorResponse::Acknowledged)
    ));
}

#[test]
fn restart_between_eligibility_and_delivery_reuses_one_idempotency_key() {
    let temp = TempDir::new().unwrap();
    let request = input("restart-window");
    let first_store = ProactiveCycleStore::new(temp.path());
    first_store.evaluate_once(&policy(), &request).unwrap();
    let first = first_store.delivery_permit(&request.event_id).unwrap();
    drop(first_store);

    let restarted = ProactiveCycleStore::new(temp.path());
    let replay = restarted.evaluate_once(&policy(), &request).unwrap();
    assert_eq!(replay.status, ProactiveEvaluationStatus::AlreadyRecorded);
    assert_eq!(restarted.delivery_permit(&request.event_id).unwrap(), first);
}

#[test]
fn concurrent_retries_record_one_evaluation_and_one_delivery() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();
    let request = Arc::new(input("concurrent-retry"));
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let root = root.clone();
            let request = Arc::clone(&request);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                ProactiveCycleStore::new(&root)
                    .evaluate_once(&policy(), &request)
                    .unwrap()
                    .status
            })
        })
        .collect::<Vec<_>>();
    let statuses = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == ProactiveEvaluationStatus::Recorded)
            .count(),
        1
    );

    let store = ProactiveCycleStore::new(&root);
    let DeliveryPermit::Ready {
        idempotency_key, ..
    } = store.delivery_permit(&request.event_id).unwrap()
    else {
        panic!("concurrent evaluation must produce one delivery permit")
    };
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let root = root.clone();
            let event_id = request.event_id.clone();
            let idempotency_key = idempotency_key.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                ProactiveCycleStore::new(&root)
                    .record_delivery(
                        &event_id,
                        &idempotency_key,
                        "orome-message:concurrent",
                        Utc.with_ymd_and_hms(2026, 8, 9, 12, 1, 0).unwrap(),
                    )
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let appended = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|appended| *appended)
        .count();
    assert_eq!(appended, 1);
    assert_eq!(store.load_all().unwrap().deliveries.len(), 1);
}

#[test]
fn p5_4_policy_fixtures_are_durable_and_fail_closed() {
    let temp = TempDir::new().unwrap();
    let store = ProactiveCycleStore::new(temp.path());

    let mut stale = input("stale-evidence");
    stale.expires_at = stale.evaluated_at;
    assert_eq!(
        store
            .evaluate_once(&policy(), &stale)
            .unwrap()
            .disposition
            .outcome,
        ProactiveEventOutcome::Ignore
    );

    let mut quiet = input("quiet-window");
    quiet.in_quiet_window = true;
    assert_eq!(
        store
            .evaluate_once(&policy(), &quiet)
            .unwrap()
            .disposition
            .outcome,
        ProactiveEventOutcome::IncludeInNextDigest
    );

    let mut burden = input("high-interruption-burden");
    burden.fatigue_level = OperatorFatigueLevel::High;
    let burden = store.evaluate_once(&policy(), &burden).unwrap();
    assert_eq!(
        burden.disposition.outcome,
        ProactiveEventOutcome::PersistSilently
    );
    assert!(matches!(
        store.delivery_permit("high-interruption-burden").unwrap(),
        DeliveryPermit::NotAuthorized
    ));

    let mut failed_run = input("failed-project-run");
    failed_run.origin = ProactiveEventOrigin::ObservedEvent;
    failed_run.action_class = ProactiveActionClass::Proposal;
    assert_eq!(
        store
            .evaluate_once(&policy(), &failed_run)
            .unwrap()
            .disposition
            .outcome,
        ProactiveEventOutcome::PrepareProposal
    );

    let mut overnight = input("overnight-review");
    overnight.action_class = ProactiveActionClass::ReversiblePreAuthorized;
    overnight.human_impact_review.consent_authority = ConsentAuthority::PolicyAllowed;
    let overnight = store.evaluate_once(&policy(), &overnight).unwrap();
    assert_eq!(
        overnight.disposition.outcome,
        ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction
    );
    assert!(overnight.disposition.action_authorized);
    assert!(!overnight.disposition.approval_granted);

    let mut health = input("casual-health-text");
    health.source_quality = ProactiveSourceQuality::SensitiveInference;
    let health = store.evaluate_once(&policy(), &health).unwrap();
    assert_eq!(
        health.disposition.outcome,
        ProactiveEventOutcome::FailClosed
    );
    assert!(!health.disposition.delivery_authorized);
    assert!(!health.disposition.action_authorized);

    for event_id in ["external-message", "payment"] {
        let mut external = input(event_id);
        external.action_class = ProactiveActionClass::ExternalSideEffect;
        let external = store.evaluate_once(&policy(), &external).unwrap();
        assert_eq!(
            external.disposition.outcome,
            ProactiveEventOutcome::RequestApproval
        );
        assert!(external.disposition.delivery_authorized);
        assert!(!external.disposition.action_authorized);
        assert!(!external.disposition.approval_granted);
    }

    drop(store);
    let restarted = ProactiveCycleStore::new(temp.path());
    assert_eq!(restarted.load_all().unwrap().evaluations.len(), 8);
}

#[test]
fn conflicting_replays_and_corrupt_tails_are_rejected_without_message_content_storage() {
    let temp = TempDir::new().unwrap();
    let store = ProactiveCycleStore::new(temp.path());
    let request = input("privacy-and-conflict");
    store.evaluate_once(&policy(), &request).unwrap();

    let mut conflict = request.clone();
    conflict.uncertainty = 0.2;
    assert!(matches!(
        store.evaluate_once(&policy(), &conflict),
        Err(ProactiveCycleError::EventConflict { .. })
    ));

    let raw = fs::read_to_string(store.ledger_path()).unwrap();
    assert!(!raw.contains("private message body"));
    assert!(!raw.contains("concise_summary"));

    fs::write(store.ledger_path(), format!("{raw}{{bad-tail\n")).unwrap();
    assert!(matches!(
        store.load_all(),
        Err(ProactiveCycleError::CorruptEntry { .. })
    ));
}
