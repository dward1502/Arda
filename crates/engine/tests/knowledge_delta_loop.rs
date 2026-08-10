use arda_engine::adapters::{
    GovernedKnowledgeDelta, KnowledgeConsumerOutcome, KnowledgeDeltaError, KnowledgeDeltaLoop,
};
use arda_varda::{EvaluationDecision, ExternalEvaluationReceipt};
use chrono::Utc;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn evaluation(content: &str, decision: EvaluationDecision) -> ExternalEvaluationReceipt {
    let approved = decision == EvaluationDecision::ApprovedSafeLocal;
    ExternalEvaluationReceipt {
        schema_version: "arda.athena.external_evaluation.v1".into(),
        suggestion_id: "suggestion-1".into(),
        dispatch_id: "dispatch-1".into(),
        observation_id: if approved {
            "observation-approved".into()
        } else {
            "observation-quarantined".into()
        },
        normalized_url: "https://example.com/arda-adapter-guidance".into(),
        retrieved_at_utc: Utc::now(),
        content_hash: digest(content),
        decision,
        rationale: "canonical evidence evaluated".into(),
        approval_reference: approved.then(|| "approval:observation-approved".into()),
    }
}

fn delta(content: &str) -> GovernedKnowledgeDelta {
    GovernedKnowledgeDelta {
        delta_id: "delta-adapter-guidance".into(),
        source_reference: "https://example.com/arda-adapter-guidance".into(),
        source_digest: format!("sha256:{}", digest(content)),
        confidence: 0.91,
        scope: "system".into(),
        consumer_id: "planner:capability-composer".into(),
        retention_policy: "retain_until_superseded".into(),
        revocation_operation: "knowledge.delta.revoke".into(),
        content: content.into(),
        correction_of: None,
    }
}

#[test]
fn approved_external_finding_is_promoted_retrieved_and_outcome_receipted() {
    let temp = TempDir::new().unwrap();
    let content = "Prefer the governed project-adapter boundary for bounded external workers.";
    let evaluation = evaluation(content, EvaluationDecision::ApprovedSafeLocal);
    let loop_store = KnowledgeDeltaLoop::new(temp.path()).unwrap();

    let first = loop_store
        .promote(&evaluation, delta(content), Utc::now())
        .unwrap();
    let replay = loop_store
        .promote(&evaluation, delta(content), Utc::now())
        .unwrap();
    assert_eq!(first, replay);
    assert_eq!(loop_store.promotions().unwrap().len(), 1);

    let used = loop_store
        .consume(
            "delta-adapter-guidance",
            "planner:capability-composer",
            "governed project adapter boundary",
            KnowledgeConsumerOutcome::Used,
            "changed selection rationale toward the bounded adapter",
            Utc::now(),
        )
        .unwrap();
    assert!(used.retrieved_memory_id.is_some());
    assert_eq!(loop_store.learning_count().unwrap(), 1);
    assert!(matches!(
        loop_store.consume(
            "delta-adapter-guidance",
            "proactive-cycle",
            "adapter",
            KnowledgeConsumerOutcome::Used,
            "wrong consumer",
            Utc::now(),
        ),
        Err(KnowledgeDeltaError::WrongConsumer { .. })
    ));

    drop(loop_store);
    let restarted = KnowledgeDeltaLoop::new(temp.path()).unwrap();
    assert_eq!(restarted.promotions().unwrap().len(), 1);
    assert_eq!(restarted.outcomes().unwrap().len(), 1);
    assert_eq!(restarted.learning_count().unwrap(), 1);
}

#[test]
fn rejected_finding_is_quarantined_without_promotion_or_learning_claim() {
    let temp = TempDir::new().unwrap();
    let content = "Untrusted external finding";
    let evaluation = evaluation(content, EvaluationDecision::ReviewRequired);
    let loop_store = KnowledgeDeltaLoop::new(temp.path()).unwrap();

    assert!(matches!(
        loop_store.promote(&evaluation, delta(content), Utc::now()),
        Err(KnowledgeDeltaError::EvaluationNotApproved)
    ));
    let receipt = loop_store
        .quarantine_evaluation(
            &evaluation,
            "planner:capability-composer",
            "privacy classification requires review",
            Utc::now(),
        )
        .unwrap();
    assert_eq!(receipt.outcome, KnowledgeConsumerOutcome::Quarantined);
    assert!(loop_store.promotions().unwrap().is_empty());
    assert_eq!(loop_store.outcomes().unwrap().len(), 1);
    assert_eq!(loop_store.learning_count().unwrap(), 0);
}

#[test]
fn source_mismatch_and_missing_delta_metadata_fail_closed() {
    let temp = TempDir::new().unwrap();
    let content = "Canonical content";
    let evaluation = evaluation(content, EvaluationDecision::ApprovedSafeLocal);
    let loop_store = KnowledgeDeltaLoop::new(temp.path()).unwrap();

    let mut mismatched = delta("Different content");
    mismatched.source_digest = format!("sha256:{}", digest("Different content"));
    assert!(matches!(
        loop_store.promote(&evaluation, mismatched, Utc::now()),
        Err(KnowledgeDeltaError::SourceMismatch)
    ));

    let mut missing_consumer = delta(content);
    missing_consumer.consumer_id.clear();
    assert!(matches!(
        loop_store.promote(&evaluation, missing_consumer, Utc::now()),
        Err(KnowledgeDeltaError::MissingField("consumer_id"))
    ));
}
