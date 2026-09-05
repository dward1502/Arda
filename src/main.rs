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
use tracing::{info, info_span, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use arda_engine::registry::Registry;
use arda_engine::supervisor::{Shutdown, Supervisor};

const OBJECTIVE_RUNTIME_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
const OBJECTIVE_RUNTIME_CAPACITY: usize = 4;
const OBJECTIVE_RUNTIME_LEASE_DURATION_MS: i64 = 300_000;

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

    /// Stable operator identity used to authorize local HUD mutations.
    #[arg(long)]
    operator_id: Option<String>,
}

/// Path to the data-driven service registry.
const SERVICES_TOML: &str = "services.toml";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let env_filter = EnvFilter::try_new(&cli.log).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(arda_aule::telemetry::tracing_layer())
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
        .init();
    let _telemetry_shutdown = arda_aule::telemetry::shutdown_guard();

    {
        let trace_name =
            std::env::var("ARDA_TRACE_NAME").unwrap_or_else(|_| "arda-daemon-startup".to_string());
        let objective_id =
            std::env::var("ARDA_TRACE_OBJECTIVE_ID").unwrap_or_else(|_| "unbound".to_string());
        let run_id = std::env::var("ARDA_TRACE_RUN_ID").unwrap_or_else(|_| "unbound".to_string());
        let node_id = std::env::var("ARDA_TRACE_NODE_ID").unwrap_or_else(|_| "unbound".to_string());
        let startup_span = info_span!(
            "arda.daemon.startup",
            "langfuse.trace.name" = %trace_name,
            "langfuse.trace.metadata.objective_id" = %objective_id,
            "langfuse.trace.metadata.run_id" = %run_id,
            "langfuse.trace.metadata.node_id" = %node_id,
            "arda.objective.id" = %objective_id,
            "arda.run.id" = %run_id,
            "arda.node.id" = %node_id,
        );
        let _entered = startup_span.enter();
        info!("arda daemon starting");
    }

    // Resolve supervised services from data (services.toml). To add/remove an
    // app (launcher, HUD, `manwe` gateway), edit the toml — not this file.
    let root = repo_root();
    let reg = Registry::load(&root.join(SERVICES_TOML))
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

    if cli.once {
        info!(
            resolved_services = services.len(),
            no_ui = cli.no_ui,
            "arda daemon: registry smoke passed; --once set, exiting before supervision"
        );
        return Ok(());
    }

    let shutdown = Shutdown::new();
    let supervisor = Supervisor::new(services, shutdown.clone());
    let service_statuses = supervisor.statuses();

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
    let presence_access_path = root.join("config/outposts/access.toml");
    let presence_inputs =
        arda_engine::harness::presence::HarnessPresenceState::load_access_contract(
            &presence_access_path,
        )
        .unwrap_or_else(|error| {
            warn!(
                "arda daemon: remote presence access disabled; canonical contract failed closed: {error}"
            );
            arda_engine::harness::presence::HarnessPresenceState::default()
        });
    let operator_id = configured_operator_id(cli.operator_id.as_deref())?;
    let harness_state = arda_engine::harness::HarnessState {
        harness_addr: arda_engine::harness::DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: harness_pids,
        service_names: Arc::new(reg.services.iter().map(|s| s.name.clone()).collect()),
        service_statuses,
        manwe_url: "http://127.0.0.1:7171".to_string(),
        client,
        manwe_proxy_timeout: arda_engine::harness::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: std::env::var("ARDA_MANWE_PROXY_BEARER").ok(),
        warden_scout_url: discover_warden_scout_url(&root),
        warden_scout_timeout: arda_engine::harness::DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs,
        workbench_root: root.clone(),
        operator_id,
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

    let objective_store =
        arda_engine::objectives::ObjectiveStore::open(root.join("data/arda/objectives.sqlite3"))?;
    let objective_executor = arda_engine::objectives::WorkbenchLeafExecution::new(&root)?;
    let mut objective_runtime = arda_engine::objectives::ObjectiveRuntime::new(
        objective_store,
        objective_executor,
        "arda-resident-objective-runtime",
        OBJECTIVE_RUNTIME_CAPACITY,
        OBJECTIVE_RUNTIME_LEASE_DURATION_MS,
    );
    let (objective_shutdown, mut objective_shutdown_rx) = tokio::sync::watch::channel(false);
    let objective_runtime_handle = tokio::spawn(async move {
        info!(
            capacity = OBJECTIVE_RUNTIME_CAPACITY,
            lease_duration_ms = OBJECTIVE_RUNTIME_LEASE_DURATION_MS,
            "arda daemon: resident objective runtime started"
        );
        loop {
            if *objective_shutdown_rx.borrow() {
                break;
            }
            if let Err(error) = objective_runtime.run_round(unix_now_ms()).await {
                warn!("arda daemon: resident objective round failed: {error:#}");
            }
            tokio::select! {
                result = objective_shutdown_rx.changed() => {
                    if result.is_err() || *objective_shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(OBJECTIVE_RUNTIME_POLL_INTERVAL) => {}
            }
        }
        info!("arda daemon: resident objective runtime stopped");
    });

    // Fire shutdown on ctrl-c.
    let shutdown_on_signal = shutdown.clone();
    let harness_shutdown_on_signal = harness_shutdown.clone();
    let objective_shutdown_on_signal = objective_shutdown.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("arda daemon: ctrl-c received, shutting down");
            shutdown_on_signal.trigger();
            harness_shutdown_on_signal.notify_waiters();
            let _ = objective_shutdown_on_signal.send(true);
        }
    });

    supervisor.run().await;
    let _ = objective_shutdown.send(true);
    objective_runtime_handle.await?;
    info!("arda daemon: stopped");
    Ok(())
}

fn unix_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn configured_operator_id(cli_value: Option<&str>) -> anyhow::Result<String> {
    let configured = cli_value
        .map(str::to_owned)
        .or_else(|| std::env::var("ARDA_OPERATOR_ID").ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    configured.ok_or_else(|| {
        anyhow::anyhow!("operator identity is required; set ARDA_OPERATOR_ID or pass --operator-id")
    })
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

fn discover_warden_scout_url(root: &std::path::Path) -> Option<String> {
    if let Ok(url) = std::env::var("ARDA_WARDEN_SCOUT_URL") {
        if !url.trim().is_empty() {
            return Some(url);
        }
    }

    let fleet = std::fs::read_to_string(root.join("config/fleet.toml")).ok()?;
    let value: toml::Value = toml::from_str(&fleet).ok()?;
    value
        .get("nodes")?
        .as_array()?
        .iter()
        .find(|node| node.get("id").and_then(toml::Value::as_str) == Some("node-pi5-warden"))?
        .get("scout_url")?
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_only_parallel_owner_profile_is_not_supported() {
        let result = Cli::try_parse_from(["arda", "--harness-only"]);
        assert!(result.is_err());
    }

    #[test]
    fn configured_operator_identity_is_trimmed() {
        assert_eq!(
            configured_operator_id(Some(" operator:primary ")).unwrap(),
            "operator:primary"
        );
    }

    #[test]
    fn blank_operator_identity_is_rejected() {
        assert!(configured_operator_id(Some("   ")).is_err());
    }
}
