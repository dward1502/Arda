#![warn(rust_2018_idioms)]
#![recursion_limit = "256"]

#[cfg(feature = "http")]
mod metrics_exporter;

use anyhow::Result;
use arda_governance::{
    build_governance_status_report, default_governance_readiness_report, global_governance_metrics,
    read_bacon_lite_summary, read_latest_bacon_lite_event, render_governance_status_human,
    BaconLiteLogPaths, BaconLiteReadWindow, MalformedLineBehavior,
};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "arda-cli", about = "Arda observability CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Plutus {
        #[command(subcommand)]
        command: PlutusCommands,
    },
    Prometheus {
        #[command(subcommand)]
        command: PrometheusCommands,
    },
    #[cfg(feature = "http")]
    Metrics {
        #[command(subcommand)]
        command: MetricsCommands,
    },
    TelemetrySchema,
    Receipt {
        id: String,
    },
    GovernancePolicy {
        policy_id: String,
    },
    GovernanceReceipt {
        receipt_id: String,
    },
    ServiceGraph,
    ToolManifest {
        agent_id: String,
    },
    RuntimeReceipt {
        run_id: String,
    },
    EvalRun {
        task_id: String,
    },
    LearningDelta {
        run_id: String,
    },
    /// Print the canonical ATHENA queue and counter projection as JSON.
    AthenaStatus {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    BaconLiteSummary {
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        strict_malformed: bool,
        #[arg(long)]
        json: bool,
    },
    GovernanceMetrics {
        #[arg(long)]
        json: bool,
    },
    GovernanceStatus {
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        strict_malformed: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PlutusCommands {
    /// Export the current economics, JouleWork, ledger, and governance state.
    Export {
        /// Override the runtime_status.json path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Emit the complete machine-readable export envelope.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PrometheusCommands {
    Serve {
        #[arg(long)]
        core_root: Option<PathBuf>,
        #[arg(long)]
        socket: Option<PathBuf>,
        #[arg(long, default_value = "127.0.0.1:5113")]
        http_addr: String,
        #[arg(long)]
        no_http: bool,
    },
    Status {
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    Thoughts {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    Escalations {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        include_resolved: bool,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    ResolveEscalation {
        escalation_id: String,
        #[arg(long)]
        note: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Roster {
        path: PathBuf,
        #[arg(long, default_value_t = 300)]
        heartbeat_timeout_secs: u64,
        #[arg(long)]
        supervisor: bool,
    },
    Plan {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
    Drift {
        #[arg(long)]
        core_root: Option<PathBuf>,
        #[arg(long)]
        reconcile: bool,
    },
    CouncilFanout {
        topic: String,
        #[arg(long, value_delimiter = ',')]
        participants: Vec<String>,
        #[arg(long)]
        context: Option<String>,
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    ReconcileRuntime {
        before: String,
        #[arg(long)]
        apply: bool,
        #[arg(long, default_value = "resolved by Prometheus runtime reconciliation")]
        note: String,
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    ExecutionIntents {
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        include_terminal: bool,
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    ExecutionIntentRecovery {
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    TransitionIntent {
        intent_id: String,
        status: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    CompactIntents {
        #[arg(long, default_value_t = 14)]
        retention_days: i64,
        #[arg(long, default_value_t = 5000)]
        max_keep: usize,
        #[arg(long)]
        core_root: Option<PathBuf>,
    },
    Autopilot {
        #[command(subcommand)]
        command: AutopilotCommands,
    },
}

#[cfg(feature = "http")]
#[derive(Subcommand, Debug)]
enum MetricsCommands {
    /// Run the Arda projection exporter as an HTTP server.
    Serve {
        /// Explicit Arda repository or migrated state root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        #[arg(long, default_value_t = 9101)]
        port: u16,
        #[arg(long, default_value_t = 15)]
        refresh_secs: u64,
        #[arg(long, default_value_t = false)]
        system_metrics: bool,
    },
    /// Print one Prometheus exposition snapshot and exit.
    Snapshot {
        /// Explicit Arda repository or migrated state root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value_t = false)]
        system_metrics: bool,
    },
}

#[derive(Subcommand, Debug)]
enum AutopilotCommands {
    Once {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        read_only: bool,
        /// Atomically publish the rendered cycle report for status consumers.
        #[arg(long, value_name = "PATH")]
        state_output: Option<PathBuf>,
    },
    Run {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value_t = 30)]
        interval: u64,
        #[arg(long)]
        read_only: bool,
    },
    Status {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Select the next operator-approved task eligible for Workbench execution.
    NextApprovedTask {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Claim and execute at most one approved canonical task through Workbench.
    ExecuteApprovedTask {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Cancel the Workbench run associated with a claimed canonical task.
    CancelApprovedTask {
        task_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Pause a canonical schedule under its objective lineage.
    PauseSchedule {
        task_id: String,
        #[arg(long)]
        objective_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Resume a paused canonical schedule under its objective lineage.
    ResumeSchedule {
        task_id: String,
        #[arg(long)]
        objective_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Requeue a failed approved task with a distinct Workbench attempt id.
    RetryApprovedTask {
        task_id: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Append an operator decision to the Arandur recommendation ledger.
    ReviewRecommendation {
        recommendation_id: String,
        #[arg(long, value_parser = ["approve", "reject"])]
        decision: String,
        #[arg(long)]
        reviewed_by: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Inspect or explicitly publish the autonomous-loop preflight projection.
    Preflight {
        #[arg(long)]
        root: Option<PathBuf>,
        /// Persist only the preflight projection; never enables promotion.
        #[arg(long)]
        write: bool,
    },
    KnowledgeTriage {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        write: bool,
        #[arg(long = "source-root")]
        source_roots: Vec<PathBuf>,
    },
    PromoteKnowledgeTasks {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value = "safe-local")]
        lane: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        write: bool,
        #[arg(long = "approval-evidence")]
        approval_evidence: Option<String>,
        #[arg(long = "source-root")]
        source_roots: Vec<PathBuf>,
    },
    ExecuteKnowledgeTasks {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        write: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::TelemetrySchema => {
            let registry = read_registry()?;
            let track = track_by_id(&registry, "arda-ecosystem-standard-track-1-observability")
                .unwrap_or(registry);
            println!("{}", serde_json::to_string_pretty(&track)?);
        }
        Commands::Receipt { id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(
                &registry,
                &["receipt_id", "run_id", "policy_id", "label"],
                &id,
            )
            .unwrap_or_else(|| not_found(&id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernancePolicy { policy_id } => {
            let registry = read_registry()?;
            let value =
                union_find_receipt(&registry, &["policy_id", "receipt_id", "label"], &policy_id)
                    .unwrap_or_else(|| not_found(&policy_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernanceReceipt { receipt_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(
                &registry,
                &["receipt_id", "policy_id", "label"],
                &receipt_id,
            )
            .unwrap_or_else(|| not_found(&receipt_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::ServiceGraph => {
            let registry = read_registry()?;
            let track = track_by_id(
                &registry,
                "arda-ecosystem-standard-track-3-agent-runtime-tooling",
            )
            .unwrap_or(registry);
            println!("{}", serde_json::to_string_pretty(&track)?);
        }
        Commands::ToolManifest { agent_id } => {
            let record = resolve_tool_manifest(&agent_id);
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        Commands::RuntimeReceipt { run_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["receipt_id", "run_id", "label"], &run_id)
                .unwrap_or_else(|| not_found(&run_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::EvalRun { task_id } => {
            let record = load_eval_run_record(&task_id);
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        Commands::LearningDelta { run_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["run_id", "receipt_id", "label"], &run_id)
                .unwrap_or_else(|| not_found(&run_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::AthenaStatus { root } => {
            let root = root
                .or_else(|| std::env::var_os("ARDA_ATHENA_ROOT").map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("data/athena"));
            let store = arda_varda::ingest::AthenaStore::new(root)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&store.operator_status()?)?
            );
        }
        Commands::Plutus {
            command: PlutusCommands::Export { path, json },
        } => {
            let export = load_plutus_export(path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&export)?);
            } else {
                print_plutus_export(&export);
            }
        }
        Commands::Prometheus { command } => handle_prometheus(command)?,
        #[cfg(feature = "http")]
        Commands::Metrics { command } => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(metrics_exporter::handle(command))?;
        }
        Commands::BaconLiteSummary {
            path,
            since,
            until,
            strict_malformed,
            json,
        } => {
            let root = std::env::var_os("ARDA_ROOT")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            let machine_path = path
                .or_else(|| std::env::var_os("ARDA_BACON_LITE_LOG_PATH").map(PathBuf::from))
                .unwrap_or_else(|| BaconLiteLogPaths::from_base_dir(root).machine);
            let parse_bound = |name: &str, value: Option<String>| -> Result<_> {
                value
                    .map(|value| {
                        chrono::DateTime::parse_from_rfc3339(&value)
                            .map(|date| date.with_timezone(&chrono::Utc))
                            .map_err(|error| anyhow::anyhow!("invalid --{name} timestamp: {error}"))
                    })
                    .transpose()
            };
            let window = BaconLiteReadWindow {
                since: parse_bound("since", since)?,
                until: parse_bound("until", until)?,
                malformed: if strict_malformed {
                    MalformedLineBehavior::Fail
                } else {
                    MalformedLineBehavior::CountAndSkip
                },
                include_rotated: true,
            };
            let summary = read_bacon_lite_summary(&machine_path, &window)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Bacon-Lite ledger: {} records, {} malformed",
                    summary.records, summary.malformed_records
                );
                for (crate_name, actions) in &summary.groups {
                    for (action, aggregate) in actions {
                        println!(
                            "- {crate_name}/{action}: count={} pass_rate={:.1}% mean_confidence={:.3} scorers={}",
                            aggregate.record_count,
                            aggregate.pass_rate * 100.0,
                            aggregate.mean_confidence,
                            aggregate
                                .scorer_versions
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                    }
                }
            }
        }
        Commands::GovernanceMetrics { json } => {
            let snapshot = global_governance_metrics().snapshot();
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                print!("{}", arda_aule::render_governance_prometheus(&snapshot));
            }
        }
        Commands::GovernanceStatus {
            path,
            since,
            until,
            strict_malformed,
            json,
        } => {
            let root = std::env::var_os("ARDA_ROOT")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            let machine_path = path
                .or_else(|| std::env::var_os("ARDA_BACON_LITE_LOG_PATH").map(PathBuf::from))
                .unwrap_or_else(|| BaconLiteLogPaths::from_base_dir(root).machine);
            let parse_bound = |name: &str, value: Option<String>| -> Result<_> {
                value
                    .map(|value| {
                        chrono::DateTime::parse_from_rfc3339(&value)
                            .map(|date| date.with_timezone(&chrono::Utc))
                            .map_err(|error| anyhow::anyhow!("invalid --{name} timestamp: {error}"))
                    })
                    .transpose()
            };
            let window = BaconLiteReadWindow {
                since: parse_bound("since", since)?,
                until: parse_bound("until", until)?,
                malformed: if strict_malformed {
                    MalformedLineBehavior::Fail
                } else {
                    MalformedLineBehavior::CountAndSkip
                },
                include_rotated: true,
            };
            let recent_ledger = read_bacon_lite_summary(&machine_path, &window)?;
            let latest_event = read_latest_bacon_lite_event(&machine_path, true)?;
            let report = build_governance_status_report(
                default_governance_readiness_report(),
                recent_ledger,
                global_governance_metrics().snapshot(),
                latest_event,
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render_governance_status_human(&report));
            }
        }
    }
    Ok(())
}

fn handle_prometheus(command: PrometheusCommands) -> Result<()> {
    use arda_aule::prometheus::{
        AgentRosterSnapshot, OrderStore, PrometheusService, ThoughtLedger,
    };

    let arda_root = || {
        std::env::var_os("ARDA_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };
    let orders_root =
        |root: Option<PathBuf>| root.unwrap_or_else(|| arda_root().join("data/prometheus"));
    let thoughts_root =
        |root: Option<PathBuf>| root.unwrap_or_else(|| arda_root().join("data/minds/machine"));
    match command {
        PrometheusCommands::Serve {
            core_root,
            socket,
            http_addr,
            no_http,
        } => {
            let core_root = core_root.unwrap_or_else(|| PathBuf::from("core"));
            let service = arda_aule::prometheus::PrometheusService::from_core(&core_root)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            let mut config = arda_aule::prometheus::transport::PrometheusDaemonConfig::default();
            if let Some(socket) = socket {
                config.socket_path = socket;
            }
            config.http_addr = http_addr;
            config.http_enabled = !no_http;
            let daemon = arda_aule::prometheus::transport::PrometheusDaemon::new(service, config);
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime
                .block_on(daemon.run())
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        }
        PrometheusCommands::Status { core_root } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!("{}", serde_json::to_string_pretty(&service.status()?)?);
        }
        PrometheusCommands::Thoughts { root, limit } => {
            let thoughts = ThoughtLedger::new(thoughts_root(root))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&thoughts.recent(limit)?)?
            );
        }
        PrometheusCommands::Escalations {
            root,
            include_resolved,
            limit,
        } => {
            let store = OrderStore::new(orders_root(root))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&store.list_escalations(include_resolved, limit)?)?
            );
        }
        PrometheusCommands::ResolveEscalation {
            escalation_id,
            note,
            root,
        } => {
            let store = OrderStore::new(orders_root(root))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&store.resolve_escalation(&escalation_id, &note)?)?
            );
        }
        PrometheusCommands::Roster {
            path,
            heartbeat_timeout_secs,
            supervisor,
        } => {
            let roster = if supervisor {
                AgentRosterSnapshot::from_supervisor_state_file(&path)
            } else {
                AgentRosterSnapshot::from_world_file(&path, heartbeat_timeout_secs)
            }
            .ok_or_else(|| anyhow::anyhow!("unable to read roster from {}", path.display()))?;
            println!("{}", serde_json::to_string_pretty(&roster)?);
        }
        PrometheusCommands::Plan {
            state_root,
            queue_path,
        } => {
            let state = arda_core::state::StateRoot::new(
                state_root.unwrap_or_else(|| arda_root().join("core/state")),
            );
            let pass = arda_aule::prometheus::run_planner(&state, queue_path.as_deref())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "contract": "arda.aule.prometheus.plan.v1",
                    "goals_considered": pass.goals_considered,
                    "plans_written": pass.plans_written,
                    "plans_skipped_existing": pass.plans_skipped_existing,
                    "goals_without_recipe": pass.goals_without_recipe,
                    "goals_inactive": pass.goals_inactive,
                    "tasks_emitted": pass.tasks_emitted,
                }))?
            );
        }
        PrometheusCommands::Drift {
            core_root,
            reconcile,
        } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&service.drift_detect_reconcile(reconcile)?)?
            );
        }
        PrometheusCommands::CouncilFanout {
            topic,
            participants,
            context,
            core_root,
        } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            let context = context
                .map(|raw| serde_json::from_str(&raw))
                .transpose()
                .map_err(|error| anyhow::anyhow!("invalid --context JSON: {error}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&service.council_fanout(
                    &topic,
                    participants,
                    context,
                )?)?
            );
        }
        PrometheusCommands::ReconcileRuntime {
            before,
            apply,
            note,
            core_root,
        } => {
            let cutoff = chrono::DateTime::parse_from_rfc3339(&before)?.with_timezone(&chrono::Utc);
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&service.reconcile_runtime(cutoff, apply, &note)?)?
            );
        }
        PrometheusCommands::ExecutionIntents {
            limit,
            include_terminal,
            core_root,
        } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&service.execution_intents(limit, include_terminal)?)?
            );
        }
        PrometheusCommands::ExecutionIntentRecovery { core_root } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&service.execution_intents_recovery()?)?
            );
        }
        PrometheusCommands::TransitionIntent {
            intent_id,
            status,
            note,
            core_root,
        } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&service.transition_execution_intent(
                    &intent_id,
                    &status,
                    note.as_deref(),
                )?)?
            );
        }
        PrometheusCommands::CompactIntents {
            retention_days,
            max_keep,
            core_root,
        } => {
            let service = PrometheusService::from_core(
                core_root.unwrap_or_else(|| arda_root().join("core")),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &service.compact_execution_intents(retention_days, max_keep)?
                )?
            );
        }
        PrometheusCommands::Autopilot { command } => handle_autopilot(command, arda_root())?,
    }
    Ok(())
}

