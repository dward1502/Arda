//! Explicit offline terminal revocation; never implies worker cleanup ACK.
use super::{keeper_managed as managed, keeper_storage};
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    action: Action,
    #[arg(long, global = true, required = false)]
    durable: PathBuf,
    #[arg(long, global = true, required = false)]
    runtime: PathBuf,
    #[arg(long, global = true, required = false)]
    owner: String,
    #[arg(long, global = true, required = false)]
    run: String,
    #[arg(long, global = true)]
    json: bool,
}
#[derive(Subcommand)]
enum Action {
    Inspect,
    StopProof {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        confirm_managed_stop: bool,
    },
    Revoke {
        #[arg(long)]
        expect_record_digest: String,
        #[arg(long)]
        managed_stop_evidence: PathBuf,
        #[arg(long)]
        request_id: String,
        #[arg(long)]
        operator: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        confirm_terminal_revocation: bool,
    },
}
// Keep the historical allocation tuple's serialized shape unchanged.
type AllocationRecord = (
    String,
    String,
    String,
    Option<u64>,
    Option<u64>,
    Option<String>,
);

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Record {
    run: String,
    workspace: String,
    identity: String,
    state: String,
    authority: Option<String>,
    allocation: Option<AllocationRecord>,
    binding: managed::Binding,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Evidence {
    version: u32,
    owner: String,
    run: String,
    record_digest: String,
    binding: managed::Binding,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u32,
    disposition: String,
    proof: String,
    worker_cleanup_ack: bool,
    artifacts_retained: bool,
    cleanup_verified: bool,
    request_id: String,
    operator: String,
    reason: String,
    owner: String,
    run: String,
    record_digest: String,
    evidence_digest: String,
    prior_state: String,
    allocation_prior_state: Option<String>,
}
fn hash<T: Serialize>(value: &T) -> Result<String> {
    Ok(managed::digest(&serde_json::to_vec(value)?))
}
fn table(db: &Connection, name: &str) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}
fn record(db: &Connection, owner: &str, run: &str, runtime: &Path) -> Result<Record> {
    let (workspace, identity, state, authority): (String, String, String, Option<String>) = db
        .query_row(
            "SELECT workspace,identity,state,authority FROM snapshots WHERE run=?1",
            [run],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
    if let Some(bytes) = &authority {
        let _: arda_engine::objectives::RetainedSnapshot = serde_json::from_str(bytes)?;
    }
    let binding = managed::load(db, run)?;
    if binding.owner != owner || binding.runtime != runtime {
        bail!("managed owner/runtime binding mismatch");
    }
    let allocation = if table(db, "runtime_allocations")? {
        db.query_row("SELECT template_digest,source,state,device,inode,policy FROM runtime_allocations WHERE run=?1", [run],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?
    } else {
        None
    };
    Ok(Record {
        run: run.to_owned(),
        workspace,
        identity,
        state,
        authority,
        allocation,
        binding,
    })
}
fn saved_receipt(db: &Connection, request: &str) -> Result<Option<Receipt>> {
    if !table(db, "snapshot_reconciliations")? {
        return Ok(None);
    }
    let text: Option<String> = db
        .query_row(
            "SELECT receipt FROM snapshot_reconciliations WHERE request_id=?1",
            [request],
            |r| r.get(0),
        )
        .optional()?;
    text.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
}
fn validate_receipt(db: &Connection, receipt: &Receipt, mut current: Record) -> Result<()> {
    if receipt.version != 1
        || receipt.disposition != "terminal_revocation"
        || receipt.proof != "managed_cgroup_teardown_or_reboot"
        || receipt.worker_cleanup_ack
        || !receipt.artifacts_retained
        || receipt.cleanup_verified
        || receipt.run != current.run
        || receipt.owner != current.binding.owner
        || current.state != "reconciled_revoked"
    {
        bail!("invalid terminal reconciliation receipt");
    }
    current.state.clone_from(&receipt.prior_state);
    match (&mut current.allocation, &receipt.allocation_prior_state) {
        (Some(allocation), Some(prior)) if allocation.2 == "reconciled_revoked" => {
            allocation.2.clone_from(prior)
        }
        (None, None) => (),
        _ => bail!("reconciliation allocation binding mismatch"),
    }
    if hash(&current)? != receipt.record_digest {
        bail!("reconciliation record binding mismatch");
    }
    let saved = saved_receipt(db, &receipt.request_id)?.context("reconciliation receipt absent")?;
    if saved != *receipt {
        bail!("reconciliation receipt conflict");
    }
    Ok(())
}
pub fn release_ack(db: &Connection, run: &str, runtime: &Path) -> Result<()> {
    let text: String = db.query_row(
        "SELECT receipt FROM snapshot_reconciliations WHERE run=?1",
        [run],
        |r| r.get(0),
    )?;
    let receipt: Receipt = serde_json::from_str(&text)?;
    let owner: String = db.query_row("SELECT id FROM owner_identity", [], |r| r.get(0))?;
    validate_receipt(db, &receipt, record(db, &owner, run, runtime)?)
}
fn evidence(path: &Path) -> Result<Evidence> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || meta.nlink() != 1
    {
        bail!("stop evidence must be a private owned regular file");
    }
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        bail!("stop evidence exceeds limit");
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn apply(
    db: &mut Connection,
    wanted: Receipt,
    evidence: &Evidence,
    runtime: &Path,
) -> Result<Receipt> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if let Some(saved) = saved_receipt(&tx, &wanted.request_id)? {
        if saved != wanted {
            bail!("reconciliation request ID payload conflict");
        }
        validate_receipt(
            &tx,
            &saved,
            record(&tx, &wanted.owner, &wanted.run, runtime)?,
        )?;
        return Ok(saved);
    }
    let current = record(&tx, &wanted.owner, &wanted.run, runtime)?;
    if hash(&current)? != wanted.record_digest || current.binding != evidence.binding {
        bail!("record changed since inspection");
    }
    if !matches!(
        current.state.as_str(),
        "preparing" | "prepared" | "releasing" | "lost"
    ) {
        bail!("snapshot is not eligible for terminal revocation");
    }
    managed::stopped(&current.binding)?;
    commit_revocation(tx, wanted, current.allocation.is_some(), || {
        managed::stopped(&current.binding)
    })
}

fn commit_revocation(
    tx: rusqlite::Transaction<'_>,
    wanted: Receipt,
    allocation: bool,
    recheck: impl FnOnce() -> Result<()>,
) -> Result<Receipt> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS snapshot_reconciliations(request_id TEXT PRIMARY KEY, run TEXT NOT NULL UNIQUE, receipt TEXT NOT NULL);
        CREATE TRIGGER IF NOT EXISTS reconciliation_no_update BEFORE UPDATE ON snapshot_reconciliations BEGIN SELECT RAISE(ABORT,'immutable reconciliation'); END;
        CREATE TRIGGER IF NOT EXISTS reconciliation_no_delete BEFORE DELETE ON snapshot_reconciliations BEGIN SELECT RAISE(ABORT,'immutable reconciliation'); END;")?;
    tx.execute(
        "INSERT INTO snapshot_reconciliations(request_id,run,receipt) VALUES(?1,?2,?3)",
        rusqlite::params![
            wanted.request_id,
            wanted.run,
            serde_json::to_string(&wanted)?
        ],
    )?;
    #[cfg(test)]
    tests::crash("receipt");
    tx.execute(
        "UPDATE snapshots SET state='reconciled_revoked' WHERE run=?1",
        [&wanted.run],
    )?;
    #[cfg(test)]
    tests::crash("snapshot");
    if allocation {
        tx.execute(
            "UPDATE runtime_allocations SET state='reconciled_revoked' WHERE run=?1",
            [&wanted.run],
        )?;
    }
    #[cfg(test)]
    tests::crash("allocation");
    recheck()?;
    #[cfg(test)]
    tests::crash("precommit");
    tx.commit()?;
    #[cfg(test)]
    tests::crash("committed");
    Ok(wanted)
}

