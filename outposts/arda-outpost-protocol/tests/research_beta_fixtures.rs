use arda_outpost_protocol::{
    disabled_watchlist_templates, inspect_untrusted_content, ResearchBetaPolicy,
    PROPOSAL_ONLY_AUTHORITY,
};

const POISONED_SOURCE: &str = include_str!("fixtures/poisoned_source.html");

#[test]
fn poisoned_fixture_is_evidence_only_and_never_an_operator_instruction() {
    let inspection = inspect_untrusted_content(POISONED_SOURCE);
    assert!(inspection.untrusted);
    assert!(inspection.prompt_injection_detected);
    assert!(inspection
        .signals
        .iter()
        .any(|signal| signal == "ignore_previous_instructions"));
    assert_eq!(
        inspection.boundary,
        "source_text_untrusted_instructions_ignored"
    );
}

#[test]
fn beta_defaults_are_bounded_and_templates_need_explicit_opt_in() {
    let policy = ResearchBetaPolicy::default();
    policy.validate().expect("valid beta policy");
    assert!(policy.max_attempts <= 3);
    assert!(policy.max_results <= 100);
    assert!(policy.max_fetch_bytes > 0);
    assert!(policy.max_tokens > 0);
    assert!(policy.retained_preview_volume > 0);

    let templates = disabled_watchlist_templates();
    assert_eq!(templates.len(), 6);
    assert!(templates
        .iter()
        .all(|template| !template.enabled_by_default
            && template.authority == PROPOSAL_ONLY_AUTHORITY));
}