fn handle_autopilot(command: AutopilotCommands, default_root: PathBuf) -> Result<()> {
    use arda_aule::prometheus::autopilot::{
        ceo_loop, execute_knowledge_task_queue, inspect_autonomy_preflight,
        promote_knowledge_tasks, review_arandur_recommendation, run_knowledge_triage,
        write_autonomy_preflight, AutopilotConfig, CeoAutopilot, KnowledgeTriageConfig,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

    let resolve_root = |root: Option<PathBuf>| root.unwrap_or_else(|| default_root.clone());
    match command {
        AutopilotCommands::Once {
            root,
            read_only,
            state_output,
        } => {
            let root = resolve_root(root);
            arda_aule::prometheus::core_link::refresh_queue_projections(&root.join("core"));
            let mut config = AutopilotConfig::from_root(root);
            config.read_only = read_only;
            let mut autopilot = CeoAutopilot::from_world(config);
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let rendered = serde_json::to_string_pretty(&runtime.block_on(autopilot.run_cycle()))?;
            if let Some(path) = state_output {
                write_atomic_snapshot(&path, rendered.as_bytes())?;
            }
            println!("{rendered}");
        }
        AutopilotCommands::Run {
            root,
            interval,
            read_only,
        } => {
            let root = resolve_root(root);
            arda_aule::prometheus::core_link::refresh_queue_projections(&root.join("core"));
            let mut config = AutopilotConfig::from_root(root);
            config.interval = Duration::from_secs(interval);
            config.read_only = read_only;
            let autopilot = CeoAutopilot::from_world(config);
            let stop = Arc::new(AtomicBool::new(false));
            let signal = stop.clone();
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async move {
                tokio::spawn(async move {
                    let _ = tokio::signal::ctrl_c().await;
                    signal.store(true, std::sync::atomic::Ordering::SeqCst);
                });
                ceo_loop(autopilot, stop).await;
            });
        }
        AutopilotCommands::Status { root } => {
            let root = resolve_root(root);
            arda_aule::prometheus::core_link::refresh_queue_projections(&root.join("core"));
            let path = root.join("data/ceo/autopilot.state.json");
            let value = std::fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .unwrap_or_else(|| {
                    json!({"error": "autopilot state not found", "path": path.display().to_string()})
                });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AutopilotCommands::NextApprovedTask { root } => {
            let root = resolve_root(root);
            let selected = arda_aule::prometheus::autopilot::ActiveQueueExecutor::new(&root)
                .select_next_approved()?;
            println!("{}", serde_json::to_string_pretty(&selected)?);
        }
        AutopilotCommands::ExecuteApprovedTask { root } => {
            let root = resolve_root(root);
            let executor = arda_aule::prometheus::autopilot::WorkbenchQueueExecutor::new(root)?;
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let receipt = runtime.block_on(executor.execute_once())?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        AutopilotCommands::PauseSchedule {
            task_id,
            objective_id,
            reason,
            root,
        } => {
            let root = resolve_root(root);
            let ledger = arda_aule::prometheus::autopilot::ScheduleLedger::new(
                root.join("core/projects/tasks/schedules.jsonl"),
            );
            let record = ledger.pause(&task_id, &objective_id, chrono::Utc::now(), &reason)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        AutopilotCommands::ResumeSchedule {
            task_id,
            objective_id,
            reason,
            root,
        } => {
            let root = resolve_root(root);
            let ledger = arda_aule::prometheus::autopilot::ScheduleLedger::new(
                root.join("core/projects/tasks/schedules.jsonl"),
            );
            let record = ledger.resume(&task_id, &objective_id, chrono::Utc::now(), &reason)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        AutopilotCommands::CancelApprovedTask {
            task_id,
            reason,
            root,
        } => {
            let root = resolve_root(root);
            let executor = arda_aule::prometheus::autopilot::WorkbenchQueueExecutor::new(root)?;
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let receipt = runtime.block_on(executor.cancel_task(&task_id, &reason))?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        AutopilotCommands::RetryApprovedTask { task_id, root } => {
            let root = resolve_root(root);
            let task = arda_aule::prometheus::autopilot::ActiveQueueExecutor::new(&root)
                .retry_failed(&task_id)?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        AutopilotCommands::ReviewRecommendation {
            recommendation_id,
            decision,
            reviewed_by,
            note,
            root,
        } => {
            let root = resolve_root(root);
            let receipt = review_arandur_recommendation(
                root.join("data/arandur/recommendations.jsonl"),
                &recommendation_id,
                decision == "approve",
                &reviewed_by,
                note.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        AutopilotCommands::Preflight { root, write } => {
            let root = resolve_root(root);
            let report = inspect_autonomy_preflight(&root)?;
            let output = if write {
                Some(write_autonomy_preflight(&root)?.display().to_string())
            } else {
                None
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "report": report,
                    "output": output,
                    "task_promotion_allowed": false,
                }))?
            );
        }
        AutopilotCommands::KnowledgeTriage {
            root,
            dry_run,
            write,
            source_roots,
        } => {
            let mut config =
                KnowledgeTriageConfig::for_root(resolve_root(root)).with_dry_run(dry_run || !write);
            if !source_roots.is_empty() {
                config = config.with_source_roots(source_roots);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&run_knowledge_triage(&config)?)?
            );
        }
        AutopilotCommands::PromoteKnowledgeTasks {
            root,
            lane,
            dry_run,
            write,
            approval_evidence,
            source_roots,
        } => {
            anyhow::ensure!(
                lane == "safe-local",
                "unsupported promotion lane '{lane}'; only safe-local is write-eligible"
            );
            let mut config =
                KnowledgeTriageConfig::for_root(resolve_root(root)).with_dry_run(dry_run || !write);
            if let Some(evidence) = approval_evidence {
                config = config.with_approval_evidence(evidence);
            }
            if !source_roots.is_empty() {
                config = config.with_source_roots(source_roots);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&promote_knowledge_tasks(&config)?)?
            );
        }
        AutopilotCommands::ExecuteKnowledgeTasks {
            root,
            dry_run,
            write,
        } => {
            let config =
                KnowledgeTriageConfig::for_root(resolve_root(root)).with_dry_run(dry_run || !write);
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_knowledge_task_queue(&config)?)?
            );
        }
    }
    Ok(())
}

fn write_atomic_snapshot(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&temporary, bytes)?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn load_plutus_export(path: Option<PathBuf>) -> Result<Value> {
    let path = match path {
        Some(path) => path,
        None => {
            let home = std::env::var_os("ARDA_PLUTUS_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::var_os("ARDA_ROOT")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                        })
                        .join("data/plutus")
                });
            home.join("runtime_status.json")
        }
    };
    let events_path = path.with_file_name("runtime_events.jsonl");
    let events_total = std::fs::read_to_string(&events_path)
        .map(|raw| raw.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or_default();

    if !path.exists() {
        return Ok(json!({
            "contract": "arda.plutus.export.v1",
            "found": false,
            "missing": true,
            "path": path,
            "events_path": events_path,
            "events_total": events_total,
        }));
    }

    let raw = std::fs::read_to_string(&path)?;
    let snapshot: Value = serde_json::from_str(&raw)
        .map_err(|error| anyhow::anyhow!("invalid Plutus snapshot {}: {error}", path.display()))?;
    Ok(json!({
        "contract": "arda.plutus.export.v1",
        "found": true,
        "missing": false,
        "path": path,
        "events_path": events_path,
        "events_total": events_total,
        "snapshot": snapshot,
    }))
}

fn print_plutus_export(export: &Value) {
    let path = export["path"].as_str().unwrap_or("unknown");
    if export["found"] != true {
        println!("Plutus state: not initialized");
        println!("- expected snapshot: {path}");
        return;
    }

    let snapshot = &export["snapshot"];
    println!("Plutus economics: {path}");
    println!(
        "- budget: spent={:.3} remaining={:.3} usage={:.1}% alert={}",
        snapshot["economics"]["total_spend"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["economics"]["budget_remaining"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["economics"]["budget_usage_percent"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["economics"]["budget_alert"]
            .as_str()
            .unwrap_or("none")
    );
    println!(
        "- providers={} accounts={} governance_records={} append_only_events={}",
        snapshot["economics"]["providers"]
            .as_array()
            .map(Vec::len)
            .unwrap_or_default(),
        snapshot["ledger"]["accounts_total"]
            .as_u64()
            .unwrap_or_default(),
        snapshot["governance"]["records_total"]
            .as_u64()
            .unwrap_or_default(),
        export["events_total"].as_u64().unwrap_or_default(),
    );
    println!(
        "- joulework_total={:.3} relationships={}",
        snapshot["joulework"]["total_joulework"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["love_equation"]["relationships_total"]
            .as_u64()
            .unwrap_or_default(),
    );
}

fn read_registry() -> Result<Value> {
    let path = candidate_paths()
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!("missing registry"))?;
    let raw = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn union_find_receipt(registry: &Value, keys: &[&str], id: &str) -> Option<Value> {
    let tracks = registry.get("tracks")?.as_array()?;
    for track in tracks {
        let stores = track.get("receipt_stores")?.as_array()?;
        for store in stores {
            let base = store.as_str()?.trim_end_matches('/');
            let path = PathBuf::from(base);
            if !path.exists() {
                continue;
            }
            let candidates = if path.is_dir() {
                std::fs::read_dir(&path)
                    .ok()?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect::<Vec<_>>()
            } else {
                vec![path]
            };
            for candidate in candidates {
                if candidate.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    if let Ok(raw) = std::fs::read_to_string(&candidate) {
                        for line in raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                            if let Ok(value) = serde_json::from_str::<Value>(line) {
                                if id_matches_any(&value, id, keys) {
                                    return Some(value);
                                }
                            }
                        }
                    }
                    continue;
                }
                if let Ok(raw) = std::fs::read_to_string(&candidate) {
                    if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                        let arr = data
                            .as_array()
                            .or_else(|| data.get("receipts").and_then(|v| v.as_array()))
                            .or_else(|| data.get("recent_receipts").and_then(|v| v.as_array()))
                            .cloned()
                            .unwrap_or_default();
                        for rec in arr {
                            if id_matches_any(&rec, id, keys) {
                                return Some(rec);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(value) = find_receipt_in_known_backing_stores(id, keys) {
        return Some(value);
    }
    None
}

fn find_receipt_in_known_backing_stores(id: &str, keys: &[&str]) -> Option<Value> {
    let root = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let candidates = [
        root.join("core/state/runtime_admission_receipts.json"),
        root.join("data/prometheus/runtime_admission_shed_receipts.jsonl"),
    ];

    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        if candidate.extension().map(|e| e == "jsonl").unwrap_or(false) {
            if let Ok(raw) = std::fs::read_to_string(&candidate) {
                for line in raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                    if let Ok(value) = serde_json::from_str::<Value>(line) {
                        if id_matches_any(&value, id, keys) {
                            return Some(value);
                        }
                    }
                }
            }
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&candidate) {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                let arr = data
                    .as_array()
                    .or_else(|| data.get("receipts").and_then(|v| v.as_array()))
                    .or_else(|| data.get("recent_receipts").and_then(|v| v.as_array()))
                    .cloned()
                    .unwrap_or_default();
                for rec in arr {
                    if id_matches_any(&rec, id, keys) {
                        return Some(rec);
                    }
                }
            }
        }
    }

    None
}

fn load_eval_run_record(task_id: &str) -> Value {
    let root = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let candidate_paths = [
        root.join("core/state/queue_summary.json"),
        root.join("core/state/project_task_executor.json"),
        root.join("core/state/queue_active.json"),
    ];

    if let Some(path) = candidate_paths.iter().find(|p| p.exists()) {
        if let Ok(raw) = std::fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                if let Some(entry) = find_task_id(&data, task_id) {
                    return json!({
                        "contract": "arda.eval.run.v1",
                        "task_id": task_id,
                        "status": "found",
                        "source": path.to_string_lossy().to_string(),
                        "record": entry,
                    });
                }
            }
        }
    }

    json!({
        "contract": "arda.eval.run.v1",
        "task_id": task_id,
        "status": "queued",
        "note": "no persistent eval record found yet",
    })
}

fn find_task_id(data: &Value, task_id: &str) -> Option<Value> {
    find_task_id_by_keys(data, &["task_id", "id"], task_id)
}

fn find_task_id_by_keys(data: &Value, keys: &[&str], task_id: &str) -> Option<Value> {
    if let Some(map) = data.as_object() {
        for key in keys {
            if map.get(*key).and_then(|v| v.as_str()) == Some(task_id) {
                return Some(data.clone());
            }
        }
        for (_, child) in map {
            if let Some(found) = find_task_id_by_keys(child, keys, task_id) {
                return Some(found);
            }
        }
    } else if let Some(arr) = data.as_array() {
        for item in arr {
            if let Some(found) = find_task_id_by_keys(item, keys, task_id) {
                return Some(found);
            }
        }
    }
    None
}

fn resolve_tool_manifest(agent_id: &str) -> Value {
    let root = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let personalities_path = root.join("core/state/agent_personalities.json");
    let framework_path = root.join("core/state/agent_framework_alignment.json");

    let personalities = read_json_file(&personalities_path);
    let framework = read_json_file(&framework_path);

    let mut record = json!({
        "contract": "arda.tool.manifest.v1",
        "agent_id": agent_id,
        "status": "not_found",
    });

    if let Some(profile) = personalities.as_object().and_then(|obj| obj.get(agent_id)) {
        record = json!({
            "contract": "arda.tool.manifest.v1",
            "agent_id": agent_id,
            "status": "found",
            "personality": profile,
            "framework_alignment": framework,
        });
    } else if !framework.is_null() {
        record = json!({
            "contract": "arda.tool.manifest.v1",
            "agent_id": agent_id,
            "status": "partial",
            "framework_alignment": framework,
            "note": "personality record missing; framework alignment present",
        });
    }

    record
}

fn read_json_file(path: &Path) -> Value {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn not_found(id: &str) -> Value {
    json!({"contract":"arda.registry.not_found.v1","id":id,"status":"not_found"})
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let from = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    out.push(from.join("core/state/contract_registry.json"));
    out
}

fn id_matches_any(value: &Value, id: &str, keys: &[&str]) -> bool {
    for key in keys {
        if value
            .get(*key)
            .and_then(|v| v.as_str())
            .map(|v| v == id)
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn track_by_id(registry: &Value, track_id: &str) -> Option<Value> {
    let tracks = registry.get("tracks")?.as_array()?;
    let track = tracks
        .iter()
        .find(|t| t.get("track_id").and_then(|v| v.as_str()) == Some(track_id))?;
    Some(track.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plutus_export_command() {
        let cli = Cli::try_parse_from(["arda-cli", "plutus", "export", "--json"])
            .expect("parse plutus export");
        assert!(matches!(
            cli.command,
            Commands::Plutus {
                command: PlutusCommands::Export { json: true, .. }
            }
        ));
    }

    #[test]
    fn parses_prometheus_status_command() {
        let cli = Cli::try_parse_from(["arda-cli", "prometheus", "status"])
            .expect("parse prometheus status");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Status { .. }
            }
        ));
    }

    #[test]
    fn parses_athena_status_command() {
        let cli = Cli::try_parse_from([
            "arda-cli",
            "athena-status",
            "--root",
            "/tmp/athena-status-test",
        ])
        .expect("parse athena status");
        assert!(matches!(
            cli.command,
            Commands::AthenaStatus { root }
                if root == Some(PathBuf::from("/tmp/athena-status-test"))
        ));
    }

    #[test]
    fn parses_prometheus_autopilot_once_command() {
        let cli = Cli::try_parse_from([
            "arda-cli",
            "prometheus",
            "autopilot",
            "once",
            "--read-only",
            "--state-output",
            "/tmp/aule-state.json",
        ])
        .expect("parse autopilot once");
        let Commands::Prometheus {
            command:
                PrometheusCommands::Autopilot {
                    command:
                        AutopilotCommands::Once {
                            read_only,
                            state_output,
                            ..
                        },
                },
        } = cli.command
        else {
            panic!("expected prometheus autopilot once command");
        };
        assert!(read_only);
        assert_eq!(state_output, Some(PathBuf::from("/tmp/aule-state.json")));
    }

    #[test]
    fn atomic_snapshot_creates_parent_and_replaces_destination() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("nested/autopilot.state.json");
        write_atomic_snapshot(&path, br#"{"cycle":1}"#).expect("first snapshot");
        write_atomic_snapshot(&path, br#"{"cycle":2}"#).expect("replacement snapshot");
        assert_eq!(
            std::fs::read(&path).expect("read snapshot"),
            br#"{"cycle":2}"#
        );
        assert!(!path
            .with_extension(format!("tmp-{}", std::process::id()))
            .exists());
    }

    #[test]
    fn parses_prometheus_execution_intents_command() {
        let cli = Cli::try_parse_from([
            "arda-cli",
            "prometheus",
            "execution-intents",
            "--include-terminal",
        ])
        .expect("parse execution intents");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::ExecutionIntents {
                    include_terminal: true,
                    ..
                }
            }
        ));
    }

    #[test]
    fn plutus_export_reads_snapshot_and_event_count() {
        let temp = tempfile::tempdir().expect("tempdir");
        let status_path = temp.path().join("runtime_status.json");
        std::fs::write(
            &status_path,
            serde_json::to_vec(&json!({
                "schema_version": "arda.plutus.runtime.v2",
                "economics": {"total_spend": 2.0},
            }))
            .expect("snapshot json"),
        )
        .expect("snapshot");
        std::fs::write(
            temp.path().join("runtime_events.jsonl"),
            "{\"action\":\"one\"}\n{\"action\":\"two\"}\n",
        )
        .expect("events");

        let export = load_plutus_export(Some(status_path)).expect("export");
        assert_eq!(export["contract"], "arda.plutus.export.v1");
        assert_eq!(export["found"], true);
        assert_eq!(export["events_total"], 2);
        assert_eq!(export["snapshot"]["economics"]["total_spend"], 2.0);
    }

    #[test]
    fn plutus_export_reports_missing_state_without_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let export = load_plutus_export(Some(temp.path().join("missing.json"))).expect("export");
        assert_eq!(export["found"], false);
        assert_eq!(export["missing"], true);
    }
}
