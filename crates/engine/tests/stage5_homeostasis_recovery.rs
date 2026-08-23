use arda_engine::adapters::{
    evaluate_conservation, synthesize_health, AttemptState, AuthorityEnvelope,
    ConservationDisposition, ConservationLimits, ConservationObservation, DirectHealthEvidence,
    HomeostasisStore, InterruptedAttempt, OrganismHealthState, RecoveryDisposition,
    RecoveryRequest, RecoveryTarget,
};
use serde_json::json;
use std::process::Command;

fn authority() -> AuthorityEnvelope {
    AuthorityEnvelope {
        approval_class: "safe_local".into(),
        allowed_capabilities: vec!["summarize".into()],
        allowed_data_domains: vec!["system".into()],
        egress_allowed: false,
    }
}

fn ready_evidence(node_id: &str, now: u64, source: &str) -> DirectHealthEvidence {
    DirectHealthEvidence {
        node_id: node_id.into(),
        enrollment_status: "active".into(),
        observed_at_unix_ms: now,
        heartbeat_at_unix_ms: Some(now),
        endpoint_reachable: Some(true),
        service_active: Some(true),
        minimal_work_succeeded: Some(true),
        queue_pressure: Some(0.1),
        resource_pressure: Some(0.2),
        memory_available: Some(true),
        configured_route: Some("local".into()),
        observed_route: Some("local".into()),
        source_refs: vec![source.into()],
    }
}

fn attempt() -> InterruptedAttempt {
    InterruptedAttempt {
        run_id: "stage5-run".into(),
        attempt_id: "attempt-worker-a".into(),
        work_id: "bounded-summary".into(),
        worker_id: "worker-a".into(),
        node_id: "node-worker-a".into(),
        state: AttemptState::Running,
        idempotency_key: "stage5-run:bounded-summary".into(),
        external_side_effect: false,
        side_effect_idempotent: true,
        terminal_receipt_ref: None,
        authority: authority(),
        source_refs: vec!["process:worker-a".into(), "attempt:worker-a".into()],
    }
}

#[test]
fn real_worker_process_kill_is_degraded_reassigned_and_restart_safe() {
    let temp = tempfile::tempdir().unwrap();
    let now = 1_800_000_000_000;
    let mut worker = Command::new("/bin/sh")
        .args(["-c", "exec sleep 60"])
        .spawn()
        .unwrap();
    let mut target_worker = Command::new("/bin/sh")
        .args(["-c", "exec sleep 60"])
        .spawn()
        .unwrap();
    let worker_pid = worker.id();
    let target_worker_pid = target_worker.id();
    assert!(worker.try_wait().unwrap().is_none());
    assert!(target_worker.try_wait().unwrap().is_none());

    let before = synthesize_health(
        &ready_evidence(
            "node-worker-a",
            now,
            &format!("process:{worker_pid}:running"),
        ),
        now,
        5_000,
    );
    assert_eq!(before.state, OrganismHealthState::Ready);

    worker.kill().unwrap();
    let exit = worker.wait().unwrap();
    assert!(!exit.success());
    assert!(worker.try_wait().unwrap().is_some());

    let mut failed_evidence = ready_evidence(
        "node-worker-a",
        now + 1_000,
        &format!("process:{worker_pid}:exited:{exit}"),
    );
    failed_evidence.service_active = Some(false);
    failed_evidence.minimal_work_succeeded = Some(false);
    let failed = synthesize_health(&failed_evidence, now + 1_000, 5_000);
    assert_eq!(failed.state, OrganismHealthState::ServiceDown);
    assert!(!failed.ready_for_new_work);

    let target_health = synthesize_health(
        &ready_evidence(
            "node-worker-b",
            now + 1_000,
            &format!("process:{target_worker_pid}:running"),
        ),
        now + 1_000,
        5_000,
    );
    let request = RecoveryRequest {
        recovery_key: "stage5-run:attempt-worker-a:recovery-1".into(),
        interrupted_health: failed.state,
        attempt: attempt(),
        target: Some(RecoveryTarget {
            worker_id: "worker-b".into(),
            node_id: "node-worker-b".into(),
            health: target_health.state,
            authority: authority(),
            source_refs: target_health.source_refs.clone(),
        }),
        retry_count: 0,
        max_retries: 1,
        recorded_at_unix_ms: now + 1_000,
    };
    let first = HomeostasisStore::new(temp.path())
        .reconcile(&request)
        .unwrap();
    assert_eq!(first.disposition, RecoveryDisposition::Reassign);
    assert_eq!(first.target_node_id.as_deref(), Some("node-worker-b"));
    assert!(first.authority_preserved);
    assert!(!first.duplicate_mutation_allowed);

    let ledger = temp.path().join("data/homeostasis/recovery-receipts.jsonl");
    let before_restart = std::fs::read(&ledger).unwrap();
    let replay = HomeostasisStore::new(temp.path())
        .reconcile(&request)
        .unwrap();
    let after_restart = std::fs::read(&ledger).unwrap();
    assert_eq!(first, replay);
    assert_eq!(before_restart, after_restart);
    assert_eq!(
        HomeostasisStore::new(temp.path()).receipts().unwrap().len(),
        1
    );
    target_worker.kill().unwrap();
    target_worker.wait().unwrap();

    if let Ok(path) = std::env::var("STAGE5_EVIDENCE_PATH") {
        let artifact = json!({
            "schema_version": "arda.digital-organism.stage5-proof.v1",
            "live_failure": {
                "pid": worker_pid,
                "exit_status": exit.to_string(),
                "before": before,
                "after": failed
            },
            "target_health": target_health,
            "target_process_pid": target_worker_pid,
            "recovery_receipt": first,
            "restart_replay": {
                "ledger_rows": 1,
                "ledger_byte_stable": before_restart == after_restart,
                "duplicate_terminal_mutation": false
            }
        });
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }
}

