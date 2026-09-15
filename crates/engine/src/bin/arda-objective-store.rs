//! Offline provisioning; never invoked by the daemon or HTTP ingress.
use anyhow::{bail, Context, Result};
use arda_engine::objectives::ObjectiveStore;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Action {
    Init,
    Adopt,
    Check,
}

#[derive(Parser)]
#[command(
    about = "Provision/check objective authority. Stop all writers before init/adopt; never delete authority markers to recover lost history."
)]
struct Cli {
    action: Action,
    #[arg(long)]
    database: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = match cli.action {
        Action::Init => ObjectiveStore::initialize(&cli.database)?,
        Action::Adopt => {
            // This is explicit legacy adoption, not a reset path. Reject empty
            // files and missing DBs before allowing the provisioning migration.
            let connection = rusqlite::Connection::open_with_flags(
                &cli.database,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
            )
            .context("adoption requires an existing legacy ObjectiveStore")?;
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='objectives')",
                [], |row| row.get(0),
            )?;
            if !exists {
                bail!("adoption requires existing objective history schema");
            }
            drop(connection);
            ObjectiveStore::open(&cli.database)?
        }
        Action::Check => ObjectiveStore::open_existing(&cli.database)?,
    };
    let count = store.list_objectives()?.len();
    println!("objective authority verified; objectives={count}");
    Ok(())
}
