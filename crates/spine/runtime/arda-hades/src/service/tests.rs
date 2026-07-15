#![allow(clippy::await_holding_lock)]

use super::{append_jsonl, read_sigil, HadesService};
use crate::types::{
    ActionKind, HumanLifecycleReviewItem, QuorumProof, SigilState, SigilVacuumRule, TaskItem,
};
use arda_core::try_run_bounded_async;
use arda_plutus::PlutusService;
use chrono::{Duration, Utc};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::{tempdir, tempdir_in};
use tokio::sync::Notify;

// Tests that mutate process environment variables must serialize across await
// points so no other test observes a partially configured runtime. This is
// test-scaffolding only; production code must not hold std mutex guards across
// async boundaries.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn sweep_discovers_orphan_and_status_updates() {
    let base = std::env::current_dir().expect("cwd");
    let dir = tempdir_in(base).expect("tempdir in workspace");
    let plutus_home = dir.path().join("plutus");
    // SAFETY: warden-owned by `arda-hades` test scaffolding — single-threaded
    // test process with no concurrent env reader at this point.
    unsafe {
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
    }
    let svc = HadesService::new(dir.path()).expect("service");
    let target = dir.path().join("mystery.txt");
    fs::write(&target, "no sigil").expect("write");
    let sweep = svc
        .sweep("manual", Some(&dir.path().display().to_string()))
        .expect("sweep");
    assert!(sweep.orphans_found >= 1);
    let status = svc.status().expect("status");
    assert!(status.pending_actions >= 1);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let plutus = PlutusService::from_home(&plutus_home).expect("plutus");
    let mut total = 0.0;
    for _ in 0..20 {
        total = rt.block_on(plutus.status()).expect("plutus status")["joulework"]["total"]
            .as_f64()
            .unwrap_or(0.0);
        if total > 0.0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(total > 0.0);
    // SAFETY: warden-owned by `ARDA-hades` test scaffolding — single-threaded
    // test process with no concurrent env reader at this point.
    unsafe {
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }
}

#[test]
fn notify_warden_defaults_to_hades_runtime_root_not_crate_local_data() {
    let _guard = env_guard();
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    // SAFETY: warden-owned by `ARDA-hades` test scaffolding — single-threaded
    // test process with no concurrent env reader at this point.
    unsafe {
        std::env::remove_var("ARDA_WARDEN_QUEUE_PATH");
    }

    let target = dir.path().join("runtime-orphan.txt");
    fs::write(&target, "orphan").expect("target");
    let out = svc.notify_warden("orphan_found", &target).expect("notify");

    assert_eq!(out["global_queue_written"], true);
    assert!(dir.path().join("warden/informant_queue.jsonl").exists());
    assert!(dir
        .path()
        .join("warden/informant_queue.jsonl")
        .starts_with(dir.path()));
}

#[test]
fn queue_remove_writes_task() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let proof = QuorumProof {
        approvers: vec!["aurelius".to_string(), "bacon".to_string()],
        evidence: vec!["ticket:test-1".to_string()],
        asserted_at_utc: None,
    };
    let out = svc
        .queue_remove_with_proof("/tmp/does_not_exist.jsonl", "orchestrator", Some(proof))
        .expect("queue");
    assert!(out.task_id.starts_with("hds_"));
    let queue = svc.queue(10).expect("queue");
    assert!(!queue.is_empty());
}

#[test]
fn queue_remove_denies_without_quorum() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let err = svc
        .queue_remove("/tmp/does_not_exist.jsonl", "orchestrator")
        .expect_err("expected quorum denial");
    assert!(err.to_string().contains("destructive quorum denied"));
}

