//! Synthetic grants for isolated scheduling/adapter tests; never live authority.
use arda_core::run_graph::{NodeId, RunId};
use arda_engine::runs::{RecoveryBindings, RecoveryGrant, RECOVERY_WINDOW_MS};
use std::collections::BTreeMap;

pub fn grant(run: &RunId, verify: &NodeId, review: &NodeId, now: u64) -> RecoveryGrant {
    let digest = format!("sha256:{}", "a".repeat(64));
    RecoveryGrant {
        authenticated_event_id: "synthetic-recovery-event".into(),
        authenticated_payload_digest: digest.clone(),
        activated_at_unix_ms: now,
        expires_at_unix_ms: now + RECOVERY_WINDOW_MS,
        bindings: RecoveryBindings {
            objective_id: "synthetic-objective".into(),
            objective_revision: 1,
            stop_generation: 1,
            leaf_id: "synthetic-leaf".into(),
            run_id: run.clone(),
            failed_event_sequence: 17,
            failed_event_digest: digest.clone(),
            execute_receipt_digest: digest.clone(),
            project_contract_digest: digest.clone(),
            context_capsule_digest: digest.clone(),
            context_use_receipt_digest: digest.clone(),
            retained_authority_digest: digest,
            verify_node_id: verify.clone(),
            review_node_id: review.clone(),
            close_node_id: NodeId::new("close").unwrap(),
            prior_verify_starts: 2,
            provider_start_ceilings: BTreeMap::from([
                (verify.as_str().into(), 3),
                (review.as_str().into(), 2),
            ]),
        },
    }
}
