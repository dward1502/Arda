//! Keeper-first deny-only maintenance. Engine reservations remain untouched.
use super::{keeper_managed as managed, keeper_reconcile, keeper_storage};
use anyhow::{bail, Result};
use arda_engine::objectives::abandonment::AbandonmentAuthorization;
use rusqlite::Connection;
use std::path::Path;

pub(super) fn apply(
    db: &mut Connection,
    auth: &AbandonmentAuthorization,
    owner: &str,
    runtime: &Path,
    provenance: &[u8],
    mut stopped: impl FnMut(&managed::Binding) -> Result<()>,
) -> Result<Vec<serde_json::Value>> {
    use rusqlite::{params, OptionalExtension, TransactionBehavior};
    auth.manifest.validate()?;
    if managed::digest(provenance) != auth.manifest.mutation_provenance_digest {
        bail!("abandonment mutation provenance digest mismatch");
    }
    // Caller holds owner/endpoint exclusion. Never start the keeper to migrate:
    // its ordinary startup sweep is inappropriate for damaged historical rows.
    keeper_storage::migrate_abandonments(db)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let auth_digest = managed::digest(&serde_json::to_vec(auth)?);
    let mut receipts = Vec::new();
    let mut bindings = Vec::new();
    let mut existing = 0;
    for target in &auth.manifest.targets {
        if target.keeper_owner != owner {
            bail!("abandonment keeper owner mismatch");
        }
        let (record, binding, record_digest) =
            keeper_reconcile::abandonment_record(&tx, owner, &target.run_id, runtime)?;
        if record_digest != target.keeper_record_digest {
            bail!("abandonment keeper record changed");
        }
        let receipt = serde_json::json!({
            "version":1,"disposition":"operator_abandonment",
            "application_id":auth_digest,"authorization_event_id":auth.event_id,
            "authorization_digest":auth_digest,"operator_id":auth.operator_id,
            "target":target,"keeper_record":record,
            "mutation_provenance_digest":auth.manifest.mutation_provenance_digest,
            "original_paired_lineage_unrecoverable":true,
            "cessation_proof":"managed_cgroup_teardown_or_reboot",
            "worker_cleanup_ack":false,"cleanup_verified":false,
            "artifacts_retained":true,"engine_reservations_retired":false
        });
        let prior: Option<String> = tx
            .query_row(
                "SELECT receipt_json FROM snapshot_operator_abandonments WHERE run=?1",
                [&target.run_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(prior) = prior {
            if serde_json::from_str::<serde_json::Value>(&prior)? != receipt {
                bail!("abandonment tombstone replay changed scope");
            }
            existing += 1;
        }
        receipts.push(receipt);
        bindings.push(binding);
    }
    if existing != 0 && existing != receipts.len() {
        bail!("partial abandonment keeper set");
    }
    // Preflight the entire set before cessation callbacks or any insertion.
    for binding in &bindings {
        stopped(binding)?;
    }
    if existing == 0 {
        for (target, receipt) in auth.manifest.targets.iter().zip(&receipts) {
            tx.execute(
                "INSERT INTO snapshot_operator_abandonments(run,receipt_json) VALUES(?1,?2)",
                params![target.run_id, serde_json::to_string(receipt)?],
            )?;
            #[cfg(test)]
            tests::crash("insert");
        }
    }
    // Revalidate maintenance through commit, including on replay. Transaction
    // rollback on any error leaves no partial target set. A crash after commit
    // leaves all five deny-only; it never implies Engine reservation release.
    for binding in &bindings {
        stopped(binding)?;
    }
    if chrono::Utc::now().timestamp_millis() >= auth.expires_at_ms {
        bail!("abandonment authorization expired before keeper commit");
    }
    #[cfg(test)]
    tests::crash("precommit");
    tx.commit()?;
    #[cfg(test)]
    tests::crash("committed");
    Ok(receipts)
}

#[derive(clap::Parser)]
#[command(
    about = "Apply authenticated keeper-first deny-only tombstones; never retire Engine reservations"
)]
struct Cli {
    #[arg(long)]
    engine_db: std::path::PathBuf,
    #[arg(long)]
    event: String,
    #[arg(long)]
    operator: String,
    #[arg(long)]
    durable: std::path::PathBuf,
    #[arg(long)]
    runtime: std::path::PathBuf,
    #[arg(long)]
    owner: String,
    #[arg(long)]
    provenance: std::path::PathBuf,
    #[arg(long)]
    confirm_deny_only: bool,
}
pub(super) fn engine_stopped() -> Result<()> {
    let output = std::process::Command::new("/usr/bin/timeout")
        .args([
            "15",
            "/usr/bin/systemctl",
            "--user",
            "show",
            "arda.service",
            "--property=LoadState,UnitFileState,ActiveState,MainPID,ControlGroup",
        ])
        .output()?;
    if !output.status.success() {
        bail!("Engine maintenance query failed");
    }
    let text = std::str::from_utf8(&output.stdout)?;
    let properties: std::collections::BTreeMap<_, _> = text
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    if properties.get("LoadState") != Some(&"masked")
        || properties.get("UnitFileState") != Some(&"masked-runtime")
        || !matches!(properties.get("ActiveState"), Some(&"inactive" | &"failed"))
        || properties.get("MainPID") != Some(&"0")
        || properties.get("ControlGroup") != Some(&"")
    {
        bail!("arda.service must be stopped and runtime-masked with no remaining cgroup");
    }
    Ok(())
}
pub(super) fn main(args: Vec<std::ffi::OsString>) -> Result<()> {
    use clap::Parser;
    let cli = match Cli::try_parse_from(
        std::iter::once(std::ffi::OsString::from("abandon-deny")).chain(args),
    ) {
        Ok(cli) => cli,
        Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if !cli.confirm_deny_only {
        bail!("explicit deny-only confirmation required");
    }
    // Do not accept serialized authority from a caller or initialize a database.
    engine_stopped()?;
    let store = arda_engine::objectives::ObjectiveStore::open_existing_maintenance(&cli.engine_db)?;
    let provenance = std::fs::read(&cli.provenance)?;
    let mut offline = keeper_storage::offline::open(&cli.durable, &cli.runtime, &cli.owner, true)?;
    let receipts = store.with_fenced_abandonment_authorization(
        &cli.event,
        &cli.operator,
        chrono::Utc::now().timestamp_millis(),
        |auth| {
            apply(
                &mut offline.db,
                auth,
                &cli.owner,
                &cli.runtime,
                &provenance,
                |binding| {
                    engine_stopped()?;
                    managed::stopped(binding)
                },
            )
        },
    )?;
    // Read back exact immutable targets, not just a successful INSERT result.
    for receipt in &receipts {
        let run = receipt["target"]["run_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("receipt run absent"))?;
        let stored: String = offline.db.query_row(
            "SELECT receipt_json FROM snapshot_operator_abandonments WHERE run=?1",
            [run],
            |r| r.get(0),
        )?;
        if serde_json::from_str::<serde_json::Value>(&stored)? != *receipt {
            bail!("abandonment receipt readback mismatch");
        }
    }
    println!(
        "{}",
        serde_json::json!({"status":"keeper_deny_only","engine_reservations_retired":false,"receipts":receipts})
    );
    Ok(())
}
#[cfg(test)]
mod tests;
