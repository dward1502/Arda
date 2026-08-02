use arda_outpost_protocol::{
    validate_research_chain, AcknowledgementReceipt, DispatchDisposition,
    ExternalObservationReceipt, ResearchDispatch, ResearchSuggestion, ADVISORY_RESEARCH_AUTHORITY,
};
use chrono::{Duration, Utc};

#[test]
fn complete_advisory_research_chain_validates_with_normalized_url_and_parents() {
    let now = Utc::now();
    let suggestion = ResearchSuggestion::new(
        "rust governance evidence",
        "suggestion-key-1",
        now,
        now + Duration::minutes(15),
        3,
        500,
    )
    .expect("suggestion");
    let dispatch =
        ResearchDispatch::accepted(&suggestion, "dispatch-key-1", now, 2).expect("dispatch");
    let observation = ExternalObservationReceipt::completed(
        &suggestion,
        &dispatch,
        "https://Example.com:443/path?b=2&a=1#fragment",
        "a".repeat(64),
        "b".repeat(64),
        now,
    )
    .expect("observation");
    let acknowledgement =
        AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now)
            .expect("acknowledgement");

    validate_research_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
        .expect("valid complete advisory chain");
    assert_eq!(suggestion.authority, ADVISORY_RESEARCH_AUTHORITY);
    assert_eq!(
        observation.normalized_url,
        "https://example.com/path?a=1&b=2"
    );
    assert_eq!(acknowledgement.disposition, DispatchDisposition::Completed);
}

#[test]
fn research_chain_rejects_expired_suggestion_and_parent_mismatch() {
    let now = Utc::now();
    let suggestion = ResearchSuggestion::new(
        "bounded query",
        "suggestion-key-2",
        now - Duration::minutes(10),
        now - Duration::minutes(1),
        1,
        100,
    )
    .expect("suggestion shape");
    let dispatch =
        ResearchDispatch::accepted(&suggestion, "dispatch-key-2", now, 1).expect("dispatch shape");
    let mut observation = ExternalObservationReceipt::completed(
        &suggestion,
        &dispatch,
        "https://example.com/result",
        "c".repeat(64),
        "d".repeat(64),
        now,
    )
    .expect("observation shape");
    let acknowledgement =
        AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now)
            .expect("acknowledgement shape");
    observation.suggestion_id = "different-parent".to_owned();

    let error =
        validate_research_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
            .expect_err("expired or malformed parent chain must not validate");
    assert!(error.to_string().contains("expired") || error.to_string().contains("parent"));
}