#[cfg(test)]
mod tests;

pub fn main(args: Vec<OsString>) -> Result<()> {
    let cli = match Cli::try_parse_from(std::iter::once(OsString::from("reconcile")).chain(args)) {
        Ok(cli) => cli,
        Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if cli.run.is_empty() || cli.run.len() > 1024 {
        bail!("invalid run identifier");
    }
    let writable = matches!(cli.action, Action::Revoke { .. });
    let mut offline =
        keeper_storage::offline::open(&cli.durable, &cli.runtime, &cli.owner, writable)?;
    let current = record(&offline.db, &cli.owner, &cli.run, &cli.runtime)?;
    match cli.action {
        Action::Inspect => {
            let blockers = managed::stopped(&current.binding)
                .err()
                .map(|error| error.to_string());
            println!(
                "{}",
                serde_json::json!({"owner":cli.owner,"run":cli.run,"state":current.state,
                "record_digest":hash(&current)?,"managed_unit":current.binding.unit,"blockers":blockers,
                "authority_present":current.authority.is_some(),"artifacts_retained":true})
            );
        }
        Action::StopProof {
            output,
            confirm_managed_stop,
        } => {
            if !confirm_managed_stop {
                bail!("explicit managed-stop confirmation required");
            }
            managed::confirm_stop(&current.binding)?;
            let proof = Evidence {
                version: 1,
                owner: cli.owner,
                run: cli.run,
                record_digest: hash(&current)?,
                binding: current.binding,
            };
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&output)?;
            file.write_all(&serde_json::to_vec(&proof)?)?;
            file.sync_all()?;
            fs::File::open(output.parent().context("evidence parent missing")?)?.sync_all()?;
            println!(
                "{}",
                serde_json::json!({"evidence_digest":hash(&proof)?,"worker_cleanup_ack":false})
            );
        }
        Action::Revoke {
            expect_record_digest,
            managed_stop_evidence,
            request_id,
            operator,
            reason,
            confirm_terminal_revocation,
        } => {
            if !confirm_terminal_revocation
                || operator.trim().is_empty()
                || reason.trim().is_empty()
                || operator.len() > 256
                || reason.len() > 4096
            {
                bail!("explicit terminal revocation identity and reason required");
            }
            uuid::Uuid::parse_str(&request_id)?;
            let proof = evidence(&managed_stop_evidence)?;
            if proof.version != 1
                || proof.owner != cli.owner
                || proof.run != cli.run
                || proof.record_digest != expect_record_digest
                || proof.binding != current.binding
            {
                bail!("managed-stop evidence binding mismatch");
            }
            let prior = saved_receipt(&offline.db, &request_id)?;
            let wanted = Receipt {
                version: 1,
                disposition: "terminal_revocation".into(),
                proof: "managed_cgroup_teardown_or_reboot".into(),
                worker_cleanup_ack: false,
                artifacts_retained: true,
                cleanup_verified: false,
                request_id,
                operator,
                reason,
                owner: cli.owner,
                run: cli.run,
                record_digest: expect_record_digest,
                evidence_digest: hash(&proof)?,
                prior_state: prior
                    .as_ref()
                    .map(|r| r.prior_state.clone())
                    .unwrap_or(current.state),
                allocation_prior_state: prior
                    .as_ref()
                    .map(|r| r.allocation_prior_state.clone())
                    .unwrap_or_else(|| current.allocation.map(|a| a.2)),
            };
            println!(
                "{}",
                serde_json::to_string(&apply(&mut offline.db, wanted, &proof, &cli.runtime)?)?
            );
        }
    }
    Ok(())
}
