//! The `arda` daemon — the single Rust entry point for the Arda system.
//!
//! Responsibilities:
//!   - boot the engine (Annunimas spine),
//!   - supervise the Tauri `arda-launcher` (and the HUD, when built),
//!   - shut everything down cleanly on ctrl-c.

use std::path::PathBuf;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use arda_engine::supervisor::{Service, Shutdown, Supervisor};

#[derive(Parser, Debug)]
#[command(name = "arda", version, about = "Arda system daemon")]
struct Cli {
    /// Log filter (e.g. `arda=debug,annunimas_charon=trace`).
    #[arg(short, long, default_value = "info")]
    log: String,

    /// Skip supervision and exit after boot (smoke test).
    #[arg(long)]
    once: bool,
}

/// Locate the launcher/HUD binary that `pnpm tauri build` / `tauri dev`
/// produces. Falls back across debug|release and a couple of locations so the
/// daemon works both in dev and after a release build.
fn find_exe(candidate_dirs: &[&str], names: &[&str]) -> Option<PathBuf> {
    for dir in candidate_dirs {
        for name in names {
            let p = PathBuf::from(dir).join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(&cli.log).unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    info!("arda daemon starting");
    arda_engine::boot()?;

    if cli.once {
        info!("arda daemon: --once set, exiting after boot");
        return Ok(());
    }

    // Resolve supervised services. The launcher is always expected; the HUD is
    // optional (separate repo, only present after its own build).
    let launcher_dirs = [
        "apps/arda-launcher/src-tauri/target/debug",
        "apps/arda-launcher/src-tauri/target/release",
    ];
    let launcher_exe = find_exe(&launcher_dirs, &["arda-launcher", "arda_launcher"]);
    let mut services = Vec::new();
    if let Some(exe) = launcher_exe {
        services.push(Service {
            name: "arda-launcher",
            exe,
            args: vec![],
        });
    } else {
        tracing::warn!(
            "arda daemon: launcher binary not found under apps/arda-launcher/src-tauri/target/* — skipping"
        );
    }

    let shutdown = Shutdown::new();
    let supervisor = Supervisor::new(services, shutdown.clone());

    // Fire shutdown on ctrl-c.
    let shutdown_on_signal = shutdown.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("arda daemon: ctrl-c received, shutting down");
            shutdown_on_signal.trigger();
        }
    });

    supervisor.run().await;
    info!("arda daemon: stopped");
    Ok(())
}
