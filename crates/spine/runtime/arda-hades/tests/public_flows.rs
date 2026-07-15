use arda_hades::{ActionKind, HadesService, QuorumProof, TaskItem};
use chrono::{Duration, Utc};
use std::sync::Mutex;
use tempfile::tempdir_in;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn workspace_tempdir() -> tempfile::TempDir {
    let base = std::env::current_dir().expect("cwd");
    tempdir_in(base).expect("tempdir in workspace")
}

#[test]
fn public_remove_flow_queues_destructive_action_with_quorum() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path()).expect("service");
    let target = dir.path().join("artifact.jsonl");
    std::fs::write(&target, "{\"ok\":true}\n").expect("write target");

    let queued = service
        .queue_remove_with_proof(
            target.to_string_lossy().as_ref(),
            "orchestrator",
            Some(QuorumProof {
                approvers: vec!["aurelius".to_owned(), "bacon".to_owned()],
                evidence: vec!["ticket:public-1".to_owned()],
                asserted_at_utc: Some("2026-04-21T00:00:00Z".to_owned()),
            }),
        )
        .expect("queue remove");

    assert_eq!(queued.authorized_by.as_deref(), Some("orchestrator"));
    assert_eq!(queued.file, target.to_string_lossy());

    let queue = service.queue(10).expect("queue state");
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].file, target.to_string_lossy());
}

#[test]
fn public_sweep_flow_discovers_orphan_and_updates_status() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let plutus_home = dir.path().join("plutus");
    // SAFETY: warden-owned by `arda-hades` test scaffolding — single-threaded
    // test process with no concurrent env reader at this point.
    unsafe {
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
    }
    let service = HadesService::new(dir.path()).expect("service");
    let target = dir.path().join("mystery.txt");
    std::fs::write(&target, "no sigil").expect("write target");

    let sweep = service
        .sweep("manual", Some(dir.path().to_string_lossy().as_ref()))
        .expect("sweep");
    assert!(sweep.orphans_found >= 1);

    let status = service.status().expect("status");
    assert!(status.pending_actions >= 1);

    // SAFETY: warden-owned by `arda-hades` test scaffolding — single-threaded
    // test process with no concurrent env reader at this point.
    unsafe {
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }
}

#[test]
fn public_due_remove_flow_archives_human_context_file() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path()).expect("service");
    let human_dir = dir.path().join("human");
    std::fs::create_dir_all(&human_dir).expect("human dir");
    let target = human_dir.join("journal.md");
    std::fs::write(&target, "# sigil: EYE\nkeep the thread intact\n").expect("write target");

    let task = TaskItem {
        task_id: "hds_public_archive".to_owned(),
        queued_at_utc: Utc::now().to_rfc3339(),
        action: ActionKind::Remove,
        file: target.to_string_lossy().into_owned(),
        authorized_by: Some("orchestrator".to_owned()),
        reason: "public archive flow".to_owned(),
        execute_after_utc: Some((Utc::now() - Duration::minutes(5)).to_rfc3339()),
        quorum_proof: Some(QuorumProof {
            approvers: vec!["aurelius".to_owned(), "bacon".to_owned()],
            evidence: vec!["ticket:public-archive".to_owned()],
            asserted_at_utc: Some("2026-04-21T00:00:00Z".to_owned()),
        }),
    };
    let queue_line = serde_json::to_string(&task).expect("serialize task");
    std::fs::write(
        dir.path().join("action_queue.jsonl"),
        format!("{queue_line}\n"),
    )
    .expect("write queue");

    let sweep = service
        .sweep("manual", Some(dir.path().to_string_lossy().as_ref()))
        .expect("sweep");
    assert!(sweep.actions_taken >= 1);
    assert!(!target.exists());

    let archive_root = dir
        .path()
        .join("archive")
        .join(Utc::now().format("%Y-%m-%d").to_string());
    let archived = archive_root.join("journal.md");
    assert!(archived.exists());

    let queue = service.queue(10).expect("queue state");
    assert!(queue
        .iter()
        .all(|item| !(matches!(item.action, ActionKind::Remove)
            && item.file == target.to_string_lossy())));
}

