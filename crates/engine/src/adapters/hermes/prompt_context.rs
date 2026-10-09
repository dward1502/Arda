//! Worker prompt projection; canonical task authority remains outside rendering.
use super::*;

pub(super) fn project(
    task: &HermesNodeTask,
    project_root: &Path,
) -> Result<serde_json::Value, HermesAdapterError> {
    let context = serde_json::json!({
        "run_id": task.run_id,
        "node": task.node,
        "objective": task.objective,
        "instructions": task.instructions,
        "checks": task.checks,
        "check_commands": task.check_commands,
        "project_contract_digest": task.project_contract_digest,
        "project_root": project_root,
        "organism_context_projection": task.context_assembly.as_ref().map(|assembly| project_capsule(&assembly.capsule)).transpose()?,
        "context_use_receipt": task.context_assembly.as_ref().map(|assembly| &assembly.use_receipt),
    });
    Ok(context)
}

/// Excluded IDs are audit metadata, not worker evidence. Preserve their commitment
/// and count without spending prompt space enumerating inaccessible memories.
/// This is explicitly a projection, never a replacement canonical capsule.
fn project_capsule(
    capsule: &arda_vaire::OrganismContextCapsule,
) -> Result<serde_json::Value, HermesAdapterError> {
    let mut context = serde_json::to_value(&capsule.context)?;
    let excluded = context
        .as_object_mut()
        .expect("typed context object")
        .remove("excluded_refs")
        .expect("typed context exclusion list");
    Ok(serde_json::json!({
        "schema_version": "arda.worker-context-projection.v1",
        "source_capsule_id": capsule.capsule_id,
        "source_capsule_digest": capsule.capsule_digest,
        "context": context,
        "memories": capsule.memories,
        "excluded_memory_count": capsule.context.excluded_refs.len(),
        "excluded_memory_refs_digest": format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&excluded)?)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_exclusion_audit_does_not_crowd_out_complete_report_content() {
        let capsule: arda_vaire::OrganismContextCapsule = serde_json::from_value(serde_json::json!({
            "schema_version": "arda.organism-context-capsule.v1", "capsule_id": "capsule:fixture", "capsule_digest": "sha256:fixture",
            "context": {
                "schema_version": "arda.organism-context.v1", "organism_id": "arda:resident", "generated_at_unix_ms": 1, "expires_at_unix_ms": 2,
                "consumer": {"consumer_id":"fixture", "role":"worker", "authority_ceiling":"execute_with_approval", "operator_authorized":true, "memory_domains":["system"], "data_classes":["internal"], "permitted_egress":["local_device"], "compute_node_refs":[], "agent_ref":null},
                "lineage": {"objective_id":"fixture", "project_id":null,"run_id":"run", "task_id":null,"session_ref":null,"parent_receipts":[]},
                "objective": {"requested_outcome":"compare both reports", "acceptance_conditions":[], "required_capabilities":[], "forbidden_capabilities":[]},
                "evidence_refs":[], "memory_refs":["report"], "excluded_refs":(0..1000).map(|i|format!("excluded-{i}-{}", "x".repeat(64))).collect::<Vec<_>>(), "unresolved_failures":[],
                "return_contract":{"schema_version":"arda.context-return.v1","required_receipt_types":[],"max_output_bytes":65536}
            },
            "memories":[{"memory_id":"report", "domain":"system", "content":format!("{}TAIL", "evidence ".repeat(900)), "content_digest":"sha256:content", "source_agent":"vaire", "last_seen_at_unix_ms":1}]
        })).unwrap();
        let before = serde_json::to_value(&capsule).unwrap();
        let projected = project_capsule(&capsule).unwrap();
        assert_eq!(projected["memories"], before["memories"]);
        assert_eq!(projected["excluded_memory_count"], 1000);
        assert!(projected["context"].get("excluded_refs").is_none());
        assert_eq!(projected["source_capsule_digest"], before["capsule_digest"]);
        assert_eq!(serde_json::to_value(&capsule).unwrap(), before);
        let prompt = HermesAdapter::render_context(&projected, NodeKind::Inspect).unwrap();
        assert!(prompt.contains("TAIL"));
        assert!(
            crate::objectives::runtime_operation::RuntimeOperation::Chat {
                query: prompt,
                max_turns: 10,
                toolsets: vec!["file".into()],
                workspace_writable: false
            }
            .validate()
            .is_ok()
        );
        assert!(serde_json::to_vec(&before).unwrap().len() > 65536);
    }
}
