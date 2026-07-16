use arda_athena::human::{scan_human_root, HumanIngestionRecord};
use arda_athena::ingest::AthenaStore;
use arda_core::error::Result;
use arda_core::llm::{ChatRequest, ChatResponse, LlmProvider};
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use tempfile::TempDir;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("ATHENA integration-test env lock should not be poisoned")
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set_path(key: &'static str, value: &Path) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var(self.key, previous);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .expect("jsonl fixture should be readable")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("jsonl line should parse as JSON"))
        .collect()
}

struct FixedScoreLlm {
    score: &'static str,
}

#[async_trait]
impl LlmProvider for FixedScoreLlm {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        Ok(ChatResponse {
            content: self.score.to_string(),
            model: "fixed-score-test".to_string(),
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    fn provider_name(&self) -> &str {
        "fixed-score-test"
    }

    fn default_model(&self) -> &str {
        "fixed-score-test"
    }
}

#[test]
fn uncertainty_sampler_uses_llm_score_for_query_matches() {
    let _guard = env_lock();
    let temp = TempDir::new().expect("temp workspace should be created");
    let root = temp.path().join("arda-root");
    let store_root = temp.path().join("athena-store");
    let human_library = root.join("human/library/athena");
    let machine_library = root.join("data/knowledge/athena");
    let hades_queue = root.join("data/hades/action_queue.jsonl");
    let warden_queue = root.join("data/warden/informant_queue.jsonl");
    let bacon_machine = root.join("data/governance/bacon_lite.jsonl");
    let bacon_human = root.join("human/library/governance/bacon_lite.md");

    let _root_env = EnvVarGuard::set_path("ARDA_ROOT", &root);
    let _human_env = EnvVarGuard::set_path("ARDA_VARDA_HUMAN_LIBRARY_ROOT", &human_library);
    let _machine_env =
        EnvVarGuard::set_path("ARDA_VARDA_MACHINE_LIBRARY_ROOT", &machine_library);
    let _hades_env = EnvVarGuard::set_path("ARDA_HADES_ACTION_QUEUE_PATH", &hades_queue);
    let _warden_env = EnvVarGuard::set_path("ARDA_WARDEN_QUEUE_PATH", &warden_queue);
    let _bacon_machine_env = EnvVarGuard::set_path("ARDA_BACON_LITE_LOG_PATH", &bacon_machine);
    let _bacon_human_env = EnvVarGuard::set_path("ARDA_BACON_LITE_HUMAN_PATH", &bacon_human);

    let llm: Arc<dyn LlmProvider> = Arc::new(FixedScoreLlm { score: "0.73" });
    let store = AthenaStore::new(&store_root)
        .expect("AthenaStore should initialize inside tempdir")
        .with_llm(llm);
    let record = store
        .ingest(
            "Uncertainty sampler deterministic active reading note",
            "integration_test",
            "verify uncertainty sampler scoring",
        )
        .expect("fixture ingest should succeed");

    let chunks = store
        .select_uncertain_chunks("deterministic active reading", 4)
        .expect("uncertainty sampler should score query matches");

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].source_id, record.id);
    assert_eq!(chunks[0].uncertainty_score, 0.73);

    let receipt = store
        .select_and_record_uncertain_chunks("deterministic active reading", 4)
        .expect("uncertainty sampler should persist selected chunks");
    assert_eq!(receipt.event, "uncertainty_selection_recorded");
    assert_eq!(receipt.total_selected, 1);
    assert_eq!(receipt.chunks[0].source_id, record.id);

    let records = read_jsonl(store.uncertainty_selections_path());
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].get("query").and_then(|value| value.as_str()),
        Some("deterministic active reading")
    );
    assert_eq!(
        records[0]
            .get("chunks")
            .and_then(|value| value.as_array())
            .and_then(|chunks| chunks.first())
            .and_then(|chunk| chunk.get("uncertainty_score"))
            .and_then(|score| score.as_f64()),
        Some(0.73)
    );
}

