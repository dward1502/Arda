use super::*;
use crate::objectives::snapshot_protocol::{Manifest, RuntimeBundle};
use crate::objectives::{ControlAction, NewObjective, RetainedSnapshot};
use crate::runs::{RunEventDraft, RunEventKind};
use arda_core::run_graph::NodeId;
use arda_vaire::service::{
    scope_policy::{ConsumerContext, MemoryDomain},
    MnemosyneService,
};
use rusqlite::params;
use serde_json::json;
use std::sync::atomic::Ordering;

#[path = "../../../tests/fixtures/context_provider.rs"]
mod provider;
mod restart;
mod transport;

#[test]
fn authenticated_unfinished_recovery_completes_and_replays() {
    if std::env::var_os("ARDA_UNFINISHED_RECOVERY_CHILD").is_none() {
        for mode in [
            "success",
            "completion-ack",
            "completion-cancel",
            "memory-write",
            "memory-write-cancel",
        ] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "harness::operator_messages::unfinished_tests::authenticated_unfinished_recovery_completes_and_replays",
                    "--nocapture",
                ])
                .env("ARDA_UNFINISHED_RECOVERY_CHILD", mode)
                .env("ARDA_HERMES_LOCAL_CAPABILITY", "fixture-token")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{mode}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(run());
}

