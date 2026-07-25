use std::time::Duration;

use arda_core::Task;
use arda_governance::{
    evaluate_realm_governance, load_realm_policy_from_str, GovernanceReadinessEvidence,
    GovernanceReadinessLevel, GovernanceReadinessReport, GovernanceScoreFuture,
    GovernanceScoreReceipt, GovernanceScoreRequest, GovernanceScorer, GovernanceScorerError,
    GovernanceScorerErrorKind, GovernanceScorerState, GovernanceSubsystemReadiness,
    LocalGovernanceScorer, RealmPolicyReloadStatus, RealmPolicyStore, RuntimeBlockingAuthority,
};

fn ready_report(subsystem: &str, review_receipt: &str) -> GovernanceReadinessReport {
    GovernanceReadinessReport {
        schema_version: "arda.governance.readiness.v1".to_string(),
        contract: "phase8-test".to_string(),
        default_autonomy_ready: false,
        requirements: Vec::new(),
        subsystems: vec![GovernanceSubsystemReadiness {
            subsystem: subsystem.to_string(),
            current_level: GovernanceReadinessLevel::AutonomyReadyForScope,
            claimed_level: GovernanceReadinessLevel::AutonomyReadyForScope,
            evidence: GovernanceReadinessEvidence {
                documentation: true,
                local_heuristic: true,
                source_metadata: true,
                runtime_receipts: true,
                policy_enforcement: true,
                independent_review_receipts: true,
                scoped_autonomy_policy: true,
            },
            missing_evidence: Vec::new(),
            receipts: vec![review_receipt.to_string()],
            maturity_label: "scope_ready".to_string(),
            autonomy_ready: true,
        }],
    }
}

struct FixtureScorer;

impl GovernanceScorer for FixtureScorer {
    fn scorer_id(&self) -> &'static str {
        "fixture-scorer"
    }

    fn score<'a>(&'a self, request: GovernanceScoreRequest) -> GovernanceScoreFuture<'a> {
        Box::pin(async move {
            let score = match request.lens_id.as_str() {
                "aurelius" => 0.6,
                "bacon" => 0.9,
                "sun_tzu" => 0.2,
                _ => 0.0,
            };
            Ok(GovernanceScoreReceipt::complete_local(
                &request,
                self.scorer_id(),
                score,
            ))
        })
    }
}

#[tokio::test]
async fn realm_fixtures_change_bacon_and_sun_tzu_emphasis_without_code_changes() {
    let research = load_realm_policy_from_str(include_str!("fixtures/realm_policy_research.toml"))
        .expect("research policy");
    let operations =
        load_realm_policy_from_str(include_str!("fixtures/realm_policy_operations.toml"))
            .expect("operations policy");

    let research_rule = research
        .resolve("research", "evidence_assessment")
        .expect("research scope");
    let operations_rule = operations
        .resolve("operations", "deployment")
        .expect("operations scope");

    assert!(research_rule.weights["bacon"] > research_rule.weights["sun_tzu"]);
    assert!(operations_rule.weights["sun_tzu"] > operations_rule.weights["bacon"]);
    assert_ne!(
        research_rule.thresholds["bacon"],
        operations_rule.thresholds["bacon"]
    );

    let task = Task::new("same scorer inputs across realms", "governance");
    let research_verdict = evaluate_realm_governance(
        &task,
        &research,
        "research",
        "evidence_assessment",
        &FixtureScorer,
        Duration::from_millis(100),
    )
    .await
    .expect("research verdict");
    let operations_verdict = evaluate_realm_governance(
        &task,
        &operations,
        "operations",
        "deployment",
        &FixtureScorer,
        Duration::from_millis(100),
    )
    .await
    .expect("operations verdict");
    assert!(research_verdict.weighted_score > operations_verdict.weighted_score);
}

#[test]
fn invalid_policy_rejects_unknown_lenses_thresholds_and_global_blocking() {
    let base = include_str!("fixtures/realm_policy_research.toml");

    let unknown = base.replace(
        "required_lenses = [\"aurelius\", \"bacon\", \"sun_tzu\"]",
        "required_lenses = [\"aurelius\", \"unknown_oracle\"]",
    );
    assert!(load_realm_policy_from_str(&unknown)
        .expect_err("unknown lens must fail")
        .to_string()
        .contains("unknown lens"));

    let invalid_threshold = base.replacen("bacon = 0.50", "bacon = 1.50", 1);
    assert!(load_realm_policy_from_str(&invalid_threshold)
        .expect_err("threshold must fail")
        .to_string()
        .contains("threshold"));

    let global_blocking = base.replacen(
        "autonomous_blocking_enabled = false",
        "autonomous_blocking_enabled = true",
        1,
    );
    assert!(load_realm_policy_from_str(&global_blocking)
        .expect_err("global blocking must fail")
        .to_string()
        .contains("global autonomous blocking"));
}

