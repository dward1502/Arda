use arda_core::next_action::{
    select_next_action, NextActionAuthorityState, NextActionCandidate, NextActionFreshness,
    NextActionSourceKind, NextActionStatus,
};
use chrono::{TimeZone, Utc};

fn candidate(id: &str, priority: u8) -> NextActionCandidate {
    NextActionCandidate {
        id: id.to_string(),
        title: format!("Action {id}"),
        source_kind: NextActionSourceKind::Queue,
        source_ref: format!("core/projects/tasks/queue.jsonl#{id}"),
        reason: "Operator-authored current commitment".to_string(),
        freshness: NextActionFreshness::Fresh,
        authority_state: NextActionAuthorityState::ReviewRequired,
        next_operator_action: format!("Review and start {id}"),
        priority,
        operator_authored: true,
        terminal: false,
        future_gated: false,
        inferred_without_review: false,
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 20, 18, 0, 0).unwrap()
}

#[test]
fn empty_inputs_return_an_honest_empty_projection() {
    let projection = select_next_action(Vec::new(), now());

    assert_eq!(projection.status, NextActionStatus::Empty);
    assert!(projection.selected.is_none());
    assert_eq!(
        projection.reason,
        "No current trustworthy action is available."
    );
}

#[test]
fn blocked_current_work_returns_the_smallest_unblock_action() {
    let mut blocked = candidate("blocked-run", 100);
    blocked.source_kind = NextActionSourceKind::Workbench;
    blocked.authority_state = NextActionAuthorityState::Blocked;
    blocked.next_operator_action =
        "Inspect the blocked run and choose recovery or cancellation".into();

    let projection = select_next_action(vec![blocked], now());

    assert_eq!(projection.status, NextActionStatus::Blocked);
    assert_eq!(projection.selected.unwrap().id, "blocked-run");
}

#[test]
fn stale_terminal_future_and_unreviewed_inference_are_excluded() {
    let mut stale = candidate("stale", 100);
    stale.freshness = NextActionFreshness::Stale;
    let mut terminal = candidate("terminal", 100);
    terminal.terminal = true;
    let mut future = candidate("future", 100);
    future.future_gated = true;
    let mut inferred = candidate("inferred", 100);
    inferred.operator_authored = false;
    inferred.inferred_without_review = true;

    let projection = select_next_action(vec![stale, terminal, future, inferred], now());

    assert_eq!(projection.status, NextActionStatus::Empty);
    assert!(projection.selected.is_none());
    assert_eq!(projection.excluded.stale, 1);
    assert_eq!(projection.excluded.terminal, 1);
    assert_eq!(projection.excluded.future_gated, 1);
    assert_eq!(projection.excluded.inferred_without_review, 1);
}

#[test]
fn conflicting_priorities_choose_the_highest_then_deterministic_identity() {
    let low = candidate("low", 30);
    let high_b = candidate("high-b", 90);
    let high_a = candidate("high-a", 90);

    let projection = select_next_action(vec![low, high_b, high_a], now());

    assert_eq!(projection.status, NextActionStatus::Ready);
    assert_eq!(projection.selected.unwrap().id, "high-a");
}