#[test]
fn public_due_remove_flow_holds_conflicting_coin_human_context() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path()).expect("service");
    let human_dir = dir.path().join("human");
    std::fs::create_dir_all(&human_dir).expect("human dir");
    let target = human_dir.join("danger.md");
    std::fs::write(&target, "# sigil: COIN\nprotected destructive conflict\n")
        .expect("write target");

    let task = TaskItem {
        task_id: "hds_public_hold".to_owned(),
        queued_at_utc: Utc::now().to_rfc3339(),
        action: ActionKind::Remove,
        file: target.to_string_lossy().into_owned(),
        authorized_by: Some("orchestrator".to_owned()),
        reason: "public hold flow".to_owned(),
        execute_after_utc: Some((Utc::now() - Duration::minutes(5)).to_rfc3339()),
        quorum_proof: Some(QuorumProof {
            approvers: vec!["aurelius".to_owned(), "bacon".to_owned()],
            evidence: vec!["ticket:public-hold".to_owned()],
            asserted_at_utc: Some("2026-04-21T00:00:00Z".to_owned()),
        }),
    };
    let queue_line = serde_json::to_string(&task).expect("serialize task");
    std::fs::write(
        dir.path().join("action_queue.jsonl"),
        format!("{queue_line}\n"),
    )
    .expect("write queue");

    let sweep = service
        .sweep("manual", Some(dir.path().to_string_lossy().as_ref()))
        .expect("sweep");
    assert!(sweep.files_scanned >= 1);
    assert!(target.exists());

    let queue = service.queue(10).expect("queue state");
    assert!(queue
        .iter()
        .any(|item| matches!(item.action, ActionKind::Remove)
            && item.file == target.to_string_lossy()));

    let log = service.log(20, None, None).expect("log");
    assert!(log
        .iter()
        .any(|entry| entry.event == "soterion_consistency_hold"
            && entry.file.as_deref() == Some(target.to_string_lossy().as_ref())));
}