#[tokio::test]
async fn sweep_sheds_excess_burst_work() {
    let _guard = env_guard();
    let dir = tempdir().expect("tempdir");
    std::env::set_var("ARDA_HADES_SWEEP_MAX_CONCURRENCY", "1");
    let svc = HadesService::new(dir.path()).expect("service");
    let acquired = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let holder_acquired = Arc::clone(&acquired);
    let holder_release = Arc::clone(&release);

    let holder = tokio::spawn(async move {
        loop {
            let acquired = Arc::clone(&holder_acquired);
            let release = Arc::clone(&holder_release);
            let result = try_run_bounded_async("hades_sweep", 1, || async move {
                acquired.notify_waiters();
                release.notified().await;
            })
            .await;
            if result.is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
    });
    acquired.notified().await;

    let err = svc
        .sweep("manual", Some(&dir.path().display().to_string()))
        .expect_err("saturated");
    assert!(err.to_string().contains("sweep concurrency gate saturated"));

    release.notify_waiters();
    holder.await.expect("holder");
    std::env::remove_var("ARDA_HADES_SWEEP_MAX_CONCURRENCY");
}

#[test]
fn read_sigil_supports_frontmatter_and_toml() {
    let dir = tempdir().expect("tempdir");
    let md = dir.path().join("note.md");
    let toml = dir.path().join("policy.toml");
    let rs = dir.path().join("mod.rs");
    let sh = dir.path().join("job.sh");

    fs::write(
        &md,
        r#"---
soterion:
  sigil: COIN
---
content
"#,
    )
    .expect("write md");
    fs::write(&toml, r#"sigil = "REPAIR""#).expect("write toml");
    fs::write(&rs, "// sigil: ANKH\nfn main() {}\n").expect("write rs");
    fs::write(&sh, "# sigil: SCROLL\necho ok\n").expect("write sh");

    assert!(matches!(read_sigil(&md), Some(SigilState::Coin)));
    assert!(matches!(read_sigil(&toml), Some(SigilState::Repair)));
    assert!(matches!(read_sigil(&rs), Some(SigilState::Ankh)));
    assert!(matches!(read_sigil(&sh), Some(SigilState::Scroll)));
}

#[test]
fn sigil_match_filters_jsonl_by_regex_and_retention() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let path = dir.path().join("charon_state.jsonl");
    append_jsonl(
        &path,
        &serde_json::json!({
            "event": "provider_result",
            "payload": {
                "provider_id": "edge_hub_3080",
                "soterion": {
                    "sigil_code": "SG_ROUTE_FAILOVER",
                    "sigil_tags": ["routing", "fallback"],
                    "sigil_retention": "summarize",
                    "sigil_source": "charon"
                }
            }
        }),
    )
    .expect("append failover");
    append_jsonl(
        &path,
        &serde_json::json!({
            "event": "provider_result",
            "payload": {
                "provider_id": "edge_hub_3080",
                "soterion": {
                    "sigil_code": "SG_ROUTE_EDGE_DOWN",
                    "sigil_tags": ["routing", "edge"],
                    "sigil_retention": "keep",
                    "sigil_source": "charon"
                }
            }
        }),
    )
    .expect("append down");

    let matches = svc
        .sigil_match(
            &path.display().to_string(),
            &SigilVacuumRule {
                code_regex: Some("^SG_ROUTE_".to_string()),
                retention: Some("summarize".to_string()),
                tag: Some("fallback".to_string()),
                source: Some("charon".to_string()),
            },
            10,
        )
        .expect("sigil match");

    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0]["payload"]["soterion"]["sigil_code"]
            .as_str()
            .unwrap_or(""),
        "SG_ROUTE_FAILOVER"
    );
}

#[test]
fn read_sigil_supports_live_config_contract_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf();
    let hermes_bridge = root.join("config/hermes_agent_bridge.toml");
    let arda_settings = root.join("config/arda_hud.settings.json");
    let litellm_proxy = root.join("config/litellm.proxy.yaml");

    assert!(matches!(
        read_sigil(&hermes_bridge),
        Some(SigilState::Scroll)
    ));
    assert!(matches!(
        read_sigil(&arda_settings),
        Some(SigilState::Scroll)
    ));
    assert!(matches!(
        read_sigil(&litellm_proxy),
        Some(SigilState::Scroll)
    ));
}

