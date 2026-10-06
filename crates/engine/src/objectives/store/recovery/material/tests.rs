use super::*;
use crate::objectives::{ControlAction, NewObjective, SnapshotAdmission};
use crate::runs::RECOVERY_WINDOW_MS;
use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
use serde_json::json;

struct NoTransport(std::sync::atomic::AtomicBool);
impl SnapshotAdmission for NoTransport {
    fn prepare(&self, _: &str, _: &std::path::Path, _: &str) -> Result<RetainedSnapshot> {
        bail!("unexpected prepare")
    }
    fn commit(&self, _: &RetainedSnapshot, _: &str, _: i64, _: &str, _: i64) -> Result<()> {
        if self.0.swap(false, std::sync::atomic::Ordering::SeqCst) {
            bail!("fixture lost acknowledgement");
        }
        Ok(())
    }
    fn release(&self, _: &RetainedSnapshot, _: &str) -> Result<()> {
        bail!("unexpected release")
    }
}

#[test]
fn recovery_material_reads_canonical_stores_and_rejects_drift() {
    let (temp, run_store, _, mut bindings, _) = crate::runs::recovery_evidence::tests::fixture();
    let root = temp.path();
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let historical = now - 120_000;
    let memory = MnemosyneService::new(root.join("data/vaire"))
        .unwrap()
        .with_contract_memory_root(root.join("core/state/memory"));
    let request = serde_json::from_value(json!({
        "schema_version":arda_vaire::OrganismContext::SCHEMA_VERSION, "organism_id":"arda:fixture",
        "generated_at_unix_ms":historical,"expires_at_unix_ms":historical+60_000,
        "objective":{"requested_outcome":"fixture","acceptance_conditions":["cited inspection"],
            "required_capabilities":["file"],"forbidden_capabilities":["terminal"]},
        "consumer":{"consumer_id":"fixture","role":"worker","authority_ceiling":"read_only",
            "operator_authorized":false,"memory_domains":["system"],"data_classes":["internal"],
            "permitted_egress":["local_device"],"compute_node_refs":["local"],"agent_ref":null},
        "lineage":{"objective_id":bindings.objective_id,"project_id":null,"task_id":bindings.leaf_id,
            "run_id":bindings.run_id,"parent_receipts":[],"session_ref":null},
        "return_contract":{"schema_version":"arda.organism-outcome.v1",
            "required_receipt_types":["arda.context-use-receipt.v1"],"max_output_bytes":4096},
        "evidence_refs":[],"memory_refs":[],"unresolved_failures":[]
    })).unwrap();
    let mut consumer = ConsumerContext::new("fixture", vec![MemoryDomain::System]);
    consumer.purpose = Some("fixture".into());
    let context = memory
        .assemble_organism_context(request, &consumer, u128::from(historical))
        .unwrap();
    assert!(context.capsule.validate(u128::from(now)).is_err());
    let snapshot = RetainedSnapshot {
        endpoint: "fixture://not-a-live-worker".into(),
        capability: "synthetic-test-capability".into(),
        manifest_digest: format!("sha256:{}", "a".repeat(64)),
    };
    bindings.context_capsule_digest = context.capsule.capsule_digest.clone();
    bindings.context_use_receipt_digest = context.use_receipt.receipt_digest.clone();
    bindings.retained_authority_digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&snapshot).unwrap())
    );
    let grant = RecoveryGrant {
        authenticated_event_id: "fixture-material-event".into(),
        authenticated_payload_digest: format!("sha256:{}", "b".repeat(64)),
        bindings,
        activated_at_unix_ms: now,
        expires_at_unix_ms: now + RECOVERY_WINDOW_MS,
    };
    let b = &grant.bindings;
    let store = ObjectiveStore::open(root.join("objectives.sqlite3"))
        .unwrap()
        .with_snapshot_admission(std::sync::Arc::new(NoTransport(
            std::sync::atomic::AtomicBool::new(true),
        )));
    let objective: NewObjective = serde_json::from_value(json!({
        "id":b.objective_id,"source_id":"fixture","idempotency_key":"fixture",
        "operator_id":"operator:fixture","text":"fixture", "priority":1,
        "projects":[{"project_id":"project-a","contract_digest":b.project_contract_digest,"authority":"operator_test","checks":["build","lint"]}],
        "leaves":[{"id":b.leaf_id,"project_id":"project-a","workspace_root":root.to_str().unwrap(),
            "authority":"read_only","dependencies":[],"execution":null}]
    }))
    .unwrap();
    store.create_authenticated_objective(objective, 1).unwrap();
    store
        .apply_control(
            &b.objective_id,
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator:fixture",
            2,
        )
        .unwrap();
    store
        .apply_control(
            &b.objective_id,
            ControlAction::Pause,
            "pause",
            "operator:fixture",
            3,
        )
        .unwrap();
    let db = store.connection().unwrap();
    let execution = json!({"objective":"fixture","execution_prompt":"inspect","verification_prompt":"verify",
        "review_prompt":"review","approval_envelope":{},"objective_plan_receipt":"fixture"});
    db.execute("UPDATE leaves SET stage='execute',attempt=1,execution_run_id=?1,context_bound=1,execution_json=?2 WHERE id=?3",
        params![b.run_id.as_str(),execution.to_string(),b.leaf_id]).unwrap();
    let execution_spec: crate::objectives::LeafExecutionSpec =
        serde_json::from_value(execution.clone()).unwrap();
    let request_digest = format!("sha256:{:x}",Sha256::digest(serde_json::to_vec(&json!({
        "objective_id":b.objective_id,"leaf_id":b.leaf_id,"project_id":"project-a",
        "project_contract_digest":b.project_contract_digest,"workspace_root":root.to_str().unwrap(),
        "authority":"read_only","execution":execution_spec,"dependencies":[],
    })).unwrap()));
    db.execute("INSERT INTO resident_context_bindings(run_id,request_digest,assembly_json) VALUES (?1,?2,?3)",
        params![b.run_id.as_str(),request_digest,serde_json::to_string(&context).unwrap()]).unwrap();
    db.execute(
        "INSERT INTO retained_workspace_snapshots VALUES (?1,?2,?3,1)",
        params![
            b.leaf_id,
            b.run_id.as_str(),
            serde_json::to_string(&snapshot).unwrap()
        ],
    )
    .unwrap();
    let check = || {
        store.with_recovery_control_fence("operator:fixture", &grant, |tx| {
            store.validate_retained_recovery_material(root, tx, &grant)
        })
    };
    let material = check().unwrap();
    assert_eq!(material.context, context);
    assert!(material.snapshot == snapshot);
    assert_eq!(material.graph.run_id, b.run_id);
    assert_eq!(material.execution.objective, "fixture");
    for (break_sql, restore_sql) in [
        ("UPDATE leaves SET execution_json=replace(execution_json,'fixture','changed')", "UPDATE leaves SET execution_json=replace(execution_json,'changed','fixture')"),
        ("UPDATE leaves SET workspace_root=workspace_root || '/changed'", "UPDATE leaves SET workspace_root=replace(workspace_root,'/changed','')"),
        ("UPDATE resident_context_bindings SET deleted_by_operator_ms=1", "UPDATE resident_context_bindings SET deleted_by_operator_ms=NULL"),
        ("UPDATE retained_workspace_snapshots SET committed_generation=2", "UPDATE retained_workspace_snapshots SET committed_generation=1"),
        ("UPDATE leaves SET authority='write'", "UPDATE leaves SET authority='read_only'"),
        ("INSERT INTO retained_snapshot_releases SELECT leaf_id FROM retained_workspace_snapshots", "DELETE FROM retained_snapshot_releases"),
    ] {
        db.execute_batch(break_sql).unwrap();
        assert!(check().is_err(),"{break_sql}");
        db.execute_batch(restore_sql).unwrap();
    }
    let mut changed = snapshot.clone();
    changed.capability.push_str("-changed");
    db.execute(
        "UPDATE retained_workspace_snapshots SET capability_json=?1",
        [serde_json::to_string(&changed).unwrap()],
    )
    .unwrap();
    assert!(check().is_err());
    db.execute(
        "UPDATE retained_workspace_snapshots SET capability_json=?1",
        [serde_json::to_string(&snapshot).unwrap()],
    )
    .unwrap();
    let mut wrong = grant.clone();
    wrong.bindings.context_use_receipt_digest = format!("sha256:{}", "f".repeat(64));
    assert!(store
        .with_recovery_control_fence("operator:fixture", &wrong, |tx| store
            .validate_retained_recovery_material(root, tx, &wrong))
        .is_err());
    // Exercise the real phased claim API against the full canonical validator.
    let validate = || {
        store.with_recovery_control_fence("operator:fixture", &grant, |tx| {
            store.validate_retained_recovery_material(root, tx, &grant)
        })
    };
    db.execute(
        "INSERT INTO lease_workspace_identities VALUES (?1,'fixture-identity')",
        [&b.leaf_id],
    )
    .unwrap();
    store
        .bind_gateway_event(
            &grant.authenticated_event_id,
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
        )
        .unwrap();
    let admitted = store
        .prepare_recovery_admission(
            root,
            "operator:fixture",
            &grant.authenticated_event_id,
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
            &b.objective_id,
            &b.leaf_id,
            b.run_id.as_str(),
        )
        .unwrap();
    assert_eq!(admitted.bindings, grant.bindings);
    let grant = admitted;
    let b = &grant.bindings;
    let replay = store
        .prepare_recovery_admission(
            root,
            "operator:fixture",
            &grant.authenticated_event_id,
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
            &b.objective_id,
            &b.leaf_id,
            b.run_id.as_str(),
        )
        .unwrap();
    assert_eq!(
        replay, grant,
        "saved admission must retain the original deadline"
    );
    for (owner, digest, leaf) in [
        (
            "another-operator",
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
            b.leaf_id.as_str(),
        ),
        ("operator:fixture", "sha256:wrong", b.leaf_id.as_str()),
        (
            "operator:fixture",
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
            "sibling-leaf",
        ),
    ] {
        assert!(store
            .prepare_recovery_admission(
                root,
                owner,
                &grant.authenticated_event_id,
                digest,
                &b.objective_id,
                leaf,
                b.run_id.as_str()
            )
            .is_err());
    }
    run_store.activate_recovery(grant.clone()).unwrap();
    let claim = || {
        store.claim_validated_retained_recovery(
            root,
            "operator:fixture",
            &grant.authenticated_event_id,
            "fixture-owner",
            60_000,
        )
    };
    assert!(claim().unwrap_err().to_string().contains("acknowledgement"));
    assert!(
        validate().is_err(),
        "unacknowledged material must not authorize dispatch"
    );
    let before = || {
        store.with_recovery_control_fence("operator:fixture", &grant, |tx| {
            store.validate_recovery_before_reconciliation(root, tx, &grant)
        })
    };
    assert!(before().is_ok());
    for (break_sql,restore_sql) in [
        ("UPDATE retained_snapshot_lease_intents SET lease_owner='wrong'", "UPDATE retained_snapshot_lease_intents SET lease_owner='fixture-owner'"),
        ("UPDATE retained_snapshot_lease_intents SET recovery_event_id=NULL", "UPDATE retained_snapshot_lease_intents SET recovery_event_id=(SELECT authenticated_event_id FROM recovery_admissions)"),
        ("UPDATE leaves SET attempt=3", "UPDATE leaves SET attempt=2"),
    ] {
        db.execute(break_sql,[]).unwrap();
        assert!(before().is_err(), "must reject unrelated pending acknowledgement");
        db.execute(restore_sql,[]).unwrap();
    }
    let recovered = claim().unwrap();
    assert_eq!(recovered.attempt, 2);
    assert!(validate().is_ok());
    let replayed = claim().unwrap();
    assert_eq!(
        (replayed.attempt, replayed.lease_expires_ms),
        (recovered.attempt, recovered.lease_expires_ms)
    );
    use crate::objectives::RecoveryAuthorization;
    let event_id = grant.authenticated_event_id.clone();
    use arda_aule::prometheus::autopilot::workbench_executor::ExplicitWorkspaceAuthorization;
    let authority = RecoveryAuthorization::new(
        root.to_owned(),
        store.clone(),
        "operator:fixture".into(),
        event_id.clone(),
        &recovered,
    )
    .unwrap();
    let item = authority.canonical_item().unwrap();
    assert!(item.read_only);
    assert_eq!(item.run_id, b.run_id.as_str());
    assert_eq!(item.context_assembly.as_ref().unwrap(), &context);
    authority.authorize(&item).unwrap();
    let checked_lease: crate::objectives::snapshot_protocol::Lease =
        serde_json::from_value(authority.provider_lease(&item).unwrap()).unwrap();
    assert_eq!(checked_lease.generation, recovered.attempt);
    assert_eq!(checked_lease.expires_ms, recovered.lease_expires_ms);
    let window = authority.recovery_window(&item).unwrap().unwrap();
    assert_eq!(window.event_id, event_id);
    assert_eq!(window.expires_at_unix_ms, grant.expires_at_unix_ms);
    for (field, value) in [
        ("read_only", json!(false)),
        ("objective", json!("changed")),
        ("verification_prompt", json!("changed")),
        ("context_assembly", serde_json::Value::Null),
        ("workspace_root", json!("/elsewhere")),
    ] {
        let mut changed = serde_json::to_value(&item).unwrap();
        changed[field] = value;
        let changed = serde_json::from_value(changed).unwrap();
        assert!(authority.authorize(&changed).is_err());
        assert!(authority.provider_lease(&changed).is_err());
        assert!(authority.recovery_window(&changed).is_err());
    }
    let mut wrong_claim = recovered.clone();
    wrong_claim.attempt += 1;
    assert!(RecoveryAuthorization::new(
        root.to_owned(),
        store.clone(),
        "operator:fixture".into(),
        event_id.clone(),
        &wrong_claim
    )
    .unwrap()
    .canonical_item()
    .is_err());
    db.execute(
        "UPDATE resident_context_bindings SET deleted_by_operator_ms=1",
        [],
    )
    .unwrap();
    assert!(authority.authorize(&item).is_err());
    db.execute(
        "UPDATE resident_context_bindings SET deleted_by_operator_ms=NULL",
        [],
    )
    .unwrap();
    authority.authorize(&item).unwrap();
    // Use the same payload builder and durable stage binder as Workbench.
    let mut verify_body = None;
    for stage in ["verify", "review"] {
        let graph = run_store
            .validate_recovery_run_evidence(&grant.bindings)
            .unwrap();
        let parents = graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == stage)
            .unwrap()
            .parent_receipts
            .clone();
        let projected = item
            .stage_context(root, stage, parents.clone())
            .unwrap()
            .unwrap();
        let mut body = item
            .provider_request_body(root, stage, Some(projected.clone()))
            .unwrap();
        body["expected_retained_lease"] = authority.provider_lease(&item).unwrap();
        body["recovery_event_id"] = json!(event_id);
        assert!(
            authority.validate_provider_request(stage, &body).is_err(),
            "a projected but unretained stage context is insufficient"
        );
        let bound = memory
            .bind_run_stage_context_for_recovery(
                item.context_assembly.as_ref().unwrap(),
                stage,
                &projected.use_receipt.purpose,
                parents,
                u128::try_from(chrono::Utc::now().timestamp_millis()).unwrap(),
                u128::from(grant.activated_at_unix_ms)..u128::from(grant.expires_at_unix_ms),
            )
            .unwrap();
        body["context_assembly"] = serde_json::to_value(bound).unwrap();
        authority.validate_provider_request(stage, &body).unwrap();
        assert_eq!(
            authority.prepare_provider_request(stage).unwrap(),
            body,
            "connected driver must construct the canonical bound provider request"
        );
        if stage == "verify" {
            verify_body = Some(body.clone());
        }
        for key in [
            "objective",
            "envelope",
            "context_assembly",
            "expected_retained_lease",
            "recovery_event_id",
        ] {
            let mut altered = body.clone();
            altered[key] = serde_json::Value::Null;
            assert!(
                authority
                    .validate_provider_request(stage, &altered)
                    .is_err(),
                "{stage}: {key}"
            );
        }
        let mut extra = body.clone();
        extra["run_variables"] = json!({"scope":"changed"});
        assert!(authority.validate_provider_request(stage, &extra).is_err());
        for forbidden in ["execute", "close", "approval", "another-node"] {
            assert!(authority
                .validate_provider_request(forbidden, &body)
                .is_err());
        }
        let sibling = if stage == "verify" {
            "review"
        } else {
            "verify"
        };
        assert!(authority.validate_provider_request(sibling, &body).is_err());
    }

    // No sleeps: validation starts with a live lease and ends exactly at expiry.
    let lease: crate::objectives::snapshot_protocol::Lease =
        serde_json::from_value(authority.provider_lease(&item).unwrap()).unwrap();
    let original_request: String = db
        .query_row(
            "SELECT request_digest FROM resident_context_bindings WHERE run_id=?1",
            [grant.bindings.run_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    for clock_fails in [false, true] {
        let mut samples = 0;
        let rejected = store.with_retained_recovery_clock(
            "operator:fixture", &event_id, &lease,
            |tx, _, _| {
                tx.execute("UPDATE resident_context_bindings SET request_digest='must-rollback' WHERE run_id=?1",
                    [grant.bindings.run_id.as_str()])?;
                Ok(())
            },
            || {
                samples += 1;
                if samples == 1 { return Ok(lease.expires_ms - 1); }
                if clock_fails { anyhow::bail!("fixture clock unavailable"); }
                Ok(lease.expires_ms)
            },
        );
        let message = rejected.unwrap_err().to_string();
        assert!(
            message.contains(if clock_fails {
                "clock unavailable"
            } else {
                "lease expired during validation"
            }),
            "{message}"
        );
        assert_eq!(samples, 2);
        let retained_request: String = db
            .query_row(
                "SELECT request_digest FROM resident_context_bindings WHERE run_id=?1",
                [grant.bindings.run_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained_request, original_request);
    }
    let mut samples = 0;
    store
        .with_retained_recovery_clock(
            "operator:fixture",
            &event_id,
            &lease,
            |_, _, _| Ok(()),
            || {
                samples += 1;
                Ok(lease.expires_ms - 1)
            },
        )
        .unwrap();
    assert_eq!(samples, 2);
    authority.authorize(&item).unwrap();

    let body = verify_body.unwrap();
    let retained = crate::objectives::RetainedExecution {
        snapshot: snapshot.clone(),
        lease: lease.clone(),
    };
    let draft = crate::runs::RunEventDraft {
        node_id: grant.bindings.verify_node_id.clone(),
        idempotency_key: "verify:provider-running:3".into(),
        kind: crate::runs::RunEventKind::NodeTransition {
            state: arda_core::run_graph::NodeState::Running,
        },
        receipt_digest: None,
    };
    let original_graph = run_store
        .validate_recovery_run_evidence(&grant.bindings)
        .unwrap();
    let expected_node = original_graph
        .nodes
        .iter()
        .find(|node| node.id == draft.node_id)
        .unwrap()
        .clone();
    let mut changed_graph = original_graph.clone();
    changed_graph
        .nodes
        .iter_mut()
        .find(|node| node.id == draft.node_id)
        .unwrap()
        .timeout_ms += 1;
    run_store.write_checkpoint(&changed_graph).unwrap();
    let rejection = authority
        .launch_provider_request_checked(
            &body,
            &retained,
            &run_store,
            draft.clone(),
            Some(&expected_node),
            || -> Result<()> { panic!("stale captured node authority dispatched") },
        )
        .unwrap_err();
    assert!(
        rejection.to_string().contains("node authority changed"),
        "{rejection:#}"
    );
    run_store.write_checkpoint(&original_graph).unwrap();
    let mut changed = body.clone();
    changed["objective"] = json!("tampered");
    assert!(authority
        .launch_provider_request(
            &changed,
            &retained,
            &run_store,
            draft.clone(),
            || -> Result<()> { panic!("tampered request dispatched") }
        )
        .is_err());
    let mut ready = draft.clone();
    ready.idempotency_key = "verify:recovery-ready".into();
    ready.kind = crate::runs::RunEventKind::NodeTransition {
        state: arda_core::run_graph::NodeState::Ready,
    };
    run_store.append(ready).unwrap();
    let copied_root = tempfile::TempDir::new().unwrap();
    let copied_store =
        crate::runs::RunStore::open(copied_root.path(), grant.bindings.run_id.clone()).unwrap();
    std::fs::copy(run_store.events_path(), copied_store.events_path()).unwrap();
    let sent = std::cell::Cell::new(false);
    let outcome = authority
        .launch_provider_request(&body, &retained, &run_store, draft.clone(), || {
            // Separate connection must not acquire the stop/lease write fence while
            // the synchronous launch callback owns it.
            db.busy_timeout(std::time::Duration::ZERO)?;
            assert!(db.execute_batch("BEGIN IMMEDIATE").is_err());
            sent.set(true);
            Ok(())
        })
        .unwrap();
    assert!(sent.get());
    assert!(matches!(
        outcome,
        crate::runs::AppendOutcome::Appended { .. }
    ));
    let copied_before = std::fs::read(copied_store.events_path()).unwrap();
    let captured = json!({"request": body, "start_key": "verify:provider-running:3",
        "attempt": 3, "error_key": "verify:provider-error:3", "error": "isolated fixture outcome"});
    run_store
        .write_recovery_outcome_evidence(&grant.bindings.verify_node_id, &captured)
        .unwrap();
    let publication = crate::objectives::RecoveryPublication {
        key: "verify:provider-running:3:terminal".into(),
        kind: "provider-finalization".into(),
        node_id: grant.bindings.verify_node_id.clone(),
        payload: captured.clone(),
    };
    let before_publication = std::fs::read(run_store.events_path()).unwrap();
    let mut samples = 0;
    let expired = store.with_retained_recovery_clock(
        "operator:fixture",
        &event_id,
        &lease,
        |tx, saved, _| {
            let guard = run_store.lock_recovery_mutation(
                saved,
                &publication.node_id,
                lease.expires_ms.try_into()?,
            )?;
            store.insert_recovery_publication_in(tx, saved, &lease, &publication)?;
            Ok(guard)
        },
        || {
            samples += 1;
            Ok(if samples == 1 {
                lease.expires_ms - 1
            } else {
                lease.expires_ms
            })
        },
    );
    assert!(expired.is_err());
    let count: i64 = db
        .query_row("SELECT count(*) FROM recovery_publications", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0, "expiry must roll back publication authority");
    assert_eq!(
        before_publication,
        std::fs::read(run_store.events_path()).unwrap()
    );
    assert!(run_store
        .read_execution_receipt(&publication.node_id)
        .unwrap()
        .is_none());
    assert_eq!(
        run_store
            .read_recovery_outcome_evidence(&publication.node_id, "verify:provider-running:3")
            .unwrap(),
        Some(captured.clone())
    );
    authority
        .commit_provider_publication(&publication.node_id, &body, captured.clone())
        .unwrap();
    let mut altered = captured.clone();
    altered["error"] = json!("altered outcome");
    assert!(authority
        .commit_provider_publication(&publication.node_id, &body, altered)
        .is_err());
    let effects = std::cell::Cell::new(0);
    let crash = authority.reconcile_publications(|guard, saved| {
        assert_eq!(saved, &publication);
        if matches!(
            guard.append(crate::runs::RunEventDraft {
                node_id: saved.node_id.clone(),
                idempotency_key: "fixture-publication-projection".into(),
                kind: crate::runs::RunEventKind::ResultProjected,
                receipt_digest: None,
            })?,
            crate::runs::AppendOutcome::Appended { .. }
        ) {
            effects.set(effects.get() + 1);
        }
        anyhow::bail!("injected crash after a committed effect")
    });
    assert!(crash.is_err());
    for (supersede, restore) in [
        (
            "UPDATE leaves SET attempt=attempt+1",
            "UPDATE leaves SET attempt=attempt-1",
        ),
        (
            "UPDATE retained_workspace_snapshots SET committed_generation=committed_generation+1",
            "UPDATE retained_workspace_snapshots SET committed_generation=committed_generation-1",
        ),
        (
            "UPDATE objectives SET stop_generation=stop_generation+1",
            "UPDATE objectives SET stop_generation=stop_generation-1",
        ),
    ] {
        db.execute(supersede, []).unwrap();
        assert!(store
            .reconcile_recovery_publications_clock(
                root,
                "operator:fixture",
                &event_id,
                |_, _| panic!("superseded publication produced an effect"),
                || Ok(i64::try_from(grant.expires_at_unix_ms).unwrap() + 1)
            )
            .is_err());
        db.execute(restore, []).unwrap();
    }
    let journal_before_replay = std::fs::read(run_store.events_path()).unwrap();
    store
        .reconcile_recovery_publications_clock(
            root,
            "operator:fixture",
            &event_id,
            |guard, saved| {
                assert_eq!(saved, &publication);
                assert!(matches!(
                    guard.append(crate::runs::RunEventDraft {
                        node_id: saved.node_id.clone(),
                        idempotency_key: "fixture-publication-projection".into(),
                        kind: crate::runs::RunEventKind::ResultProjected,
                        receipt_digest: None,
                    })?,
                    crate::runs::AppendOutcome::AlreadyApplied { .. }
                ));
                Ok(())
            },
            || Ok(i64::try_from(grant.expires_at_unix_ms).unwrap() + 1),
        )
        .unwrap();
    assert_eq!(
        journal_before_replay,
        std::fs::read(run_store.events_path()).unwrap()
    );
    authority
        .reconcile_publications(|_, _| panic!("applied publication ran again"))
        .unwrap();
    assert_eq!(effects.get(), 1);
    assert!(authority
        .launch_provider_request(
            &body,
            &retained,
            &copied_store,
            draft.clone(),
            || -> Result<()> { panic!("same-ID copied journal dispatched a second start") }
        )
        .is_err());
    assert_eq!(
        copied_before,
        std::fs::read(copied_store.events_path()).unwrap()
    );
    let _ =
        authority.launch_provider_request(&body, &retained, &run_store, draft, || -> Result<()> {
            panic!("durable start replay dispatched")
        });

    let guarded = run_store
        .lock_recovery_mutation(
            &grant,
            &grant.bindings.verify_node_id,
            lease.expires_ms.try_into().unwrap(),
        )
        .unwrap();
    let independent = crate::runs::RunStore::open(root, grant.bindings.run_id.clone()).unwrap();
    let cancelled_node = grant.bindings.verify_node_id.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        independent
            .append(crate::runs::RunEventDraft {
                node_id: cancelled_node,
                idempotency_key: "fixture-cancelled".into(),
                kind: crate::runs::RunEventKind::Cancelled {
                    reason: "fixture".into(),
                },
                receipt_digest: None,
            })
            .unwrap();
        done_tx.send(()).unwrap();
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert!(done_rx
        .recv_timeout(std::time::Duration::from_millis(100))
        .is_err());
    let checkpoint = guarded.recover().unwrap().checkpoint.unwrap();
    guarded.write_checkpoint(&checkpoint).unwrap();
    drop(guarded);
    done_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    writer.join().unwrap();
    // Model an unacknowledged publication at reopen after cancellation.
    db.execute("UPDATE recovery_publications SET applied_at_ms=NULL", [])
        .unwrap();
    assert!(store
        .reconcile_recovery_publications_clock(
            root,
            "operator:fixture",
            &event_id,
            |_, _| panic!("cancelled publication produced an effect"),
            || Ok(i64::try_from(grant.expires_at_unix_ms).unwrap() + 1)
        )
        .is_err());
    assert!(run_store
        .lock_recovery_mutation(
            &grant,
            &grant.bindings.verify_node_id,
            lease.expires_ms.try_into().unwrap()
        )
        .is_err());
    assert!(check().is_err());
    assert!(authority.authorize(&item).is_err());
    assert!(authority.canonical_item().is_err());
}
