//! Learning interop consumer commands for `arda-cli`.
//!
//! These commands expose `arda-core` learning state through the
//! `arda-aule` CLI so external tooling can consume governance
//! learning without depending on `arda-core` directly.

#![cfg(feature = "full-cli")]

use arda_core::learning::{LearningState, LearningStore};
use arda_core::learning_adapter::build_learning_ledger_receipt;
use clap::Subcommand;
use serde_json::json;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum LearningCommands {
    /// Build a learning ledger receipt from the current learning state.
    Ledger {
        /// Domain tag for the receipt
        #[arg(long, default_value = "governance")]
        domain: String,
        /// Consumer tag for the receipt
        #[arg(long, default_value = "arda-cli")]
        consumer: String,
        /// Minimum observations before an insight is retained
        #[arg(long, default_value_t = 2)]
        min_observations: u64,
        /// Optional path to learning store JSON; falls back to default location
        #[arg(long)]
        learning_path: Option<PathBuf>,
    },
}

pub(crate) fn handle(cmd: LearningCommands) -> anyhow::Result<()> {
    match cmd {
        LearningCommands::Ledger {
            domain,
            consumer,
            min_observations,
            learning_path,
        } => learning_ledger(domain, consumer, min_observations, learning_path),
    }
}

fn learning_ledger(
    domain: String,
    consumer: String,
    min_observations: u64,
    learning_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let root = arda_root();
    let state_root = root.join("core/state");
    let store_path = learning_path.unwrap_or_else(|| state_root.join("learning/learn.json"));
    let store = LearningStore::new(&store_path);
    let learning = store.load();

    let receipt = build_learning_ledger_receipt(&learning, &domain, &consumer, min_observations);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "contract": "arda.learning.interop.v1",
            "receipt": receipt,
            "meta": {
                "learning_path": store_path.display().to_string(),
                "retained_count": receipt.retained.len(),
                "ignored_count": receipt.ignored_count,
            }
        }))?
    );

    Ok(())
}