#[test]
fn lifecycle_policy_automation_summarizes_findings_without_authorizing_cleanup() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let human_dir = dir.path().join("human");
    let audit_dir = dir.path().join("audit/cache");
    let state_dir = dir.path().join("core/state");
    std::fs::create_dir_all(&human_dir).expect("human dir");
    std::fs::create_dir_all(&audit_dir).expect("audit dir");
    std::fs::create_dir_all(&state_dir).expect("state dir");
    std::fs::write(
        human_dir.join("obsolete_context.md"),
        "status: obsolete\narchive after approval\n",
    )
    .expect("write human candidate");
    std::fs::write(
        audit_dir.join("generated-delete-candidate.tmp"),
        "generated-delete-candidate=true\n",
    )
    .expect("write generated candidate");
    std::fs::write(state_dir.join("runtime.json"), "{\"ok\":true}\n").expect("write state");

    let report = service
        .lifecycle_policy_automation_report(dir.path(), 20)
        .expect("policy report");

    assert_eq!(
        report["contract"],
        "arda.hades.lifecycle_policy_automation_report.v1"
    );
    assert_eq!(report["no_delete"], true);
    assert_eq!(report["cleanup_authorized"], false);
    assert_eq!(report["requires_operator_approval_for_mutation"], true);
    assert!(
        report["policy_summary"]["by_disposition"]["archive"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
    assert!(
        report["policy_summary"]["by_disposition"]["remove"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
    assert!(report["policy_items"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .all(|item| item["destructive_allowed"] == false));
    assert!(service
        .log(
            20,
            Some("lifecycle_policy_automation_report_generated"),
            None,
        )
        .expect("log")
        .iter()
        .any(|entry| entry.event == "lifecycle_policy_automation_report_generated"));
}

#[test]
fn warden_hades_operator_review_packet_projects_raw_queue_without_authorizing_mutation() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let warden_dir = dir.path().join("data/warden");
    let human_dir = dir.path().join("human");
    std::fs::create_dir_all(&warden_dir).expect("warden dir");
    std::fs::create_dir_all(&human_dir).expect("human dir");
    let raw_queue_path = warden_dir.join("informant_queue.jsonl");
    let raw_queue = concat!(
        "{\"event_type\":\"orphan_found\",\"severity\":\"warning\",\"file\":\"human/a.md\"}\n",
        "{\"event_type\":\"destructive_quorum_denied\",\"severity\":\"critical\",\"file\":\"human/b.md\"}\n",
        "{\"event_type\":\"repair_detected\",\"severity\":\"info\",\"file\":\"core/state/c.json\"}\n",
    );
    std::fs::write(&raw_queue_path, raw_queue).expect("write raw queue");
    std::fs::write(
        human_dir.join("obsolete_context.md"),
        "status: obsolete\narchive after approval\n",
    )
    .expect("write lifecycle candidate");

    let out_dir = dir.path().join("audit/warden_hades_review");
    let packet = service
        .project_warden_hades_operator_review_packet(dir.path(), &raw_queue_path, 10, &out_dir)
        .expect("operator review packet");

    assert_eq!(
        packet["contract"],
        "arda.hades.warden_operator_review_packet.v1"
    );
    assert_eq!(packet["packet_is_authorization"], false);
    assert_eq!(packet["clear_archive_allowed"], false);
    assert_eq!(packet["delete_allowed"], false);
    assert_eq!(packet["move_allowed"], false);
    assert_eq!(packet["archive_allowed"], false);
    assert_eq!(
        packet["requires_explicit_operator_approval_for_any_mutation"],
        true
    );
    assert_eq!(packet["raw_queue_retained"], true);
    assert_eq!(packet["raw_queue"]["line_count"], 3);
    assert_eq!(
        packet["raw_queue"]["sha256"]
            .as_str()
            .unwrap_or_default()
            .len(),
        64
    );
    assert!(packet["review_items_total"].as_u64().unwrap_or(0) >= 4);
    assert!(
        std::path::Path::new(packet["review_queue_path"].as_str().unwrap_or_default()).exists()
    );
    assert!(
        std::path::Path::new(packet["markdown_summary_path"].as_str().unwrap_or_default()).exists()
    );
    assert_eq!(
        std::fs::read_to_string(&raw_queue_path).expect("read raw queue"),
        raw_queue
    );
    assert!(packet["review_items"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .all(|item| item["approval_status"] == "pending_operator_review"
            && item["outcome"]["status"] == "pending_operator_review"
            && item["outcome"]["append_only_closeout"] == true
            && item["outcome"]["destructive_allowed"] == false
            && item["evidence_artifact"]["contract"]
                == "arda.hades.warden_queue_evidence_artifact.v1"
            && std::path::Path::new(
                item["evidence_artifact"]["path"]
                    .as_str()
                    .unwrap_or_default()
            )
            .exists()
            && item["destructive_allowed"] == false));
}

#[test]
fn warden_hades_signed_approval_packet_selects_review_ids_without_mutating_queues() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let warden_dir = dir.path().join("data/warden");
    std::fs::create_dir_all(&warden_dir).expect("warden dir");
    let raw_queue_path = warden_dir.join("informant_queue.jsonl");
    let raw_queue = concat!(
        "{\"event_type\":\"orphan_found\",\"severity\":\"warning\",\"file\":\"human/a.md\"}\n",
        "{\"event_type\":\"destructive_quorum_denied\",\"severity\":\"critical\",\"file\":\"human/b.md\"}\n",
    );
    std::fs::write(&raw_queue_path, raw_queue).expect("write raw queue");

    let out_dir = dir.path().join("audit/warden_hades_review");
    let review_packet = service
        .project_warden_hades_operator_review_packet(dir.path(), &raw_queue_path, 10, &out_dir)
        .expect("operator review packet");
    let review_packet_path = review_packet["packet_path"]
        .as_str()
        .expect("review packet path")
        .to_owned();
    let review_queue_path = review_packet["review_queue_path"]
        .as_str()
        .expect("review queue path")
        .to_owned();
    let review_queue_before =
        std::fs::read_to_string(&review_queue_path).expect("read review queue");

    let approval_path = dir
        .path()
        .join("audit/warden_hades_review/operator_signed_approval_packet.json");
    let approval = service
        .warden_hades_signed_approval_packet(
            &review_packet_path,
            &["whr_raw_1".to_owned()],
            "operator:test",
            "defer_retain_raw",
            "ticket:test-approval",
            &approval_path,
        )
        .expect("signed approval packet");

    assert_eq!(
        approval["contract"],
        "arda.hades.warden_operator_signed_approval_packet.v1"
    );
    assert_eq!(
        approval["approval_status"],
        "signed_operator_decision_recorded"
    );
    assert_eq!(approval["selected_review_ids_total"], 1);
    assert_eq!(approval["selected_review_ids"][0], "whr_raw_1");
    assert_eq!(approval["operator_id"], "operator:test");
    assert_eq!(approval["operator_decision"], "defer_retain_raw");
    assert_eq!(approval["source_packet_is_authorization"], false);
    assert_eq!(approval["cleanup_authorized"], false);
    assert_eq!(approval["mutation_authorized_without_dry_run"], false);
    assert_eq!(approval["selected_items"][0]["review_id"], "whr_raw_1");
    assert!(approval_path.exists());
    assert_eq!(
        std::fs::read_to_string(&raw_queue_path).expect("read raw queue after approval"),
        raw_queue
    );
    assert_eq!(
        std::fs::read_to_string(&review_queue_path).expect("read review queue after approval"),
        review_queue_before
    );
}

#[test]
fn warden_hades_dry_run_receipt_requires_signed_packet_and_does_not_mutate() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let warden_dir = dir.path().join("data/warden");
    std::fs::create_dir_all(&warden_dir).expect("warden dir");
    let raw_queue_path = warden_dir.join("informant_queue.jsonl");
    let raw_queue = concat!(
        "{\"event_type\":\"orphan_found\",\"severity\":\"warning\",\"file\":\"human/a.md\"}\n",
        "{\"event_type\":\"destructive_quorum_denied\",\"severity\":\"critical\",\"file\":\"human/b.md\"}\n",
    );
    std::fs::write(&raw_queue_path, raw_queue).expect("write raw queue");

    let out_dir = dir.path().join("audit/warden_hades_review");
    let review_packet = service
        .project_warden_hades_operator_review_packet(dir.path(), &raw_queue_path, 10, &out_dir)
        .expect("operator review packet");
    let review_packet_path = review_packet["packet_path"]
        .as_str()
        .expect("review packet path")
        .to_owned();
    let review_queue_path = review_packet["review_queue_path"]
        .as_str()
        .expect("review queue path")
        .to_owned();

    let approval_path = dir
        .path()
        .join("audit/warden_hades_review/operator_signed_approval_packet.json");
    let approval = service
        .warden_hades_signed_approval_packet(
            &review_packet_path,
            &["whr_raw_1".to_owned(), "whr_raw_2".to_owned()],
            "operator:test",
            "defer_retain_raw",
            "ticket:test-approval",
            &approval_path,
        )
        .expect("signed approval packet");
    assert_eq!(approval["authorizes_next_gate"], "dry_run_receipt_only");

    let raw_queue_before = std::fs::read_to_string(&raw_queue_path).expect("read raw before");
    let review_queue_before =
        std::fs::read_to_string(&review_queue_path).expect("read review before");
    let receipt_path = dir
        .path()
        .join("audit/warden_hades_review/dry_run_receipt.json");

    let receipt = service
        .warden_hades_dry_run_receipt(
            &approval_path,
            &review_packet_path,
            "retain_acknowledgement",
            &receipt_path,
        )
        .expect("dry-run receipt");

    assert_eq!(
        receipt["contract"],
        "arda.hades.warden_operator_dry_run_receipt.v1"
    );
    assert_eq!(
        receipt["source_approval_packet_path"],
        approval_path.display().to_string()
    );
    assert_eq!(receipt["source_review_packet_path"], review_packet_path);
    assert_eq!(receipt["selected_review_ids_total"], 2);
    assert_eq!(receipt["selected_review_ids"][0], "whr_raw_1");
    assert_eq!(receipt["selected_review_ids"][1], "whr_raw_2");
    assert_eq!(receipt["intended_action"], "retain_acknowledgement");
    assert_eq!(receipt["dry_run_only"], true);
    assert_eq!(receipt["mutation_performed"], false);
    assert_eq!(receipt["apply_authorized"], false);
    assert_eq!(receipt["cleanup_authorized"], false);
    assert_eq!(receipt["raw_queue_retention_verified"], true);
    assert_eq!(receipt["review_queue_retention_verified"], true);
    assert_eq!(receipt["rollback_plan_required"], true);
    assert_eq!(receipt["no_file_moves_or_deletes_performed"], true);
    assert!(
        receipt["source_approval_packet_sha256"]
            .as_str()
            .expect("approval hash")
            .len()
            == 64
    );
    assert!(
        receipt["source_review_packet_sha256"]
            .as_str()
            .expect("review hash")
            .len()
            == 64
    );
    assert!(receipt_path.exists());
    assert_eq!(
        std::fs::read_to_string(&raw_queue_path).expect("read raw after dry-run"),
        raw_queue_before
    );
    assert_eq!(
        std::fs::read_to_string(&review_queue_path).expect("read review after dry-run"),
        review_queue_before
    );

    let missing_receipt = dir
        .path()
        .join("audit/warden_hades_review/missing_approval_receipt.json");
    let missing = service.warden_hades_dry_run_receipt(
        dir.path().join("missing_approval_packet.json"),
        &review_packet_path,
        "retain_acknowledgement",
        &missing_receipt,
    );
    assert!(missing.is_err());
    assert!(!missing_receipt.exists());
}

#[test]
fn warden_hades_mutation_plan_receipt_requires_signed_mutation_approval_and_does_not_mutate() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let warden_dir = dir.path().join("data/warden");
    std::fs::create_dir_all(&warden_dir).expect("warden dir");
    let raw_queue_path = warden_dir.join("informant_queue.jsonl");
    let raw_queue = concat!(
        "{\"event_type\":\"orphan_found\",\"severity\":\"warning\",\"file\":\"human/a.md\"}\n",
        "{\"event_type\":\"destructive_quorum_denied\",\"severity\":\"critical\",\"file\":\"human/b.md\"}\n",
    );
    std::fs::write(&raw_queue_path, raw_queue).expect("write raw queue");

    let out_dir = dir.path().join("audit/warden_hades_review");
    let review_packet = service
        .project_warden_hades_operator_review_packet(dir.path(), &raw_queue_path, 10, &out_dir)
        .expect("operator review packet");
    let review_packet_path = review_packet["packet_path"]
        .as_str()
        .expect("review packet path")
        .to_owned();
    let review_queue_path = review_packet["review_queue_path"]
        .as_str()
        .expect("review queue path")
        .to_owned();

    let dry_only_approval_path = out_dir.join("operator_signed_approval_packet.json");
    service
        .warden_hades_signed_approval_packet(
            &review_packet_path,
            &["whr_raw_1".to_owned(), "whr_raw_2".to_owned()],
            "operator:test",
            "defer_retain_raw",
            "ticket:test-approval",
            &dry_only_approval_path,
        )
        .expect("signed dry-only approval packet");
    let dry_run_receipt_path = out_dir.join("dry_run_receipt.json");
    service
        .warden_hades_dry_run_receipt(
            &dry_only_approval_path,
            &review_packet_path,
            "retain_acknowledgement",
            &dry_run_receipt_path,
        )
        .expect("dry-run receipt");

    let raw_queue_before = std::fs::read_to_string(&raw_queue_path).expect("read raw before");
    let review_queue_before =
        std::fs::read_to_string(&review_queue_path).expect("read review before");
    let mutation_plan_path = out_dir.join("mutation_plan_receipt.json");
    let dry_only_plan = service.warden_hades_mutation_plan_receipt(
        &dry_only_approval_path,
        &review_packet_path,
        &dry_run_receipt_path,
        "archive_after_approval",
        &mutation_plan_path,
    );
    assert!(dry_only_plan.is_err());
    assert!(!mutation_plan_path.exists());

    let mutation_approval_path = out_dir.join("operator_signed_mutation_approval_packet.json");
    let mutation_approval = service
        .warden_hades_signed_mutation_approval_packet(
            &dry_run_receipt_path,
            "operator:test",
            "archive_after_approval",
            "ticket:test-mutation-approval",
            &mutation_approval_path,
        )
        .expect("signed mutation approval packet");
    assert_eq!(
        mutation_approval["contract"],
        "arda.hades.warden_operator_signed_mutation_approval_packet.v1"
    );
    assert_eq!(
        mutation_approval["authorizes_next_gate"],
        "mutation_plan_receipt_only"
    );

    let mutation_plan = service
        .warden_hades_mutation_plan_receipt(
            &mutation_approval_path,
            &review_packet_path,
            &dry_run_receipt_path,
            "archive_after_approval",
            &mutation_plan_path,
        )
        .expect("mutation plan receipt");
    assert_eq!(
        mutation_plan["contract"],
        "arda.hades.warden_operator_mutation_plan_receipt.v1"
    );
    assert_eq!(
        mutation_plan["planned_mutation_action"],
        "archive_after_approval"
    );
    assert_eq!(mutation_plan["mutation_performed"], false);
    assert_eq!(mutation_plan["apply_authorized"], false);
    assert_eq!(mutation_plan["archive_authorized"], false);
    assert_eq!(mutation_plan["delete_authorized"], false);
    assert_eq!(mutation_plan["clear_authorized"], false);
    assert_eq!(mutation_plan["requires_final_apply_approval"], true);
    assert_eq!(mutation_plan["selected_review_ids_total"], 2);
    assert_eq!(mutation_plan["raw_queue_retention_verified"], true);
    assert_eq!(mutation_plan["review_queue_retention_verified"], true);
    assert_eq!(mutation_plan["no_file_moves_or_deletes_performed"], true);
    assert!(mutation_plan_path.exists());
    assert_eq!(
        std::fs::read_to_string(&raw_queue_path).expect("read raw after mutation plan"),
        raw_queue_before
    );
    assert_eq!(
        std::fs::read_to_string(&review_queue_path).expect("read review after mutation plan"),
        review_queue_before
    );
}

#[test]
fn warden_hades_final_apply_approval_requires_mutation_plan_and_rollback_without_mutating() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let warden_dir = dir.path().join("data/warden");
    std::fs::create_dir_all(&warden_dir).expect("warden dir");
    let raw_queue_path = warden_dir.join("informant_queue.jsonl");
    let raw_queue = concat!(
        "{\"event_type\":\"repair_detected\",\"severity\":\"info\",\"file\":\"core/a.jsonl\"}\n",
        "{\"event_type\":\"destructive_quorum_denied\",\"severity\":\"warning\",\"file\":\"/tmp/b.jsonl\"}\n",
    );
    std::fs::write(&raw_queue_path, raw_queue).expect("write raw queue");

    let out_dir = dir.path().join("audit/warden_hades_review");
    let review_packet = service
        .project_warden_hades_operator_review_packet(dir.path(), &raw_queue_path, 10, &out_dir)
        .expect("operator review packet");
    let review_packet_path = review_packet["packet_path"]
        .as_str()
        .expect("review packet path")
        .to_owned();
    let review_queue_path = review_packet["review_queue_path"]
        .as_str()
        .expect("review queue path")
        .to_owned();

    let dry_only_approval_path = out_dir.join("operator_signed_approval_packet.json");
    service
        .warden_hades_signed_approval_packet(
            &review_packet_path,
            &["whr_raw_1".to_owned(), "whr_raw_2".to_owned()],
            "operator:test",
            "defer_retain_raw",
            "ticket:test-approval",
            &dry_only_approval_path,
        )
        .expect("signed dry-only approval packet");
    let dry_run_receipt_path = out_dir.join("dry_run_receipt.json");
    service
        .warden_hades_dry_run_receipt(
            &dry_only_approval_path,
            &review_packet_path,
            "retain_acknowledgement",
            &dry_run_receipt_path,
        )
        .expect("dry-run receipt");
    let mutation_approval_path = out_dir.join("operator_signed_mutation_approval_packet.json");
    service
        .warden_hades_signed_mutation_approval_packet(
            &dry_run_receipt_path,
            "operator:test",
            "archive_after_approval",
            "ticket:test-mutation-approval",
            &mutation_approval_path,
        )
        .expect("signed mutation approval packet");
    let mutation_plan_path = out_dir.join("mutation_plan_receipt.json");
    service
        .warden_hades_mutation_plan_receipt(
            &mutation_approval_path,
            &review_packet_path,
            &dry_run_receipt_path,
            "archive_after_approval",
            &mutation_plan_path,
        )
        .expect("mutation plan receipt");

    let final_apply_path = out_dir.join("final_apply_approval_packet.json");
    let missing_rollback = service.warden_hades_final_apply_approval_packet(
        &mutation_plan_path,
        "operator:test",
        "archive_after_approval",
        "",
        "ticket:test-final-apply",
        &final_apply_path,
    );
    assert!(missing_rollback.is_err());
    assert!(!final_apply_path.exists());

    let raw_queue_before = std::fs::read_to_string(&raw_queue_path).expect("read raw before");
    let review_queue_before =
        std::fs::read_to_string(&review_queue_path).expect("read review before");
    let final_apply = service
        .warden_hades_final_apply_approval_packet(
            &mutation_plan_path,
            "operator:test",
            "archive_after_approval",
            "rollback: restore raw/review queues from recorded sha256-backed packet artifacts before retry",
            "ticket:test-final-apply",
            &final_apply_path,
        )
        .expect("final apply approval packet");
    assert_eq!(
        final_apply["contract"],
        "arda.hades.warden_operator_final_apply_approval_packet.v1"
    );
    assert_eq!(
        final_apply["approval_status"],
        "signed_final_apply_decision_recorded"
    );
    assert_eq!(
        final_apply["approved_mutation_action"],
        "archive_after_approval"
    );
    assert_eq!(final_apply["apply_authorized"], true);
    assert_eq!(final_apply["archive_authorized"], true);
    assert_eq!(final_apply["delete_authorized"], false);
    assert_eq!(final_apply["clear_authorized"], false);
    assert_eq!(final_apply["mutation_performed"], false);
    assert_eq!(final_apply["fresh_hash_verification_passed"], true);
    assert_eq!(final_apply["rollback_plan_recorded"], true);
    assert_eq!(final_apply["authorizes_next_gate"], "final_apply_execution");
    assert_eq!(final_apply["no_file_moves_or_deletes_performed"], true);
    assert!(final_apply_path.exists());
    assert_eq!(
        std::fs::read_to_string(&raw_queue_path).expect("read raw after final apply approval"),
        raw_queue_before
    );
    assert_eq!(
        std::fs::read_to_string(&review_queue_path)
            .expect("read review after final apply approval"),
        review_queue_before
    );
}

