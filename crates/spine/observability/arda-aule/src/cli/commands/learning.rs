//! Learning interop consumer commands for `arda-cli`.
//!
//! These commands expose `arda-core` learning state through the
//! `arda-aule` CLI so external tooling can consume governance
//! learning without depending on `arda-core` directly.

use arda_core::learning::LearningState;
use arda_core::learning_adapter::build_learning_ledger_receipt;
use clap::Subcommand;
use serde_json::json;

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
    },
}

pub(crate) fn handle(cmd: LearningCommands) -> anyhow::Result<()> {
    match cmd {
        LearningCommands::Ledger {
            domain,
            consumer,
            min_observations,
        } => learning_ledger(domain, consumer, min_observations),
    }
}

fn learning_ledger(
    domain: String,
    consumer: String,
    min_observations: u64,
) -> anyhow::Result<()> {
    let learning = LearningState::default();
    let receipt = build_learning_ledger_receipt(&learning, &domain, &consumer, min_observations);

    println!("{}", serde_json::to_string_pretty(&json!({
        "contract": "arda.learning.interop.v1",
        "receipt": receipt,
    }))?);

    Ok(())
}
