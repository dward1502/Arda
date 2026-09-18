//! Terminal-only cleanup never obtains permission to project success or dispatch.
use super::*;
use crate::objectives::{ReceiptStage, RetainedSnapshot};
use crate::runs::RunStore;
use sha2::{Digest, Sha256};
use std::path::Path;

fn digest(value: &impl serde::Serialize) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}

struct Historical {
    identity: String,
    snapshot: RetainedSnapshot,
    applied: bool,
    stopped: bool,
}

type CompletionRow = (
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    i64,
    Option<i64>,
);

fn historical(
    tx: &Transaction<'_>,
    grant: &RecoveryGrant,
    operator: &str,
) -> Result<Option<Historical>> {
    let event = &grant.authenticated_event_id;
    let row: Option<CompletionRow> = tx.query_row(
        "SELECT kind,node_id,payload_json,payload_digest,grant_digest,lease_generation,lease_owner,lease_expires_ms,applied_at_ms
         FROM recovery_publications WHERE authenticated_event_id=?1 AND publication_key='completion'",
        [event], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).optional()?;
    let Some((
        kind,
        node,
        payload,
        payload_digest,
        grant_digest,
        generation,
        owner,
        expiry,
        applied,
    )) = row
    else {
        return Ok(None);
    };
    let publication = RecoveryPublication {
        key: "completion".into(),
        kind,
        node_id: arda_core::run_graph::NodeId::new(node)?,
        payload: serde_json::from_str(&payload)?,
    };
    anyhow::ensure!(
        publication.kind == "completion"
            && publication.node_id == grant.bindings.close_node_id
            && digest(&publication)? == payload_digest
            && digest(grant)? == grant_digest,
        "cleanup publication identity changed"
    );
    let b = &grant.bindings;
    let row:(String,i64,i64,Option<i64>,String)=tx.query_row(
        "SELECT s.capability_json,o.stop_generation,o.revision,o.approved_revision,o.state FROM leaves l
         JOIN objectives o ON o.id=l.objective_id
         JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
         JOIN retained_snapshot_lease_intents i ON i.leaf_id=l.id AND i.generation=l.attempt
         WHERE l.id=?1 AND l.objective_id=?2 AND o.operator_id=?3 AND l.execution_run_id=?4
         AND l.stage='complete' AND l.authority='read_only' AND l.lease_owner IS NULL AND l.lease_expires_ms IS NULL
         AND l.attempt=?5 AND s.committed_generation=?5 AND i.lease_owner=?6 AND i.lease_expires_ms=?7 AND i.recovery_event_id=?8",
        params![b.leaf_id,b.objective_id,operator,b.run_id.as_str(),generation,owner,expiry,event],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).context("cleanup retained identity changed")?;
    let snapshot: RetainedSnapshot = serde_json::from_str(&row.0)?;
    anyhow::ensure!(
        digest(&snapshot)? == b.retained_authority_digest,
        "cleanup snapshot authority changed"
    );
    let receipts: Vec<StageReceipt> =
        serde_json::from_value(publication.payload["receipts"].clone())?;
    anyhow::ensure!(
        receipts.iter().map(|r| r.stage).eq([
            ReceiptStage::Execute,
            ReceiptStage::Verify,
            ReceiptStage::Review,
            ReceiptStage::Close
        ]) && receipts[0].digest == b.execute_receipt_digest,
        "cleanup receipt chain changed"
    );
    let mut parent = None;
    for receipt in &receipts {
        super::super::validate_receipt(receipt)?;
        anyhow::ensure!(
            receipt.predecessor_digest == parent
                && receipt.binding_digest.as_deref()
                    == Some(receipt.computed_binding_digest()?.as_str()),
            "cleanup receipt binding changed"
        );
        let exact:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM stage_receipts WHERE leaf_id=?1 AND stage=?2
            AND contract=?3 AND digest=?4 AND predecessor_digest IS ?5 AND run_path=?6 AND provider=?7 AND model=?8
            AND started_at_ms=?9 AND completed_at_ms=?10 AND verdict=?11 AND context_outcome_receipt_id IS ?12
            AND context_outcome_receipt_digest IS ?13 AND binding_digest IS ?14)",
            params![b.leaf_id,receipt.stage.as_str(),receipt.contract,receipt.digest,receipt.predecessor_digest,receipt.run_path,
                receipt.provider,receipt.model,receipt.started_at_ms,receipt.completed_at_ms,receipt.verdict,
                receipt.context_outcome_receipt_id,receipt.context_outcome_receipt_digest,receipt.binding_digest],|r|r.get(0))?;
        anyhow::ensure!(exact, "cleanup SQL receipt changed");
        parent = Some(receipt.digest.clone());
    }
    let close: Option<String> = tx.query_row(
        "SELECT current_receipt_digest FROM leaves WHERE id=?1",
        [&b.leaf_id],
        |r| r.get(0),
    )?;
    anyhow::ensure!(close == parent, "cleanup terminal receipt changed");
    let stop = i64::try_from(b.stop_generation)?;
    anyhow::ensure!(
        row.1 >= stop && row.2 >= b.objective_revision,
        "cleanup controls regressed"
    );
    use crate::objectives::{ControlAction, ObjectiveState};
    let state = ObjectiveState::parse(&row.4).context("invalid cleanup objective state")?;
    anyhow::ensure!(
        row.3.is_none_or(|approved| approved <= row.2),
        "invalid cleanup objective control state"
    );
    let mut controls = tx.prepare(
        "SELECT action_json,stop_generation FROM controls
        WHERE objective_id=?1 AND operator_id=?2 ORDER BY rowid DESC",
    )?;
    let controls = controls
        .query_map(params![b.objective_id, operator], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let latest = controls
        .into_iter()
        .map(|(json, generation)| {
            Ok((
                serde_json::from_str::<crate::objectives::ControlAction>(&json)?,
                generation,
            ))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .find(|(action, _)| {
            !matches!(
                action,
                crate::objectives::ControlAction::Reprioritize { .. }
                    | crate::objectives::ControlAction::DeleteRecoveryContext { .. }
            )
        });
    let resumed = matches!(&latest, Some((ControlAction::Resume, generation)) if *generation==row.1)
        && if row.3 == Some(row.2) {
            matches!(
                state,
                ObjectiveState::Approved
                    | ObjectiveState::Running
                    | ObjectiveState::Completed
                    | ObjectiveState::Failed
            )
        } else {
            state == ObjectiveState::PendingApproval
        };
    let stopped = row.1 > stop || row.2 > b.objective_revision || resumed;
    if stopped {
        anyhow::ensure!(
            matches!(&latest, Some((action,generation)) if *generation==row.1 && match action {
                ControlAction::Pause => state==ObjectiveState::Paused,
                ControlAction::Cancel => state==ObjectiveState::Cancelled,
                ControlAction::Resume => resumed,
                ControlAction::Revise {..} => state==ObjectiveState::PendingApproval && row.3.is_none(),
                ControlAction::Approve {revision} => *revision==row.2 && row.3==Some(row.2) && matches!(state,ObjectiveState::Approved|ObjectiveState::Running|ObjectiveState::Completed|ObjectiveState::Failed),
                ControlAction::Reject => state == ObjectiveState::Cancelled,
                _ => false,
            }),
            "cleanup lacks matching recorded control"
        );
    }
    anyhow::ensure!(
        stopped || (state == ObjectiveState::Paused && row.3 == Some(row.2)),
        "cleanup controls changed without a monotone control record"
    );
    Ok(Some(Historical {
        identity: digest(&(
            grant_digest,
            payload_digest,
            generation,
            owner,
            expiry,
            &snapshot,
        ))?,
        snapshot,
        applied: applied.is_some(),
        stopped,
    }))
}

impl ObjectiveStore {
    /// Returns true only for a later control/cancellation, after terminal release.
    /// Suppression is committed before the RPC and never means rollback of an
    /// effect that might already have happened before a missing SQL ACK.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cleanup_stopped_recovery_completion(
        &self,
        root: &Path,
        operator: &str,
        event: &str,
        payload: &str,
        objective: &str,
        leaf: &str,
        run_id: &str,
    ) -> Result<bool> {
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(grant) = read_admission(&tx, operator, event)? else {
            return Ok(false);
        };
        anyhow::ensure!(
            grant.authenticated_payload_digest == format!("sha256:{payload}")
                && grant.bindings.objective_id == objective
                && grant.bindings.leaf_id == leaf
                && grant.bindings.run_id.as_str() == run_id,
            "cleanup replay target changed"
        );
        let Some(history) = historical(&tx, &grant, operator)? else {
            return Ok(false);
        };
        let run = RunStore::open(root, grant.bindings.run_id.clone())?;
        let terminal=run.with_recovery_cleanup_history(&grant,|cancelled| {
            let suppressed:Option<String>=tx.query_row("SELECT identity_digest FROM recovery_completion_suppressions WHERE authenticated_event_id=?1",[event],|r|r.get(0)).optional()?;
            if let Some(identity)=&suppressed {anyhow::ensure!(identity==&history.identity && !history.applied,"cleanup suppression identity changed");}
            if !history.stopped && !cancelled && suppressed.is_none() { return Ok(false) }
            // Historical receipt identity is required even when current run state
            // is cancelled; this is NOT a relaxation of success validation.
            for stage in ["execute","verify","review","close"] {
                let node=arda_core::run_graph::NodeId::new(stage)?;
                let actual:crate::adapters::HermesExecutionReceipt=serde_json::from_value(run.read_execution_receipt(&node)?.context("cleanup receipt missing")?)?;
                let expected:String=tx.query_row("SELECT digest FROM stage_receipts WHERE leaf_id=?1 AND stage=?2",params![leaf,stage],|r|r.get(0))?;
                anyhow::ensure!(actual.has_valid_digest()? && actual.receipt_digest==expected && actual.run_id==run_id && actual.node_id==stage
                    && actual.project_contract_digest==grant.bindings.project_contract_digest,"cleanup run receipt changed");
            }
            if !history.applied && suppressed.is_none() {
                tx.execute("INSERT INTO recovery_completion_suppressions VALUES (?1,'completion',?2,?3,?4)",
                    params![event,history.identity,if cancelled {"run-cancelled"} else {"later-control"},chrono::Utc::now().timestamp_millis()])?;
            }
            tx.commit()?;
            Ok(true)
        })?;
        if !terminal {
            return Ok(false);
        }
        // Re-resolve identity under the very writer transaction that performs
        // terminal-only release. Never call the general admission reconciler.
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let saved = read_admission(&tx, operator, event)?.context("cleanup admission missing")?;
        anyhow::ensure!(saved == grant, "cleanup admission changed before release");
        let current = historical(&tx, &saved, operator)?.context("cleanup completion missing")?;
        anyhow::ensure!(
            current.identity == history.identity,
            "cleanup identity changed before release"
        );
        let released: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM retained_snapshot_releases WHERE leaf_id=?1)",
            [leaf],
            |r| r.get(0),
        )?;
        if !released {
            self.snapshot_admission
                .as_deref()
                .context("cleanup keeper is not configured")?
                .release(&current.snapshot, run_id)?;
            tx.execute("INSERT INTO retained_snapshot_releases VALUES (?1)", [leaf])?;
        }
        tx.commit()?;
        Ok(true)
    }
}