async fn run() {
    let mode = std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD").unwrap();
    let first_effect_only = mode.starts_with("memory-write");
    let mode = match mode.as_str() {
        "memory-write" => "completion-ack",
        "memory-write-cancel" => "completion-cancel",
        other => other,
    };
    let (temp, run, mut graph, bindings, mut execute) =
        crate::runs::recovery_evidence::tests::fixture();
    let root = temp.path();
    let now = chrono::Utc::now().timestamp_millis() as u64;
    provider::install_fake_hermes(root);
    let mut contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    contract["workspace"]["root"] = json!(".");
    let contract: arda_core::project_contract::ProjectContract =
        serde_json::from_value(contract).unwrap();
    let project = contract.identity.project_id.to_string();
    let contract_digest = crate::harness::projects::contract_digest(&contract).unwrap();
    std::fs::create_dir_all(root.join("data/workbench")).unwrap();
    std::fs::write(
        root.join("data/workbench/projects.json"),
        serde_json::to_vec(&json!({
            "schema_version": "arda.workbench.project-registry.v1",
            "projects": [{
                "contract": contract,
                "approval_id": "fixture-approval",
                "proposal_id": "fixture-proposal",
                "idempotency_key": "fixture-attach"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    execute.schema_version = "arda.execution-receipt.v3".into();
    execute.project_contract_digest = contract_digest.clone();
    execute.receipt_digest = execute.computed_digest().unwrap();
    let mut events = run.recover().unwrap().events;
    for event in &mut events {
        if event.node_id.as_str() == "execute" && event.receipt_digest.is_some() {
            event.receipt_digest = Some(execute.receipt_digest.clone());
        }
    }
    std::fs::write(
        run.events_path(),
        events
            .iter()
            .map(|event| serde_json::to_string(event).unwrap() + "\n")
            .collect::<String>(),
    )
    .unwrap();
    run.write_execution_receipt(
        &NodeId::new("execute").unwrap(),
        &serde_json::to_value(&execute).unwrap(),
    )
    .unwrap();
    graph.provenance.project_contract_digest = contract_digest.clone();
    graph.objective_id = arda_core::run_graph::ObjectiveId::new(&bindings.leaf_id).unwrap();
    graph.provenance.parent_receipts = vec!["fixture-approval".into(), "fixture-plan".into()];
    for node in &mut graph.nodes {
        node.timeout_ms = 10_000;
        if node.id.as_str() == "execute" {
            node.output_digest = Some(execute.receipt_digest.clone());
        }
        if node.id.as_str() == "verify" {
            node.parent_receipts = vec![execute.receipt_digest.clone()];
        }
        if ["execute", "verify", "review"].contains(&node.id.as_str()) {
            let review = node.id.as_str() == "review";
            let inspection = node.id.as_str() == "execute";
            node.worker = Some(serde_json::from_value(json!({
                "role": if inspection {"local_summary_classification"} else if review {"security_privacy_critic"} else {"independent_verifier"},
                "worker_id": format!("fixture-{}", node.id.as_str()),
                "route_id": "hosted:hermes-workbench",
                "route_class": "hosted",
                "prompt_digest": format!("sha256:{}", "1".repeat(64)),
                "allowed_toolsets": ["file"],
                "dependencies": [if inspection {"approval"} else if review {"verify"} else {"execute"}],
                "deadline_unix_ms": now - 60_000,
                "output_contract": "arda.hermes-job-result.v1",
                "evidence_policy": if review || inspection {"worker_report"} else {"project_native_checks"}
            })).unwrap());
        }
    }
    graph.edges = serde_json::from_value(json!([
        {"id": "approval-execute", "from": "approval", "to": "execute", "parent_receipt": execute.parent_receipts[0]},
        {"id": "execute-verify", "from": "execute", "to": "verify", "parent_receipt": execute.receipt_digest},
        {"id": "verify-review", "from": "verify", "to": "review", "parent_receipt": null},
        {"id": "review-close", "from": "review", "to": "close", "parent_receipt": null}
    ]))
    .unwrap();
    run.write_checkpoint(&graph).unwrap();
    run.append(RunEventDraft {
        node_id: NodeId::new("approval").unwrap(),
        idempotency_key: "fixture-planned".into(),
        kind: RunEventKind::Planned {
            project_id: project.clone(),
            approval_id: "fixture-approval".into(),
        },
        receipt_digest: Some("fixture-plan".into()),
    })
    .unwrap();
    let memory = MnemosyneService::new(root.join("data/vaire"))
        .unwrap()
        .with_contract_memory_root(root.join("core/state/memory"));
    let request = serde_json::from_value(json!({
        "schema_version": arda_vaire::OrganismContext::SCHEMA_VERSION,
        "organism_id": "arda:fixture",
        "generated_at_unix_ms": now - 120_000,
        "expires_at_unix_ms": now - 60_000,
        "objective": {
            "requested_outcome": "fixture",
            "acceptance_conditions": ["cited inspection"],
            "required_capabilities": ["file"],
            "forbidden_capabilities": ["terminal"]
        },
        "consumer": {
            "consumer_id": "fixture",
            "role": "worker",
            "authority_ceiling": "read_only",
            "operator_authorized": false,
            "memory_domains": ["system"],
            "data_classes": ["internal"],
            "permitted_egress": ["local_device"],
            "compute_node_refs": ["local"],
            "agent_ref": null
        },
        "lineage": {
            "objective_id": bindings.objective_id,
            "project_id": project,
            "task_id": bindings.leaf_id,
            "run_id": bindings.run_id,
            "parent_receipts": [],
            "session_ref": null
        },
        "return_contract": {
            "schema_version": "arda.organism-outcome.v1",
            "required_receipt_types": ["arda.context-use-receipt.v1"],
            "max_output_bytes": 4096
        },
        "evidence_refs": [],
        "memory_refs": [],
        "unresolved_failures": []
    }))
    .unwrap();
    let mut consumer = ConsumerContext::new("fixture", vec![MemoryDomain::System]);
    consumer.purpose = Some("fixture".into());
    let context = memory
        .assemble_organism_context(request, &consumer, u128::from(now - 120_000))
        .unwrap();
    let execute_context = memory
        .bind_run_stage_context(
            &context,
            "execute",
            "inspect",
            execute.parent_receipts.clone(),
            u128::from(now - 120_000),
        )
        .unwrap();
    let old_execute_digest = execute.receipt_digest.clone();
    execute.context_capsule_id = Some(execute_context.capsule.capsule_id.clone());
    execute.context_capsule_digest = Some(execute_context.capsule.capsule_digest.clone());
    execute.context_use_receipt_ref = Some(execute_context.use_receipt.receipt_ref());
    execute.receipt_digest = execute.computed_digest().unwrap();
    run.write_execution_receipt(
        &NodeId::new("execute").unwrap(),
        &serde_json::to_value(&execute).unwrap(),
    )
    .unwrap();
    let seeded_journal = std::fs::read_to_string(run.events_path()).unwrap();
    std::fs::write(
        run.events_path(),
        seeded_journal.replace(&old_execute_digest, &execute.receipt_digest),
    )
    .unwrap();
    let checkpoint = std::fs::read_to_string(run.checkpoint_path()).unwrap();
    std::fs::write(
        run.checkpoint_path(),
        checkpoint.replace(&old_execute_digest, &execute.receipt_digest),
    )
    .unwrap();
    let manifest = Manifest {
        version: 2,
        capability: "fixture-capability".into(),
        root: root.to_owned(),
        device: 1,
        inode: 1,
        topology_digest: "fixture".into(),
        runtime_bundle: Some(RuntimeBundle {
            version: 1,
            policy_digest: "fixture".into(),
            grants: vec![],
        }),
        admission_digest: Some("a".repeat(64)),
    };
    let snapshot = RetainedSnapshot {
        endpoint: root.join("worker.sock").to_str().unwrap().into(),
        capability: manifest.capability.clone(),
        manifest_digest: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&manifest).unwrap())
        ),
    };
    let transport = transport::start(root, &snapshot, manifest);
    std::env::set_var("ARDA_SNAPSHOT_KEEPER_SOCKET", root.join("keeper.sock"));
    std::env::set_var(
        "ARDA_RETAINED_HERMES_CONFIG_OVERRIDE",
        root.join("config/adapters/hermes-workbench.toml"),
    );
    let store =
        crate::objectives::ObjectiveStore::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let envelope = json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "fixture-proposal",
            "approval_id": "fixture-approval",
            "ledger_writes": ["fixture-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-07-31T00:00:00Z"
        },
        "idempotency_key": "fixture-execute"
    });
    let execution: crate::objectives::LeafExecutionSpec = serde_json::from_value(json!({
        "objective": "fixture",
        "execution_prompt": "inspect",
        "verification_prompt": "verify",
        "review_prompt": "review",
        "approval_envelope": envelope,
        "objective_plan_receipt": "fixture-plan"
    }))
    .unwrap();
    let objective: NewObjective = serde_json::from_value(json!({
        "id": bindings.objective_id,
        "source_id": "fixture",
        "idempotency_key": "fixture",
        "operator_id": "operator:fixture",
        "text": "fixture",
        "priority": 1,
        "projects": [{
            "project_id": project,
            "contract_digest": contract_digest
        }],
        "leaves": [{
            "id": bindings.leaf_id,
            "project_id": project,
            "workspace_root": root,
            "authority": "read_only",
            "dependencies": [],
            "execution": execution
        }]
    }))
    .unwrap();
    store.create_authenticated_objective(objective, 1).unwrap();
    store
        .apply_control(
            &bindings.objective_id,
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator:fixture",
            2,
        )
        .unwrap();
    store
        .apply_control(
            &bindings.objective_id,
            ControlAction::Pause,
            "pause",
            "operator:fixture",
            3,
        )
        .unwrap();
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    db.execute(
        "UPDATE leaves SET stage='execute',attempt=1,execution_run_id=?1,context_bound=1 WHERE id=?2",
        params![bindings.run_id.as_str(), bindings.leaf_id],
    )
    .unwrap();
    let request_digest = format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({
                "objective_id": bindings.objective_id,
                "leaf_id": bindings.leaf_id,
                "project_id": project,
                "project_contract_digest": contract_digest,
                "workspace_root": root,
                "authority": "read_only",
                "execution": execution,
                "dependencies": []
            }))
            .unwrap()
        )
    );
    db.execute(
        "INSERT INTO resident_context_bindings(run_id,request_digest,assembly_json) VALUES (?1,?2,?3)",
        params![bindings.run_id.as_str(), request_digest, serde_json::to_string(&context).unwrap()],
    )
    .unwrap();
    db.execute(
        "INSERT INTO retained_workspace_snapshots VALUES (?1,?2,?3,1)",
        params![
            bindings.leaf_id,
            bindings.run_id.as_str(),
            serde_json::to_string(&snapshot).unwrap()
        ],
    )
    .unwrap();
    db.execute(
        "INSERT INTO lease_workspace_identities VALUES (?1,'fixture-identity')",
        [&bindings.leaf_id],
    )
    .unwrap();
    let before = std::fs::read(run.events_path()).unwrap();
    let execute_path = root
        .join("data/runs")
        .join(bindings.run_id.as_str())
        .join("execution-receipts/execute.json");
    let execute_before = std::fs::read(&execute_path).unwrap();
    use std::sync::Arc;
    let state = HarnessState {
        research_store_policy: super::super::ResearchStorePolicy::Isolated,
        harness_addr: "127.0.0.1:0".into(),
        child_pids: Arc::new(tokio::sync::RwLock::new(vec![])),
        service_names: Arc::new(vec![]),
        service_statuses: Arc::new(tokio::sync::RwLock::new(vec![])),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: super::super::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: super::super::DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: super::super::presence::HarnessPresenceState::default(),
        workbench_root: root.to_path_buf(),
        operator_id: "operator:fixture".into(),
    };
    let message = json!({
        "operator": {
            "operator_id": "operator:fixture",
            "authenticated": true,
            "authentication_method": "local_session",
            "authenticated_at": Utc::now().to_rfc3339()
        },
        "adapter_id": "hermes-cli",
        "event": {
            "text": format!(
                "arda recover-retained {} {} {} one-verify-start 30m reuse-expired-context",
                bindings.objective_id, bindings.leaf_id, bindings.run_id.as_str()
            ),
            "message_type": "text",
            "user_id": "operator:fixture",
            "user_name": "fixture",
            "source": {
                "platform": "cli",
                "chat_id": "fixture-session",
                "chat_type": "private",
                "thread_id": null,
                "message_id": "unfinished-recovery"
            },
            "message_id": "unfinished-recovery",
            "media_urls": [],
            "media_types": [],
            "timestamp": Utc::now().to_rfc3339(),
            "prompt_response": null
        }
    });
    let socket = root.join("keeper.sock");
    let recovery_jobs = super::super::RecoveryJobs::default();
    let invoke = |authorized: bool| {
        let state = state.clone();
        let message = message.clone();
        let socket = socket.clone();
        let recovery_jobs = recovery_jobs.clone();
        async move {
            let mut headers = HeaderMap::new();
            if authorized {
                headers.insert("x-arda-local-capability", "fixture-token".parse().unwrap());
            }
            ingest_local_operator_message(
                State(state),
                ConnectInfo("127.0.0.1:1234".parse().unwrap()),
                Some(axum::Extension(super::super::RuntimePrerequisites {
                    recovery_jobs,
                    keeper_socket: Some(socket),
                })),
                headers,
                Json(serde_json::from_value(message).unwrap()),
            )
            .await
        }
    };
    assert!(invoke(false).await.is_err());
    assert_eq!(transport.chats.load(std::sync::atomic::Ordering::SeqCst), 0);
    if mode == "drop-characterization" {
        let mut waiter = Box::pin(invoke(true));
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            tokio::select! {
                result = &mut waiter => panic!("ingress returned before first Chat: {result:?}"),
                _ = transport.drop_started.notified() => {}
            }
        })
        .await
        .unwrap();
        let spent = std::fs::read(run.events_path()).unwrap();
        assert!(spent.starts_with(&before));
        let last: serde_json::Value =
            serde_json::from_str(std::str::from_utf8(&spent).unwrap().lines().last().unwrap())
                .unwrap();
        assert_eq!(last["node_id"], "verify");
        assert_eq!(last["kind"]["type"], "node_transition");
        assert_eq!(last["kind"]["state"], "running");
        assert!(!root
            .join("data/runs/recovery-evidence-fixture/recovery-outcomes")
            .exists());
        drop(waiter);
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            transport.drop_closed.notified(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(run.events_path()).unwrap(), spent);
        assert!(!root
            .join("data/runs/recovery-evidence-fixture/recovery-outcomes")
            .exists());
        assert_eq!(
            std::fs::read(run.execution_receipt_path(&NodeId::new("execute").unwrap())).unwrap(),
            execute_before
        );
        assert_eq!(transport.chats.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            transport.exports.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(
            transport.releases.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM recovery_publications", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        transport.stop().await;
        return;
    }
    let mut failed_effects = None;
    if mode != "success" {
        let fault = root.join("core/state/memory/.governed-records.lock");
        if first_effect_only {
            std::fs::create_dir_all(&fault).unwrap();
        } else {
            db.execute_batch("CREATE TRIGGER test_fail_completion_ack BEFORE UPDATE OF applied_at_ms ON recovery_publications WHEN NEW.publication_key='completion' BEGIN SELECT RAISE(ABORT,'injected completion ACK failure'); END;")
                .unwrap();
        }
        let error = invoke(true)
            .await
            .expect_err("fault did not interrupt completion");
        assert!(
            format!("{error:?}").contains(if first_effect_only {
                "Is a directory"
            } else {
                "injected completion ACK failure"
            }),
            "{error:?}"
        );
        assert_eq!(transport.chats.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(
            std::fs::read_to_string(root.join("data/vaire/context_outcome_receipts.jsonl"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM recovery_publications WHERE publication_key='completion' AND applied_at_ms IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        failed_effects = Some(completion_effects(root, &db, !first_effect_only));
        assert_eq!(
            transport.releases.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        if first_effect_only {
            std::fs::remove_dir(&fault).unwrap();
        } else {
            db.execute_batch("DROP TRIGGER test_fail_completion_ack")
                .unwrap();
        }
        if mode == "completion-cancel" {
            store
                .apply_control(
                    &bindings.objective_id,
                    ControlAction::Cancel,
                    "cancel-after-partial-completion",
                    "operator:fixture",
                    now as i64,
                )
                .unwrap();
        }
    }
    let mut replay_effects = None;
    let mut replay_journal = None;
    for _ in 0..2 {
        match invoke(true).await {
            Ok(result) => assert!(result.0.summary.contains(if mode == "completion-cancel" {
                "cleanup"
            } else {
                "reconciled"
            })),
            Err(error) => panic!("{error:?}"),
        }
        let effects = completion_effects(
            root,
            &db,
            !(first_effect_only && mode == "completion-cancel"),
        );
        if let Some(previous) = &failed_effects {
            assert_eq!(
                effects.0, previous.0,
                "context outcome changed after replay"
            );
            if !first_effect_only || mode == "completion-cancel" {
                assert_eq!(&effects, previous, "prior effects changed after replay");
            }
        }
        if let Some(previous) = &replay_effects {
            assert_eq!(
                &effects, previous,
                "duplicate replay changed destination bytes"
            );
        }
        replay_effects = Some(effects);
        let journal = std::fs::read(run.events_path()).unwrap();
        if let Some(previous) = &replay_journal {
            assert_eq!(&journal, previous, "duplicate ingress altered journal");
        }
        replay_journal = Some(journal);
        assert_eq!(
            transport.commits.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            transport.releases.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            transport.exports.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
    }
    assert_eq!(transport.chats.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(std::fs::read(&execute_path).unwrap(), execute_before);
    assert!(std::fs::read(run.events_path())
        .unwrap()
        .starts_with(&before));
    assert_eq!(
        db.query_row("SELECT stage FROM leaves", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "complete"
    );
    assert_eq!(
        db.query_row("SELECT state FROM objectives", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        if mode == "completion-cancel" {
            "cancelled"
        } else {
            "paused"
        }
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM recovery_publications WHERE applied_at_ms IS NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        i64::from(mode == "completion-cancel")
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM recovery_completion_suppressions",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        i64::from(mode == "completion-cancel")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("data/vaire/context_outcome_receipts.jsonl"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    transport.stop().await;
}

fn completion_effects(
    root: &std::path::Path,
    db: &rusqlite::Connection,
    memory_present: bool,
) -> (Vec<u8>, Option<Vec<u8>>) {
    let serialized: String = db
        .query_row(
            "SELECT payload_json FROM recovery_publications WHERE publication_key='completion'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let publication: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    let prepared = &publication["prepared"];
    let context = std::fs::read(root.join("data/vaire/context_outcome_receipts.jsonl")).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&context).unwrap(),
        prepared["context_outcome"]
    );
    let memory = &prepared["memory"];
    let path = root
        .join("core/state/memory")
        .join(memory["kind"].as_str().unwrap())
        .join(format!("{}.json", memory["id"].as_str().unwrap()));
    let bytes = if memory_present {
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            *memory
        );
        Some(bytes)
    } else {
        assert!(!path.exists(), "uncommitted governed memory was published");
        None
    };
    (context, bytes)
}

/// S1 positive regression: Harness-owned recovery survives response-waiter loss
/// and records an exact-start error instead of leaving the node Running without
/// outcome or publication. Held at a barrier until the first Verify Chat arrives,
/// then the response waiter is dropped and a known synthetic error is injected.
#[test]
fn harness_owned_recovery_survives_waiter_loss_and_records_exact_start_error() {
    if std::env::var_os("ARDA_UNFINISHED_RECOVERY_CHILD").is_none() {
        for mode in [
            "waiter-loss-positive",
            "waiter-loss-evidence-error",
            "waiter-loss-http",
            "waiter-loss-cancel",
            "waiter-loss-pause",
            "waiter-loss-lease-expiry",
            "waiter-loss-grant-expiry",
            "waiter-loss-supersession",
            "waiter-loss-shutdown",
            "waiter-loss-cancel-success",
            "waiter-loss-pause-success",
            "waiter-loss-lease-expiry-success",
            "waiter-loss-grant-expiry-success",
            "waiter-loss-supersession-success",
            "waiter-loss-shutdown-success",
        ] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "harness::operator_messages::unfinished_tests::harness_owned_recovery_survives_waiter_loss_and_records_exact_start_error",
                "--nocapture",
            ])
            .env("ARDA_UNFINISHED_RECOVERY_CHILD", mode)
            .env("ARDA_HERMES_LOCAL_CAPABILITY", "fixture-token")
            .output()
            .unwrap();
            assert!(
                output.status.success(),
                "{mode}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_waiter_loss_positive());
}

async fn run_waiter_loss_positive() {
    let mode = std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD").unwrap();
    let successful_result = mode.ends_with("-success");
    let fence_mode = mode.strip_suffix("-success").unwrap_or(&mode);
    let fenced = matches!(
        fence_mode,
        "waiter-loss-cancel"
            | "waiter-loss-pause"
            | "waiter-loss-lease-expiry"
            | "waiter-loss-grant-expiry"
            | "waiter-loss-supersession"
            | "waiter-loss-shutdown"
    );
    let http_disconnect =
        std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD").unwrap() == "waiter-loss-http";
    let evidence_error =
        std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD").unwrap() == "waiter-loss-evidence-error";
    let publication_barrier = tempfile::tempdir().unwrap();
    if successful_result {
        std::env::set_var(
            "ARDA_TEST_RECOVERY_PUBLICATION_BARRIER",
            publication_barrier.path(),
        );
    }
    if fence_mode == "waiter-loss-lease-expiry" {
        std::env::set_var("ARDA_TEST_RECOVERY_LEASE_CLOCK", "1");
    }
    let (temp, run, mut graph, bindings, mut execute) =
        crate::runs::recovery_evidence::tests::fixture();
    let root = temp.path();
    let now = chrono::Utc::now().timestamp_millis() as u64;
    provider::install_fake_hermes(root);
    let mut contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    contract["workspace"]["root"] = json!(".");
    let contract: arda_core::project_contract::ProjectContract =
        serde_json::from_value(contract).unwrap();
    let project = contract.identity.project_id.to_string();
    let contract_digest = crate::harness::projects::contract_digest(&contract).unwrap();
    std::fs::create_dir_all(root.join("data/workbench")).unwrap();
    std::fs::write(
        root.join("data/workbench/projects.json"),
        serde_json::to_vec(&json!({
            "schema_version": "arda.workbench.project-registry.v1",
            "projects": [{
                "contract": contract,
                "approval_id": "fixture-approval",
                "proposal_id": "fixture-proposal",
                "idempotency_key": "fixture-attach"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    execute.schema_version = "arda.execution-receipt.v3".into();
    execute.project_contract_digest = contract_digest.clone();
    execute.receipt_digest = execute.computed_digest().unwrap();
    let mut events = run.recover().unwrap().events;
    for event in &mut events {
        if event.node_id.as_str() == "execute" && event.receipt_digest.is_some() {
            event.receipt_digest = Some(execute.receipt_digest.clone());
        }
    }
    std::fs::write(
        run.events_path(),
        events
            .iter()
            .map(|event| serde_json::to_string(event).unwrap() + "\n")
            .collect::<String>(),
    )
    .unwrap();
    run.write_execution_receipt(
        &NodeId::new("execute").unwrap(),
        &serde_json::to_value(&execute).unwrap(),
    )
    .unwrap();
    graph.provenance.project_contract_digest = contract_digest.clone();
    graph.objective_id = arda_core::run_graph::ObjectiveId::new(&bindings.leaf_id).unwrap();
    graph.provenance.parent_receipts = vec!["fixture-approval".into(), "fixture-plan".into()];
    for node in &mut graph.nodes {
        node.timeout_ms = 10_000;
        if node.id.as_str() == "execute" {
            node.output_digest = Some(execute.receipt_digest.clone());
        }
        if node.id.as_str() == "verify" {
            node.parent_receipts = vec![execute.receipt_digest.clone()];
        }
        if ["execute", "verify", "review"].contains(&node.id.as_str()) {
            let review = node.id.as_str() == "review";
            let inspection = node.id.as_str() == "execute";
            node.worker = Some(serde_json::from_value(json!({
                "role": if inspection {"local_summary_classification"} else if review {"security_privacy_critic"} else {"independent_verifier"},
                "worker_id": format!("fixture-{}", node.id.as_str()),
                "route_id": "hosted:hermes-workbench",
                "route_class": "hosted",
                "prompt_digest": format!("sha256:{}", "1".repeat(64)),
                "allowed_toolsets": ["file"],
                "dependencies": [if inspection {"approval"} else if review {"verify"} else {"execute"}],
                "deadline_unix_ms": now - 60_000,
                "output_contract": "arda.hermes-job-result.v1",
                "evidence_policy": if review || inspection {"worker_report"} else {"project_native_checks"}
            })).unwrap());
        }
    }
    graph.edges = serde_json::from_value(json!([
        {"id": "approval-execute", "from": "approval", "to": "execute", "parent_receipt": execute.parent_receipts[0]},
        {"id": "execute-verify", "from": "execute", "to": "verify", "parent_receipt": execute.receipt_digest},
        {"id": "verify-review", "from": "verify", "to": "review", "parent_receipt": null},
        {"id": "review-close", "from": "review", "to": "close", "parent_receipt": null}
    ]))
    .unwrap();
    run.write_checkpoint(&graph).unwrap();
    run.append(RunEventDraft {
        node_id: NodeId::new("approval").unwrap(),
        idempotency_key: "fixture-planned".into(),
        kind: RunEventKind::Planned {
            project_id: project.clone(),
            approval_id: "fixture-approval".into(),
        },
        receipt_digest: Some("fixture-plan".into()),
    })
    .unwrap();
    let memory = MnemosyneService::new(root.join("data/vaire"))
        .unwrap()
        .with_contract_memory_root(root.join("core/state/memory"));
    let request = serde_json::from_value(json!({
        "schema_version": arda_vaire::OrganismContext::SCHEMA_VERSION,
        "organism_id": "arda:fixture",
        "generated_at_unix_ms": now - 120_000,
        "expires_at_unix_ms": now - 60_000,
        "objective": {
            "requested_outcome": "fixture",
            "acceptance_conditions": ["cited inspection"],
            "required_capabilities": ["file"],
            "forbidden_capabilities": ["terminal"]
        },
        "consumer": {
            "consumer_id": "fixture",
            "role": "worker",
            "authority_ceiling": "read_only",
            "operator_authorized": false,
            "memory_domains": ["system"],
            "data_classes": ["internal"],
            "permitted_egress": ["local_device"],
            "compute_node_refs": ["local"],
            "agent_ref": null
        },
        "lineage": {
            "objective_id": bindings.objective_id,
            "project_id": project,
            "task_id": bindings.leaf_id,
            "run_id": bindings.run_id,
            "parent_receipts": [],
            "session_ref": null
        },
        "return_contract": {
            "schema_version": "arda.organism-outcome.v1",
            "required_receipt_types": ["arda.context-use-receipt.v1"],
            "max_output_bytes": 4096
        },
        "evidence_refs": [],
        "memory_refs": [],
        "unresolved_failures": []
    }))
    .unwrap();
    let mut consumer = ConsumerContext::new("fixture", vec![MemoryDomain::System]);
    consumer.purpose = Some("fixture".into());
    let context = memory
        .assemble_organism_context(request, &consumer, u128::from(now - 120_000))
        .unwrap();
    let execute_context = memory
        .bind_run_stage_context(
            &context,
            "execute",
            "inspect",
            execute.parent_receipts.clone(),
            u128::from(now - 120_000),
        )
        .unwrap();
    let old_execute_digest = execute.receipt_digest.clone();
    execute.context_capsule_id = Some(execute_context.capsule.capsule_id.clone());
    execute.context_capsule_digest = Some(execute_context.capsule.capsule_digest.clone());
    execute.context_use_receipt_ref = Some(execute_context.use_receipt.receipt_ref());
    execute.receipt_digest = execute.computed_digest().unwrap();
    run.write_execution_receipt(
        &NodeId::new("execute").unwrap(),
        &serde_json::to_value(&execute).unwrap(),
    )
    .unwrap();
    let seeded_journal = std::fs::read_to_string(run.events_path()).unwrap();
    std::fs::write(
        run.events_path(),
        seeded_journal.replace(&old_execute_digest, &execute.receipt_digest),
    )
    .unwrap();
    let checkpoint = std::fs::read_to_string(run.checkpoint_path()).unwrap();
    std::fs::write(
        run.checkpoint_path(),
        checkpoint.replace(&old_execute_digest, &execute.receipt_digest),
    )
    .unwrap();
    let manifest = Manifest {
        version: 2,
        capability: "fixture-capability".into(),
        root: root.to_owned(),
        device: 1,
        inode: 1,
        topology_digest: "fixture".into(),
        runtime_bundle: Some(RuntimeBundle {
            version: 1,
            policy_digest: "fixture".into(),
            grants: vec![],
        }),
        admission_digest: Some("a".repeat(64)),
    };
    let snapshot = RetainedSnapshot {
        endpoint: root.join("worker.sock").to_str().unwrap().into(),
        capability: manifest.capability.clone(),
        manifest_digest: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&manifest).unwrap())
        ),
    };
    let transport = transport::start(root, &snapshot, manifest);
    std::env::set_var("ARDA_SNAPSHOT_KEEPER_SOCKET", root.join("keeper.sock"));
    std::env::set_var(
        "ARDA_RETAINED_HERMES_CONFIG_OVERRIDE",
        root.join("config/adapters/hermes-workbench.toml"),
    );
    let store = ObjectiveStore::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let envelope = json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "fixture-proposal",
            "approval_id": "fixture-approval",
            "ledger_writes": ["fixture-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-07-31T00:00:00Z"
        },
        "idempotency_key": "fixture-execute"
    });
    let execution: crate::objectives::LeafExecutionSpec = serde_json::from_value(json!({
        "objective": "fixture",
        "execution_prompt": "inspect",
        "verification_prompt": "verify",
        "review_prompt": "review",
        "approval_envelope": envelope,
        "objective_plan_receipt": "fixture-plan"
    }))
    .unwrap();
    let objective: NewObjective = serde_json::from_value(json!({
        "id": bindings.objective_id,
        "source_id": "fixture",
        "idempotency_key": "fixture",
        "operator_id": "operator:fixture",
        "text": "fixture",
        "priority": 1,
        "projects": [{
            "project_id": project,
            "contract_digest": contract_digest
        }],
        "leaves": [{
            "id": bindings.leaf_id,
            "project_id": project,
            "workspace_root": root,
            "authority": "read_only",
            "dependencies": [],
            "execution": execution
        }]
    }))
    .unwrap();
    store.create_authenticated_objective(objective, 1).unwrap();
    store
        .apply_control(
            &bindings.objective_id,
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator:fixture",
            2,
        )
        .unwrap();
    store
        .apply_control(
            &bindings.objective_id,
            ControlAction::Pause,
            "pause",
            "operator:fixture",
            3,
        )
        .unwrap();
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    db.execute(
        "UPDATE leaves SET stage='execute',attempt=1,execution_run_id=?1,context_bound=1 WHERE id=?2",
        params![bindings.run_id.as_str(), bindings.leaf_id],
    )
    .unwrap();
    let request_digest = format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({
                "objective_id": bindings.objective_id,
                "leaf_id": bindings.leaf_id,
                "project_id": project,
                "project_contract_digest": contract_digest,
                "workspace_root": root,
                "authority": "read_only",
                "execution": execution,
                "dependencies": []
            }))
            .unwrap()
        )
    );
    db.execute(
        "INSERT INTO resident_context_bindings(run_id,request_digest,assembly_json) VALUES (?1,?2,?3)",
        params![bindings.run_id.as_str(), request_digest, serde_json::to_string(&context).unwrap()],
    )
    .unwrap();
    db.execute(
        "INSERT INTO retained_workspace_snapshots VALUES (?1,?2,?3,1)",
        params![
            bindings.leaf_id,
            bindings.run_id.as_str(),
            serde_json::to_string(&snapshot).unwrap()
        ],
    )
    .unwrap();
    db.execute(
        "INSERT INTO lease_workspace_identities VALUES (?1,'fixture-identity')",
        [&bindings.leaf_id],
    )
    .unwrap();
    let before = std::fs::read(run.events_path()).unwrap();
    let execute_path = root
        .join("data/runs")
        .join(bindings.run_id.as_str())
        .join("execution-receipts/execute.json");
    let execute_before = std::fs::read(&execute_path).unwrap();
    use std::sync::Arc;
    let state = HarnessState {
        research_store_policy: super::super::ResearchStorePolicy::Isolated,
        harness_addr: "127.0.0.1:0".into(),
        child_pids: Arc::new(tokio::sync::RwLock::new(vec![])),
        service_names: Arc::new(vec![]),
        service_statuses: Arc::new(tokio::sync::RwLock::new(vec![])),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: super::super::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: super::super::DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: super::super::presence::HarnessPresenceState::default(),
        workbench_root: root.to_path_buf(),
        operator_id: "operator:fixture".into(),
    };
    let message = json!({
        "operator": {
            "operator_id": "operator:fixture",
            "authenticated": true,
            "authentication_method": "local_session",
            "authenticated_at": Utc::now().to_rfc3339()
        },
        "adapter_id": "hermes-cli",
        "event": {
            "text": format!(
                "arda recover-retained {} {} {} one-verify-start 30m reuse-expired-context",
                bindings.objective_id, bindings.leaf_id, bindings.run_id.as_str()
            ),
            "message_type": "text",
            "user_id": "operator:fixture",
            "user_name": "fixture",
            "source": {
                "platform": "cli",
                "chat_id": "fixture-session",
                "chat_type": "private",
                "thread_id": null,
                "message_id": "unfinished-recovery"
            },
            "message_id": "unfinished-recovery",
            "media_urls": [],
            "media_types": [],
            "timestamp": Utc::now().to_rfc3339(),
            "prompt_response": null
        }
    });
    let socket = root.join("keeper.sock");
    let recovery_jobs = super::super::RecoveryJobs::default();
    let invoke = |authorized: bool| {
        let state = state.clone();
        let message = message.clone();
        let socket = socket.clone();
        let recovery_jobs = recovery_jobs.clone();
        async move {
            let mut headers = HeaderMap::new();
            if authorized {
                headers.insert("x-arda-local-capability", "fixture-token".parse().unwrap());
            }
            ingest_local_operator_message(
                State(state),
                ConnectInfo("127.0.0.1:1234".parse().unwrap()),
                Some(axum::Extension(super::super::RuntimePrerequisites {
                    recovery_jobs,
                    keeper_socket: Some(socket),
                })),
                headers,
                Json(serde_json::from_value(message).unwrap()),
            )
            .await
        }
    };
    assert!(invoke(false).await.is_err());
    assert_eq!(transport.chats.load(Ordering::SeqCst), 0);
    let http_stop = crate::supervisor::Shutdown::new();
    let (http_server, http_client) = if http_disconnect {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = super::super::router(state.clone(), Default::default()).layer(axum::Extension(
            super::super::RuntimePrerequisites {
                keeper_socket: Some(socket.clone()),
                recovery_jobs: recovery_jobs.clone(),
            },
        ));
        let stop = http_stop.clone();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .with_graceful_shutdown(async move { stop.wait().await })
            .await
            .unwrap();
        });
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        let body = serde_json::to_vec(&message).unwrap();
        let headers = format!("POST /v1/operator/local-messages HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nx-arda-local-capability: fixture-token\r\nContent-Length: {}\r\n\r\n", body.len());
        client.write_all(headers.as_bytes()).await.unwrap();
        client.write_all(&body).await.unwrap();
        client.flush().await.unwrap();
        (Some(server), Some(client))
    } else {
        (None, None)
    };
    let request = invoke(true);
    let mut waiter_task = tokio::spawn(async move {
        if http_disconnect {
            std::future::pending().await
        } else {
            request.await
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::select! {
            result = &mut waiter_task => panic!("ingress returned before first Chat: {result:?}"),
            _ = transport.drop_started.notified() => {}
        }
    })
    .await
    .expect("worker never accepted first Chat");
    let started_events = run.recover().unwrap().events;
    let start = started_events.last().unwrap();
    assert_eq!(start.node_id.as_str(), "verify");
    assert!(matches!(
        start.kind,
        RunEventKind::NodeTransition {
            state: arda_core::run_graph::NodeState::Running
        }
    ));
    let start_key = start.idempotency_key.clone();
    if mode.starts_with("waiter-loss-crash-") {
        std::fs::write(
            root.join("restart-message.json"),
            serde_json::to_vec(&message).unwrap(),
        )
        .unwrap();
        let handoff =
            std::path::PathBuf::from(std::env::var_os("ARDA_RECOVERY_CRASH_HANDOFF").unwrap());
        std::fs::write(handoff.join("root"), root.as_os_str().as_encoded_bytes()).unwrap();
        if mode == "waiter-loss-crash-before-outcome" {
            std::process::exit(77); // No destructors or runtime drain.
        }
    }
    let error_key = start_key.replace(":provider-running:", ":provider-error:");
    assert!(super::super::runs::provider_is_registered(bindings.run_id.as_str(), "verify").await);
    // A concurrent caller subscribes without acquiring another lease or Chat.
    let mut duplicate = Box::pin(invoke(true));
    assert!(futures::poll!(&mut duplicate).is_pending());
    // Dropping a JoinHandle only detaches it. Abort and join to prove the
    // response future has actually been destroyed before releasing the worker.
    assert!(!waiter_task.is_finished());
    let waiter_task = if !fenced {
        waiter_task.abort();
        assert!(waiter_task.await.unwrap_err().is_cancelled());
        None
    } else {
        Some(waiter_task)
    };
    // HTTP/1.1: full Content-Length body was sent before first Chat; close
    // both TCP directions without reading a response. This is not an RST test.
    drop(http_client);
    drop(duplicate);
    let started_bytes = std::fs::read(run.events_path()).unwrap();
    if successful_result {
        transport.worker_go.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !publication_barrier.path().join("arrived").exists() {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(
            &std::fs::read(publication_barrier.path().join("arrived")).unwrap(),
        )
        .unwrap();
        assert!(
            payload.get("receipt").is_some(),
            "not a successful adapter result: {payload}"
        );
        assert_eq!(payload["start_key"], start_key);
    }
    match fence_mode {
        "waiter-loss-cancel" | "waiter-loss-pause" => {
            store
                .apply_control(
                    &bindings.objective_id,
                    if fence_mode == "waiter-loss-cancel" {
                        ControlAction::Cancel
                    } else {
                        ControlAction::Pause
                    },
                    "post-dispatch-stop",
                    "operator:fixture",
                    chrono::Utc::now().timestamp_millis(),
                )
                .unwrap();
        }
        "waiter-loss-lease-expiry" => {
            let (expiry, intent): (i64, i64) = db.query_row(
                "SELECT l.lease_expires_ms,i.lease_expires_ms FROM leaves l JOIN retained_snapshot_lease_intents i ON i.leaf_id=l.id AND i.generation=l.attempt WHERE l.id=?1",
                [&bindings.leaf_id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            assert_eq!(expiry, intent);
            let event_id: String = db.query_row("SELECT recovery_event_id FROM retained_snapshot_lease_intents WHERE leaf_id=?1 AND generation=2", [&bindings.leaf_id], |r| r.get(0)).unwrap();
            store
                .with_recorded_recovery_admission("operator:fixture", &event_id, |_, grant| {
                    assert!(grant.is_active((expiry + 1) as u64));
                    Ok(())
                })
                .unwrap();
            std::env::set_var("ARDA_TEST_RECOVERY_NOW_MS", (expiry + 1).to_string());
        }
        "waiter-loss-supersession" => {
            assert_eq!(db.execute("UPDATE leaves SET lease_owner='replacement-owner',attempt=attempt+1 WHERE id=?1", [&bindings.leaf_id]).unwrap(), 1);
        }
        "waiter-loss-grant-expiry" => {
            let event_id: String = db.query_row("SELECT recovery_event_id FROM retained_snapshot_lease_intents WHERE leaf_id=?1 AND generation=2", [&bindings.leaf_id], |r| r.get(0)).unwrap();
            let expiry = store
                .with_recorded_recovery_admission("operator:fixture", &event_id, |_, grant| {
                    Ok(grant.expires_at_unix_ms)
                })
                .unwrap();
            std::env::set_var("ARDA_TEST_RECOVERY_GRANT_NOW_MS", (expiry + 1).to_string());
        }
        "waiter-loss-shutdown" => recovery_jobs.shutdown().trigger(),
        _ => {}
    }
    if evidence_error {
        // A real filesystem failure, after dispatch but before outcome write.
        std::fs::write(
            root.join("data/runs/recovery-evidence-fixture/recovery-outcomes"),
            b"not a directory",
        )
        .unwrap();
    }
    if successful_result {
        std::fs::write(publication_barrier.path().join("release"), b"go").unwrap();
    } else {
        transport.worker_go.notify_one();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            transport.drop_closed.notified(),
        )
        .await
        .unwrap();
    }
    // Drop every subscription. The retained join, not a response future or a
    // sleep, is our independent persistence/cleanup barrier.
    let outcomes = tokio::time::timeout(std::time::Duration::from_secs(5), recovery_jobs.settle())
        .await
        .expect("owner did not finish");
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].is_err());
    if let Some(waiter_task) = waiter_task {
        assert!(waiter_task.await.unwrap().is_err());
    }
    if mode == "waiter-loss-crash-after-outcome" {
        std::process::exit(77);
    }
    assert!(!super::super::runs::provider_is_registered(bindings.run_id.as_str(), "verify").await);
    if fenced {
        assert_eq!(std::fs::read(&execute_path).unwrap(), execute_before);
        assert_eq!(std::fs::read(run.events_path()).unwrap(), started_bytes);
        let outcome = run
            .read_recovery_outcome_evidence(&NodeId::new("verify").unwrap(), &start_key)
            .unwrap()
            .unwrap();
        assert_eq!(outcome["start_key"], start_key);
        if successful_result {
            assert!(outcome.get("receipt").is_some());
        } else {
            assert_eq!(outcome["error_key"], error_key);
            if fence_mode == "waiter-loss-shutdown" {
                assert_eq!(outcome["error"], "Hermes process was cancelled");
            } else {
                assert!(
                    outcome["error"].as_str().unwrap().contains("71"),
                    "{outcome}"
                );
            }
        }
        let outcome_dir = root.join("data/runs/recovery-evidence-fixture/recovery-outcomes/verify");
        let files: Vec<_> = std::fs::read_dir(&outcome_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);
        let outcome_bytes = std::fs::read(&files[0]).unwrap();
        let reconnect = invoke(true)
            .await
            .expect_err("fenced reconnect succeeded");
        use axum::response::IntoResponse;
        let response = reconnect.into_response();
        assert_eq!(
            response.status(),
            if fence_mode == "waiter-loss-shutdown" {
                axum::http::StatusCode::SERVICE_UNAVAILABLE
            } else {
                axum::http::StatusCode::CONFLICT
            }
        );
        let body = axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap();
        let refusal: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let expected = match fence_mode {
            "waiter-loss-cancel" | "waiter-loss-pause" => "retained recovery rejected: recovery control was superseded or canonical lineage changed",
            "waiter-loss-lease-expiry" => "retained recovery rejected: recovery retained lease is missing, fenced, expired or released",
            "waiter-loss-grant-expiry" => "retained recovery rejected: recovery window is not active",
            "waiter-loss-supersession" => "retained recovery rejected: Query returned no rows",
            "waiter-loss-shutdown" => "Harness is stopping; durable run evidence is retained",
            _ => unreachable!(),
        };
        assert_eq!(refusal["message"], expected);
        assert_eq!(std::fs::read(&files[0]).unwrap(), outcome_bytes);
        assert_eq!(std::fs::read_dir(&outcome_dir).unwrap().count(), 1);
        assert_eq!(std::fs::read(run.events_path()).unwrap(), started_bytes);
        assert_eq!(std::fs::read(&execute_path).unwrap(), execute_before);
        assert_eq!(
            run.read_recovery_outcome_evidence(&NodeId::new("verify").unwrap(), &start_key)
                .unwrap()
                .unwrap(),
            outcome
        );
        for node in ["verify", "review", "close"] {
            assert!(run
                .read_execution_receipt(&NodeId::new(node).unwrap())
                .unwrap()
                .is_none());
        }
        assert_eq!(transport.chats.load(Ordering::SeqCst), 1);
        assert_eq!(
            transport.exports.load(Ordering::SeqCst),
            usize::from(successful_result)
        );
        assert_eq!(transport.releases.load(Ordering::SeqCst), 0);
        assert_eq!(
            db.query_row("SELECT count(*) FROM recovery_publications", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        transport.stop().await;
        return;
    }
    if evidence_error {
        let error = outcomes.into_iter().next().unwrap().err().unwrap();
        assert!(format!("{error:?}").contains("directory"), "{error:?}");
        assert_eq!(run.recover().unwrap().events.len(), started_events.len());
        assert_eq!(std::fs::read(&execute_path).unwrap(), execute_before);
        assert!(invoke(true).await.is_err());
        assert_eq!(transport.chats.load(Ordering::SeqCst), 1);
        assert_eq!(transport.exports.load(Ordering::SeqCst), 0);
        assert_eq!(transport.releases.load(Ordering::SeqCst), 0);
        assert_eq!(
            db.query_row("SELECT count(*) FROM recovery_publications", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        transport.stop().await;
        return;
    }
    // Independent owner persisted the exact-start error.
    assert_eq!(transport.chats.load(Ordering::SeqCst), 1);
    assert_eq!(transport.exports.load(Ordering::SeqCst), 0);
    assert_eq!(transport.releases.load(Ordering::SeqCst), 0);
    let spent = std::fs::read(run.events_path()).unwrap();
    assert!(spent.starts_with(&before));
    let last: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&spent).unwrap().lines().last().unwrap()).unwrap();
    assert_eq!(last["node_id"], "verify");
    assert_eq!(last["kind"]["type"], "node_transition");
    assert_eq!(last["kind"]["state"], "failed");
    let recovered = run.recover().unwrap();
    assert_eq!(recovered.events.len(), started_events.len() + 1);
    for node in ["verify", "review", "close"] {
        assert!(run
            .read_execution_receipt(&NodeId::new(node).unwrap())
            .unwrap()
            .is_none());
    }
    // Recovery outcome evidence written with the synthetic error.
    let outcome_dir = root.join("data/runs/recovery-evidence-fixture/recovery-outcomes/verify");
    assert!(outcome_dir.exists());
    let outcome_entries: Vec<_> = std::fs::read_dir(&outcome_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(outcome_entries.len(), 1);
    let outcome_bytes = std::fs::read(outcome_entries[0].path()).unwrap();
    let outcome: serde_json::Value = serde_json::from_slice(&outcome_bytes).unwrap();
    assert_eq!(outcome["start_key"], start_key);
    assert_eq!(outcome["error_key"], error_key);
    assert_eq!(
        outcome["error"],
        crate::adapters::HermesAdapterError::ProcessFailed {
            code: Some(71),
            stderr: String::new(),
        }
        .to_string()
    );
    // Execute preserved.
    assert_eq!(std::fs::read(&execute_path).unwrap(), execute_before);
    // A reconnect consults saved exact-start evidence, not another dispatch.
    assert!(invoke(true).await.is_err());
    assert_eq!(std::fs::read(run.events_path()).unwrap(), spent);
    assert_eq!(
        std::fs::read(outcome_entries[0].path()).unwrap(),
        outcome_bytes
    );
    assert_eq!(transport.chats.load(Ordering::SeqCst), 1);
    // No Review or additional Verify dispatched.
    assert_eq!(transport.exports.load(Ordering::SeqCst), 0);
    assert_eq!(
        db.query_row("SELECT count(*) FROM recovery_publications", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    if let Some(server) = http_server {
        http_stop.trigger();
        tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
    }
    transport.stop().await;
}