#[test]
fn status_reports_malformed_record_counts() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    fs::write(
        dir.path().join("action_queue.jsonl"),
        "{\"task_id\":\"h1\",\"queued_at_utc\":\"2026-03-09T11:00:00Z\",\"action\":\"remove\",\"file\":\"alpha\",\"authorized_by\":\"orchestrator\",\"reason\":\"test\",\"execute_after_utc\":null,\"quorum_proof\":null}\n{bad\n",
    )
    .expect("queue write");
    fs::write(
        dir.path().join("hades_log.jsonl"),
        "{\"ts\":\"2026-03-09T11:00:00Z\",\"event\":\"repair_detected\",\"file\":\"alpha\",\"details\":{}}\n{bad\n",
    )
    .expect("log write");
    fs::write(
        dir.path().join("joulework.jsonl"),
        "{\"ts_utc\":\"2026-03-09T11:00:00Z\",\"component\":\"hades\"}\n{bad\n",
    )
    .expect("joulework write");
    fs::write(
        dir.path().join("warden_queue.jsonl"),
        "{\"event\":\"handoff\"}\n{bad\n",
    )
    .expect("warden queue write");
    fs::write(
        dir.path().join("athena_handoff_queue.jsonl"),
        "{\"status\":\"queued_fallback\"}\n{bad\n",
    )
    .expect("athena handoff write");
    let status = svc.status().expect("status");
    assert_eq!(status.malformed_queue_records, 1);
    assert_eq!(status.malformed_log_records, 1);
    assert_eq!(status.malformed_joulework_records, 1);
    assert_eq!(status.malformed_warden_queue_records, 1);
    assert_eq!(status.malformed_athena_handoff_records, 1);
}

#[test]
fn notify_warden_marks_low_value_repair_as_observed() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let target = Path::new("core/metrics/history/20260313T000000Z/hades/joulework.jsonl");

    svc.notify_warden("repair_detected", target)
        .expect("warden notify");

    let contents = fs::read_to_string(dir.path().join("warden_queue.jsonl")).expect("queue");
    let record: serde_json::Value =
        serde_json::from_str(contents.lines().last().expect("latest queue line")).expect("json");
    assert_eq!(record["event_type"], "repair_detected");
    assert_eq!(record["source"], "repair_pipeline_low_value");
    assert_eq!(record["status"], "observed");
    assert_eq!(record["severity"], "info");
    assert_eq!(record["repair_class"], "metrics_history");
    assert_eq!(record["synced"], true);
}

#[test]
fn append_jsonl_serializes_concurrent_writers() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("concurrent.jsonl");
    let mut threads = Vec::new();
    for idx in 0..12 {
        let path = path.clone();
        threads.push(std::thread::spawn(move || {
            append_jsonl(&path, &serde_json::json!({"idx": idx})).expect("append");
        }));
    }
    for thread in threads {
        thread.join().expect("thread join");
    }
    let content = fs::read_to_string(&path).expect("read");
    let lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(lines, 12);
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line).expect("valid json");
    }
}

#[test]
fn pending_removals_archive_human_context_files() {
    let _guard = env_guard();
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let human_dir = dir.path().join("human");
    fs::create_dir_all(&human_dir).expect("human dir");
    let note = human_dir.join("note.md");
    fs::write(&note, "# sigil: REPAIR\nhuman context\n").expect("write note");

    append_jsonl(
        &svc.queue_path,
        &TaskItem {
            task_id: "hds_test_archive".to_string(),
            queued_at_utc: Utc::now().to_rfc3339(),
            action: ActionKind::Remove,
            file: note.display().to_string(),
            authorized_by: Some("orchestrator".to_string()),
            reason: "test".to_string(),
            execute_after_utc: Some((Utc::now() - Duration::minutes(1)).to_rfc3339()),
            quorum_proof: Some(QuorumProof {
                approvers: vec!["aurelius".to_string(), "bacon".to_string()],
                evidence: vec!["ticket:test-archive".to_string()],
                asserted_at_utc: None,
            }),
        },
    )
    .expect("queue task");

    let removed = svc.process_pending_removals().expect("process");
    assert_eq!(removed, 1);
    assert!(!note.exists());
    let archived = svc
        .archive_root
        .join(Utc::now().format("%Y-%m-%d").to_string())
        .join("note.md");
    assert!(archived.exists());
}

