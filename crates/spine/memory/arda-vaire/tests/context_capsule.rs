use arda_core::capability_composition::{
    CompositionAuthorityClass, DataClass, EgressTarget, RoleKind,
};
use arda_core::contract::{MemoryKind, MemoryRecord, MemoryState};
use arda_core::run_graph::{ObjectiveId, RunId};
use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
use arda_vaire::{
    ContextConsumer, ContextLineage, ContextObjective, ContextReturnContract, MnemosyneService,
    OrganismContext, CONTEXT_CAPSULE_SCHEMA_VERSION, CONTEXT_USE_RECEIPT_SCHEMA_VERSION,
};
use tempfile::TempDir;

fn context(now_ms: u128, memory_refs: Vec<String>) -> OrganismContext {
    OrganismContext {
        schema_version: OrganismContext::SCHEMA_VERSION.into(),
        organism_id: "arda:mythos:primary".into(),
        generated_at_unix_ms: now_ms,
        expires_at_unix_ms: now_ms + 60_000,
        consumer: ContextConsumer {
            consumer_id: "hermes:fresh-worker-1".into(),
            role: RoleKind::Worker,
            authority_ceiling: CompositionAuthorityClass::ExecuteWithApproval,
            operator_authorized: false,
            memory_domains: vec![MemoryDomain::System],
            data_classes: vec![DataClass::Internal],
            permitted_egress: vec![EgressTarget::LocalDevice],
            compute_node_refs: vec!["node:arda-root".into()],
            agent_ref: Some("hermes:worker-attempt-1".into()),
        },
        lineage: ContextLineage {
            objective_id: ObjectiveId::new("objective-context-bootstrap").unwrap(),
            project_id: None,
            run_id: Some(RunId::new("run-context-bootstrap").unwrap()),
            task_id: Some("digital-organism-s1-context-bootstrap".into()),
            session_ref: None,
            parent_receipts: vec!["receipt:operator-approval".into()],
        },
        objective: ContextObjective {
            requested_outcome: "Read the bounded constraints and report the next action.".into(),
            acceptance_conditions: vec![
                "name the objective and next action".into(),
                "do not claim access to prior conversation".into(),
            ],
            required_capabilities: vec!["bounded-context-read".into()],
            forbidden_capabilities: vec!["ambient-transcript-read".into()],
        },
        evidence_refs: vec!["arda://varda/evidence/context-bootstrap".into()],
        memory_refs,
        unresolved_failures: Vec::new(),
        return_contract: ContextReturnContract {
            schema_version: "arda.organism-outcome.v1".into(),
            required_receipt_types: vec![
                "arda.hermes-execution-receipt.v1".into(),
                "arda.context-use-receipt.v1".into(),
                "arda.handoff-receipt.v1".into(),
            ],
            max_output_bytes: 32_768,
        },
    }
}

fn service(temp: &TempDir) -> MnemosyneService {
    MnemosyneService::new(temp.path().join("vaire"))
        .unwrap()
        .with_contract_memory_root(temp.path().join("memory"))
}

fn consumer() -> ConsumerContext {
    let mut consumer = ConsumerContext::new("hermes:fresh-worker-1", vec![MemoryDomain::System]);
    consumer.purpose = Some("Read the bounded constraints and report the next action.".into());
    consumer
}

fn write_memory(service: &MnemosyneService, id: &str, content: &str) {
    let mut memory = MemoryRecord::new(id, MemoryKind::Semantic, "vaire", content);
    memory
        .extensions
        .insert("memory_domain".into(), serde_json::json!("system"));
    service
        .write_governed_memory(memory, Some(&consumer()))
        .unwrap();
}

