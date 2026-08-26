use arda_outpost_protocol::{
    ContradictionPolicy, ResearchQuestion, ResearchQuestionSpec, ResearchWatchlist,
    WatchlistBudgets, WatchlistCadence, WatchlistError, WatchlistEvidenceRequirements,
    WatchlistNotificationPolicy, WatchlistSourcePolicy, WatchlistState, WATCHLIST_SCHEMA_VERSION,
};
use chrono::{Duration, Utc};

fn question() -> ResearchQuestion {
    ResearchQuestion::new(ResearchQuestionSpec {
        owner: "operator@example.test".into(),
        question: "What changed in the Arda runtime this week?".into(),
        rationale: "Keep the operator brief current without granting execution authority.".into(),
        tags: vec!["runtime".into(), "release".into()],
        cadence: WatchlistCadence::Interval {
            every_seconds: 86_400,
        },
        expires_at_utc: Utc::now() + Duration::hours(24),
        source_policy: WatchlistSourcePolicy {
            policy_id: "public-docs-v1".into(),
            allowed_sources: vec!["docs".into(), "release-notes".into()],
            max_sources_per_run: 8,
            allow_private_targets: false,
        },
        evidence_requirements: WatchlistEvidenceRequirements {
            minimum_canonical_sources: 2,
            require_canonical_fetch: true,
            max_source_age_seconds: 86_400,
        },
        contradiction_policy: ContradictionPolicy::RequireDisclosure,
        budgets: WatchlistBudgets {
            max_results: 10,
            max_fetch_bytes: 256_000,
            max_tokens: 8_000,
            max_attempts: 2,
        },
        notification_policy: WatchlistNotificationPolicy {
            enabled: true,
            destination: Some("operator".into()),
        },
    })
    .expect("valid question contract")
}

#[test]
fn question_and_watchlist_round_trip_preserves_product_contract() {
    let question = question();
    assert_eq!(question.schema_version, WATCHLIST_SCHEMA_VERSION);
    assert!(question.backend_suggestion_ids.is_empty());
    assert_eq!(question.state, WatchlistState::Enabled);

    let encoded = serde_json::to_value(&question).expect("encode question");
    let decoded: ResearchQuestion = serde_json::from_value(encoded).expect("decode question");
    assert_eq!(decoded, question);

    let watchlist = ResearchWatchlist::new("Runtime watch", vec![question.question_id.clone()])
        .expect("valid watchlist");
    assert_eq!(watchlist.question_ids, vec![question.question_id]);
}

#[test]
fn lifecycle_pause_resume_and_retire_are_explicit_and_expiry_gated() {
    let mut question = question();
    question.pause().expect("pause enabled question");
    assert_eq!(question.state, WatchlistState::Paused);
    question
        .resume(Utc::now())
        .expect("resume unexpired question");
    assert_eq!(question.state, WatchlistState::Enabled);

    question.expires_at_utc = Utc::now() - Duration::seconds(1);
    assert_eq!(
        question.validate_at(Utc::now()),
        Err(WatchlistError::Expired)
    );
    assert_eq!(question.resume(Utc::now()), Err(WatchlistError::Expired));

    question.retire();
    assert_eq!(question.pause(), Err(WatchlistError::InvalidTransition));
    assert_eq!(
        question.validate_at(Utc::now()),
        Err(WatchlistError::Retired)
    );
}

#[test]
fn malformed_and_unknown_product_fields_are_rejected() {
    let mut value = serde_json::to_value(question()).expect("encode question");
    value["unexpected"] = serde_json::json!(true);
    let error = serde_json::from_value::<ResearchQuestion>(value).expect_err("unknown field");
    assert!(error.to_string().contains("unknown field"));

    let mut invalid = question();
    invalid.cadence = WatchlistCadence::Interval { every_seconds: 0 };
    assert_eq!(
        invalid.validate_at(Utc::now()),
        Err(WatchlistError::InvalidField("cadence"))
    );

    let invalid_watchlist = ResearchWatchlist::new("", vec![]);
    assert_eq!(
        invalid_watchlist,
        Err(WatchlistError::InvalidField("watchlist"))
    );
}