#[test]
fn terminal_and_unknown_side_effects_are_never_repeated() {
    let temp = tempfile::tempdir().unwrap();
    let mut terminal = attempt();
    terminal.state = AttemptState::Succeeded;
    terminal.terminal_receipt_ref = Some("receipt:terminal".into());
    let terminal_request = RecoveryRequest {
        recovery_key: "terminal-recovery".into(),
        interrupted_health: OrganismHealthState::ServiceDown,
        attempt: terminal,
        target: Some(RecoveryTarget {
            worker_id: "worker-b".into(),
            node_id: "node-worker-b".into(),
            health: OrganismHealthState::Ready,
            authority: authority(),
            source_refs: vec!["health:worker-b".into()],
        }),
        retry_count: 0,
        max_retries: 1,
        recorded_at_unix_ms: 10,
    };
    let preserved = HomeostasisStore::new(temp.path())
        .reconcile(&terminal_request)
        .unwrap();
    assert_eq!(preserved.disposition, RecoveryDisposition::PreserveTerminal);
    assert!(preserved.completed_evidence_preserved);

    let mut unsafe_attempt = attempt();
    unsafe_attempt.external_side_effect = true;
    unsafe_attempt.side_effect_idempotent = false;
    let unsafe_request = RecoveryRequest {
        recovery_key: "unknown-side-effect-recovery".into(),
        interrupted_health: OrganismHealthState::Unreachable,
        attempt: unsafe_attempt,
        target: None,
        retry_count: 0,
        max_retries: 1,
        recorded_at_unix_ms: 11,
    };
    let unknown = HomeostasisStore::new(temp.path())
        .reconcile(&unsafe_request)
        .unwrap();
    assert_eq!(unknown.disposition, RecoveryDisposition::MarkUnknown);
    assert!(!unknown.duplicate_mutation_allowed);
}

#[test]
fn conservation_policy_bounds_every_declared_resource_and_attention_axis() {
    let limits = ConservationLimits {
        max_concurrency: 2,
        max_retries: 1,
        max_elapsed_ms: 10_000,
        max_context_tokens: 4_096,
        max_output_tokens: 1_024,
        max_cost_microunits: 50_000,
        max_cpu_ratio: 0.90,
        max_gpu_ratio: 0.90,
        max_ram_ratio: 0.85,
        max_thermal_ratio: 0.85,
        max_power_ratio: 0.90,
        max_network_bytes: 1_000_000,
        max_storage_bytes: 10_000_000,
        max_operator_attention_units: 1,
    };
    let normal = ConservationObservation {
        concurrency: 1,
        retries: 0,
        elapsed_ms: 1_000,
        context_tokens: 1_000,
        output_tokens: 200,
        cost_microunits: 1_000,
        cpu_ratio: 0.2,
        gpu_ratio: 0.2,
        ram_ratio: 0.2,
        thermal_ratio: 0.2,
        power_ratio: 0.2,
        network_bytes: 100,
        storage_bytes: 100,
        operator_attention_units: 0,
        optional_work: false,
        consequential_action: false,
    };
    assert_eq!(
        evaluate_conservation(&limits, &normal).disposition,
        ConservationDisposition::Continue
    );

    let optional_pressure = ConservationObservation {
        ram_ratio: 0.95,
        optional_work: true,
        ..normal.clone()
    };
    let shed = evaluate_conservation(&limits, &optional_pressure);
    assert_eq!(shed.disposition, ConservationDisposition::ShedOptional);
    assert_eq!(shed.exceeded_limits, vec!["ram"]);

    let consequential_pressure = ConservationObservation {
        cost_microunits: 60_000,
        consequential_action: true,
        ..normal.clone()
    };
    assert_eq!(
        evaluate_conservation(&limits, &consequential_pressure).disposition,
        ConservationDisposition::RequestReview
    );

    let exhausted = ConservationObservation {
        retries: 2,
        ..normal
    };
    assert_eq!(
        evaluate_conservation(&limits, &exhausted).disposition,
        ConservationDisposition::Stop
    );
}

#[test]
fn health_states_and_authority_widening_fail_closed() {
    let now = 1_000;
    let intentional = DirectHealthEvidence {
        enrollment_status: "offline".into(),
        ..ready_evidence("optional-node", now, "fleet:optional-node")
    };
    assert_eq!(
        synthesize_health(&intentional, now, 100).state,
        OrganismHealthState::IntentionalOffline
    );

    let mut stale = ready_evidence("stale-node", now, "heartbeat:stale-node");
    stale.heartbeat_at_unix_ms = Some(1);
    assert_eq!(
        synthesize_health(&stale, now, 100).state,
        OrganismHealthState::Unobserved
    );

    let temp = tempfile::tempdir().unwrap();
    let mut widened = authority();
    widened.egress_allowed = true;
    widened.allowed_capabilities.push("shell".into());
    let request = RecoveryRequest {
        recovery_key: "authority-widening".into(),
        interrupted_health: OrganismHealthState::ServiceDown,
        attempt: attempt(),
        target: Some(RecoveryTarget {
            worker_id: "worker-unsafe".into(),
            node_id: "node-unsafe".into(),
            health: OrganismHealthState::Ready,
            authority: widened,
            source_refs: vec!["health:node-unsafe".into()],
        }),
        retry_count: 0,
        max_retries: 1,
        recorded_at_unix_ms: now,
    };
    let receipt = HomeostasisStore::new(temp.path())
        .reconcile(&request)
        .unwrap();
    assert_eq!(receipt.disposition, RecoveryDisposition::Stop);
    assert!(!receipt.authority_preserved);
}