#[test]
fn blocking_authority_requires_named_ready_scope_review_rollback_and_operator_control() {
    let policy = load_realm_policy_from_str(include_str!("fixtures/realm_policy_operations.toml"))
        .expect("operations policy");
    let readiness = ready_report(
        "realm:operations:action:deployment",
        "review-operations-deployment-001",
    );

    let enabled =
        RuntimeBlockingAuthority::evaluate(&policy, "operations", "deployment", &readiness, true);
    assert!(enabled.blocking_enabled);
    assert_eq!(enabled.scope_id.as_deref(), Some("operations-strategy"));

    let operator_disabled =
        RuntimeBlockingAuthority::evaluate(&policy, "operations", "deployment", &readiness, false);
    assert!(!operator_disabled.blocking_enabled);
    assert!(operator_disabled.reason.contains("operator"));

    let global =
        RuntimeBlockingAuthority::evaluate(&policy, "unconfigured", "deployment", &readiness, true);
    assert!(!global.blocking_enabled);
    assert!(global.scope_id.is_none());

    let mut invalid = policy.clone();
    invalid.global_default.autonomous_blocking_enabled = true;
    let invalid_decision =
        RuntimeBlockingAuthority::evaluate(&invalid, "operations", "deployment", &readiness, true);
    assert!(!invalid_decision.blocking_enabled);
    assert!(invalid_decision.reason.contains("invalid realm policy"));
}

#[tokio::test]
async fn deterministic_local_scorer_is_async_first_and_receipted() {
    let task = Task::new("verify deployment with evidence and rollback", "governance");
    let request = GovernanceScoreRequest::new(task, "bacon");
    let scorer = LocalGovernanceScorer;

    let first = arda_governance::score_governance_with_timeout(
        &scorer,
        request.clone(),
        Duration::from_millis(100),
    )
    .await;
    let second = arda_governance::score_governance_with_timeout(
        &scorer,
        request,
        Duration::from_millis(100),
    )
    .await;

    assert_eq!(first.state, GovernanceScorerState::Complete);
    assert_eq!(first.score, second.score);
    assert_eq!(first.task_hash, second.task_hash);
    assert_eq!(first.provider, "local");
    assert_eq!(first.model, "structured_evidence_v2");
    assert!(!first.provenance.is_empty());
    assert!(!first.reproducibility_limits.is_empty());
}

struct SlowScorer;

impl GovernanceScorer for SlowScorer {
    fn scorer_id(&self) -> &'static str {
        "slow-test"
    }

    fn score<'a>(&'a self, request: GovernanceScoreRequest) -> GovernanceScoreFuture<'a> {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(GovernanceScoreReceipt::complete_local(
                &request,
                self.scorer_id(),
                0.9,
            ))
        })
    }
}

struct ErrorScorer;

impl GovernanceScorer for ErrorScorer {
    fn scorer_id(&self) -> &'static str {
        "error-test"
    }

    fn score<'a>(&'a self, _request: GovernanceScoreRequest) -> GovernanceScoreFuture<'a> {
        Box::pin(async {
            Err(GovernanceScorerError::new(
                GovernanceScorerErrorKind::Unavailable,
                "test backend unavailable",
            ))
        })
    }
}

#[tokio::test]
async fn timeout_and_unavailable_scorers_fail_to_degraded_non_passing_states() {
    let task = Task::new("deploy without verified evidence", "deployment");
    let request = GovernanceScoreRequest::new(task.clone(), "bacon");
    let timeout = arda_governance::score_governance_with_timeout(
        &SlowScorer,
        request,
        Duration::from_millis(1),
    )
    .await;
    assert_eq!(timeout.state, GovernanceScorerState::Timeout);
    assert_eq!(timeout.score, 0.0);

    let unavailable = arda_governance::score_governance_with_timeout(
        &ErrorScorer,
        GovernanceScoreRequest::new(task.clone(), "bacon"),
        Duration::from_millis(100),
    )
    .await;
    assert_eq!(unavailable.state, GovernanceScorerState::Unavailable);
    assert_eq!(unavailable.score, 0.0);

    let policy = load_realm_policy_from_str(include_str!("fixtures/realm_policy_operations.toml"))
        .expect("operations policy");
    let verdict = evaluate_realm_governance(
        &task,
        &policy,
        "operations",
        "deployment",
        &ErrorScorer,
        Duration::from_millis(100),
    )
    .await
    .expect("valid scope");
    assert!(verdict.degraded);
    assert!(!verdict.passed);
}

