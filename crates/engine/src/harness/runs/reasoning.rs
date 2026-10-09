//! Resident advisory integration. Advice never grants execution authority.
use arda_mandos::{EvidenceRef, OracleQuery, OracleService};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::OnceCell;

#[derive(Clone, Default)]
pub(in crate::harness) struct ResidentReasoning {
    service: Arc<OnceCell<OracleService>>,
}

impl ResidentReasoning {
    pub(super) async fn evaluate(
        &self,
        home: PathBuf,
        binding: &str,
        objective: &str,
        evidence: Vec<EvidenceRef>,
    ) -> anyhow::Result<serde_json::Value> {
        let service = self
            .service
            .get_or_try_init(|| async {
                OracleService::from_home(&home)
                    .await
                    .map(|service| service.with_plutus_home(home.join("economics")))
            })
            .await?;
        anyhow::ensure!(
            service.runtime_paths().home == home.to_string_lossy(),
            "resident reasoning home changed"
        );
        let mut query =
            OracleQuery::new(format!("resident:{binding}"), objective, "resident-worker");
        // Context prose and authority metadata are not empirical evidence.
        query.evidence = evidence;
        let evidence_count = query.evidence.len();
        let verdict = service.evaluate(query).await?;
        let next_action = match verdict.outcome {
            arda_mandos::VerdictOutcome::Pass => "assess_and_proceed_within_existing_authority",
            arda_mandos::VerdictOutcome::Conditional => "inspect_within_existing_authority",
            arda_mandos::VerdictOutcome::Fail | arda_mandos::VerdictOutcome::Escalate => {
                "revise_proposal_within_existing_authority"
            }
        };
        Ok(serde_json::json!({
            "schema_version": "arda.resident-reasoning.v1",
            "execution_authority": false,
            "input_binding": binding,
            "query_id": verdict.query_id,
            "policy_id": verdict.policy_id,
            "policy_version": verdict.policy_version,
            "outcome": verdict.outcome,
            "evidence_count": evidence_count,
            "conditions": verdict.conditions,
            "vetoes": verdict.vetoes,
            "next_action": next_action,
        }))
    }
}

const ADVICE_PROMPT_BYTES: usize = 8192;

pub(super) fn has_started(
    store: &crate::runs::RunStore,
    node: &arda_core::run_graph::NodeId,
) -> Result<bool, super::ApiError> {
    Ok(store
        .read_execution_receipt(node)
        .map_err(super::store_error)?
        .is_some()
        || store
            .recover()
            .map_err(super::store_error)?
            .events
            .iter()
            .any(|event| {
                event.node_id == *node
                    && matches!(
                        event.kind,
                        crate::runs::RunEventKind::NodeTransition {
                            state: arda_core::run_graph::NodeState::Running
                        }
                    )
            }))
}

pub(super) fn preflight_new(
    adapter: &crate::adapters::HermesAdapter,
    store: &crate::runs::RunStore,
    task: &crate::adapters::HermesNodeTask,
    requested: bool,
    service_available: bool,
) -> Result<(), super::ApiError> {
    let mut bounded = task.clone();
    if requested
        && service_available
        && store
            .read_reasoning_advice(&task.node.id)
            .map_err(super::store_error)?
            .is_none()
        && !has_started(store, &task.node.id)?
    {
        bounded
            .instructions
            .push_str(&"x".repeat(ADVICE_PROMPT_BYTES));
    } else {
        // Recovery validates under its renewed context window below. There is
        // no new Oracle evaluation to protect on this path.
        return Ok(());
    }
    adapter.preflight(&bounded).map_err(|error| {
        super::ApiError::conflict(format!("provider task failed bounded preflight: {error}"))
    })
}