#[test]
fn pending_removals_hold_on_soterion_scope_conflict() {
    let _guard = env_guard();
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let human_dir = dir.path().join("human");
    fs::create_dir_all(&human_dir).expect("human dir");
    let note = human_dir.join("danger.md");
    fs::write(&note, "# sigil: COIN\nhuman context\n").expect("write note");

    append_jsonl(
        &svc.queue_path,
        &TaskItem {
            task_id: "hds_test_hold".to_string(),
            queued_at_utc: Utc::now().to_rfc3339(),
            action: ActionKind::Remove,
            file: note.display().to_string(),
            authorized_by: Some("orchestrator".to_string()),
            reason: "test".to_string(),
            execute_after_utc: Some((Utc::now() - Duration::minutes(1)).to_rfc3339()),
            quorum_proof: Some(QuorumProof {
                approvers: vec!["aurelius".to_string(), "bacon".to_string()],
                evidence: vec!["ticket:test-hold".to_string()],
                asserted_at_utc: None,
            }),
        },
    )
    .expect("queue task");

    let removed = svc.process_pending_removals().expect("process");
    assert_eq!(removed, 0);
    assert!(note.exists());
    let queue = svc.queue(10).expect("queue");
    assert_eq!(queue.len(), 1);
    let log = svc
        .log(20, Some("soterion_consistency_hold"), None)
        .expect("log");
    assert_eq!(log.len(), 1);
    assert_eq!(
        log[0].details["memory_scope"].as_str(),
        Some("human_context")
    );
}

#[test]
fn imports_human_scan_records_into_lifecycle_review_queue_without_destructive_action() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    let input_path = dir.path().join("human_ingestion_results.jsonl");
    fs::write(
        &input_path,
        concat!(
            "{\"contract\":\"arda.human_ingestion_result.v1\",",
            "\"source_path\":\"human/example.md\",",
            "\"content_hash\":\"sha256:test\",",
            "\"detected_status\":\"working\",",
            "\"detected_authority\":\"human\",",
            "\"source_type\":\"note\",",
            "\"affected_agents\":[\"mnemosyne\"],",
            "\"affected_paths\":[\"human/\"],",
            "\"summary\":\"Example note\",",
            "\"conflicts\":[],",
            "\"recommendation\":\"retain-working\",",
            "\"review_required\":true,",
            "\"frontmatter_valid\":false,",
            "\"missing_frontmatter_keys\":[\"arda_contract\"],",
            "\"generated_at_utc\":\"2026-05-14T00:00:00Z\"}\n"
        ),
    )
    .expect("write input");

    let report = svc
        .import_human_lifecycle_reviews(&input_path, 20)
        .expect("import human reviews");
    assert_eq!(report.scanned_total, 1);
    assert_eq!(report.queued_total, 1);
    assert_eq!(report.skipped_total, 0);
    assert_eq!(report.malformed_total, 0);

    let queue_content = fs::read_to_string(dir.path().join("athena_handoff_queue.jsonl"))
        .expect("read handoff queue");
    let review: HumanLifecycleReviewItem =
        serde_json::from_str(queue_content.lines().next().expect("review line"))
            .expect("parse review");
    assert_eq!(review.contract, "arda.hades.human_lifecycle_review.v1");
    assert_eq!(review.source_path, "human/example.md");
    assert_eq!(review.lifecycle_action, "review_required");
    assert!(review.review_required);
    assert!(!review.destructive_allowed);
    assert_eq!(
        review.evidence["safety"]["deletes_files"].as_bool(),
        Some(false)
    );
    assert_eq!(
        review.evidence["safety"]["promotes_to_canonical"].as_bool(),
        Some(false)
    );
    assert_eq!(review.severity, "medium");
}