#[test]
fn concurrent_stage_binding_appends_one_receipt() {
    let temp = TempDir::new().unwrap();
    let svc = service(&temp);
    let now = 1_787_340_000_000;
    let original = svc
        .assemble_organism_context(context(now, vec![]), &consumer(), now)
        .unwrap();
    let barrier = std::sync::Barrier::new(12);
    std::thread::scope(|scope| {
        let handles = (0..12)
            .map(|_| {
                let original = &original;
                let barrier = &barrier;
                let temp = &temp;
                scope.spawn(move || {
                    let svc = service(temp);
                    barrier.wait();
                    svc.bind_run_stage_context(
                        original,
                        "verify",
                        "verify",
                        vec!["receipt:execute".into()],
                        now + 1,
                    )
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }
    });
    let ledger =
        std::fs::read_to_string(temp.path().join("vaire/context_use_receipts.jsonl")).unwrap();
    assert_eq!(
        ledger.lines().count(),
        2,
        "one original and one stage receipt"
    );
}

#[test]
fn stage_context_preserves_snapshot_and_expiry_across_reopen() {
    let temp = TempDir::new().unwrap();
    let original_service = service(&temp);
    write_memory(&original_service, "mem-stage", "original context");
    let now = 1_787_340_000_000;
    let original = original_service
        .assemble_organism_context(context(now, vec!["mem-stage".into()]), &consumer(), now)
        .unwrap();
    let parents = vec!["receipt:execute".into()];
    let bound = original_service
        .bind_run_stage_context(
            &original,
            "verify",
            "verify result",
            parents.clone(),
            now + 1,
        )
        .unwrap();
    assert_eq!(bound.capsule.memories, original.capsule.memories);
    assert_eq!(
        bound.capsule.context.expires_at_unix_ms,
        original.capsule.context.expires_at_unix_ms
    );
    assert_eq!(bound.capsule.context.lineage.parent_receipts, parents);
    assert_eq!(
        bound.capsule.context.objective.requested_outcome,
        "verify result"
    );
    assert_ne!(
        bound.use_receipt.receipt_id,
        original.use_receipt.receipt_id
    );
    let path = temp.path().join("vaire/context_use_receipts.jsonl");
    let before = std::fs::read(&path).unwrap();
    let reopened = service(&temp);
    assert_eq!(
        reopened
            .bind_run_stage_context(
                &original,
                "verify",
                "verify result",
                parents.clone(),
                now + 2
            )
            .unwrap(),
        bound
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        original
            .for_run_stage("verify", "verify result", parents.clone())
            .unwrap(),
        bound
    );
    assert!(reopened
        .bind_run_stage_context(
            &original,
            "verify",
            "verify result",
            parents.clone(),
            now + 60_000
        )
        .is_err());
    write_memory(&reopened, "mem-stage", "corrected context");
    assert!(reopened
        .bind_run_stage_context(&original, "review", "review result", parents, now + 3)
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    reopened
        .validate_context_assembly_for_execution(&bound, now + 3)
        .unwrap_err();
}

#[test]
fn vaire_assembles_a_bounded_policy_filtered_capsule_and_use_receipt() {
    let temp = TempDir::new().unwrap();
    let service = service(&temp);
    write_memory(
        &service,
        "mem-next-action",
        "Next action: have a fresh Hermes worker report objective and constraints.",
    );
    let now_ms = 1_787_340_000_000;

    let assembled = service
        .assemble_organism_context(
            context(now_ms, vec!["mem-next-action".into()]),
            &consumer(),
            now_ms,
        )
        .expect("governed context assembly");

    assert_eq!(
        assembled.capsule.schema_version,
        CONTEXT_CAPSULE_SCHEMA_VERSION
    );
    assert_eq!(
        assembled.use_receipt.schema_version,
        CONTEXT_USE_RECEIPT_SCHEMA_VERSION
    );
    assert_eq!(assembled.capsule.memories.len(), 1);
    assert_eq!(assembled.capsule.memories[0].memory_id, "mem-next-action");
    assert_eq!(
        assembled.capsule.context.consumer.consumer_id,
        "hermes:fresh-worker-1"
    );
    assert_eq!(
        assembled.capsule.context.memory_refs,
        vec!["mem-next-action"]
    );
    assert_eq!(
        assembled.use_receipt.capsule_digest,
        assembled.capsule.capsule_digest
    );
    assert_eq!(assembled.use_receipt.memory_refs, vec!["mem-next-action"]);
    assert!(assembled.capsule.capsule_digest.starts_with("sha256:"));
    assert!(assembled.use_receipt.receipt_digest.starts_with("sha256:"));

    let wire = serde_json::to_string(&assembled).unwrap();
    assert!(!wire.contains("\"transcript\":"));
    assert!(!wire.contains("\"session_id\":"));
}

#[test]
fn cached_capsule_rechecks_current_memory_authority_without_new_receipts() {
    let temp = TempDir::new().unwrap();
    let service = service(&temp);
    write_memory(&service, "mem-cached", "original content");
    let now = 1_787_340_000_000;
    let assembly = service
        .assemble_organism_context(context(now, vec!["mem-cached".into()]), &consumer(), now)
        .unwrap();
    let receipt_path = temp.path().join("vaire/context_use_receipts.jsonl");
    let before = std::fs::read(&receipt_path).unwrap();
    service
        .validate_context_assembly_for_execution(&assembly, now + 1)
        .unwrap();
    assert!(service
        .validate_context_assembly_for_execution(&assembly, now + 60_000)
        .is_err());
    let mut revoked = service
        .recall_governed_memories(Some(&consumer()))
        .unwrap()
        .remove(0);
    revoked.state = MemoryState::Revoked;
    service
        .write_governed_memory(revoked, Some(&consumer()))
        .unwrap();
    assert!(
        service
            .validate_context_assembly_for_execution(&assembly, now + 2)
            .is_err(),
        "cached content must not bypass revocation"
    );
    assert_eq!(std::fs::read(&receipt_path).unwrap(), before);
}

#[test]
fn capsule_rejects_revoked_or_out_of_scope_memory() {
    let temp = TempDir::new().unwrap();
    let service = service(&temp);
    write_memory(&service, "mem-stale", "stale next action");
    let mut revoked = service
        .recall_governed_memories(Some(&consumer()))
        .unwrap()
        .into_iter()
        .find(|memory| memory.id == "mem-stale")
        .unwrap();
    revoked.state = MemoryState::Revoked;
    // Persist through the canonical governed path rather than a parallel fixture store.
    service
        .write_governed_memory(revoked, Some(&consumer()))
        .unwrap();

    let error = service
        .assemble_organism_context(
            context(1_787_340_000_000, vec!["mem-stale".into()]),
            &consumer(),
            1_787_340_000_000,
        )
        .expect_err("revoked memory must fail closed");
    assert!(error.to_string().contains("mem-stale"));
}

#[test]
fn context_use_receipt_and_capsule_identity_survive_service_restart() {
    let temp = TempDir::new().unwrap();
    let now_ms = 1_787_340_000_000;
    let first = {
        let service = service(&temp);
        write_memory(&service, "mem-restart", "Next action survives restart.");
        service
            .assemble_organism_context(
                context(now_ms, vec!["mem-restart".into()]),
                &consumer(),
                now_ms,
            )
            .unwrap()
    };

    let restarted = service(&temp);
    let replay = restarted
        .assemble_organism_context(
            context(now_ms, vec!["mem-restart".into()]),
            &consumer(),
            now_ms + 1,
        )
        .expect("reassemble from durable authority after restart");
    let loaded = restarted
        .context_use_receipt(&first.use_receipt.receipt_id)
        .expect("read durable use receipt")
        .expect("receipt exists");

    assert_eq!(replay.capsule.capsule_id, first.capsule.capsule_id);
    assert_eq!(replay.capsule.capsule_digest, first.capsule.capsule_digest);
    assert_eq!(replay.use_receipt.receipt_id, first.use_receipt.receipt_id);
    assert_eq!(loaded, first.use_receipt);
}