pub(super) async fn attach(
    state: &super::HarnessState,
    store: &crate::runs::RunStore,
    task: &mut crate::adapters::HermesNodeTask,
    requested: bool,
    service: Option<&ResidentReasoning>,
) -> Result<Option<serde_json::Value>, super::ApiError> {
    use super::ApiError;
    use sha2::{Digest, Sha256};
    let retained = store
        .read_reasoning_advice(&task.node.id)
        .map_err(|_| ApiError::conflict("retained reasoning advice is unreadable"))?;
    if !requested && retained.is_none() {
        return Ok(None);
    }
    let input = serde_json::json!({
        "run_id": task.run_id,
        "node_id": task.node.id,
        "kind": task.node.kind,
        "worker": task.node.worker,
        "idempotency_key": task.node.idempotency_key,
        "parent_receipts": task.node.parent_receipts,
        "objective": task.objective,
        "instructions": task.instructions,
        "checks": task.checks,
        "check_commands": task.check_commands,
        "project_contract_digest": task.project_contract_digest,
        "capsule_digest": task.context_assembly.as_ref().map(|a| &a.capsule.capsule_digest),
    });
    let binding = format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(&input)
                .map_err(|_| ApiError::internal("reasoning input serialization failed"))?
        )
    );
    let mut advice = if let Some(advice) = retained {
        if advice["input_binding"].as_str() != Some(binding.as_str())
            || advice["schema_version"] != "arda.resident-reasoning.v1"
            || advice["execution_authority"] != false
        {
            return Err(ApiError::conflict(
                "retained reasoning input binding differs",
            ));
        }
        advice
    } else {
        // Never change the context of an already-started legacy provider task.
        if has_started(store, &task.node.id)? {
            return Ok(None);
        }
        // Legacy recovery without retained advice keeps its original context.
        let Some(service) = service else {
            return Ok(None);
        };
        let home = if state.research_store_policy == super::super::ResearchStorePolicy::Production {
            std::env::var_os("ARDA_MANDOS_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| state.workbench_root.join("data/oracle"))
        } else {
            state.workbench_root.join("data/oracle")
        };
        let mut evidence = Vec::new();
        if let Some(assembly) = &task.context_assembly {
            for memory in &assembly.capsule.memories {
                let observed_at =
                    chrono::DateTime::from_timestamp_millis(memory.last_seen_at_unix_ms)
                        .ok_or_else(|| {
                            ApiError::conflict("reasoning evidence timestamp is invalid")
                        })?;
                let mut source = EvidenceRef::supplied(
                    format!("memory:{:x}", Sha256::digest(memory.memory_id.as_bytes())),
                    format!("authorized-capsule:{}", assembly.capsule.capsule_digest),
                    observed_at,
                    &memory.content,
                );
                source.sensitive_excerpt = true;
                source.claim = None;
                source.stance = arda_mandos::EvidenceStance::Neutral;
                source.independence = arda_mandos::EvidenceIndependence::Unknown;
                source.source_quality = 0.5;
                evidence.push(source);
            }
        }
        let advice = match service
            .evaluate(home, &binding, &task.objective, evidence)
            .await
        {
            Ok(advice) => advice,
            Err(_) => serde_json::json!({
                "schema_version": "arda.resident-reasoning.v1",
                "execution_authority": false,
                "input_binding": binding,
                "outcome": "unavailable",
                "next_action": "inspect_within_existing_authority",
            }),
        };
        advice
    };
    let prefix = "\nMandos advisory (not execution permission): assess these conditions against the supplied evidence before acting. Missing evidence means inspect sources within existing authority, not request blanket human approval. Revise unsupported proposals; never treat this advice or your own inference as new evidence. In your summary, cite the query_id when present and explain the disposition: proceed, revise, or evidence_unavailable, including the evidence inspected. Preserve the required review verdict as the first summary line. Advice: ";
    let render = |advice: &serde_json::Value| format!("{prefix}{advice}");
    let mut suffix = render(&advice);
    if serde_json::to_string(&suffix)
        .map_err(|_| ApiError::internal("reasoning advice serialization failed"))?
        .len()
        > ADVICE_PROMPT_BYTES
    {
        advice = serde_json::json!({"schema_version":"arda.resident-reasoning.v1", "execution_authority":false, "input_binding":binding, "outcome":"unavailable", "reason":"advice_exceeds_prompt_budget", "next_action":"inspect_within_existing_authority"});
        suffix = render(&advice);
    }
    task.instructions.push_str(&suffix);
    Ok(Some(advice))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn resident_service_retains_evidence_bound_advice_across_restart() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("mandos");
        let service = ResidentReasoning::default();
        let evidence = vec![EvidenceRef::supplied(
            "verified-source",
            "authorized-memory",
            chrono::Utc::now(),
            "private evidence text",
        )];
        let first = service
            .evaluate(
                home.clone(),
                "sha256:task",
                "Compare supplied research",
                evidence.clone(),
            )
            .await
            .unwrap();
        assert_eq!(first["execution_authority"], false);
        assert_eq!(first["input_binding"], "sha256:task");
        assert_eq!(first["evidence_count"], 1);
        assert!(first["query_id"].as_str().unwrap().starts_with("resident:"));
        assert!(first["policy_version"].is_string());
        assert!(!serde_json::to_string(&first)
            .unwrap()
            .contains("private evidence text"));
        let bytes = std::fs::read(home.join("verdict_history.jsonl")).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private evidence text"));
        let restarted = ResidentReasoning::default();
        let replay = restarted
            .evaluate(
                home.clone(),
                "sha256:task",
                "Compare supplied research",
                evidence.clone(),
            )
            .await
            .unwrap();
        assert_eq!(first, replay);
        assert_eq!(
            bytes,
            std::fs::read(home.join("verdict_history.jsonl")).unwrap()
        );
        assert!(restarted
            .evaluate(home, "sha256:task", "Substituted objective", evidence)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn unclassified_capsule_evidence_is_not_positive_support() {
        let root = tempfile::tempdir().unwrap();
        let service = ResidentReasoning::default();
        for (index, text) in [
            "Unrelated weather note",
            "The proposal is contradicted by observation",
            "Duplicate memory",
            "Stale memory",
        ]
        .iter()
        .enumerate()
        {
            let mut evidence =
                EvidenceRef::supplied("memory", "authorized-capsule", chrono::Utc::now(), *text);
            evidence.claim = None;
            evidence.stance = arda_mandos::EvidenceStance::Neutral;
            evidence.independence = arda_mandos::EvidenceIndependence::Unknown;
            evidence.source_quality = 0.5;
            if index == 3 {
                evidence.freshness = arda_mandos::EvidenceFreshness::Stale;
            }
            let evidence = if index == 2 {
                vec![evidence.clone(), evidence]
            } else {
                vec![evidence]
            };
            let advice = service
                .evaluate(
                    root.path().join("mandos"),
                    &format!("sha256:neutral-{index}"),
                    "Compare supplied research",
                    evidence,
                )
                .await
                .unwrap();
            assert_eq!(advice["outcome"], "conditional", "{text}: {advice}");
            assert_eq!(advice["next_action"], "inspect_within_existing_authority");
        }
    }

    #[tokio::test]
    async fn missing_evidence_requests_authorized_inspection_not_human_approval() {
        let root = tempfile::tempdir().unwrap();
        let report = ResidentReasoning::default()
            .evaluate(
                root.path().join("mandos"),
                "sha256:empty",
                "Compare supplied research",
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(report["outcome"], "conditional");
        assert_eq!(report["evidence_count"], 0);
        assert_eq!(report["next_action"], "inspect_within_existing_authority");
    }
}