#[test]
fn lifecycle_audit_detects_stale_plan_archive_candidate_and_task_hygiene_without_delete() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let root = dir.path().join("workspace");
    let plans = root.join("docs/plans");
    let human = root.join("human");
    let tasks = root.join("core/projects/tasks");
    fs::create_dir_all(&plans).expect("plans dir");
    fs::create_dir_all(&human).expect("human dir");
    fs::create_dir_all(&tasks).expect("tasks dir");

    let stale_plan = plans.join("stale-roadmap.md");
    fs::write(
        &stale_plan,
        "# Stale roadmap\n\nStatus: in progress\n\nLast updated: 2024-01-01\n\n- pending migration\n",
    )
    .expect("write stale plan");
    let archive_candidate = human.join("obsolete-note.md");
    fs::write(
        &archive_candidate,
        "---\nstatus: superseded\n---\nThis note is obsolete and should be archived after approval.\n",
    )
    .expect("write archive candidate");
    fs::write(
        tasks.join("queue.jsonl"),
        concat!(
            "{\"task_id\":\"tsk_dup\",\"owner\":\"hades\",\"status\":\"pending\"}\n",
            "{\"task_id\":\"tsk_dup\",\"owner\":\"hades\",\"status\":\"pending\"}\n",
            "{\"task_id\":\"tsk_no_owner\",\"status\":\"pending\"}\n",
            "{bad\n"
        ),
    )
    .expect("write task queue");

    let report = svc
        .audit_lifecycle_review(&root, 100)
        .expect("audit lifecycle review");

    assert_eq!(report.contract, "arda.hades.lifecycle_audit_report.v1");
    assert!(report.no_delete);
    assert_eq!(report.stale_plan_total, 1);
    assert_eq!(report.archive_candidate_total, 1);
    assert_eq!(report.task_queue_hygiene_total, 3);
    assert_eq!(report.findings_total, 5);
    assert!(stale_plan.exists());
    assert!(archive_candidate.exists());
    assert!(report
        .findings
        .iter()
        .all(|finding| finding.review_required && !finding.destructive_allowed));
    assert!(report.findings.iter().any(|finding| {
        finding.finding_type == "stale_plan" && finding.recommendation == "review-plan-authority"
    }));
    assert!(report.findings.iter().any(|finding| {
        finding.finding_type == "archive_candidate"
            && finding.recommendation == "archive-after-human-approval"
    }));
    assert!(report.findings.iter().any(|finding| {
        finding.finding_type == "task_queue_hygiene"
            && finding.evidence["issue"] == "duplicate_task_id"
    }));
}

#[test]
fn lifecycle_audit_accepts_id_based_task_queue_records_for_projection_repair() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let root = dir.path().join("workspace");
    let tasks = root.join("core/projects/tasks");
    fs::create_dir_all(&tasks).expect("tasks dir");
    fs::write(
        tasks.join("queue.jsonl"),
        concat!(
            "{\"id\":\"l3-id-only\",\"owner\":\"prometheus\",\"status\":\"pending\"}\n",
            "{\"source_record_id\":\"l3-source-only\",\"owner\":\"hades\",\"status\":\"completed\"}\n",
            "{\"id\":\"l3-id-only\",\"owner\":\"prometheus\",\"status\":\"completed\"}\n"
        ),
    )
    .expect("write task queue");

    let report = svc
        .audit_lifecycle_review(&root, 100)
        .expect("audit lifecycle review");

    assert_eq!(report.task_queue_hygiene_total, 1);
    assert_eq!(report.findings_total, 1);
    assert!(report.findings.iter().all(|finding| {
        finding.finding_type == "task_queue_hygiene"
            && finding.evidence["issue"] != "missing_task_id"
    }));
    assert!(report.findings.iter().any(|finding| {
        finding.evidence["issue"] == "duplicate_task_id"
            && finding.evidence["extra"]["task_id"] == "l3-id-only"
    }));
}

