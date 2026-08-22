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
