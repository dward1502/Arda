use arda_engine::adapters::{
    evaluate_nightly_intents, AssimilationError, AssimilationEvidence, AssimilationState,
    AssimilationStore, NightlyEvaluationPolicy, NightlyIntent, NightlyIntentRequest,
};
use chrono::{Duration, TimeZone, Utc};
use std::collections::BTreeSet;
use std::io::Write;
use tempfile::TempDir;

fn evidence_collected() -> AssimilationEvidence {
    AssimilationEvidence {
        canonical_source: Some("https://github.com/NousResearch/hermes-agent.git#b3aa561f".into()),
        license: Some("MIT".into()),
        source_digest: Some(
            "sha256:529a213ed88ab3fe91c29136b0e940a66e2c08fa1e9417ae59cdf74d03a4c9de".into(),
        ),
        sbom_digest: Some(
            "sha256:1ce77070728a7acf603f2b026a73d7da87e6ba118ed3e2bbebe470c7a9d5bab9".into(),
        ),
        changes_dependency: true,
        ..AssimilationEvidence::default()
    }
}

fn proposal_evidence() -> AssimilationEvidence {
    AssimilationEvidence {
        security_classification: Some("bounded-process-reviewed".into()),
        privacy_classification: Some("private-egress-allowlist".into()),
        implementation_comparison: Some(
            "retain mature adapter; native rewrite has no measured benefit".into(),
        ),
        patch_provenance: Some("clean-room:no-native-patch".into()),
        test_receipt: Some("cargo-test:hermes-adapter".into()),
        failure_receipt: Some("fixture:timeout-cancel-denied".into()),
        removal_proof: Some("operation:hermes.adapter.disable".into()),
        rollback_proof: Some("operation:hermes.profile.restore".into()),
        ..AssimilationEvidence::default()
    }
}

#[test]
fn hermes_adapter_first_proof_is_restart_safe_and_retains_arda_authority() {
    let temp = TempDir::new().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 12, 0, 0).unwrap();
    let store = AssimilationStore::new(temp.path());
    store
        .discover("hermes-workbench-need", "hermes-workbench", now)
        .unwrap();
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::EvidenceCollected,
            evidence_collected(),
            now + Duration::seconds(1),
        )
        .unwrap();
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::NeedMatched,
            AssimilationEvidence {
                objective_id: Some("objective:verified-project-work".into()),
                ..AssimilationEvidence::default()
            },
            now + Duration::seconds(2),
        )
        .unwrap();
    for (offset, state) in [AssimilationState::Isolated, AssimilationState::TrialActive]
        .into_iter()
        .enumerate()
    {
        store
            .advance(
                "hermes-workbench-need",
                state,
                AssimilationEvidence::default(),
                now + Duration::seconds(3 + offset as i64),
            )
            .unwrap();
    }
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::Measured,
            AssimilationEvidence {
                usage_receipt: Some("hermes-execution-receipt:fixture-1".into()),
                ..AssimilationEvidence::default()
            },
            now + Duration::seconds(5),
        )
        .unwrap();
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::ProposalReady,
            proposal_evidence(),
            now + Duration::seconds(6),
        )
        .unwrap();
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::AwaitingGovernance,
            AssimilationEvidence::default(),
            now + Duration::seconds(7),
        )
        .unwrap();
    assert!(matches!(
        store.advance(
            "hermes-workbench-need",
            AssimilationState::Accepted,
            AssimilationEvidence::default(),
            now + Duration::seconds(8),
        ),
        Err(AssimilationError::MissingEvidence("approval_reference"))
    ));
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::Accepted,
            AssimilationEvidence {
                approval_reference: Some("operator-approval:retain-hermes".into()),
                ..AssimilationEvidence::default()
            },
            now + Duration::seconds(8),
        )
        .unwrap();
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::AdapterRetained,
            AssimilationEvidence::default(),
            now + Duration::seconds(9),
        )
        .unwrap();
    store
        .advance(
            "hermes-workbench-need",
            AssimilationState::Verified,
            AssimilationEvidence::default(),
            now + Duration::seconds(10),
        )
        .unwrap();
    drop(store);

    let restarted = AssimilationStore::new(temp.path());
    let candidate = restarted
        .load_all()
        .unwrap()
        .remove("hermes-workbench-need")
        .unwrap();
    assert_eq!(candidate.state, AssimilationState::Verified);
    assert_eq!(candidate.adapter_id, "hermes-workbench");
    assert_eq!(candidate.transition_count, 10);
}

#[test]
fn invalid_transition_and_corrupt_restart_fail_closed() {
    let temp = TempDir::new().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 12, 0, 0).unwrap();
    let store = AssimilationStore::new(temp.path());
    store.discover("candidate", "adapter", now).unwrap();
    assert!(matches!(
        store.advance(
            "candidate",
            AssimilationState::Measured,
            AssimilationEvidence::default(),
            now,
        ),
        Err(AssimilationError::InvalidTransition { .. })
    ));
    std::fs::OpenOptions::new()
        .append(true)
        .open(store.ledger_path())
        .unwrap()
        .write_all(b"{bad-tail\n")
        .unwrap();
    assert!(matches!(
        store.load_all(),
        Err(AssimilationError::CorruptEntry { .. })
    ));
}

#[test]
fn nightly_routine_allows_evaluation_but_denies_authority_expansion() {
    let policy = NightlyEvaluationPolicy {
        approved_sources: BTreeSet::from(["https://github.com/NousResearch/hermes-agent".into()]),
        allow_bounded_patch: true,
        allow_tests: true,
    };
    let mut requests = vec![
        NightlyIntentRequest {
            intent: NightlyIntent::RefreshApprovedSource,
            source: Some("https://github.com/NousResearch/hermes-agent".into()),
        },
        NightlyIntentRequest {
            intent: NightlyIntent::CompareMeasuredGap,
            source: None,
        },
        NightlyIntentRequest {
            intent: NightlyIntent::RunIsolatedReadOnlyFixture,
            source: None,
        },
        NightlyIntentRequest {
            intent: NightlyIntent::GenerateReport,
            source: None,
        },
        NightlyIntentRequest {
            intent: NightlyIntent::PrepareBoundedPatch,
            source: None,
        },
        NightlyIntentRequest {
            intent: NightlyIntent::RunTests,
            source: None,
        },
        NightlyIntentRequest {
            intent: NightlyIntent::PrepareAdoptionProposal,
            source: None,
        },
    ];
    requests.extend(
        [
            NightlyIntent::ScrapeArbitraryCode,
            NightlyIntent::InstallDependency,
            NightlyIntent::ExpandNetworkOrSecretAccess,
            NightlyIntent::MutatePrivateData,
            NightlyIntent::PromoteObservationToTask,
            NightlyIntent::MergeConsequentialPatch,
        ]
        .into_iter()
        .map(|intent| NightlyIntentRequest {
            intent,
            source: None,
        }),
    );

    let plan = evaluate_nightly_intents(&policy, &requests);
    assert_eq!(plan.allowed.len(), 7);
    assert_eq!(plan.denied.len(), 6);
    assert!(plan
        .denied
        .iter()
        .all(|(_, reason)| reason.contains("authority")));
}