#[test]
fn lifecycle_audit_detects_docs_plan_obsolete_as_archive_candidate() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let root = dir.path().join("workspace");
    let plans = root.join("docs/plans");
    fs::create_dir_all(&plans).expect("plans dir");
    let obsolete_plan = plans.join("obsolete-adoption-plan.md");
    fs::write(
        &obsolete_plan,
        "---\nstatus: obsolete\n---\n# Obsolete adoption plan\n\nSuperseded by the current gate.\n",
    )
    .expect("write obsolete plan");

    let report = svc
        .audit_lifecycle_review(&root, 10)
        .expect("audit lifecycle review");

    let finding = report
        .findings
        .iter()
        .find(|finding| finding.path == "docs/plans/obsolete-adoption-plan.md")
        .expect("obsolete docs plan finding");
    assert_eq!(finding.finding_type, "archive_candidate");
    assert_eq!(finding.lifecycle_class, "archive_candidate");
    assert_eq!(finding.recommendation, "archive-after-human-approval");
    assert!(finding.review_required);
    assert!(!finding.destructive_allowed);
    assert_eq!(
        finding.evidence["evidence_path"],
        "docs/plans/obsolete-adoption-plan.md"
    );
    assert!(obsolete_plan.exists());
}

#[test]
fn lifecycle_approval_packet_keeps_docs_plan_archive_candidate_gated() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let root = dir.path().join("workspace");
    let plans = root.join("docs/plans");
    fs::create_dir_all(&plans).expect("plans dir");
    let superseded_plan = plans.join("superseded-rollout.md");
    fs::write(
        &superseded_plan,
        "# Superseded rollout\n\nStatus: superseded\n\nArchive after approval; current plan is elsewhere.\n",
    )
    .expect("write superseded plan");
    let packet_path = dir.path().join("approval_packet.json");

    let packet = svc
        .lifecycle_operator_approval_packet(&root, 10, &packet_path)
        .expect("approval packet");

    assert_eq!(packet["approval_status"], "pending_operator_review");
    assert_eq!(packet["no_file_moves_or_deletes_performed"], true);
    let candidate = packet["candidates"]
        .as_array()
        .and_then(|candidates| {
            candidates
                .iter()
                .find(|candidate| candidate["path"] == "docs/plans/superseded-rollout.md")
        })
        .expect("docs plan archive candidate");
    assert_eq!(candidate["classification"], "archive_candidate");
    assert_eq!(candidate["recommendation"], "archive-after-human-approval");
    assert_eq!(candidate["operator_decision_required"], true);
    assert_eq!(candidate["destructive_allowed_before_approval"], false);
    assert_eq!(
        candidate["evidence_path"],
        "docs/plans/superseded-rollout.md"
    );
    assert!(superseded_plan.exists());
}

#[test]
fn lifecycle_audit_ignores_active_docs_plan_without_archive_markers() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let root = dir.path().join("workspace");
    let plans = root.join("docs/plans");
    fs::create_dir_all(&plans).expect("plans dir");
    let active_plan = plans.join("active-adoption-plan.md");
    fs::write(
        &active_plan,
        "---\nstatus: active\n---\n# Active adoption plan\n\nCurrent work remains active.\n",
    )
    .expect("write active plan");

    let report = svc
        .audit_lifecycle_review(&root, 10)
        .expect("audit lifecycle review");

    assert!(report
        .findings
        .iter()
        .all(|finding| finding.path != "docs/plans/active-adoption-plan.md"));
    assert!(active_plan.exists());
}

