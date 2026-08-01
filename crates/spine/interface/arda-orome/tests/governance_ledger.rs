use arda_core::governance_gates::GovernanceGates;
use arda_orome::{GovernanceHooks, InterruptionLedgerDecision, InterruptionMessage};

#[test]
fn task_approval_envelope_is_typed_and_appended_to_ledger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let hooks =
        GovernanceHooks::new(GovernanceGates::permissive(), dir.path()).expect("governance hooks");

    let envelope = hooks
        .record_task_approval("proposal-1", "approval-1")
        .expect("approval envelope");

    assert_eq!(envelope.schema_version, "arda.orome.task_approval.v1");
    assert_eq!(envelope.proposal_id, "proposal-1");
    assert_eq!(envelope.approval_id, "approval-1");
    assert_eq!(envelope.decision, InterruptionLedgerDecision::PolicySafe);
    assert_eq!(envelope.ledger_writes.len(), 1);
    let ledger = std::fs::read_to_string(&envelope.ledger_writes[0]).expect("ledger content");
    assert!(ledger.contains("proposal-1"));
    assert!(ledger.contains("approval-1"));
}

#[test]
fn interruption_uses_central_action_policy_and_writes_decision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gates = GovernanceGates::load_from_str(
        r#"
default:
  policy_mode: record_and_proceed
action_classes:
  provider_reroute:
    policy_mode: escalate_to_human
  destructive_override:
    policy_mode: block_on_fail
"#,
    )
    .expect("policy config");
    let hooks = GovernanceHooks::new(gates, dir.path()).expect("governance hooks");

    let review = hooks
        .record_interruption(
            "interrupt-review",
            InterruptionMessage::new("hud", "operator", "reroute edge work"),
            "provider_reroute",
        )
        .expect("review envelope");
    assert_eq!(
        review.decision,
        InterruptionLedgerDecision::RequiresOperatorReview
    );

    let blocked = hooks
        .record_interruption(
            "interrupt-blocked",
            InterruptionMessage::new("hud", "operator", "override policy"),
            "destructive_override",
        )
        .expect("blocked envelope");
    assert_eq!(blocked.decision, InterruptionLedgerDecision::PolicyBlocked);

    let ledger = std::fs::read_to_string(&blocked.ledger_writes[0]).expect("ledger content");
    assert!(ledger.contains("interrupt-review"));
    assert!(ledger.contains("interrupt-blocked"));
    assert!(ledger.contains("requires_operator_review"));
    assert!(ledger.contains("policy_blocked"));
}
