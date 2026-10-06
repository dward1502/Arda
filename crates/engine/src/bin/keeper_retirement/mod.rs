//! Explicit retirement dispatcher; never starts the keeper or creates authority.
use anyhow::{bail, Result};
use std::ffi::OsString;

#[derive(clap::Parser)]
#[command(
    about = "Apply authenticated keeper-first Engine retirement; preserves artifacts, never acknowledges cleanup. Expired keeper-only events remain permanently refused.",
    after_help = "Prerequisites: arda.service must be stopped and runtime-masked with no remaining cgroup; keeper ownership must be exclusively available and managed workers stopped. Baseline must exactly match durable authenticated intent. Provenance bytes must match its digest. These files do not grant authority.

A nonzero exit or missing JSON means UNKNOWN/POSSIBLY PARTIAL outcome: keeper denial may already be committed, or Engine commit may precede output failure. Inspect exact durable records; never assume rollback or waive expiry.

Example (replace every path and identifier with verified values):
  arda-snapshot-keeper abandon-retire --engine-db /path/objectives.sqlite3 --event EVENT --operator OPERATOR --durable /path/keeper-owner --runtime /path/keeper-runtime --owner OWNER_UUID --provenance /path/provenance --baseline /path/baseline.json --confirm-retirement"
)]
struct Cli {
    /// Existing authority-bound Engine database; never initialized
    #[arg(long)]
    engine_db: std::path::PathBuf,
    /// Existing authenticated abandonment event identifier
    #[arg(long)]
    event: String,
    /// Operator bound to the durable event
    #[arg(long)]
    operator: String,
    /// Canonical private keeper owner directory
    #[arg(long)]
    durable: std::path::PathBuf,
    /// Canonical keeper endpoint directory
    #[arg(long)]
    runtime: std::path::PathBuf,
    /// Exact durable keeper owner UUID
    #[arg(long)]
    owner: String,
    /// Retained mutation-provenance bytes bound by the manifest digest
    #[arg(long)]
    provenance: std::path::PathBuf,
    /// Exact AbandonmentManifest JSON matching durable authenticated intent
    #[arg(long)]
    baseline: std::path::PathBuf,
    /// Explicitly confirm retirement, not cleanup or artifact deletion
    #[arg(long)]
    confirm_retirement: bool,
}

pub(super) fn main(args: Vec<OsString>) -> Result<()> {
    use clap::Parser;
    let cli =
        match Cli::try_parse_from(std::iter::once(OsString::from("abandon-retire")).chain(args)) {
            Ok(cli) => cli,
            Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
                error.print()?;
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
    if !cli.confirm_retirement {
        bail!("explicit retirement confirmation required");
    }
    // Fail before opening either store when the installed service is running.
    super::keeper_abandonment::engine_stopped()?;
    let maintenance =
        arda_engine::objectives::ObjectiveStore::open_existing_maintenance(&cli.engine_db)?;
    let records = maintenance.apply_abandonment(
        &cli.event,
        &cli.operator,
        &cli.durable,
        &cli.runtime,
        &cli.owner,
        &cli.provenance,
        &cli.baseline,
    )?;
    println!(
        "{}",
        serde_json::json!({"status":"operator_abandonment","engine_reservations_retired":true,"worker_cleanup_ack":false,"cleanup_verified":false,"artifacts_retained":true,"records":records})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retirement_help_is_non_executing() {
        main(vec!["--help".into()]).unwrap();
    }
    #[test]
    fn retirement_requires_confirmation_before_opening_paths() {
        let args = [
            "--engine-db",
            "/absent-engine",
            "--event",
            "event",
            "--operator",
            "operator",
            "--durable",
            "/absent-keeper",
            "--runtime",
            "/absent-runtime",
            "--owner",
            "owner",
            "--provenance",
            "/absent-provenance",
            "--baseline",
            "/absent-baseline",
        ];
        let error = main(args.into_iter().map(OsString::from).collect()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("explicit retirement confirmation required"),
            "{error}"
        );
    }
}
