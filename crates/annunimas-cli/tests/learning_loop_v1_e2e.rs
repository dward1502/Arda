use annunimas_athena::learning::{
    emit_delta_to_root, KnowledgeDelta, KNOWLEDGE_DELTA_RELATIVE_PATH,
};
use annunimas_mnemosyne::{InformantEvent, MnemosyneService};
use annunimas_oracle::{DefaultTruthScorer, TruthScorer};
use annunimas_warden::{DefaultGateScorer, GateScorer};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn learning_loop_v1_fixture_runs_delta_to_arda_without_destructive_side_effects() {
    let temp_dir = tempdir().expect("tempdir");
    let root = temp_dir.path();
    let cleanup_candidate = root.join("tmp/stale-learning-loop-artifact.txt");
    fs::create_dir_all(cleanup_candidate.parent().expect("parent")).expect("create parent");
    fs::write(&cleanup_candidate, "stale but must not be deleted").expect("write candidate");

    let delta = KnowledgeDelta::new(
        "human/library/athena/sources/operator_note.md",
        0.91,
        0.08,
        "Confirmed truth with high confidence: route queue projections should be kept fresh after append-only task closeout.",
        3600,
    );
    emit_delta_to_root(&delta, root).expect("emit athena delta");
    assert_jsonl_len(root.join(KNOWLEDGE_DELTA_RELATIVE_PATH), 1);

    let mnemosyne = MnemosyneService::new(root).expect("mnemosyne service");
    let memory = mnemosyne
        .encode(InformantEvent {
            informant_id: "athena_learning_loop_v1".to_string(),
            crate_name: "athena".to_string(),
            event_type: "knowledge_delta".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: delta.delta_content.clone(),
            confidence_hint: Some(delta.confidence as f64),
            tags: vec![
                "learning_loop_v1".to_string(),
                "knowledge_delta".to_string(),
                "queue_projection".to_string(),
            ],
        })
        .expect("encode memory")
        .expect("memory produced");

    let recalled = mnemosyne
        .recall_relevant("queue projections", 24, Some("athena"), None, 10)
        .expect("recall");
    assert!(
        recalled
            .iter()
            .any(|entry| entry.memory_id == memory.memory_id),
        "encoded memory should be recalled"
    );

    let safe_proposal = json!({
        "schema_version": "annunimas.prometheus.learning_task_proposal.v1",
        "proposal_id": "proposal_queue_projection_refresh",
        "source_memory_id": memory.memory_id,
        "title": "Keep queue projections fresh",
        "proposal": "This safe low risk proposal uses confirmed truth and confidence evidence to refresh queue projections after append-only task closeout.",
        "status": "proposed",
        "risk_class": "low",
    });
    let destructive_proposal = json!({
        "schema_version": "annunimas.prometheus.learning_task_proposal.v1",
        "proposal_id": "proposal_delete_stale_artifact",
        "title": "Delete stale artifact",
        "proposal": "This destructive high risk autonomous proposal would delete a stale artifact without approval.",
        "status": "proposed",
        "risk_class": "high",
        "candidate_path": rel_path(&cleanup_candidate, root),
    });
    let proposal_path = root.join("data/prometheus/learning_task_proposals.jsonl");
    append_jsonl(&proposal_path, &safe_proposal);
    append_jsonl(&proposal_path, &destructive_proposal);
    assert_jsonl_len(&proposal_path, 2);

    let truth_scorer = DefaultTruthScorer::new();
    let gate_scorer = DefaultGateScorer::new();
    let safe_text = safe_proposal
        .get("proposal")
        .and_then(Value::as_str)
        .expect("safe proposal text");
    let destructive_text = destructive_proposal
        .get("proposal")
        .and_then(Value::as_str)
        .expect("destructive proposal text");
    let safe_truth = truth_scorer.score_truth_confidence(safe_text);
    let safe_gate = gate_scorer.score_gate(safe_text);
    let destructive_gate = gate_scorer.score_gate(destructive_text);

    assert!(safe_truth.confidence >= 0.8);
    assert!(
        !safe_gate.gated,
        "safe low-risk proposal should not be gated"
    );
    assert!(
        destructive_gate.gated,
        "destructive proposal must become HADES approval packet"
    );

    let verdict_path = root.join("data/oracle/learning_loop_verdicts.jsonl");
    append_jsonl(
        &verdict_path,
        &json!({
            "schema_version": "annunimas.oracle_warden.learning_loop_verdict.v1",
            "proposal_id": safe_proposal["proposal_id"],
            "truth_confidence": safe_truth.confidence,
            "operational_risk": safe_gate.operational_risk,
            "autonomy_readiness": safe_gate.autonomy_readiness,
            "gated": safe_gate.gated,
        }),
    );
    append_jsonl(
        &verdict_path,
        &json!({
            "schema_version": "annunimas.oracle_warden.learning_loop_verdict.v1",
            "proposal_id": destructive_proposal["proposal_id"],
            "operational_risk": destructive_gate.operational_risk,
            "autonomy_readiness": destructive_gate.autonomy_readiness,
            "gated": destructive_gate.gated,
            "gating_reason": destructive_gate.gating_reason,
        }),
    );
    assert_jsonl_len(&verdict_path, 2);

    let lifecycle_packet_path = root.join("data/hades/learning_loop_lifecycle_packets.jsonl");
    append_jsonl(
        &lifecycle_packet_path,
        &json!({
            "schema_version": "annunimas.hades.learning_loop_lifecycle_packet.v1",
            "packet_id": "hades_packet_delete_stale_artifact",
            "proposal_id": destructive_proposal["proposal_id"],
            "candidate_path": rel_path(&cleanup_candidate, root),
            "requested_action": "delete",
            "approval_required": true,
            "mutation_policy": "proposal_only_no_source_deletion",
        }),
    );
    assert_jsonl_len(&lifecycle_packet_path, 1);
    assert!(
        cleanup_candidate.exists(),
        "HADES packet must not delete source artifact in v1"
    );

    let receipt_path = root.join(format!(
        "audit/chronos-runs/{}/learning_loop_v1_fixture.json",
        Utc::now().format("%Y-%m-%d")
    ));
    write_json(
        &receipt_path,
        &json!({
            "schema_version": "annunimas.chronos.learning_loop_v1_receipt.v1",
            "mode": "read_only_fixture",
            "knowledge_deltas": 1,
            "memories_recalled": recalled.len(),
            "proposals": 2,
            "gated_proposals": 1,
            "destructive_actions_performed": 0,
        }),
    );

    let learning_state_path = root.join("core/state/learning_loop_v1.json");
    write_json(
        &learning_state_path,
        &json!({
            "schema_version": "annunimas.learning_loop_v1.state.v1",
            "latest_cycle": {
                "status": "healthy",
                "knowledge_deltas": 1,
                "memory_recalls": recalled.len(),
                "task_proposals": 2,
                "gated_proposals": 1,
                "chronos_receipt": rel_path(&receipt_path, root),
                "destructive_actions_performed": 0,
            }
        }),
    );
    let arda_path = root.join("core/state/arda_snapshot.json");
    write_json(
        &arda_path,
        &json!({
            "schema_version": "annunimas.arda.snapshot.v1",
            "learning_loop_v1": {
                "status": "healthy",
                "blockers": [],
                "recent_delta_count": 1,
                "proposal_counts": {
                    "total": 2,
                    "gated": 1,
                    "safe_local": 1
                },
                "next_action": "review HADES proposal packet before any cleanup mutation"
            }
        }),
    );

    let arda = read_json(&arda_path);
    assert_eq!(
        arda.pointer("/learning_loop_v1/status"),
        Some(&json!("healthy"))
    );
    assert_eq!(
        arda.pointer("/learning_loop_v1/proposal_counts/gated"),
        Some(&json!(1))
    );
    assert_eq!(
        read_json(&learning_state_path).pointer("/latest_cycle/destructive_actions_performed"),
        Some(&json!(0))
    );
}

fn append_jsonl(path: impl AsRef<Path>, value: &Value) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("open jsonl");
    writeln!(file, "{}", serde_json::to_string(value).expect("serialize")).expect("write jsonl");
}

fn write_json(path: impl AsRef<Path>, value: &Value) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value).expect("serialize json"),
    )
    .expect("write json");
}

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("parse json")
}

fn assert_jsonl_len(path: impl AsRef<Path>, expected: usize) {
    let actual = fs::read_to_string(path)
        .expect("read jsonl")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(actual, expected);
}

fn rel_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}
