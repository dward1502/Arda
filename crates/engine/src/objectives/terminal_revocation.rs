//! Terminal retirement evidence, not execution authority or worker cleanup ACK.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerminalRevocationReceipt {
    pub version: u32,
    pub disposition: String,
    pub proof: String,
    pub worker_cleanup_ack: bool,
    pub artifacts_retained: bool,
    pub cleanup_verified: bool,
    pub request_id: String,
    pub operator: String,
    pub reason: String,
    pub owner: String,
    pub run: String,
    pub record_digest: String,
    pub evidence_digest: String,
    pub prior_state: String,
    pub allocation_prior_state: Option<String>,
}

/// Private local-transport proof of an already revoked historical admission.
/// It does not attest ownership of a capability that the keeper no longer has.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerminalRevocationProof {
    pub run: String,
    pub workspace: String,
    pub identity: String,
    pub receipt: TerminalRevocationReceipt,
}

pub(super) fn retain(
    tx: &rusqlite::Transaction<'_>,
    keeper: &dyn super::SnapshotAdmission,
    leaf: &str,
    run: &str,
) -> anyhow::Result<()> {
    use anyhow::{bail, Context};
    use rusqlite::{params, OptionalExtension};
    let (objective, execution_run, identity, terminal, paused): (String,String,Option<String>,bool,bool) = tx.query_row(
        "SELECT l.objective_id,l.execution_run_id,i.identity_json,
         (l.stage IN ('complete','cancelled','failed') OR o.state IN ('completed','cancelled','failed')),
         o.state='paused' FROM leaves l JOIN objectives o ON o.id=l.objective_id
         LEFT JOIN lease_workspace_identities i ON i.leaf_id=l.id WHERE l.id=?1",
        [leaf], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
    )?;
    if paused || !terminal || execution_run != run {
        bail!("terminal revocation requires exact non-paused terminal lineage");
    }
    let identity = identity.context("terminal revocation requires historical identity")?;
    // Parse the persisted tuple only. Never reconstruct from today's filesystem.
    let (version, workspace, anchor, object, fingerprint): (
        u32,
        String,
        String,
        Option<(u64, u64)>,
        String,
    ) = serde_json::from_str(&identity)?;
    if !matches!(version, 2 | 3)
        || !std::path::Path::new(&workspace).is_absolute()
        || !std::path::Path::new(&anchor).is_absolute()
        || object.is_none()
        || fingerprint.len() != 64
        || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("unsupported historical workspace identity");
    }
    let proof = keeper.terminal_revocation(run, &workspace, &identity)?;
    let receipt = &proof.receipt;
    if proof.run != run
        || proof.workspace != workspace
        || proof.identity != identity
        || receipt.run != run
        || receipt.version != 1
        || receipt.disposition != "terminal_revocation"
        || receipt.proof != "managed_cgroup_teardown_or_reboot"
        || receipt.worker_cleanup_ack
        || receipt.cleanup_verified
        || !receipt.artifacts_retained
        || [
            &receipt.owner,
            &receipt.operator,
            &receipt.reason,
            &receipt.request_id,
            &receipt.prior_state,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
        || [&receipt.record_digest, &receipt.evidence_digest]
            .iter()
            .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        bail!("invalid bound terminal revocation proof");
    }
    let encoded = serde_json::to_string(&proof)?;
    let existing: Option<(String, String, String, String, String, String)> = tx
        .query_row(
            "SELECT objective_id,run_id,workspace,identity_json,disposition,proof_json
         FROM retained_snapshot_terminal_revocations WHERE leaf_id=?1",
            [leaf],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?;
    if let Some(existing) = existing {
        if existing
            != (
                objective,
                run.to_owned(),
                workspace,
                identity,
                "terminal_revocation".into(),
                encoded,
            )
        {
            bail!("terminal revocation proof conflicts with retained evidence");
        }
    } else {
        tx.execute(
            "INSERT INTO retained_snapshot_terminal_revocations
            (leaf_id,objective_id,run_id,workspace,identity_json,disposition,proof_json)
            VALUES (?1,?2,?3,?4,?5,'terminal_revocation',?6)",
            params![leaf, objective, run, workspace, identity, encoded],
        )?;
    }
    // Caller writes the execution-retirement marker in this same transaction.
    // This is not a worker cleanup ACK and does not delete retained artifacts.
    Ok(())
}
