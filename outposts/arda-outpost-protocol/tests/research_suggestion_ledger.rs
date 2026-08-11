use arda_outpost_protocol::{ResearchSuggestion, ResearchSuggestionLedger};
use chrono::{Duration, Utc};
use tempfile::tempdir;

#[test]
fn suggestion_ingress_is_idempotent_and_cursor_survives_restart() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("suggestions.jsonl");
    let ledger = ResearchSuggestionLedger::open(&path).unwrap();
    let now = Utc::now();
    let suggestion = ResearchSuggestion::new(
        "bounded query",
        "aule:bounded-query",
        now,
        now + Duration::minutes(15),
        3,
        4096,
    )
    .unwrap();

    let first = ledger.append(&suggestion).unwrap();
    let second = ledger.append(&suggestion).unwrap();
    assert_eq!(first, second);
    assert_eq!(ledger.suggestions().unwrap().len(), 1);
    ledger
        .advance_cursor("suggestions", 1, &suggestion.suggestion_id)
        .unwrap();

    let reopened = ResearchSuggestionLedger::open(&path).unwrap();
    let cursor = reopened.read_cursor("suggestions").unwrap();
    assert_eq!(cursor.sequence, 1);
    assert_eq!(
        cursor.last_id.as_deref(),
        Some(suggestion.suggestion_id.as_str())
    );
}