#[test]
fn lifecycle_audit_scans_required_roots_and_appends_findings() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    fs::create_dir_all(dir.path().join("human")).expect("human dir");
    fs::create_dir_all(dir.path().join("docs/plans")).expect("plans dir");
    fs::create_dir_all(dir.path().join("audit/generated")).expect("audit dir");
    fs::create_dir_all(dir.path().join("core/projects/tasks")).expect("tasks dir");
    fs::create_dir_all(dir.path().join("core/state")).expect("state dir");

    let human = dir.path().join("human/old_note.md");
    let plan = dir.path().join("docs/plans/stale.md");
    let generated = dir.path().join("audit/generated/cache.md");
    let state = dir.path().join("core/state/bad.json");
    let queue = dir.path().join("core/projects/tasks/queue.jsonl");
    fs::write(&human, "status: obsolete\narchive after approval").expect("human");
    fs::write(&plan, "status: pending\nstale\nlast updated: 2023").expect("plan");
    fs::write(&generated, "generated-delete-candidate").expect("generated");
    fs::write(&state, "{bad json").expect("state");
    fs::write(
        &queue,
        "{\"task_id\":\"dup\",\"status\":\"pending\",\"owner\":\"prometheus\"}\n{\"task_id\":\"dup\",\"status\":\"pending\",\"owner\":\"\"}\n",
    )
    .expect("queue");

    let report = svc.audit_lifecycle_review(dir.path(), 20).expect("audit");
    assert!(report.no_delete);
    assert!(report.scanned_files_total >= 5);
    let classes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.lifecycle_class.as_str())
        .collect();
    assert!(classes.contains(&"archive_candidate"));
    assert!(classes.contains(&"review"));
    assert!(classes.contains(&"generated_delete_candidate"));
    assert!(classes.contains(&"quarantine_candidate"));
    assert!(report.findings.iter().all(|finding| {
        finding.evidence.get("evidence_path").is_some()
            && finding.evidence.get("reason").is_some()
            && !finding.destructive_allowed
    }));
    assert!(svc.root.join("lifecycle_findings.jsonl").exists());
    assert!(human.exists());
    assert!(plan.exists());
    assert!(generated.exists());
    assert!(state.exists());
}

#[test]
fn lifecycle_l2_l3_l4_are_gated_and_dry_run_first() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path()).expect("service");
    fs::create_dir_all(dir.path().join("human")).expect("human dir");
    let target = dir.path().join("human/old_note.md");
    fs::write(&target, "status: obsolete\narchive after approval").expect("target");

    let projection = svc
        .project_lifecycle_review_queue(dir.path(), 10)
        .expect("projection");
    assert_eq!(projection["no_delete"], true);
    assert!(svc.root.join("lifecycle_review_queue.jsonl").exists());

    let packet_path = dir.path().join("approval_packet.json");
    let packet = svc
        .lifecycle_operator_approval_packet(dir.path(), 10, &packet_path)
        .expect("packet");
    assert_eq!(packet["approval_status"], "pending_operator_review");
    assert!(packet_path.exists());

    let rollback_path = dir.path().join("rollback.json");
    let dry_run = svc
        .execute_lifecycle_cleanup_plan(&packet_path, false, &rollback_path)
        .expect("dry run");
    assert_eq!(dry_run["executed"], false);
    assert_eq!(dry_run["no_file_moves_or_deletes_performed"], true);
    assert!(rollback_path.exists());
    assert!(target.exists());

    let blocked_apply = svc
        .execute_lifecycle_cleanup_plan(&packet_path, true, &rollback_path)
        .expect("blocked apply");
    assert_eq!(blocked_apply["executed"], false);
    assert_eq!(
        blocked_apply["blocked_reason"],
        "approval_packet_not_approved"
    );
    assert!(target.exists());
}