#[test]
fn athena_store_ingest_query_and_deep_queue_are_workspace_isolated() {
    let _guard = env_lock();
    let temp = TempDir::new().expect("temp workspace should be created");
    let root = temp.path().join("arda-root");
    let store_root = temp.path().join("athena-store");
    let human_library = root.join("human/library/athena");
    let machine_library = root.join("data/knowledge/athena");
    let hades_queue = root.join("data/hades/action_queue.jsonl");
    let warden_queue = root.join("data/warden/informant_queue.jsonl");
    let bacon_machine = root.join("data/governance/bacon_lite.jsonl");
    let bacon_human = root.join("human/library/governance/bacon_lite.md");

    let _root_env = EnvVarGuard::set_path("ARDA_ROOT", &root);
    let _human_env = EnvVarGuard::set_path("ARDA_VARDA_HUMAN_LIBRARY_ROOT", &human_library);
    let _machine_env =
        EnvVarGuard::set_path("ARDA_VARDA_MACHINE_LIBRARY_ROOT", &machine_library);
    let _hades_env = EnvVarGuard::set_path("ARDA_HADES_ACTION_QUEUE_PATH", &hades_queue);
    let _warden_env = EnvVarGuard::set_path("ARDA_WARDEN_QUEUE_PATH", &warden_queue);
    let _bacon_machine_env = EnvVarGuard::set_path("ARDA_BACON_LITE_LOG_PATH", &bacon_machine);
    let _bacon_human_env = EnvVarGuard::set_path("ARDA_BACON_LITE_HUMAN_PATH", &bacon_human);

    let store =
        AthenaStore::new(&store_root).expect("AthenaStore should initialize inside tempdir");
    assert_eq!(store.root(), store_root.as_path());
    assert!(store.digest_path().starts_with(&store_root));
    assert!(store.deep_queue_path().starts_with(&store_root));
    assert!(
        store.digest_path().exists(),
        "digest jsonl should be initialized"
    );
    assert!(
        store.deep_queue_path().exists(),
        "deep queue jsonl should be initialized"
    );
    assert!(
        hades_queue.exists(),
        "HADES queue should be redirected to temp root"
    );
    assert!(
        warden_queue.exists(),
        "WARDEN queue should be redirected to temp root"
    );

    let source = "Gate3 deterministic knowledge mesh\nThis local note verifies ATHENA ingestion without network, credentials, or live Charon.";
    let record = store
        .ingest(source, "integration_test", "gate3 athena local harness")
        .expect("local raw-note ingest should succeed");
    assert!(
        !record.deduplicated,
        "first ingest should create a new book entry"
    );
    assert_eq!(record.digest_status, "shallow");
    assert!(record
        .book_ref
        .starts_with(&store_root.display().to_string()));
    assert!(record.book_ref.ends_with(".jsonl"));

    let query = store
        .query("Gate3 deterministic knowledge mesh", 4)
        .expect("local query should use deterministic digest index");
    assert_eq!(query.total_matches, 1);
    let matched = query
        .matches
        .first()
        .expect("query should return the ingested source");
    assert_eq!(matched.source_id, record.id);
    assert!(matched.title.contains("Gate3 deterministic knowledge mesh"));

    let queued = store
        .queue_deep_analysis(
            &record.id,
            "integration_test",
            "verify deterministic queue side effects",
        )
        .expect("deep queue event should be appended under tempdir");
    assert_eq!(queued.source_id, record.id);
    assert_eq!(queued.status, "pending_deep");

    let digest_events = read_jsonl(store.digest_path());
    assert!(
        digest_events
            .iter()
            .any(|event| event.get("id") == Some(&Value::String(record.id.clone()))),
        "digest should contain the ingest record"
    );
    assert!(
        digest_events
            .iter()
            .any(|event| event.get("event") == Some(&Value::String("deep_queued".to_string()))),
        "digest should contain the deep queue event"
    );
    assert!(
        bacon_machine.exists(),
        "bacon-lite machine log should stay in temp root"
    );
    assert!(
        bacon_human.exists(),
        "bacon-lite human log should stay in temp root"
    );
    assert!(machine_library.join("index/sources.jsonl").exists());
    assert!(human_library.join("sources").exists());
}

#[test]
fn human_root_scan_classifies_fixture_and_preserves_provenance() {
    let temp = TempDir::new().expect("temp human fixture should be created");
    let human_root = temp.path().join("human");
    let decisions = human_root.join("decisions");
    fs::create_dir_all(&decisions).expect("fixture decisions directory should be created");
    let note_path = decisions.join("mesh-policy.md");
    fs::write(
        &note_path,
        r#"---
arda_contract: human.note.v1
title: Gate 3 Mesh Policy
status: canonical
source_type: decision
authority: human
owner: mythos
created: 2026-05-20
updated: 2026-05-20
supersedes: none
superseded_by: none
affected_agents: ATHENA, PROMETHEUS
affected_paths: crates/arda-varda, crates/arda-prometheus
privacy: internal
review_required: false
confidence: high
sigils: ◈, ↝
---

Gate 3 canonical decision for ATHENA and PROMETHEUS.
"#,
    )
    .expect("fixture note should be written");

    let output_path = temp.path().join("out/human-scan.jsonl");
    let contradictions_path = temp.path().join("out/contradictions.jsonl");
    let report = scan_human_root(&human_root, &output_path, Some(&contradictions_path), None)
        .expect("human-root scan should succeed against temp fixture");

    assert_eq!(report.scanned_total, 1);
    assert_eq!(report.emitted_total, 1);
    assert_eq!(report.contradiction_total, 0);
    assert_eq!(report.human_root, human_root.display().to_string());
    assert_eq!(report.output_path, output_path.display().to_string());

    let records = fs::read_to_string(&output_path).expect("human scan output should be readable");
    let record: HumanIngestionRecord = serde_json::from_str(
        records
            .lines()
            .next()
            .expect("human scan output should contain one record"),
    )
    .expect("human scan record should deserialize");

    assert_eq!(record.source_path, "human/decisions/mesh-policy.md");
    assert_eq!(record.detected_status, "canonical");
    assert_eq!(record.detected_authority, "human");
    assert_eq!(record.source_type, "decision");
    assert!(record.frontmatter_valid);
    assert!(!record.review_required);
    assert!(record.affected_agents.iter().any(|agent| agent == "athena"));
    assert!(record
        .affected_agents
        .iter()
        .any(|agent| agent == "prometheus"));
    assert!(record.affected_paths.iter().any(|path| path == "crates/"));
    assert!(record.content_hash.starts_with("sha256:"));
    assert!(
        contradictions_path.exists(),
        "empty contradiction jsonl should still be created"
    );
    assert_eq!(
        fs::read_to_string(&contradictions_path).expect("contradiction output should be readable"),
        ""
    );
}
