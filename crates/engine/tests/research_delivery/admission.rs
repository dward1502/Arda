//! Synthetic trusted-store fixtures exercise admission, not provider acceptance.
use super::*;

pub(super) async fn exercise(
    root: &TempDir,
    bound: std::net::SocketAddr,
    client: &reqwest::Client,
    original: &Value,
    crash_stage: &str,
) {
    let original_id = original["brief_id"].as_str().unwrap();
    let refusal = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "admit-empty",
            &format!("arda objective-from-brief {original_id} {PROJECT_ID} Inspect fixture"),
        ))
        .send()
        .await
        .unwrap();
    // Original publication was tampered by the reader test: do not admit it.
    assert!(!refusal.status().is_success());
    let db = root.path().join("data/arda/objectives.sqlite3");
    let conn = rusqlite::Connection::open(&db).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM objectives", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );

    let mut contract: Value = serde_json::from_str(include_str!(
        "../../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    let second_project = "550e8400-e29b-41d4-a716-446655440001";
    fs::create_dir_all(root.path().join("project-one")).unwrap();
    fs::create_dir_all(root.path().join("project-two")).unwrap();
    contract["workspace"]["root"] = json!("project-one");
    contract["runtime"]["adapter"] = json!("python3");
    contract["commands"] = json!([{"id":"test","program":"python3","args":["verify-context-bootstrap.py"],"working_dir":"."}]);
    contract["artifacts"] = json!([]);
    contract["permissions"]["filesystem"]["write"] = json!(false);
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract":contract,"envelope":mutation_envelope("attach-brief-fixture")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    contract["identity"]["project_id"] = json!(second_project);
    contract["identity"]["name"] = json!("second-admission-fixture");
    contract["workspace"]["root"] = json!("project-two");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract":contract,"envelope":mutation_envelope("attach-second-brief-fixture")}))
        .send().await.unwrap().error_for_status().unwrap();
    // Valid content-addressed envelopes must still fail the semantic ingress gate.
    // Each case gets a distinct event, so replay cannot mask the actual refusal.
    for case in [
        "empty",
        "not-ready",
        "low-confidence",
        "injection",
        "expired",
        "contradiction",
    ] {
        let (id, _, _) =
            crate::admitted_evidence::publish_with(root.path(), original, |body| match case {
                "empty" => body["citations"] = json!([]),
                "not-ready" => body["citations"][0]["policy_readiness"] = json!("unknown"),
                "low-confidence" => body["citations"][0]["confidence"] = json!(0.69),
                "injection" => body["citations"][0]["prompt_injection_detected"] = json!(true),
                "expired" => body["expires_at_utc"] = json!("2000-01-01T00:00:00Z"),
                "contradiction" => body["contradictions"] = json!(["conflicting evidence"]),
                _ => unreachable!(),
            });
        let response = client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(
                &format!("negative-{case}"),
                &format!("arda objective-from-brief {id} {PROJECT_ID} Inspect fixture"),
            ))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "{case}: {body}");
        assert!(
            body.contains("Brief evidence is not eligible for objective admission"),
            "{case}: {body}"
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM objectives", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0,
            "{case} admitted work"
        );
    }
    let (brief_id, raw, path) = crate::admitted_evidence::publish(root.path(), original);
    let message = gateway_message(
        "admit-valid-fixture",
        &format!(
            "arda objective-from-brief {brief_id} {PROJECT_ID},{second_project} Inspect fixture"
        ),
    );
    let config_dir = root.path().join("config/adapters");
    fs::create_dir_all(&config_dir).unwrap();
    let config = include_str!("../../../../config/adapters/hermes-workbench.toml");
    let low_budget = config.replace("max_prompt_bytes = 131072", "max_prompt_bytes = 1");
    assert_ne!(config, low_budget);
    fs::write(config_dir.join("hermes-workbench.toml"), low_budget).unwrap();
    let rejected = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .unwrap();
    assert!(
        !rejected.status().is_success(),
        "impossible rendered payload admitted"
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM objectives", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    fs::write(config_dir.join("hermes-workbench.toml"), config).unwrap();
    if std::env::var_os("ARDA_RETAINED_BUDGET_FIXTURE").is_some() {
        let retained = std::path::PathBuf::from(
            std::env::var_os("ARDA_HERMES_RETAINED_ADAPTER_CONFIG").unwrap(),
        );
        fs::write(
            &retained,
            config.replace("max_prompt_bytes = 131072", "max_prompt_bytes = 1"),
        )
        .unwrap();
        let rejected = client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&message)
            .send()
            .await
            .unwrap();
        assert!(
            !rejected.status().is_success(),
            "restrictive retained override was ignored"
        );
        let error = rejected.text().await.unwrap();
        assert!(
            error.contains(
                "Admitted evidence and fixed Hermes wrappers exceed the configured prompt budget"
            ),
            "{error}"
        );
        for table in ["objectives", "leaves", "objective_admissions"] {
            assert_eq!(
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        // Only retained capacity changes; the ordinary config already fits.
        fs::write(&retained, config).unwrap();
    }
    let response = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let response_body = response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {response_body}");
    let saved: String = conn
        .query_row("SELECT input_json FROM objective_admissions", [], |r| {
            r.get(0)
        })
        .unwrap();
    let saved: Value = serde_json::from_str(&saved).unwrap();
    let leaves = saved["leaves"].as_array().unwrap();
    assert_eq!(
        leaves.len(),
        3,
        "two project leaves and dependent synthesis"
    );
    assert_ne!(leaves[0]["workspace_root"], leaves[1]["workspace_root"]);
    assert_eq!(
        leaves[2]["dependencies"],
        json!([leaves[0]["id"], leaves[1]["id"]])
    );
    for leaf in leaves {
        for key in ["execution_prompt", "verification_prompt", "review_prompt"] {
            assert!(
                leaf["execution"][key].as_str().unwrap().contains(&raw),
                "{key} omitted original bytes"
            );
        }
    }
    let state: String = conn
        .query_row("SELECT state FROM objectives", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "pending_approval");
    let registry_path = root.path().join("data/workbench/projects.json");
    let registry = fs::read(&registry_path).unwrap();
    fs::remove_file(path).unwrap();
    fs::remove_file(config_dir.join("hermes-workbench.toml")).unwrap();
    fs::remove_file(root.path().join("data/workbench/projects.json")).unwrap();
    let replay = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .unwrap();
    assert!(
        replay.status().is_success(),
        "{}",
        replay.text().await.unwrap()
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM objectives", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    // Restore only runtime configuration/project authority, never source evidence.
    fs::write(registry_path, registry).unwrap();
    crate::context_provider::install_fake_hermes(root.path());
    use arda_engine::objectives::{ObjectiveRuntime, WorkbenchLeafExecution};
    let objective_id = saved["id"].as_str().unwrap();
    let store = ObjectiveStore::open(&db).unwrap();
    assert_eq!(
        store.objective(objective_id).unwrap().unwrap().state,
        ObjectiveState::PendingApproval
    );
    let task_id = leaves[0]["id"].as_str().unwrap();
    let approval = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("fixture-explicit-approval", &format!("arda approve-objective {task_id} {objective_id} operator accepts admitted evidence")))
        .send().await.unwrap();
    let status = approval.status();
    let body = approval.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(
        store.objective(objective_id).unwrap().unwrap().state,
        ObjectiveState::Approved
    );
    drop(store);
    let interrupted = super::admission_restart::interrupt(root, crash_stage).await;
    let run_id = interrupted.execution_run_id.as_deref().unwrap();
    let execute_path = root
        .path()
        .join("data/runs")
        .join(run_id)
        .join("execution-receipts/execute.json");
    let execute_receipt = fs::read(&execute_path).unwrap();
    let stage_names = ["execute", "verify", "review"];
    let prefix_len = stage_names
        .iter()
        .position(|stage| *stage == crash_stage)
        .unwrap()
        + 1;
    let receipt_dir = execute_path.parent().unwrap();
    let mut committed = Vec::new();
    for (index, stage) in stage_names.iter().enumerate() {
        let path = receipt_dir.join(format!("{stage}.json"));
        if index < prefix_len {
            committed.push((path.clone(), fs::read(path).unwrap()));
        } else {
            assert!(!path.exists(), "premature {stage} receipt");
        }
    }
    assert!(!receipt_dir.join("close.json").exists());
    let before = fs::read_to_string(
        std::path::Path::new(&interrupted.workspace_root).join("context-prompts.jsonl"),
    )
    .unwrap();
    let before_stages: Vec<_> = before
        .lines()
        .map(|line| {
            let capture: Value = serde_json::from_str(line).unwrap();
            let context = capture["prompt"]
                .as_str()
                .unwrap()
                .split_once("Canonical node context follows:\n")
                .unwrap()
                .1;
            let context = serde_json::Deserializer::from_str(context)
                .into_iter::<Value>()
                .next()
                .unwrap()
                .unwrap();
            assert_eq!(context["run_id"], run_id);
            context["node"]["id"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(before_stages, stage_names[..prefix_len]);
    let binding: String = conn
        .query_row(
            "SELECT assembly_json FROM resident_context_bindings WHERE run_id=?1",
            [run_id],
            |r| r.get(0),
        )
        .unwrap();
    let (recovered_bound, recovered_shutdown, recovered_handle) = crate::start_harness(root).await;
    for _ in 0..3 {
        // Reopen durable state and replace the resident executor between leaves.
        let store = ObjectiveStore::open(&db).unwrap();
        let executor = WorkbenchLeafExecution::with_adapter(root.path(),
            arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{recovered_bound}")).unwrap());
        let mut runtime =
            ObjectiveRuntime::new(store, executor, "admitted-evidence-worker", 1, 60_000);
        let result = runtime
            .run_round(
                Utc::now()
                    .timestamp_millis()
                    .max(interrupted.lease_expires_ms + 1),
            )
            .await;
        assert!(result.is_ok(), "{result:#?}");
        assert_eq!(result.unwrap().len(), 1);
    }
    let store = ObjectiveStore::open(&db).unwrap();
    assert_eq!(
        store.objective(objective_id).unwrap().unwrap().state,
        ObjectiveState::Completed
    );
    assert_eq!(fs::read(&execute_path).unwrap(), execute_receipt);
    for (path, bytes) in committed {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    let recovered_binding: String = conn
        .query_row(
            "SELECT assembly_json FROM resident_context_bindings WHERE run_id=?1",
            [run_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recovered_binding, binding);
    let recovered_run: String = conn
        .query_row(
            "SELECT execution_run_id FROM leaves WHERE id=?1",
            [&interrupted.leaf_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recovered_run, run_id);
    let mut canonical = std::collections::BTreeMap::<String, String>::new();
    for leaf in leaves {
        let run: String = conn
            .query_row(
                "SELECT execution_run_id FROM leaves WHERE id=?1",
                [leaf["id"].as_str().unwrap()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(canonical
            .insert(run, leaf["workspace_root"].as_str().unwrap().to_owned())
            .is_none());
    }
    let captures = ["project-one", "project-two"]
        .into_iter()
        .map(|project| {
            let workspace = root.path().join(project);
            let text = fs::read_to_string(workspace.join("context-prompts.jsonl")).unwrap();
            text.lines()
                .map(|line| (workspace.clone(), line.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .concat();
    let mut stages = std::collections::BTreeMap::<String, Vec<String>>::new();
    for (workspace, line) in captures {
        let captured: Value = serde_json::from_str(&line).unwrap();
        let prompt = captured["prompt"].as_str().unwrap();
        let context = prompt
            .split_once("Canonical node context follows:\n")
            .unwrap()
            .1;
        let context: Value = serde_json::Deserializer::from_str(context)
            .into_iter::<Value>()
            .next()
            .unwrap()
            .unwrap();
        assert!(context["objective"].as_str().unwrap().contains(&raw));
        assert_eq!(
            canonical.get(context["run_id"].as_str().unwrap()).unwrap(),
            workspace.to_str().unwrap()
        );
        stages
            .entry(context["run_id"].as_str().unwrap().to_owned())
            .or_default()
            .push(context["node"]["id"].as_str().unwrap().to_owned());
    }
    assert_eq!(stages.len(), 3);
    assert_eq!(
        stages.keys().collect::<Vec<_>>(),
        canonical.keys().collect::<Vec<_>>()
    );
    for stage_list in stages.values() {
        assert_eq!(stage_list, &["execute", "verify", "review"]);
    }
    let persisted: String = conn
        .query_row("SELECT input_json FROM objective_admissions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&persisted).unwrap(), saved);
    // Snapshot logical rows rather than SQLite file bytes (WAL/checkpoint noise).
    let canonical_rows = || {
        ["objectives", "leaves", "resident_context_bindings"].map(|table| {
            let mut statement = conn
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let columns = statement.column_count();
            statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|column| row.get::<_, rusqlite::types::Value>(column))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        })
    };
    let before_replay = canonical_rows();
    let mut replay_files = Vec::new();
    for (run, workspace) in &canonical {
        for entry in fs::read_dir(
            root.path()
                .join("data/runs")
                .join(run)
                .join("execution-receipts"),
        )
        .unwrap()
        {
            let path = entry.unwrap().path();
            replay_files.push((path.clone(), fs::read(path).unwrap()));
        }
        let path = std::path::Path::new(workspace).join("context-prompts.jsonl");
        replay_files.push((path.clone(), fs::read(path).unwrap()));
    }
    let replay = client
        .post(format!("http://{recovered_bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .unwrap();
    assert!(
        replay.status().is_success(),
        "{}",
        replay.text().await.unwrap()
    );
    recovered_shutdown.notify_waiters();
    assert_eq!(canonical_rows(), before_replay);
    let replayed_admission: String = conn
        .query_row("SELECT input_json FROM objective_admissions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&replayed_admission).unwrap(),
        saved
    );
    for (path, bytes) in replay_files {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(
        store.objective(objective_id).unwrap().unwrap().state,
        ObjectiveState::Completed
    );
    recovered_handle.await.unwrap();
}