#[test]
fn organization_audit_and_plan_are_read_only_and_persist_artifacts() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let docs = dir.path().join("docs/operations");
    fs::create_dir_all(&docs).expect("docs dir");
    let note = docs.join("NOTE.md");
    fs::write(&note, "# Note\n\nNo frontmatter yet.\n").expect("note");

    let audit = svc
        .organization_audit_report(dir.path(), 20)
        .expect("organization audit");
    assert_eq!(
        audit["contract"],
        "arda.hades.organization_audit_report.v1"
    );
    assert_eq!(audit["no_delete"], true);
    assert_eq!(audit["apply_default"], false);
    assert_eq!(audit["coin"]["hex"], "0x0001FA99");
    assert!(svc.organization_audit_report_path().exists());
    assert!(note.exists());

    let plan = svc
        .organization_plan_report(dir.path(), Some("docs/operations"), 20)
        .expect("organization plan");
    assert_eq!(plan["contract"], "arda.hades.organization_plan.v1");
    assert_eq!(plan["dry_run"], true);
    assert_eq!(plan["no_delete"], true);
    assert_eq!(plan["apply_default"], false);
    assert!(plan["candidates_total"].as_u64().unwrap_or(0) >= 1);
    assert!(svc.organization_plan_path().exists());
    assert!(svc.organization_plan_queue_path().exists());
    assert!(note.exists());
}

#[test]
fn organization_apply_requires_approved_packet_and_mutates_only_declared_candidates() {
    let dir = tempdir().expect("tempdir");
    let svc = HadesService::new(dir.path().join("hades")).expect("service");
    let docs = dir.path().join("docs/operations");
    fs::create_dir_all(&docs).expect("docs dir");
    let note = docs.join("NOTE.md");
    fs::write(
        &note,
        "---\nauthority: agent_generated\nreview_required: true\n---\n\n# Note\n\nNo frontmatter yet.\n",
    )
    .expect("note");

    let packet_path = dir.path().join("organization_approval_packet.json");
    let packet = svc
        .organization_approval_packet(
            dir.path(),
            Some("docs/operations"),
            10,
            &packet_path,
            "operator",
            false,
        )
        .expect("approval packet");
    assert_eq!(
        packet["contract"],
        "arda.hades.organization_operator_approval_packet.v1"
    );
    assert_eq!(packet["approval_status"], "pending_operator_review");
    assert!(packet_path.exists());

    let blocked = svc
        .execute_organization_apply(&packet_path, dir.path(), true)
        .expect("blocked apply");
    assert_eq!(blocked["executed"], false);
    assert_eq!(blocked["blocked_reason"], "approval_packet_not_approved");
    assert!(!docs.join("README.md").exists());
    assert!(!docs.join("INDEX.md").exists());
    assert_eq!(
        fs::read_to_string(&note).expect("note unchanged"),
        "---\nauthority: agent_generated\nreview_required: true\n---\n\n# Note\n\nNo frontmatter yet.\n"
    );

    let approved_packet_path = dir.path().join("organization_approved_packet.json");
    svc.organization_approval_packet(
        dir.path(),
        Some("docs/operations"),
        10,
        &approved_packet_path,
        "operator",
        true,
    )
    .expect("approved packet");

    let receipt = svc
        .execute_organization_apply(&approved_packet_path, dir.path(), true)
        .expect("apply receipt");
    assert_eq!(
        receipt["contract"],
        "arda.hades.organization_apply_receipt.v1"
    );
    assert_eq!(receipt["executed"], true);
    assert_eq!(receipt["destructive_actions_performed"], false);
    assert!(svc.organization_apply_receipt_path().exists());
    assert!(docs.join("README.md").exists());
    assert!(docs.join("INDEX.md").exists());
    let updated_note = fs::read_to_string(&note).expect("note updated");
    assert!(updated_note.starts_with("---\n"));
    assert!(updated_note.contains("soterion:"));
    assert!(updated_note.contains("glyph: \"📜\""));
    assert!(updated_note.contains("🜏 Soterion: 📜 documentation"));
    assert!(updated_note.contains("# Note"));
}
