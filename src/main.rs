//! The `arda` daemon — the single Rust entry point for the Arda system.
//!
//! Responsibilities:
//!   - boot the engine (Arda spine),
//!   - supervise the Tauri `arda-launcher`, the HUD, and the `manwe` gateway
//!     (declared in `services.toml`),
//!   - shut everything down cleanly on ctrl-c.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use arda_engine::registry::Registry;
use arda_engine::supervisor::{Shutdown, Supervisor};

#[derive(Parser, Debug)]
#[command(name = "arda", version, about = "Arda system daemon")]
struct Cli {
    /// Log filter (e.g. `arda=debug,arda_manwe=trace`).
    #[arg(short, long, default_value = "info")]
    log: String,

    /// Skip supervision and exit after boot (smoke test).
    #[arg(long)]
    once: bool,

    /// Run without UI surfaces (drops services flagged `no_ui = true`,
    /// e.g. the launcher/HUD — useful for a headless/CI gateway run).
    #[arg(long)]
    no_ui: bool,

    /// Harness tap-in bind address (default 127.0.0.1:7878). Hermes connects
    /// here, never to `manwe`'s internal 7171 gateway port.
    #[arg(long)]
    harness_addr: Option<String>,
}

/// Path to the data-driven service registry.
const SERVICES_TOML: &str = "services.toml";

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

    // Resolve supervised services from data (services.toml). To add/remove an
    // app (launcher, HUD, `manwe` gateway), edit the toml — not this file.
    let root = repo_root();
    let reg = Registry::load(std::path::Path::new(SERVICES_TOML))
        .map_err(|e| anyhow::anyhow!("{e}\n(running from {root:?}; expected {SERVICES_TOML})"))?;
    let (services, errors) = reg.resolve(&root, cli.no_ui);

    // Required-but-missing services are hard failures.
    if !errors.is_empty() {
        for e in &errors {
            warn!("arda daemon: {e}");
        }
        anyhow::bail!(
            "{} required service(s) could not be resolved — see warnings above",
            errors.len()
        );
    }

    if services.is_empty() {
        warn!(
            "arda daemon: no services resolved (--no-ui={}); nothing to supervise",
            cli.no_ui
        );
    } else {
        for svc in &services {
            info!(
                "arda daemon: will supervise '{}' ({})",
                svc.name,
                svc.exe.display()
            );
        }
    }

    let shutdown = Shutdown::new();
    let supervisor = Supervisor::new(services, shutdown.clone());

    // Shared live-PID mirror so the harness status surface can report what is
    // actually running without reaching into the supervisor's internals.
    let harness_pids: Arc<tokio::sync::RwLock<Vec<u32>>> =
        Arc::new(tokio::sync::RwLock::new(Vec::new()));
    supervisor.set_pid_mirror(Some(harness_pids.clone())).await;

    // Harness tap-in surface: the ONE port Hermes/Agent connects to. Honours
    // `--harness-addr` (e.g. to expose on a different interface) and falls back
    // to 127.0.0.1:7878.
    let client = reqwest::Client::builder()
        .timeout(arda_engine::harness::DEFAULT_MANWE_PROXY_TIMEOUT)
        .build()
        .unwrap_or_default();
    let harness_state = arda_engine::harness::HarnessState {
        child_pids: harness_pids,
        service_names: Arc::new(reg.services.iter().map(|s| s.name.clone()).collect()),
        manwe_url: "http://127.0.0.1:7171".to_string(),
        client,
        manwe_proxy_timeout: arda_engine::harness::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: std::env::var("ARDA_MANWE_PROXY_BEARER").ok(),
    };
    let harness_addr: Option<SocketAddr> = cli
        .harness_addr
        .as_ref()
        .map(|a| a.parse())
        .transpose()
        .map_err(|e| anyhow::anyhow!("invalid --harness-addr: {e}"))?;
    let harness_shutdown = Arc::new(tokio::sync::Notify::new());
    let (_bound, _harness_handle) =
        arda_engine::harness::serve(harness_addr, harness_state, harness_shutdown.clone()).await?;

    // Fire shutdown on ctrl-c.
    let shutdown_on_signal = shutdown.clone();
    let harness_shutdown_on_signal = harness_shutdown.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("arda daemon: ctrl-c received, shutting down");
            shutdown_on_signal.trigger();
            harness_shutdown_on_signal.notify_waiters();
        }
    });

    supervisor.run().await;
    info!("arda daemon: stopped");
    Ok(())
}

/// Best-effort repo root: the directory containing `services.toml`. We are
/// launched with the workspace root as cwd by normal invocation; fall back to
/// `.` if the marker file is not adjacent.
fn repo_root() -> PathBuf {
    if std::path::Path::new(SERVICES_TOML).exists() {
        PathBuf::from(".")
    } else {
        // Walk up to find it (handles `cargo run` from a crate dir).
        let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        loop {
            if dir.join(SERVICES_TOML).exists() {
                return dir;
            }
            if !dir.pop() {
                return PathBuf::from(".");
            }
        }
    }
}
