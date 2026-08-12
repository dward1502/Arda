use super::*;
use chrono::{TimeZone, Utc};
use serde_json::json;

fn approval() -> TaskApproval {
    TaskApproval {
        schema_version: "arda.orome.task_approval.v1".to_string(),
        proposal_id: "proposal-1".to_string(),
        approval_id: "approval-1".to_string(),
        ledger_writes: Vec::new(),
        decision: "policy_safe".to_string(),
        created_at_utc: "2026-08-11T15:59:00Z".to_string(),
    }
}

fn question_intent() -> ResearchQuestionIntent {
    ResearchQuestionIntent {
        question: "What changed in Arda?".to_string(),
        rationale: "Keep the operator brief current.".to_string(),
        tags: vec!["runtime".to_string()],
        cadence: WatchlistCadence::Manual,
        source_policy: WatchlistSourcePolicy {
            policy_id: "public-web".to_string(),
            allowed_sources: vec!["https://".to_string()],
            max_sources_per_run: 5,
            allow_private_targets: false,
        },
        evidence_requirements: WatchlistEvidenceRequirements {
            minimum_canonical_sources: 1,
            require_canonical_fetch: true,
            max_source_age_seconds: 604_800,
        },
        contradiction_policy: ContradictionPolicy::RequireDisclosure,
        budgets: WatchlistBudgets {
            max_results: 10,
            max_fetch_bytes: 2_000_000,
            max_tokens: 4_000,
            max_attempts: 2,
        },
        notification_policy: WatchlistNotificationPolicy {
            enabled: false,
            destination: None,
        },
        approval_reference: "approval-1".to_string(),
    }
}

#[test]
fn rust_owns_question_and_watchlist_identity_and_timestamps() {
    let now = Utc.with_ymd_and_hms(2026, 8, 11, 16, 0, 0).unwrap();
    let question = canonical_question(question_intent(), "operator-1", now).expect("question");
    assert!(!question.question_id.is_empty());
    assert_eq!(question.owner, "operator-1");
    assert_eq!(question.expires_at_utc, now + chrono::Duration::days(7));
    let replayed = canonical_question(question_intent(), "operator-1", now).expect("replay");
    assert_eq!(question.question_id, replayed.question_id);

    let watchlist = canonical_watchlist(ResearchWatchlistIntent {
        name: "Runtime notices".to_string(),
        question_ids: vec![question.question_id.clone()],
        approval_reference: "approval-1".to_string(),
    })
    .expect("watchlist");
    assert!(!watchlist.watchlist_id.is_empty());
    assert_ne!(watchlist.watchlist_id, question.question_id);
    let replayed = canonical_watchlist(ResearchWatchlistIntent {
        name: "Runtime notices".to_string(),
        question_ids: vec![question.question_id],
        approval_reference: "approval-1".to_string(),
    })
    .expect("watchlist replay");
    assert_eq!(watchlist.watchlist_id, replayed.watchlist_id);
}

#[test]
fn frontend_cannot_supply_research_authority_or_canonical_fields() {
    let value = json!({
        "question": "bounded question",
        "rationale": "bounded rationale",
        "tags": [],
        "cadence": {"kind": "manual"},
        "sourcePolicy": {"policy_id": "public", "allowed_sources": ["https://"], "max_sources_per_run": 2, "allow_private_targets": false},
        "evidenceRequirements": {"minimum_canonical_sources": 1, "require_canonical_fetch": true, "max_source_age_seconds": 3600},
        "contradictionPolicy": "require_disclosure",
        "budgets": {"max_results": 2, "max_fetch_bytes": 4096, "max_tokens": 512, "max_attempts": 1},
        "notificationPolicy": {"enabled": false, "destination": null},
        "approvalReference": "approval-1",
        "questionId": "frontend-owned",
        "owner": "operator-0",
        "expiresAtUtc": "2026-08-12T00:00:00Z",
        "approval": {"decision": "policy_safe"},
        "idempotencyKey": "frontend-key"
    });
    assert!(serde_json::from_value::<ResearchQuestionIntent>(value).is_err());
}

#[test]
fn research_approval_resolution_is_authenticated_exact_and_replay_stable() {
    let now = Utc.with_ymd_and_hms(2026, 8, 11, 16, 0, 0).unwrap();
    let first = resolve_research_intent_from(
        "approval-1",
        "create-question",
        "What changed in Arda?",
        "operator-1",
        approval(),
        now,
        3_600,
    )
    .expect("resolved approval");
    let second = resolve_research_intent_from(
        "approval-1",
        "create-question",
        "What changed in Arda?",
        "operator-1",
        approval(),
        now,
        3_600,
    )
    .expect("stable replay");
    assert_eq!(first.idempotency_key, second.idempotency_key);
    assert_eq!(first.approval.approval_id, "approval-1");

    let mut denied = approval();
    denied.decision = "policy_blocked".to_string();
    assert!(resolve_research_intent_from(
        "approval-1",
        "create-question",
        "resource",
        "operator-1",
        denied,
        now,
        3_600,
    )
    .is_err());
    assert!(resolve_research_intent_from(
        "other",
        "create-question",
        "resource",
        "operator-1",
        approval(),
        now,
        3_600,
    )
    .is_err());
    assert!(resolve_research_intent_from(
        "approval-1",
        "create-question",
        "resource",
        "operator-1",
        approval(),
        now,
        30,
    )
    .is_err());
}

#[test]
fn aggregate_projection_is_versioned_fresh_and_recoverable() {
    let now = Utc.with_ymd_and_hms(2026, 8, 11, 16, 0, 0).unwrap();
    let healthy = project_research_snapshot(
        Ok(json!({"questions": []})),
        Ok(json!({"watchlists": []})),
        Ok(json!({"briefs": []})),
        now,
    );
    assert_eq!(healthy.schema_version, RESEARCH_PROJECTION_SCHEMA);
    assert_eq!(healthy.state, ResearchLoadState::Healthy);
    assert!(!healthy.source_revision.is_empty());
    assert_eq!(healthy.source_time_utc, now);
    assert!(healthy.recovery_action.is_none());

    let degraded = project_research_snapshot(
        Ok(json!({"questions": []})),
        Err("watchlist registry unavailable".to_string()),
        Ok(json!({"briefs": []})),
        now,
    );
    assert_eq!(degraded.state, ResearchLoadState::Partial);
    assert_eq!(degraded.questions, Vec::<serde_json::Value>::new());
    assert!(degraded
        .recovery_action
        .as_deref()
        .unwrap()
        .contains("Refresh"));
    assert!(degraded
        .failures
        .iter()
        .any(|failure| failure.contains("watchlist")));
}