#[test]
fn policy_reload_is_versioned_audited_and_atomic_on_invalid_input() {
    let initial = load_realm_policy_from_str(include_str!("fixtures/realm_policy_research.toml"))
        .expect("initial policy");
    let store = RealmPolicyStore::new(initial).expect("validated initial policy");

    let applied = store.reload_from_str(
        "tests/fixtures/realm_policy_operations.toml",
        include_str!("fixtures/realm_policy_operations.toml"),
    );
    assert_eq!(applied.status, RealmPolicyReloadStatus::Applied);
    assert_eq!(applied.previous_policy_version, "phase8-research-v1");
    assert_eq!(
        applied.proposed_policy_version.as_deref(),
        Some("phase8-operations-v1")
    );
    assert_eq!(store.snapshot().policy_version, "phase8-operations-v1");

    let rejected = store.reload_from_str("invalid.toml", "schema_version = [");
    assert_eq!(rejected.status, RealmPolicyReloadStatus::Rejected);
    assert_eq!(store.snapshot().policy_version, "phase8-operations-v1");
    assert!(!rejected.reason.is_empty());
}

#[cfg(feature = "llm-scorer")]
mod llm_feature {
    use super::*;
    use arda_governance::{
        LlmGovernanceScorer, LlmGovernanceScorerConfig, LlmScoreBackend, LlmScoreBackendFuture,
        LlmScoreResponse,
    };

    struct FakeBackend;

    impl LlmScoreBackend for FakeBackend {
        fn score<'a>(&'a self, _request: GovernanceScoreRequest) -> LlmScoreBackendFuture<'a> {
            Box::pin(async {
                Ok(LlmScoreResponse {
                    score: 0.8,
                    provenance: "fixture://llm-score".to_string(),
                })
            })
        }
    }

    #[tokio::test]
    async fn llm_scorer_is_gated_receipted_cached_and_rejects_stale_cache() {
        let config = LlmGovernanceScorerConfig {
            enabled: true,
            provider: "fixture-provider".to_string(),
            model: "fixture-model".to_string(),
            cache_ttl: Duration::from_millis(20),
            reproducibility_limits: vec!["provider output is not bit reproducible".to_string()],
        };
        let scorer = LlmGovernanceScorer::new(config, FakeBackend);
        let request =
            GovernanceScoreRequest::new(Task::new("evaluate evidence", "governance"), "bacon");

        let first = scorer.score(request.clone()).await.expect("first score");
        assert_eq!(first.provider, "fixture-provider");
        assert_eq!(first.model, "fixture-model");
        assert_eq!(first.state, GovernanceScorerState::Complete);

        let cached = scorer.score(request.clone()).await.expect("cached score");
        assert_eq!(
            cached.cache_status,
            arda_governance::GovernanceScoreCacheStatus::Hit
        );

        tokio::time::sleep(Duration::from_millis(30)).await;
        let stale = scorer.score(request).await.expect("stale receipt");
        assert_eq!(stale.state, GovernanceScorerState::StaleCache);
        assert_eq!(stale.score, 0.0);

        let disabled = LlmGovernanceScorer::new(
            LlmGovernanceScorerConfig {
                enabled: false,
                provider: "fixture-provider".to_string(),
                model: "fixture-model".to_string(),
                cache_ttl: Duration::from_secs(1),
                reproducibility_limits: vec!["fixture".to_string()],
            },
            FakeBackend,
        );
        let disabled_error = disabled
            .score(GovernanceScoreRequest::new(
                Task::new("disabled scorer", "governance"),
                "bacon",
            ))
            .await
            .expect_err("configuration gate must disable LLM scoring");
        assert_eq!(disabled_error.kind, GovernanceScorerErrorKind::Unavailable);
    }
}
