use super::*;
use crate::objectives::{ReceiptStage, RecoveryPublication, StageReceipt};

pub(super) fn check_pending_completion_fences(
    root: &std::path::Path,
    store: &ObjectiveStore,
    db: &rusqlite::Connection,
    grant: &RecoveryGrant,
) {
    let mut receipts = Vec::new();
    for (name, stage) in [
        ("execute", ReceiptStage::Execute),
        ("verify", ReceiptStage::Verify),
        ("review", ReceiptStage::Review),
        ("close", ReceiptStage::Close),
    ] {
        let mut receipt = db.query_row("SELECT contract,digest,predecessor_digest,run_path,provider,model,started_at_ms,completed_at_ms,verdict FROM stage_receipts WHERE leaf_id=?1 AND stage=?2",
            params![grant.bindings.leaf_id,name], |row| Ok(StageReceipt {
                stage,contract:row.get(0)?,digest:row.get(1)?,predecessor_digest:row.get(2)?,
                run_path:row.get(3)?,provider:row.get(4)?,model:row.get(5)?,started_at_ms:row.get(6)?,
                completed_at_ms:row.get(7)?,verdict:row.get(8)?,context_outcome_receipt_id:None,
                context_outcome_receipt_digest:None,binding_digest:None,
            })).unwrap();
        receipt.binding_digest = Some(receipt.computed_binding_digest().unwrap());
        db.execute(
            "UPDATE stage_receipts SET binding_digest=?1 WHERE leaf_id=?2 AND stage=?3",
            params![receipt.binding_digest, grant.bindings.leaf_id, name],
        )
        .unwrap();
        receipts.push(receipt);
    }
    let publication = RecoveryPublication {
        key: "completion".into(),
        kind: "completion".into(),
        node_id: grant.bindings.close_node_id.clone(),
        payload: json!({"receipts":receipts}),
    };
    let expiry: i64 = db
        .query_row(
            "SELECT lease_expires_ms FROM retained_snapshot_lease_intents WHERE leaf_id=?1",
            [&grant.bindings.leaf_id],
            |r| r.get(0),
        )
        .unwrap();
    fn hash(value: &impl serde::Serialize) -> String {
        format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(value).unwrap())
        )
    }
    db.execute("INSERT INTO recovery_publications(authenticated_event_id,publication_key,kind,node_id,payload_json,payload_digest,grant_digest,lease_generation,lease_owner,lease_expires_ms,authorized_at_ms)
        VALUES(?1,'completion','completion','close',?2,?3,?4,1,'fixture-owner',?5,1)",
        params![grant.authenticated_event_id,publication.payload.to_string(),hash(&publication),hash(grant),expiry]).unwrap();
    for (mutate, restore) in [
        (
            "UPDATE objectives SET state='cancelled'",
            "UPDATE objectives SET state='paused'",
        ),
        (
            "UPDATE objectives SET stop_generation=stop_generation+1",
            "UPDATE objectives SET stop_generation=stop_generation-1",
        ),
        (
            "UPDATE leaves SET attempt=attempt+1",
            "UPDATE leaves SET attempt=attempt-1",
        ),
        (
            "UPDATE retained_workspace_snapshots SET committed_generation=2",
            "UPDATE retained_workspace_snapshots SET committed_generation=1",
        ),
        (
            "UPDATE retained_snapshot_lease_intents SET lease_owner='superseded'",
            "UPDATE retained_snapshot_lease_intents SET lease_owner='fixture-owner'",
        ),
        (
            "UPDATE retained_snapshot_lease_intents SET lease_expires_ms=lease_expires_ms+1",
            "UPDATE retained_snapshot_lease_intents SET lease_expires_ms=lease_expires_ms-1",
        ),
    ] {
        db.execute_batch(mutate).unwrap();
        assert!(store
            .reconcile_recovery_completion(
                root,
                "operator:fixture",
                &grant.authenticated_event_id,
                |_| panic!("superseded completion reached effect: {mutate}")
            )
            .is_err());
        db.execute_batch(restore).unwrap();
    }
    let effects = std::cell::Cell::new(0);
    store
        .reconcile_recovery_completion(
            root,
            "operator:fixture",
            &grant.authenticated_event_id,
            |_| {
                effects.set(effects.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    store
        .reconcile_recovery_completion(
            root,
            "operator:fixture",
            &grant.authenticated_event_id,
            |_| panic!("acknowledged completion applied twice"),
        )
        .unwrap();
    assert_eq!(effects.get(), 1);
}
