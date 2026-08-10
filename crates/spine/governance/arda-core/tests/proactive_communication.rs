use std::collections::BTreeSet;

use arda_core::capability_composition::ProactiveCommunicationMode;
use arda_core::governance_gates::{
    ActionReversibility, CoercionRisk, ConsentAuthority, HumanFacingActionReview,
    JouleWorkBudgetClass,
};
use arda_core::proactive_communication::{
    evaluate_proactive_communication, render_proactive_message, OperatorFatigueLevel,
    PriorOperatorResponse, ProactiveActionClass, ProactiveChannel, ProactiveCommunicationInput,
    ProactiveCommunicationPolicy, ProactiveEventOrigin, ProactiveEventOutcome,
    ProactiveMessageLabel, ProactiveSourceQuality, ProactiveUrgency,
};
use chrono::{Duration, TimeZone, Utc};

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

fn input() -> ProactiveCommunicationInput {
    let evaluated_at = Utc.with_ymd_and_hms(2026, 8, 9, 12, 0, 0).unwrap();
    ProactiveCommunicationInput {
        event_id: "event:appointment-1".into(),
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
            interruption_reason: Some("appointment starts in ten minutes".into()),
            consent_authority: ConsentAuthority::OperatorAuthored,
            uncertainty: 0.05,
            coercion_risk: CoercionRisk::Low,
        },
        evidence_refs: vec!["personal-event:appointment-1".into()],
    }
}

#[test]
fn evaluates_each_canonical_p5_2_outcome() {
    let policy = policy();
    let mut observed = BTreeSet::new();

    let notification = evaluate_proactive_communication(&policy, &input());
    observed.insert(format!("{:?}", notification.outcome));
    assert_eq!(notification.outcome, ProactiveEventOutcome::NotifyOnce);
    assert!(notification.delivery_authorized);
    assert!(!notification.action_authorized);
    assert!(!notification.approval_granted);

    let mut request = input();
    request.expires_at = request.evaluated_at;
    let ignored = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", ignored.outcome));
    assert_eq!(ignored.outcome, ProactiveEventOutcome::Ignore);

    request = input();
    request.resource_budget_available = false;
    let silent = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", silent.outcome));
    assert_eq!(silent.outcome, ProactiveEventOutcome::PersistSilently);

    request = input();
    request.urgency = ProactiveUrgency::Background;
    let digest = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", digest.outcome));
    assert_eq!(digest.outcome, ProactiveEventOutcome::IncludeInNextDigest);

    request = input();
    request.action_class = ProactiveActionClass::Proposal;
    let proposal = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", proposal.outcome));
    assert_eq!(proposal.outcome, ProactiveEventOutcome::PrepareProposal);

    request = input();
    request.action_class = ProactiveActionClass::ReversiblePreAuthorized;
    request.human_impact_review.consent_authority = ConsentAuthority::PolicyAllowed;
    let execution = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", execution.outcome));
    assert_eq!(
        execution.outcome,
        ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction
    );
    assert!(execution.action_authorized);
    assert!(!execution.approval_granted);

    request.human_impact_review.reversibility = ActionReversibility::Irreversible;
    let irreversible = evaluate_proactive_communication(&policy, &request);
    assert_ne!(
        irreversible.outcome,
        ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction
    );
    assert!(!irreversible.action_authorized);

    request = input();
    request.action_class = ProactiveActionClass::ExternalSideEffect;
    let approval = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", approval.outcome));
    assert_eq!(approval.outcome, ProactiveEventOutcome::RequestApproval);
    assert!(approval.delivery_authorized);
    assert!(!approval.action_authorized);
    assert!(!approval.approval_granted);

    request = input();
    request.source_quality = ProactiveSourceQuality::SensitiveInference;
    let failed = evaluate_proactive_communication(&policy, &request);
    observed.insert(format!("{:?}", failed.outcome));
    assert_eq!(failed.outcome, ProactiveEventOutcome::FailClosed);
    assert!(!failed.delivery_authorized);
    assert!(!failed.action_authorized);

    assert_eq!(observed.len(), 8, "every P5.2 outcome must be exercised");
}

