use arda_governance::{
    load_philosopher_profiles_from_str, PhilosopherProfileMaturity, PhilosopherProfileSet,
};

const BOOTSTRAP_PROFILES: &str = r#"
schema_version = "arda.governance.philosopher_profiles.v1"
authority = "human_authored_bootstrap"
default_maturity = "draft_human_authored"
autonomous_blocking_enabled = false
generated_corpus_promotion_enabled = false

[[profiles]]
id = "aurelius"
display_name = "Marcus Aurelius"
lens = "logic_and_temperance"
maturity = "draft_human_authored"
implementation_status = "draft_human_authored"
authority = "human_authored_bootstrap"
canonical_sources = ["Meditations"]
decision_questions = ["Does the rationale distinguish duty from appetite?"]
failure_modes = ["obedience_mistaken_for_reason"]
veto_scope = "Advisory flag for rational scope and temperance failures."
confidence_floor = 0.70
primary_questions = ["Is this rational, temperate, and within our control?"]
required_evidence = ["reasoned_rationale", "scope_boundary"]
forbidden_claims = ["autonomous_consensus_receipted"]

[[profiles]]
id = "bacon"
display_name = "Francis Bacon"
lens = "empirical_evidence"
maturity = "draft_human_authored"
implementation_status = "draft_human_authored"
authority = "human_authored_bootstrap"
canonical_sources = ["Novum Organum"]
decision_questions = ["What observation or receipt grounds the claim?"]
failure_modes = ["source_free_truth_claim"]
veto_scope = "Advisory flag for missing evidence anchors."
confidence_floor = 0.75
primary_questions = ["What evidence would disprove this claim?"]
required_evidence = ["source_path", "verification_command"]
forbidden_claims = ["proof_without_receipt"]

[[profiles]]
id = "sun_tzu"
display_name = "Sun Tzu"
lens = "strategy_and_risk"
maturity = "draft_human_authored"
implementation_status = "draft_human_authored"
authority = "human_authored_bootstrap"
canonical_sources = ["The Art of War"]
decision_questions = ["What position does this action create next?"]
failure_modes = ["overextension_without_fallback"]
veto_scope = "Advisory flag for hidden risk and missing fallback paths."
confidence_floor = 0.70
primary_questions = ["What position does this action create next?"]
required_evidence = ["risk_boundary", "fallback_path"]
forbidden_claims = ["risk_free_action"]
"#;

#[test]
fn loads_bootstrap_philosopher_profiles_without_enabling_autonomy() {
    let profiles = load_philosopher_profiles_from_str(BOOTSTRAP_PROFILES)
        .expect("bootstrap profiles should parse and validate");

    assert_eq!(
        profiles.schema_version,
        PhilosopherProfileSet::SCHEMA_VERSION
    );
    assert!(!profiles.autonomous_blocking_enabled);
    assert!(!profiles.generated_corpus_promotion_enabled);
    assert_eq!(profiles.profiles.len(), 3);
    assert!(profiles.profile("aurelius").is_some());
    assert!(profiles.profile("bacon").is_some());
    assert!(profiles.profile("sun_tzu").is_some());
    assert!(profiles
        .profiles
        .iter()
        .all(|profile| profile.maturity == PhilosopherProfileMaturity::DraftHumanAuthored));
    assert!(profiles.profiles.iter().all(|profile| {
        profile.implementation_status == PhilosopherProfileMaturity::DraftHumanAuthored
            && !profile.canonical_sources.is_empty()
            && !profile.decision_questions.is_empty()
            && !profile.failure_modes.is_empty()
            && !profile.veto_scope.is_empty()
            && profile.confidence_floor >= 0.0
            && profile.confidence_floor <= 1.0
    }));
}

#[test]
fn repository_bootstrap_config_matches_g2_contract() {
    let profiles = load_philosopher_profiles_from_str(include_str!(
        "../../../config/governance/philosophers.toml"
    ))
    .expect("repository bootstrap config should parse and validate");

    let ids: Vec<&str> = profiles
        .profiles
        .iter()
        .map(|profile| profile.id.as_str())
        .collect();
    assert_eq!(ids, vec!["aurelius", "bacon", "sun_tzu"]);
    assert!(profiles
        .profiles
        .iter()
        .all(|profile| !profile.forbidden_claims.is_empty()));
    assert!(profiles
        .profiles
        .iter()
        .all(|profile| !profile.canonical_sources.is_empty()
            && !profile.decision_questions.is_empty()
            && !profile.failure_modes.is_empty()));
}