#[test]
fn warden_hades_final_apply_execution_archives_selected_records_without_clearing_queues() {
    let _guard = test_guard();
    let dir = workspace_tempdir();
    let service = HadesService::new(dir.path().join("data/hades")).expect("service");

    let warden_dir = dir.path().join("data/warden");
    std::fs::create_dir_all(&warden_dir).expect("warden dir");
    let raw_queue_path = warden_dir.join("informant_queue.jsonl");
    let raw_queue = concat!(
        "{\"event_type\":\"repair_detected\",\"severity\":\"info\",\"file\":\"core/a.jsonl\"}\n",
        "{\"event_type\":\"destructive_quorum_denied\",\"severity\":\"warning\",\"file\":\"/tmp/b.jsonl\"}\n",
        "{\"event_type\":\"repair_detected\",\"severity\":\"info\",\"file\":\"core/c.jsonl\"}\n",
    );
    std::fs::write(&raw_queue_path, raw_queue).expect("write raw queue");

    let out_dir = dir.path().join("audit/warden_hades_review");
    let review_packet = service
        .project_warden_hades_operator_review_packet(dir.path(), &raw_queue_path, 10, &out_dir)
        .expect("operator review packet");
    let review_packet_path = review_packet["packet_path"]
        .as_str()
        .expect("review packet path")
        .to_owned();
    let review_queue_path = review_packet["review_queue_path"]
        .as_str()
        .expect("review queue path")
        .to_owned();

    let dry_only_approval_path = out_dir.join("operator_signed_approval_packet.json");
    service
        .warden_hades_signed_approval_packet(
            &review_packet_path,
            &["whr_raw_1".to_owned(), "whr_raw_2".to_owned()],
            "operator:test",
            "defer_retain_raw",
            "ticket:test-approval",
            &dry_only_approval_path,
        )
        .expect("signed dry-only approval packet");
    let dry_run_receipt_path = out_dir.join("dry_run_receipt.json");
    service
        .warden_hades_dry_run_receipt(
            &dry_only_approval_path,
            &review_packet_path,
            "retain_acknowledgement",
            &dry_run_receipt_path,
        )
        .expect("dry-run receipt");
    let mutation_approval_path = out_dir.join("operator_signed_mutation_approval_packet.json");
    service
        .warden_hades_signed_mutation_approval_packet(
            &dry_run_receipt_path,
            "operator:test",
            "archive_after_approval",
            "ticket:test-mutation-approval",
            &mutation_approval_path,
        )
        .expect("signed mutation approval packet");
    let mutation_plan_path = out_dir.join("mutation_plan_receipt.json");
    service
        .warden_hades_mutation_plan_receipt(
            &mutation_approval_path,
            &review_packet_path,
            &dry_run_receipt_path,
            "archive_after_approval",
            &mutation_plan_path,
        )
        .expect("mutation plan receipt");
    let final_apply_path = out_dir.join("final_apply_approval_packet.json");
    service
        .warden_hades_final_apply_approval_packet(
            &mutation_plan_path,
            "operator:test",
            "archive_after_approval",
            "rollback: retained raw queue and review queue hashes block drift; archive receipt is append-only evidence",
            "ticket:test-final-apply",
            &final_apply_path,
        )
        .expect("final apply approval packet");

    let raw_queue_before = std::fs::read_to_string(&raw_queue_path).expect("read raw before");
    let review_queue_before =
        std::fs::read_to_string(&review_queue_path).expect("read review before");
    let archive_path = out_dir.join("final_apply_archive.jsonl");
    let execution_receipt_path = out_dir.join("final_apply_execution_receipt.json");
    let execution = service
        .warden_hades_final_apply_execution(
            &final_apply_path,
            "archive_after_approval",
            &archive_path,
            &execution_receipt_path,
        )
        .expect("final apply execution");

    assert_eq!(
        execution["contract"],
        "arda.hades.warden_operator_final_apply_execution_receipt.v1"
    );
    assert_eq!(
        execution["executed_mutation_action"],
        "archive_after_approval"
    );
    assert_eq!(execution["mutation_performed"], true);
    assert_eq!(execution["archive_record_count"], 2);
    assert_eq!(execution["raw_queue_retained"], true);
    assert_eq!(execution["review_queue_retained"], true);
    assert_eq!(execution["clear_performed"], false);
    assert_eq!(execution["delete_performed"], false);
    assert_eq!(
        execution["archive_path"],
        archive_path.display().to_string()
    );
    assert!(archive_path.exists());
    assert!(execution_receipt_path.exists());
    let archived_lines = std::fs::read_to_string(&archive_path).expect("read archive");
    assert_eq!(archived_lines.lines().count(), 2);
    assert!(archived_lines.contains("whr_raw_1"));
    assert!(archived_lines.contains("whr_raw_2"));
    assert_eq!(
        std::fs::read_to_string(&raw_queue_path).expect("read raw after final apply execution"),
        raw_queue_before
    );
    assert_eq!(
        std::fs::read_to_string(&review_queue_path)
            .expect("read review after final apply execution"),
        review_queue_before
    );
}