#[test]
fn quiet_fatigue_delivery_and_operator_response_budgets_prevent_repetition() {
    let policy = policy();

    let mut request = input();
    request.in_quiet_window = true;
    let quiet = evaluate_proactive_communication(&policy, &request);
    assert_eq!(quiet.outcome, ProactiveEventOutcome::IncludeInNextDigest);
    assert_eq!(quiet.reason_code, "quiet_window");

    request = input();
    request.fatigue_level = OperatorFatigueLevel::High;
    let fatigued = evaluate_proactive_communication(&policy, &request);
    assert_eq!(fatigued.outcome, ProactiveEventOutcome::PersistSilently);

    request = input();
    request.prior_attempts = 1;
    let attempted = evaluate_proactive_communication(&policy, &request);
    assert_eq!(
        attempted.outcome,
        ProactiveEventOutcome::IncludeInNextDigest
    );

    request = input();
    request.prior_response = PriorOperatorResponse::Acknowledged;
    let acknowledged = evaluate_proactive_communication(&policy, &request);
    assert_eq!(acknowledged.outcome, ProactiveEventOutcome::Ignore);
}

#[test]
fn missing_evidence_invalid_review_and_sensitive_inference_fail_closed() {
    let policy = policy();
    let mut request = input();
    request.evidence_refs.clear();
    assert_eq!(
        evaluate_proactive_communication(&policy, &request).reason_code,
        "evidence_required"
    );

    request = input();
    request.human_impact_review.semantic = "uncanonical".into();
    assert_eq!(
        evaluate_proactive_communication(&policy, &request).reason_code,
        "invalid_event_contract"
    );

    request = input();
    request.action_class = ProactiveActionClass::SensitiveInference;
    let disposition = evaluate_proactive_communication(&policy, &request);
    assert_eq!(disposition.outcome, ProactiveEventOutcome::FailClosed);
    assert_eq!(disposition.reason_code, "human_impact_gate_failed");
}

#[test]
fn explanation_discloses_interruption_source_uncertainty_and_budgets() {
    let disposition = evaluate_proactive_communication(&policy(), &input());
    assert!(disposition.explanation.contains("appointment starts"));
    assert!(disposition.explanation.contains("OperatorAuthored"));
    assert!(disposition.explanation.contains("uncertainty: 0.05"));
    assert!(disposition.explanation.contains("budget class: routine"));
    assert!(disposition.explanation.contains("attempts: 0/1"));
    assert!(disposition.explanation.contains("interruptions: 0/3"));
}

#[test]
fn versioned_messages_are_labeled_routed_calm_and_bounded() {
    let base_policy = policy();
    let request = input();
    let disposition = evaluate_proactive_communication(&base_policy, &request);
    let message = render_proactive_message(
        &base_policy,
        &request,
        &disposition,
        "Appointment starts at noon.",
    )
    .unwrap();
    assert_eq!(message.label, ProactiveMessageLabel::Fact);
    assert_eq!(message.channel, ProactiveChannel::OperatorSession);
    assert_eq!(message.template_version, "arda.proactive-message.v1");
    assert!(message.text.starts_with("Fact:"));
    assert!(message.text.contains("Why now:"));
    assert!(!message.text.contains("urgent!"));

    let mut inferred = input();
    inferred.source_quality = ProactiveSourceQuality::Unverified;
    inferred.uncertainty = 0.6;
    let disposition = evaluate_proactive_communication(&base_policy, &inferred);
    let message = render_proactive_message(
        &base_policy,
        &inferred,
        &disposition,
        "This may need attention.",
    )
    .unwrap();
    assert_eq!(message.label, ProactiveMessageLabel::Inference);
    assert_eq!(message.channel, ProactiveChannel::Digest);

    let mut bounded_policy = policy();
    bounded_policy.max_message_chars = 48;
    let disposition = evaluate_proactive_communication(&bounded_policy, &request);
    let message = render_proactive_message(
        &bounded_policy,
        &request,
        &disposition,
        &"word ".repeat(100),
    )
    .unwrap();
    assert_eq!(message.text.chars().count(), 48);
    assert!(message.text.ends_with('…'));
}

#[test]
fn suppressed_sensitive_and_action_only_outcomes_do_not_render_messages() {
    let policy = policy();
    let mut request = input();
    request.source_quality = ProactiveSourceQuality::SensitiveInference;
    let disposition = evaluate_proactive_communication(&policy, &request);
    assert!(render_proactive_message(&policy, &request, &disposition, "private").is_none());

    request = input();
    request.action_class = ProactiveActionClass::ReversiblePreAuthorized;
    request.human_impact_review.consent_authority = ConsentAuthority::PolicyAllowed;
    let disposition = evaluate_proactive_communication(&policy, &request);
    assert_eq!(
        disposition.outcome,
        ProactiveEventOutcome::ExecuteReversiblePreAuthorizedAction
    );
    assert!(render_proactive_message(&policy, &request, &disposition, "done").is_none());
}

#[test]
fn repository_policy_file_loads_the_bounded_defaults() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let loaded = ProactiveCommunicationPolicy::load(
        &root.join("config/governance/autonomy_operating_loop.toml"),
    )
    .unwrap();
    assert_eq!(loaded, policy());
}
