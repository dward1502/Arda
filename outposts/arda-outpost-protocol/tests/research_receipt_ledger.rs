use arda_outpost_protocol::{
    validate_research_chain, AcknowledgementReceipt, ExternalObservationReceipt, ResearchDispatch,
    ResearchReceiptLedger, ResearchSuggestion,
};
use chrono::{Duration, Utc};
use tempfile::tempdir;

#[test]
fn ledger_persists_a_complete_chain_once_and_replays_it_after_reopen() {
    let now = Utc::now();
    let suggestion = ResearchSuggestion::new(
        "durable bounded research",
        "suggestion-idempotency",
        now,
        now + Duration::minutes(5),
        2,
        1024,
    )
    .unwrap();
    let dispatch = ResearchDispatch::accepted(&suggestion, "dispatch-idempotency", now, 1).unwrap();
    let observation = ExternalObservationReceipt::completed(
        &suggestion,
        &dispatch,
        "https://example.com/evidence",
        "e".repeat(64),
        "f".repeat(64),
        now,
    )
    .unwrap();
    let acknowledgement =
        AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now).unwrap();
    validate_research_chain(&suggestion, &dispatch, &observation, &acknowledgement, now).unwrap();

    let directory = tempdir().unwrap();
    let path = directory.path().join("warden-receipts.jsonl");
    let ledger = ResearchReceiptLedger::open(&path).unwrap();
    let first = ledger
        .append_complete_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
        .unwrap();
    let replay = ledger
        .append_complete_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
        .unwrap();
    assert_eq!(first, replay);

    let reopened = ResearchReceiptLedger::open(&path).unwrap();
    let chains = reopened.complete_chains().unwrap();
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].suggestion.suggestion_id, suggestion.suggestion_id);
    assert_eq!(chains[0].observation.content_hash, observation.content_hash);
}
