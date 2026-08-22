use arda_engine::adapters::PlacementLearningStore;
use arda_vaire::{
    ContextDisposition, ContextUseReceipt, GovernedKnowledgeReceipt, MnemosyneService,
    CONTEXT_USE_RECEIPT_SCHEMA_VERSION, GOVERNED_KNOWLEDGE_SCHEMA_VERSION,
};
use arda_varda::outcome_learning::{
    evaluate_outcome_learning, OutcomeLearningDecision, OutcomeLearningEvidence,
};
use chrono::{TimeZone, Utc};
use serde_json::json;

fn use_receipt() -> ContextUseReceipt {
    let mut receipt = ContextUseReceipt {
        schema_version: CONTEXT_USE_RECEIPT_SCHEMA_VERSION.into(),
        receipt_id: "context-use:stage4".into(),
        receipt_digest: String::new(),
        capsule_id: "capsule:stage4".into(),
        capsule_digest: "sha256:capsule-stage4".into(),
        objective_id: "objective-stage4".into(),
        run_id: Some("run-stage4".into()),
        consumer_id: "worker-beelink-reassigned".into(),
        purpose: "resume objective after node reassignment".into(),
        memory_refs: vec![
            "memory-user-correction".into(),
            "memory-project-state".into(),
        ],
        recorded_at_unix_ms: 1_000,
        expires_at_unix_ms: 10_000,
    };
    receipt.receipt_digest = receipt.computed_digest().unwrap();
    receipt
}

fn evidence() -> OutcomeLearningEvidence {
    OutcomeLearningEvidence {
        learning_id: "learning-stage4-route".into(),
        objective_id: "objective-stage4".into(),
        task_kind: "context_recovery".into(),
        role: "worker".into(),
        node_id: "beelink".into(),
        provider_id: "edge_beelink_light".into(),
        model_id: "stage4-model".into(),
        terminal_receipt_refs: vec!["arda://varda/evidence/stage4-terminal".into()],
        acceptance_conditions: vec![
            "reassigned worker resumes from governed context".into(),
            "operator correction remains current".into(),
        ],
        satisfied_conditions: vec![
            "reassigned worker resumes from governed context".into(),
            "operator correction remains current".into(),
        ],
        proposed_score_adjustment_millionths: -125_000,
    }
}

#[test]
fn stage4_context_and_learning_survive_restart_without_duplicate_application() {
    let dir = tempfile::tempdir().unwrap();
    let vaire_root = dir.path().join("vaire");
    let workbench = dir.path().join("workbench");

    let service = MnemosyneService::new(&vaire_root).unwrap();
    let use_receipt = use_receipt();
    let outcome = service
        .record_context_outcome(
            &use_receipt,
            "worker-beelink-reassigned",
            ContextDisposition::Used,
            vec!["memory-user-correction".into()],
            vec!["arda://varda/evidence/stage4-terminal".into()],
            "The reassigned worker used the current operator correction; project state was selected but did not influence the terminal result.",
            2_000,
        )
        .unwrap();
    drop(service);

    let reopened = MnemosyneService::new(&vaire_root).unwrap();
    let replayed_outcome = reopened
        .record_context_outcome(
            &use_receipt,
            "worker-beelink-reassigned",
            ContextDisposition::Used,
            vec!["memory-user-correction".into()],
            vec!["arda://varda/evidence/stage4-terminal".into()],
            "The reassigned worker used the current operator correction; project state was selected but did not influence the terminal result.",
            2_000,
        )
        .unwrap();
    assert_eq!(outcome, replayed_outcome);
    assert_eq!(reopened.context_outcome_receipts().unwrap().len(), 1);

    let evidence = evidence();
    let evaluation = evaluate_outcome_learning(&evidence);
    assert_eq!(
        evaluation.decision,
        OutcomeLearningDecision::ApprovedSafeLocal
    );
    let memory_receipt = GovernedKnowledgeReceipt {
        schema_version: GOVERNED_KNOWLEDGE_SCHEMA_VERSION.into(),
        receipt_id: "vaire-stage4-learning".into(),
        delta_id: evidence.learning_id.clone(),
        source_reference: outcome.receipt_id.clone(),
        warden_observation_id: "warden-stage4-observation".into(),
        varda_evaluation_id: evaluation.evaluation_id.clone(),
        approval_reference: "operator-policy:safe-local-placement-learning".into(),
        ingested_at_utc: "2026-08-22T12:00:00Z".into(),
        correction_of: None,
    };
    let applied_at = Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap();
    let store = PlacementLearningStore::new(&workbench);
    let first = store
        .approve(&evidence, &evaluation, &memory_receipt, applied_at)
        .unwrap();
    drop(store);

    let reopened_store = PlacementLearningStore::new(&workbench);
    let replay = reopened_store
        .approve(&evidence, &evaluation, &memory_receipt, applied_at)
        .unwrap();
    assert_eq!(first, replay);
    assert_eq!(reopened_store.receipts().unwrap().len(), 1);
    let (adjustment, sources) = reopened_store
        .adjustment(
            "context_recovery",
            "worker",
            "beelink",
            "edge_beelink_light",
            "stage4-model",
        )
        .unwrap();
    assert_eq!(adjustment, -0.125);
    assert_eq!(sources, vec![first.receipt_id.clone()]);

    if let Ok(path) = std::env::var("STAGE4_EVIDENCE_PATH") {
        let artifact = json!({
            "schema_version": "arda.digital-organism.stage4-proof.v1",
            "objective_id": evidence.objective_id,
            "origin_node": "core",
            "reassigned_node": evidence.node_id,
            "context_use_receipt": use_receipt,
            "context_outcome_receipt": outcome,
            "varda_evaluation": evaluation,
            "vaire_learning_receipt": memory_receipt,
            "placement_learning_receipt": first,
            "restart_replay": {
                "context_outcome_rows": 1,
                "placement_learning_rows": 1,
                "duplicate_application": false,
                "effective_score_adjustment": adjustment,
                "source_receipts": sources
            }
        });
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }
}

#[test]
fn incomplete_terminal_evidence_cannot_change_placement() {
    let dir = tempfile::tempdir().unwrap();
    let mut evidence = evidence();
    evidence.satisfied_conditions.pop();
    let evaluation = evaluate_outcome_learning(&evidence);
    assert_eq!(evaluation.decision, OutcomeLearningDecision::ReviewRequired);
    let memory_receipt = GovernedKnowledgeReceipt {
        schema_version: GOVERNED_KNOWLEDGE_SCHEMA_VERSION.into(),
        receipt_id: "vaire-unapproved".into(),
        delta_id: evidence.learning_id.clone(),
        source_reference: "outcome".into(),
        warden_observation_id: "observation".into(),
        varda_evaluation_id: evaluation.evaluation_id.clone(),
        approval_reference: "review-pending".into(),
        ingested_at_utc: "2026-08-22T12:00:00Z".into(),
        correction_of: None,
    };
    let error = PlacementLearningStore::new(dir.path())
        .approve(&evidence, &evaluation, &memory_receipt, Utc::now())
        .unwrap_err();
    assert!(error.to_string().contains("not approved"));
}
