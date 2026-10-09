//! Authenticated, append-only correction of a rejected independent leaf.
//! The original run and admission remain immutable; a successor needs approval.
use super::*;
use arda_vaire::ContextAssembly;

impl ObjectiveStore {
    pub(crate) fn has_critic_correction(&self, event_id: &str) -> Result<bool> {
        Ok(self.connection()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM critic_corrections WHERE event_id=?1)",
            [event_id],
            |row| row.get(0),
        )?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn admit_critic_correction(
        &self,
        root: &Path,
        operator_id: &str,
        event_id: &str,
        payload_digest: &str,
        objective_id: &str,
        leaf_id: &str,
        rejected_digest: &str,
        now_ms: i64,
    ) -> Result<String> {
        anyhow::ensure!(
            authority::normalize(&root.join("data/arda/objectives.sqlite3"))? == self.path,
            "correction root does not match canonical store"
        );
        validate_sha256(rejected_digest, "rejected critic digest")?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::super::abandonment::guards::reject_in(&tx, Some(objective_id), Some(leaf_id), None)?;
        let current = objective_in(&tx, objective_id)?.context("correction objective missing")?;
        anyhow::ensure!(
            current.operator_id == operator_id,
            "correction operator mismatch"
        );
        let bound: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM gateway_event_bindings WHERE event_id=?1 AND payload_digest=?2)", params![event_id,payload_digest], |r| r.get(0))?;
        anyhow::ensure!(bound, "correction requires authenticated event binding");
        if let Some((owner, payload, parent, objective, child, record)) = tx.query_row(
            "SELECT operator_id,payload_digest,parent_leaf_id,objective_id,child_leaf_id,record_json FROM critic_corrections WHERE event_id=?1", [event_id],
            |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?)),
        ).optional()? {
            let record: serde_json::Value = serde_json::from_str(&record)?;
            anyhow::ensure!(owner==operator_id && payload==payload_digest && parent==leaf_id && objective==objective_id && record["rejected_digest"]==rejected_digest, "correction replay conflict");
            return Ok(child);
        }
        anyhow::ensure!(
            current.state == ObjectiveState::Paused,
            "pause objective before critic correction"
        );
        let claim = Self::decode_claim(&tx, leaf_id)?;
        anyhow::ensure!(
            claim.objective_id == objective_id && claim.stage != LeafStage::Complete,
            "correction leaf lineage or terminal state mismatch"
        );
        let dependent: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaf_dependencies WHERE dependency_leaf_id=?1)",
            [leaf_id],
            |r| r.get(0),
        )?;
        anyhow::ensure!(
            !dependent,
            "critic correction of a depended-on leaf requires a newly reviewed dependency plan"
        );
        let other_active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM leaves l WHERE objective_id=?1 AND id!=?2 AND stage!='complete' AND NOT EXISTS(SELECT 1 FROM critic_corrections c WHERE c.parent_leaf_id=l.id))", params![objective_id,leaf_id], |r| r.get(0))?;
        anyhow::ensure!(
            !other_active,
            "finish or separately resolve other active leaves before correction"
        );
        let spent: i64 = tx.query_row(
            "SELECT attempt + attempt_offset FROM leaves WHERE id=?1",
            [leaf_id],
            |r| r.get(0),
        )?;
        anyhow::ensure!(
            spent > 0 && spent < MAX_OBJECTIVE_ATTEMPTS,
            "critic correction attempt budget exhausted"
        );
        let run_id = claim
            .execution_run_id
            .as_deref()
            .context("critic run missing")?;
        let (encoded_context, request_digest): (String,String) = tx.query_row(
            "SELECT assembly_json,request_digest FROM resident_context_bindings WHERE run_id=?1 AND deleted_by_operator_ms IS NULL", [run_id], |r| Ok((r.get(0)?,r.get(1)?)))?;
        let context: ContextAssembly = serde_json::from_str(&encoded_context)?;
        let item = crate::objectives::workbench::work_item(&claim, &context)?;
        let mut execution = claim
            .execution
            .clone()
            .context("critic execution missing")?;
        let expected = crate::objectives::request_binding::ResidentRequestBinding {
            objective_id,
            leaf_id,
            project_id: &item.project_id,
            project_contract_digest: &item.project_contract_digest,
            workspace_root: &claim.workspace_root,
            authority: &claim.authority,
            execution: &execution,
            dependencies: &claim.dependency_receipts,
        }
        .digest()?;
        anyhow::ensure!(
            request_digest == expected,
            "critic original request changed"
        );
        let evidence =
            crate::objectives::workbench::receipt_validation::rejected_chain(root, &item)?;
        anyhow::ensure!(
            evidence[2].receipt_digest == rejected_digest,
            "critic rejection changed"
        );
        let child = format!(
            "cc-{:x}",
            Sha256::digest(format!("{objective_id}\0{event_id}\0{rejected_digest}").as_bytes())
        );
        anyhow::ensure!(
            format!("objective-{objective_id}-leaf-{child}-attempt-1").len() <= 128,
            "critic correction execution identity exceeds run identifier limit"
        );
        let record = serde_json::json!({
            "schema_version":"arda.critic-correction.v1", "decision":"revise_task",
            "event_id":event_id, "operator_id":operator_id, "objective_id":objective_id,
            "parent_leaf_id":leaf_id, "parent_run_id":run_id, "child_leaf_id":child,
            "rejected_digest":rejected_digest, "prior_attempts":spent, "recorded_at_ms":now_ms,
            "receipts":evidence,
        });
        let record_json = serde_json::to_string(&record)?;
        anyhow::ensure!(
            record_json.len() <= 131_072,
            "critic correction evidence exceeds budget"
        );
        let correction = format!("\n\nAuthenticated critic correction: Preserve the approved outcome and declared acceptance conditions. Correct the named blocking defects while preserving authority and unrelated work, then obtain fresh verification and independent acceptance. This correction supersedes instructions that produced the rejected artifact; do not repeat the rejected implementation. The immutable rejected artifact identities and receipt chain follow.\n{record_json}");
        execution.execution_prompt.push_str(&correction);
        execution.verification_prompt.push_str(&correction);
        execution.review_prompt.push_str(&correction);
        execution.objective_plan_receipt =
            format!("sha256:{:x}", Sha256::digest(record_json.as_bytes()));
        tx.execute("INSERT INTO leaves(id,objective_id,project_id,workspace_root,authority,execution_json,stage,attempt,attempt_offset,context_bound,updated_at_ms) VALUES(?1,?2,?3,?4,?5,?6,'execute',0,?7,0,?8)",
            params![child, objective_id, claim.project_id, claim.workspace_root, claim.authority, serde_json::to_string(&execution)?, spent, now_ms])?;
        tx.execute("INSERT INTO leaf_dependencies(leaf_id,dependency_leaf_id) SELECT ?1,dependency_leaf_id FROM leaf_dependencies WHERE leaf_id=?2", params![child,leaf_id])?;
        tx.execute(
            "INSERT INTO critic_corrections VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                event_id,
                payload_digest,
                operator_id,
                objective_id,
                leaf_id,
                child,
                record_json
            ],
        )?;
        tx.execute("UPDATE leaves SET stage='failed',current_receipt_digest=?1,lease_owner=NULL,lease_expires_ms=NULL,updated_at_ms=?2 WHERE id=?3", params![rejected_digest,now_ms,leaf_id])?;
        tx.execute("UPDATE objectives SET state='pending_approval',revision=revision+1,approved_revision=NULL,updated_at_ms=?1 WHERE id=?2",params![now_ms,objective_id])?;
        tx.commit()?;
        self.notify_change();
        Ok(child)
    }
}