#[test]
fn projects_profiles_as_non_blocking_status_metadata() {
    let profiles = load_philosopher_profiles_from_str(BOOTSTRAP_PROFILES)
        .expect("bootstrap profiles should parse and validate");

    let projection = profiles.status_projection("config/governance/philosophers.toml");

    assert_eq!(
        projection.schema_version,
        PhilosopherProfileSet::SCHEMA_VERSION
    );
    assert_eq!(
        projection.profile_source,
        "config/governance/philosophers.toml"
    );
    assert_eq!(projection.chain_id, "default_triad");
    assert_eq!(projection.chain_version, "heuristic_local_v1");
    assert_eq!(projection.review_mode, "heuristic_local");
    assert_eq!(projection.profile_maturity, "draft_human_authored");
    assert!(!projection.autonomous_blocking_enabled);
    assert!(!projection.generated_corpus_promotion_enabled);
    assert_eq!(projection.profile_count, 3);
    assert_eq!(projection.profiles[0].id, "aurelius");
    assert_eq!(projection.profiles[0].display_name, "Marcus Aurelius");
    assert_eq!(projection.profiles[0].lens, "logic_and_temperance");
    assert_eq!(
        projection.profiles[0].maturity,
        PhilosopherProfileMaturity::DraftHumanAuthored
    );
    assert_eq!(
        projection.profiles[0].implementation_status,
        PhilosopherProfileMaturity::DraftHumanAuthored
    );
    assert_eq!(projection.profiles[0].confidence_floor, 0.70);
    assert!(projection
        .profiles
        .iter()
        .all(|profile| !profile.autonomous_blocking_enabled));
}

#[test]
fn rejects_missing_doctrine_fields_or_invalid_confidence_floor() {
    let missing_sources = BOOTSTRAP_PROFILES.replace(
        "canonical_sources = [\"Meditations\"]",
        "canonical_sources = []",
    );
    let sources_err = load_philosopher_profiles_from_str(&missing_sources)
        .expect_err("profile sources are required for doctrine traceability");
    assert!(sources_err.to_string().contains("canonical_sources"));

    let invalid_floor =
        BOOTSTRAP_PROFILES.replace("confidence_floor = 0.70", "confidence_floor = 1.70");
    let floor_err = load_philosopher_profiles_from_str(&invalid_floor)
        .expect_err("confidence floors must stay bounded");
    assert!(floor_err.to_string().contains("confidence_floor"));
}

#[test]
fn rejects_profile_sets_that_attempt_to_enable_autonomous_blocking() {
    let unsafe_config = BOOTSTRAP_PROFILES.replace(
        "autonomous_blocking_enabled = false",
        "autonomous_blocking_enabled = true",
    );

    let err = load_philosopher_profiles_from_str(&unsafe_config)
        .expect_err("G2 profile config must not enable autonomous blocking");

    assert!(err
        .to_string()
        .contains("autonomous_blocking_enabled must remain false"));
}

#[test]
fn rejects_duplicate_or_non_draft_bootstrap_profiles() {
    let duplicate_config = BOOTSTRAP_PROFILES.replace("id = \"sun_tzu\"", "id = \"bacon\"");
    let duplicate_err = load_philosopher_profiles_from_str(&duplicate_config)
        .expect_err("duplicate profile identifiers should be rejected");
    assert!(duplicate_err
        .to_string()
        .contains("duplicate philosopher profile id"));

    let mature_config = BOOTSTRAP_PROFILES.replace(
        "maturity = \"draft_human_authored\"",
        "maturity = \"autonomous_consensus_receipted\"",
    );
    let maturity_err = load_philosopher_profiles_from_str(&mature_config)
        .expect_err("G2 bootstrap must stay draft-human-authored only");
    assert!(maturity_err
        .to_string()
        .contains("only draft_human_authored profiles are allowed in G2"));
}
